//! `nt-object`: NT Object Manager + HandleTable.
//!
//! Responsabilidade: lifetime de objetos kernel, refcount, tabela de
//! handles geracionais. NÃO executa I/O (isso é `nt-file` etc.).
//! Modelo completo em `docs/subsystems/handles.md`.

use std::sync::{Arc, Mutex};
use winabi::{NtStatus, WindowsHandle};

/// Tipo de objeto NT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    File,
    Event,
    Mutex,
    Semaphore,
    Process,
    Thread,
    Section,
    Timer,
    RegistryKey,
    Token,
}

impl ObjectType {
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Event => "Event",
            Self::Mutex => "Mutant",
            Self::Semaphore => "Semaphore",
            Self::Process => "Process",
            Self::Thread => "Thread",
            Self::Section => "Section",
            Self::Timer => "Timer",
            Self::RegistryKey => "Key",
            Self::Token => "Token",
        }
    }
}

/// Payload por tipo. Variantes chegam com o milestone dono
/// (ex.: `Event` real em v0.3); sem placeholders especulativos.
#[derive(Debug)]
pub enum ObjectPayload {
    File(FileObject),
}

#[derive(Debug)]
pub struct FileObject {
    /// fd Linux subjacente (pode ser 1/2 para console).
    pub fd: i32,
    pub path: String,
    pub writable: bool,
    /// Leitura permitida (GENERIC_READ ou access 0). Console out/err: false.
    pub readable: bool,
    /// `true` se o runtime é dono e deve `close_fd` (arquivos criados);
    /// `false` para 1/2 do console (são do host, nunca fechar).
    pub owns_fd: bool,
}

/// Objeto kernel com refcount (`Arc`) + nome opcional + rights.
#[derive(Debug)]
pub struct KernelObject {
    pub typ: ObjectType,
    pub name: Option<String>,
    pub rights: u32,
    pub payload: Mutex<ObjectPayload>,
}

impl KernelObject {
    pub fn type_name(&self) -> &'static str {
        self.typ.name()
    }
}

/// Entrada da tabela: geração + objeto.
struct Entry {
    generation: u32,
    object: Option<Arc<KernelObject>>,
}

/// Tabela de handles com IDs geracionais.
///
/// Encoding (documentado em `docs/subsystems/handles.md`):
/// `handle = (generation << 32) | index`, com `index >= 4` e múltiplo de 4
/// (como handles user-mode reais). Geração 0 = nunca alocado; incremento
/// a cada reuso impede que handle stale ressuscite silenciosamente.
pub struct HandleTable {
    inner: Mutex<TableInner>,
}

struct TableInner {
    entries: Vec<Entry>,
    free: Vec<u32>,
}

impl HandleTable {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TableInner {
                entries: Vec::new(),
                free: Vec::new(),
            }),
        }
    }

    fn encode(index: u32, generation: u32) -> WindowsHandle {
        WindowsHandle(((generation as u64) << 32) | (index as u64))
    }

    fn decode(handle: WindowsHandle) -> Option<(u32, u32)> {
        if handle.0 == 0 || handle.0 == u64::MAX {
            return None;
        }
        let index = (handle.0 & 0xFFFF_FFFF) as u32;
        let generation = (handle.0 >> 32) as u32;
        if index % 4 != 0 || index < 4 || generation == 0 {
            return None;
        }
        Some((index, generation))
    }

    pub fn insert(&self, object: Arc<KernelObject>) -> WindowsHandle {
        let mut t = self.inner.lock().unwrap();
        if let Some(index) = t.free.pop() {
            let slot = (index / 4 - 1) as usize;
            let e = &mut t.entries[slot];
            e.generation = e.generation.wrapping_add(1).max(1);
            e.object = Some(object);
            Self::encode(index, e.generation)
        } else {
            let slot = t.entries.len() as u32;
            let index = (slot + 1) * 4;
            t.entries.push(Entry {
                generation: 1,
                object: Some(object),
            });
            Self::encode(index, 1)
        }
    }

    pub fn lookup(&self, handle: WindowsHandle) -> Result<Arc<KernelObject>, NtStatus> {
        let (index, generation) = Self::decode(handle).ok_or(NtStatus::INVALID_HANDLE)?;
        let t = self.inner.lock().unwrap();
        let slot = (index / 4 - 1) as usize;
        let e = t.entries.get(slot).ok_or(NtStatus::INVALID_HANDLE)?;
        match &e.object {
            Some(o) if e.generation == generation => Ok(o.clone()),
            _ => Err(NtStatus::INVALID_HANDLE),
        }
    }

    pub fn close(&self, handle: WindowsHandle) -> Result<(), NtStatus> {
        let (index, generation) = Self::decode(handle).ok_or(NtStatus::INVALID_HANDLE)?;
        let mut t = self.inner.lock().unwrap();
        let slot = (index / 4 - 1) as usize;
        let e = t.entries.get_mut(slot).ok_or(NtStatus::INVALID_HANDLE)?;
        match &e.object {
            Some(_) if e.generation == generation => {
                e.object = None;
                // Geração incrementada no próximo `insert` (reuso seguro).
                t.free.push(index);
                Ok(())
            }
            _ => Err(NtStatus::INVALID_HANDLE),
        }
    }

    /// Registra os pseudo-handles de console (stdout/stderr) como objetos
    /// `File` reais. Retorna os handles para `GetStdHandle`.
    pub fn init_console(&self) -> (WindowsHandle, WindowsHandle) {
        let stdout = Arc::new(KernelObject {
            typ: ObjectType::File,
            name: None,
            rights: 0x120089,
            payload: Mutex::new(ObjectPayload::File(FileObject {
                fd: 1,
                path: "\\Device\\Console\\Stdout".into(),
                writable: true,
                readable: false,
                owns_fd: false,
            })),
        });
        let stderr = Arc::new(KernelObject {
            typ: ObjectType::File,
            name: None,
            rights: 0x120089,
            payload: Mutex::new(ObjectPayload::File(FileObject {
                fd: 2,
                path: "\\Device\\Console\\Stderr".into(),
                writable: true,
                readable: false,
                owns_fd: false,
            })),
        });
        (self.insert(stdout), self.insert(stderr))
    }
}

impl Default for HandleTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_obj(fd: i32) -> Arc<KernelObject> {
        Arc::new(KernelObject {
            typ: ObjectType::File,
            name: None,
            rights: 0,
            payload: Mutex::new(ObjectPayload::File(FileObject {
                fd,
                path: "test".into(),
                writable: true,
                readable: false,
                owns_fd: false,
            })),
        })
    }

    #[test]
    fn insert_lookup_close() {
        let t = HandleTable::new();
        let h = t.insert(file_obj(1));
        assert!(t.lookup(h).is_ok());
        t.close(h).unwrap();
        assert!(matches!(t.lookup(h), Err(NtStatus::INVALID_HANDLE)));
    }

    #[test]
    fn stale_generation_rejected() {
        let t = HandleTable::new();
        let h1 = t.insert(file_obj(1));
        t.close(h1).unwrap();
        let h2 = t.insert(file_obj(2));
        assert_ne!(h1, h2); // geração diferente
        assert!(t.lookup(h1).is_err());
        assert!(t.lookup(h2).is_ok());
    }

    #[test]
    fn null_invalid_rejected() {
        let t = HandleTable::new();
        assert!(t.lookup(WindowsHandle::NULL).is_err());
        assert!(t.lookup(WindowsHandle::INVALID).is_err());
    }
}
