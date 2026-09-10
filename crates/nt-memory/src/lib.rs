//! `nt-memory`: gerenciador de memória virtual NT sobre `mmap`.
//!
//! Responsabilidade: modelo explícito de regiões Windows
//! (`MEM_RESERVE`/`COMMIT`/`DECOMMIT`/`RELEASE` + `PAGE_*`).
//! `VirtualAlloc` NÃO é `mmap` direto: é transição de estado de região,
//! com `mmap`/`mprotect` como efeito. Diagrama em `docs/subsystems/memory.md`.

use std::collections::BTreeMap;
use std::sync::Mutex;
use thiserror::Error;
use winabi::{AllocType, NtStatus, PageProtect};

#[derive(Debug, Error)]
pub enum MemError {
    #[error("invalid parameter")]
    InvalidParameter,
    #[error("no memory")]
    NoMemory,
    #[error("conflict: region already reserved")]
    Conflict,
    #[error("host error: {0}")]
    Host(#[from] host_linux::HostError),
}

impl From<MemError> for NtStatus {
    fn from(e: MemError) -> NtStatus {
        match e {
            MemError::InvalidParameter => NtStatus::INVALID_PARAMETER,
            MemError::NoMemory => NtStatus::NO_MEMORY,
            MemError::Conflict => NtStatus::INVALID_PARAMETER,
            MemError::Host(_) => NtStatus::NO_MEMORY,
        }
    }
}

/// Estado de uma região (máquina de estados documentada).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionState {
    /// Reservada (espaço de endereços, sem acesso, sem commit).
    Reserved,
    /// Committed (acessível com `protect`).
    Committed,
}

#[derive(Debug, Clone)]
struct Region {
    base: usize,
    len: usize,
    state: RegionState,
    protect: PageProtect,
}

/// Gerenciador de regiões com lock único (v0.1 single-threaded por processo
/// emulado; granularidade fina no milestone v0.4).
pub struct MemoryManager {
    inner: Mutex<Inner>,
}

struct Inner {
    regions: BTreeMap<usize, Region>,
    page: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                regions: BTreeMap::new(),
                page: host_linux::page_size(),
            }),
        }
    }

    /// `NtAllocateVirtualMemory` simplificado: RESERVE (+COMMIT opcional).
    /// Retorna o endereço base. Tamanho arredondado para página.
    pub fn allocate(
        &self,
        hint: usize,
        size: usize,
        alloc: AllocType,
        protect: PageProtect,
    ) -> Result<usize, MemError> {
        if size == 0 {
            return Err(MemError::InvalidParameter);
        }
        let mut inner = self.inner.lock().unwrap();
        let page = inner.page;
        let len = (size + page - 1) & !(page - 1);

        if alloc.contains(AllocType::RELEASE) {
            return Err(MemError::InvalidParameter);
        }

        if alloc.contains(AllocType::RESERVE) {
            // Reserva: adquire do host e registra.
            let base = if hint != 0 {
                let aligned = hint & !(page - 1);
                // Checa sobreposição com regiões existentes.
                for r in inner.regions.values() {
                    if aligned < r.base + r.len && aligned + len > r.base {
                        return Err(MemError::Conflict);
                    }
                }
                match host_linux::reserve_fixed(aligned as *mut u8, len) {
                    Ok(p) => p as usize,
                    Err(_) => return Err(MemError::NoMemory),
                }
            } else {
                match host_linux::reserve_anonymous(len) {
                    Ok(p) => p as usize,
                    Err(_) => return Err(MemError::NoMemory),
                }
            };
            let state = if alloc.contains(AllocType::COMMIT) {
                RegionState::Committed
            } else {
                RegionState::Reserved
            };
            if state == RegionState::Committed {
                let prot = win_to_host(protect);
                host_linux::protect(base as *mut u8, len, prot)?;
            } else {
                host_linux::protect(base as *mut u8, len, host_linux::Prot::NONE)?;
            }
            inner.regions.insert(
                base,
                Region {
                    base,
                    len,
                    state,
                    protect,
                },
            );
            Ok(base)
        } else if alloc.contains(AllocType::COMMIT) {
            // Commit de região previamente reservada contendo `hint`.
            let key = inner
                .regions
                .range(..=hint)
                .next_back()
                .map(|(k, _)| *k)
                .ok_or(MemError::InvalidParameter)?;
            let (base, len, st) = {
                let r = inner.regions.get(&key).ok_or(MemError::InvalidParameter)?;
                (r.base, r.len, r.state)
            };
            if st == RegionState::Committed {
                return Err(MemError::InvalidParameter);
            }
            if hint != base {
                return Err(MemError::InvalidParameter); // v0.1: commit integral
            }
            let prot = win_to_host(protect);
            host_linux::protect(base as *mut u8, len, prot)?;
            let r = inner.regions.get_mut(&key).unwrap();
            r.state = RegionState::Committed;
            r.protect = protect;
            Ok(base)
        } else {
            Err(MemError::InvalidParameter)
        }
    }

    /// Libera (`RELEASE`) ou descomite (`DECOMMIT`) região.
    pub fn free(&self, base: usize, alloc: AllocType) -> Result<(), MemError> {
        let mut inner = self.inner.lock().unwrap();
        if alloc.contains(AllocType::RELEASE) {
            let r = inner
                .regions
                .remove(&base)
                .ok_or(MemError::InvalidParameter)?;
            host_linux::release(r.base as *mut u8, r.len)?;
            Ok(())
        } else if alloc.contains(AllocType::DECOMMIT) {
            let r = inner
                .regions
                .get_mut(&base)
                .ok_or(MemError::InvalidParameter)?;
            host_linux::protect(r.base as *mut u8, r.len, host_linux::Prot::NONE)?;
            r.state = RegionState::Reserved;
            Ok(())
        } else {
            Err(MemError::InvalidParameter)
        }
    }

    /// Protege região committed (`VirtualProtect`).
    pub fn protect_region(
        &self,
        base: usize,
        protect: PageProtect,
    ) -> Result<PageProtect, MemError> {
        let mut inner = self.inner.lock().unwrap();
        let r = inner
            .regions
            .get_mut(&base)
            .ok_or(MemError::InvalidParameter)?;
        if r.state != RegionState::Committed {
            return Err(MemError::InvalidParameter);
        }
        let old = r.protect;
        host_linux::protect(r.base as *mut u8, r.len, win_to_host(protect))?;
        r.protect = protect;
        Ok(old)
    }

    pub fn region_count(&self) -> usize {
        self.inner.lock().unwrap().regions.len()
    }

    /// Mapeia proteção Windows -> Linux. `NOACCESS`->NONE, qualquer EXEC->+X,
    /// escrita->+W. GUARD sem equivalente: v0.1 trata como committed normal
    /// (quirk documentado QUI-0001, teste pendente no milestone v0.3).
    pub fn debug_regions(&self) -> Vec<(usize, usize, RegionState)> {
        self.inner
            .lock()
            .unwrap()
            .regions
            .values()
            .map(|r| (r.base, r.len, r.state))
            .collect()
    }

    /// Registra mapeamento externo (ex.: imagem PE mapeada pelo `loader`).
    /// A memória já pertence ao host; o manager apenas rastreia o estado
    /// para que `VirtualQuery`-futuro e `free` sejam coerentes.
    pub fn track_external(&self, base: usize, len: usize, protect: PageProtect) {
        self.inner.lock().unwrap().regions.insert(
            base,
            Region {
                base,
                len,
                state: RegionState::Committed,
                protect,
            },
        );
    }

    pub fn untrack(&self, base: usize) {
        self.inner.lock().unwrap().regions.remove(&base);
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Mapeia características de seção PE (`IMAGE_SCN_*`) para `PAGE_*`,
/// com a mesma semântica do loader Windows (bits 29/30/31 = X/R/W).
/// Fonte única: `loader::protect_sections` e o rastreio de imagem usam esta.
pub fn section_protect(characteristics: u32) -> PageProtect {
    let x = characteristics & 0x2000_0000 != 0;
    let r = characteristics & 0x4000_0000 != 0;
    let w = characteristics & 0x8000_0000 != 0;
    match (x, r, w) {
        (true, true, true) => PageProtect::EXECUTE_READWRITE,
        (true, true, false) => PageProtect::EXECUTE_READ,
        (true, false, _) => PageProtect::EXECUTE_READWRITE,
        (false, true, true) => PageProtect::READWRITE,
        (false, true, false) => PageProtect::READONLY,
        (false, false, true) => PageProtect::READWRITE,
        (false, false, false) => PageProtect::NOACCESS,
    }
}

/// Mapeia `PAGE_*` Windows para proteção do host. `NOACCESS`→NONE;
/// `GUARD` sem equivalente vira committed normal (quirk QUI-0001).
pub fn host_protect(p: PageProtect) -> host_linux::Prot {
    win_to_host(p)
}

fn win_to_host(p: PageProtect) -> host_linux::Prot {
    if p.contains(PageProtect::NOACCESS) {
        return host_linux::Prot::NONE;
    }
    let exec = p.intersects(
        PageProtect::EXECUTE
            | PageProtect::EXECUTE_READ
            | PageProtect::EXECUTE_READWRITE
            | PageProtect::EXECUTE_WRITECOPY,
    );
    let write = p.intersects(
        PageProtect::READWRITE
            | PageProtect::WRITECOPY
            | PageProtect::EXECUTE_READWRITE
            | PageProtect::EXECUTE_WRITECOPY,
    );
    host_linux::Prot {
        read: true,
        write,
        exec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_commit_release_cycle() {
        let m = MemoryManager::new();
        let ps = host_linux::page_size();
        let base = m
            .allocate(0, ps, AllocType::RESERVE, PageProtect::READWRITE)
            .unwrap();
        // Commit integral da região reservada.
        m.allocate(base, ps, AllocType::COMMIT, PageProtect::READWRITE)
            .unwrap();
        m.free(base, AllocType::RELEASE).unwrap();
        assert_eq!(m.region_count(), 0);
    }

    #[test]
    fn reserve_commit_atomically() {
        let m = MemoryManager::new();
        let ps = host_linux::page_size();
        let base = m
            .allocate(
                0,
                ps,
                AllocType::RESERVE | AllocType::COMMIT,
                PageProtect::READWRITE,
            )
            .unwrap();
        assert_eq!(m.region_count(), 1);
        m.free(base, AllocType::RELEASE).unwrap();
    }

    #[test]
    fn zero_size_rejected() {
        let m = MemoryManager::new();
        assert!(m
            .allocate(0, 0, AllocType::RESERVE, PageProtect::READWRITE)
            .is_err());
    }

    #[test]
    fn section_protect_matches_windows_mapping() {
        assert_eq!(section_protect(0x6000_0020), PageProtect::EXECUTE_READ); // .text
        assert_eq!(section_protect(0x4000_0040), PageProtect::READONLY); // .rdata
        assert_eq!(section_protect(0xC000_0040), PageProtect::READWRITE); // .data
        assert_eq!(section_protect(0), PageProtect::NOACCESS);
    }
}
