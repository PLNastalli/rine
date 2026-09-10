//! Modelos de estado esperado (simples e óbvios por construção).
//!
//! Um modelo NÃO é segunda implementação: é a especificação executável
//! mínima contra a qual o Rine é comparado. Divergência modelo×Rine =
//! investigar (bug no Rine OU no modelo — ambos possíveis, documentar).
//! Modelos nunca tocam syscalls/host: estado puro + `DriveMap` para paths
//! (tradução testada à parte; aqui o foco é ciclo de vida).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// ---------------------------------------------------------------------------
// Arquivos (alvo `fileops`): Create/Write/Read/Close sobre FS virtual.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RawOp {
    Create {
        path: String,
        access: u32,
        disp: u32,
    },
    Write {
        len: u32,
    },
    Read {
        len: u32,
    },
    Close,
}

/// Expectativa anotada por passo (o driver guest a embute no exe).
/// Semântica por tipo de op (documentada uma vez, aqui):
/// - Create → `valid` (handle utilizável ou INVALID).
/// - Write → `ok` (TRUE) + `count` (bytes gravados; 0 com `ok=false`).
/// - Read → `ok` (TRUE) + `count` + `verify` (buf == MSG).
/// - Close → `ok`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileExpect {
    pub valid: bool,
    pub ok: bool,
    pub count: u32,
    pub verify: bool,
}

impl FileExpect {
    fn create(valid: bool) -> Self {
        Self {
            valid,
            ok: true,
            count: 0,
            verify: false,
        }
    }
    fn count_ok(ok: bool, count: u32, verify: bool) -> Self {
        Self {
            valid: true,
            ok,
            count,
            verify,
        }
    }
    fn close(ok: bool) -> Self {
        Self {
            valid: true,
            ok,
            count: 0,
            verify: false,
        }
    }
}

#[derive(Debug, Clone)]
struct OpenFile {
    path: String,
    readable: bool,
    writable: bool,
    pos: usize,
}

#[derive(Debug, Default)]
pub struct FileModel {
    files: BTreeMap<String, Vec<u8>>,
    /// Diretórios existentes (chaves host). Só raízes + ancestrais de setup:
    /// nada cria diretórios em fileops, então aninhado falha — como no disco.
    dirs: std::collections::BTreeSet<String>,
    open: Option<OpenFile>,
}

pub const GENERIC_READ: u32 = 0x8000_0000;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const MSG: &[u8] = b"Hello File\n";

impl FileModel {
    /// `setup`: arquivos pré-existentes `{nome: bytes}` (relativos ao drive).
    pub fn new(fsys: &nt_file::DriveMap, setup: &[(String, Vec<u8>)]) -> Self {
        let mut m = Self::default();
        for r in fsys.roots() {
            m.dirs.insert(r.to_string_lossy().into_owned());
        }
        // Setup só aceita paths traduzíveis (como o runtime faria);
        // ancestrais do setup passam a existir (como no disco).
        for (name, bytes) in setup {
            if let Ok(key) = Self::key(fsys, name) {
                let mut cur = key.clone();
                while let Some(parent) = std::path::Path::new(&cur).parent() {
                    let s = parent.to_string_lossy().into_owned();
                    if s.is_empty() || !m.dirs.insert(s.clone()) {
                        break;
                    }
                    cur = s;
                }
                m.files.insert(key, bytes.clone());
            }
        }
        m
    }

    /// Snapshot `{host_path: bytes}` para `Expectation` do comparador.
    pub fn files_snapshot(&self) -> Vec<(String, Vec<u8>)> {
        self.files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Simula `ops` contra `fsys`; retorna expectativas por passo.
    /// Espelha `nt_file`/`CreateOptions` de forma legível (não otimizada).
    /// Chave = path do HOST traduzido (o que o disco realmente vê).
    pub fn simulate(&mut self, fsys: &nt_file::DriveMap, ops: &[RawOp]) -> Vec<FileExpect> {
        let mut out = Vec::new();
        for op in ops {
            out.push(match op {
                RawOp::Create { path, access, disp } => {
                    FileExpect::create(self.do_create(fsys, path, *access, *disp))
                }
                RawOp::Write { len } => {
                    let (ok, w) = self.do_write(*len as usize);
                    FileExpect::count_ok(ok, w, false)
                }
                RawOp::Read { len } => {
                    let (ok, c, v) = self.do_read(*len as usize);
                    FileExpect::count_ok(ok, c, v)
                }
                RawOp::Close => FileExpect::close(self.do_close()),
            });
        }
        out
    }

    fn key(fsys: &nt_file::DriveMap, path: &str) -> Result<String, ()> {
        fsys.translate(path)
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|_| ())
    }

    /// NOTA DE PROTOCOLO (honestidade de camada): o driver guest guarda UM
    /// handle (`rbx`); `Create` falho sobrescreve `rbx` com INVALID,
    /// destruindo o handle anterior — apps Windows reais usariam variáveis
    /// separadas. O modelo espelha o PROTOCOLO DO HARNESS (o sistema sob
    /// teste aqui), não o Windows multihandle; driver v2 cobrirá N handles.
    fn do_create(&mut self, fsys: &nt_file::DriveMap, path: &str, access: u32, disp: u32) -> bool {
        let ok = self.try_create(fsys, path, access, disp);
        if !ok {
            self.open = None;
        }
        ok
    }

    fn try_create(&mut self, fsys: &nt_file::DriveMap, path: &str, access: u32, disp: u32) -> bool {
        if !matches!(disp, 1..=5) {
            return false;
        }
        // Truncar sem GENERIC_WRITE: negado (espelha `nt_file::create_file`).
        if disp == 5 && (access & GENERIC_WRITE) == 0 {
            return false;
        }
        // Separador final = path de DIRETÓRIO: CreateFile de arquivo falha
        // (no disco, `open("x/")` falha) — como no Windows para arquivos.
        if path.ends_with(['\\', '/']) {
            return false;
        }
        let Ok(key) = Self::key(fsys, path) else {
            return false; // drive ausente, UNC, `X:rel`, ...
        };
        // Pais precisam existir (nada cria diretórios em fileops).
        // Basta o imediato: cadeias acima dele foram validadas ao entrar
        // em `dirs` (raízes do DriveMap ou ancestrais de setup).
        let immediate = std::path::Path::new(&key)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !immediate.is_empty() && !self.dirs.contains(&immediate) {
            return false;
        }
        let exists = self.files.contains_key(&key);
        match disp {
            1 => {
                if exists {
                    return false;
                }
                self.files.insert(key.clone(), Vec::new());
            }
            2 => {
                self.files.insert(key.clone(), Vec::new());
            }
            3 => {
                if !exists {
                    return false;
                }
            }
            4 => {
                self.files.entry(key.clone()).or_default();
            }
            5 => {
                if !exists {
                    return false;
                }
                self.files.insert(key.clone(), Vec::new());
            }
            _ => return false,
        }
        let readable = access & GENERIC_READ != 0 || access == 0;
        self.open = Some(OpenFile {
            path: key,
            readable,
            writable: access & GENERIC_WRITE != 0,
            pos: 0,
        });
        true
    }

    /// Retorna `(ok, bytes)`. Sem handle gravável: `(false, 0)` — o guest
    /// real recebe FALSE + `*written = 0` (ver `kernel32!WriteFile`).
    fn do_write(&mut self, len: usize) -> (bool, u32) {
        let o = match &mut self.open {
            Some(o) if o.writable => o,
            _ => return (false, 0),
        };
        let take = len.min(MSG.len());
        let data = self.files.get_mut(&o.path).expect("aberto existe");
        if o.pos + take > data.len() {
            data.resize(o.pos + take, 0);
        }
        data[o.pos..o.pos + take].copy_from_slice(&MSG[..take]);
        o.pos += take;
        (true, take as u32)
    }

    /// Retorna `(ok, count, verify)`. Sem handle legível: `(false, 0, false)`.
    /// `verify` só quando o trecho lido == MSG (setup com outro conteúdo
    /// não verifica — honestidade contra falso-positivo).
    fn do_read(&mut self, len: usize) -> (bool, u32, bool) {
        let o = match &mut self.open {
            Some(o) if o.readable => o,
            _ => return (false, 0, false),
        };
        let data = self.files.get(&o.path).expect("aberto existe");
        let avail = data.len().saturating_sub(o.pos);
        let take = (len.min(32)).min(avail) as u32;
        let v = take > 0 && data[o.pos..o.pos + take as usize] == MSG[..take as usize];
        o.pos += take as usize;
        (true, take, v)
    }

    fn do_close(&mut self) -> bool {
        if self.open.is_some() {
            self.open = None;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Handles (alvo `handles`): insert/lookup/close simbólicos.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandleOp {
    Insert,
    Lookup { id: usize },
    Close { id: usize },
}

/// Modelo: ids simbólicos → {vivo}. `Insert` aloca o menor id livre? Não:
//  id do passo = ordem de criação (lista). O executor real mapeia id→handle.
/// Rejeição do modelo ("o Rine deve recusar"). Tipo próprio em vez de
/// `()` para satisfazer `result_unit_err` com honestidade: rejeitar é um
/// veredito de primeira classe aqui, não ausência de erro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reject;

#[derive(Debug, Default)]
pub struct HandleModel {
    alive: HashMap<usize, bool>,
    next: usize,
}

impl HandleModel {
    /// Retorna `Ok` se o Rine deve aceitar, `Err(Reject)` se deve recusar.
    pub fn step(&mut self, op: &HandleOp) -> Result<Option<usize>, Reject> {
        match op {
            HandleOp::Insert => {
                let id = self.next;
                self.next += 1;
                self.alive.insert(id, true);
                Ok(Some(id))
            }
            HandleOp::Lookup { id } => {
                if self.alive.get(id).copied().unwrap_or(false) {
                    Ok(None)
                } else {
                    Err(Reject)
                }
            }
            HandleOp::Close { id } => {
                if self.alive.get(id).copied().unwrap_or(false) {
                    self.alive.insert(*id, false);
                    Ok(None)
                } else {
                    Err(Reject)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Memória (alvo `memory`): reserve/commit/protect/release simbólicos.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemOp {
    /// `protect`: ÍNDICE em `PROTECTS` (0..6). Nível gerente só recebe
    /// `PageProtect` tipado (bits inválidos morrem na façade — ver teste
    /// `kernel32`); índice mantém o diferencial exato, sem classe fantasma.
    Reserve {
        id: usize,
        size: usize,
        protect: u8,
    },
    /// `Commit` carrega `protect` (índice em `PROTECTS`) como o
    /// `VirtualAlloc(MEM_COMMIT, flProtect)` real: o commit define a
    /// proteção atual; `alloc_protect` permanece o da reserva.
    Commit {
        id: usize,
        protect: u8,
    },
    Protect {
        id: usize,
        protect: u8,
    },
    /// Consulta: endereço = base + offset (executor dobra em 4096 para ficar
    /// dentro da primeira página; id desconhecido consulta 0 = nunca mapeado).
    /// Sem efeitos em nenhum lado — puro em ambos.
    Query {
        id: usize,
        offset: u64,
    },
    Release {
        id: usize,
    },
}

/// Os 6 `PAGE_*` básicos (sem GUARD/NOCACHE — quirk QUI-0001, v0.3).
pub const PROTECTS: [u32; 6] = [0x01, 0x02, 0x04, 0x10, 0x20, 0x40];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionState {
    Reserved,
    Committed,
}

#[derive(Debug, Default)]
pub struct MemoryModel {
    regions: HashMap<usize, (usize, RegionState, u32, u32)>,
}

impl MemoryModel {
    /// `Ok` = Rine deve aceitar. Normaliza tamanhos para página (4096).
    pub fn step(&mut self, op: &MemOp) -> Result<(), Reject> {
        match op {
            MemOp::Reserve { id, size, protect } => {
                if *size == 0
                    || self.regions.contains_key(id)
                    || (*protect as usize) >= PROTECTS.len()
                {
                    return Err(Reject);
                }
                let len = (*size + 4095) & !4095;
                let bits = PROTECTS[*protect as usize];
                self.regions
                    .insert(*id, (len, RegionState::Reserved, bits, bits));
                Ok(())
            }
            MemOp::Commit { id, protect } => match self.regions.get_mut(id) {
                Some((_, s @ RegionState::Reserved, p, _))
                    if (*protect as usize) < PROTECTS.len() =>
                {
                    *s = RegionState::Committed;
                    *p = PROTECTS[*protect as usize];
                    Ok(())
                }
                _ => Err(Reject),
            },
            MemOp::Protect { id, protect } => match self.regions.get_mut(id) {
                Some((_, RegionState::Committed, p, _)) if (*protect as usize) < PROTECTS.len() => {
                    *p = PROTECTS[*protect as usize];
                    Ok(())
                }
                _ => Err(Reject),
            },
            MemOp::Query { id, .. } => {
                // Consulta pura (ver `query_desc`); Ok = id conhecido.
                if self.regions.contains_key(id) {
                    Ok(())
                } else {
                    Err(Reject)
                }
            }
            MemOp::Release { id } => {
                if self.regions.remove(id).is_some() {
                    Ok(())
                } else {
                    Err(Reject)
                }
            }
        }
    }

    /// Descritor esperado para `Query(id)`: `(len, state, protect, alloc)`.
    /// `None` = id desconhecido (ambos os lados devem achar nada).
    pub fn query_desc(&self, id: usize) -> Option<(usize, RegionState, u32, u32)> {
        self.regions.get(&id).copied()
    }

    pub fn live_count(&self) -> usize {
        self.regions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_model_lifecycle() {
        let dir = std::env::temp_dir().join(format!("rine-model-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut dm = HashMap::new();
        dm.insert('C', dir.to_string_lossy().into_owned());
        let fsys = nt_file::DriveMap::new(dm, None);
        let mut m = FileModel::new(&fsys, &[]);
        let ops = vec![
            RawOp::Create {
                path: "C:\\a.txt".into(),
                access: GENERIC_WRITE,
                disp: 2,
            },
            RawOp::Write { len: 11 },
            RawOp::Close,
            RawOp::Create {
                path: "C:\\a.txt".into(),
                access: GENERIC_READ,
                disp: 3,
            },
            RawOp::Read { len: 32 },
            RawOp::Close,
        ];
        let e = m.simulate(&fsys, &ops);
        assert!(e[0].valid);
        assert_eq!(e[1], FileExpect::count_ok(true, 11, false));
        assert!(e[2].ok);
        assert!(e[3].valid);
        assert_eq!(e[4], FileExpect::count_ok(true, 11, true));
        assert!(e[5].ok);
        let key = fsys
            .translate("C:\\a.txt")
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(m.files[&key], b"Hello File\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_model_nested_and_trailing_fail() {
        let dir = std::env::temp_dir().join(format!("rine-model-n-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut dm = HashMap::new();
        dm.insert('C', dir.to_string_lossy().into_owned());
        let fsys = nt_file::DriveMap::new(dm, None);
        let mut m = FileModel::new(&fsys, &[]);
        // Pais inexistentes e trailing separator: inválidos como no disco.
        let e = m.simulate(
            &fsys,
            &[
                RawOp::Create {
                    path: "C:\\d1\\d2\\f.txt".into(),
                    access: GENERIC_WRITE,
                    disp: 2,
                },
                RawOp::Create {
                    path: "C:\\t.txt\\".into(),
                    access: GENERIC_WRITE,
                    disp: 2,
                },
            ],
        );
        assert!(!e[0].valid && !e[1].valid);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Protocolo do harness: Create falho limpa o handle corrente (rbx=INVALID).
    #[test]
    fn failed_create_clears_handle() {
        let dir = std::env::temp_dir().join(format!("rine-model-c-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut dm = HashMap::new();
        dm.insert('C', dir.to_string_lossy().into_owned());
        let fsys = nt_file::DriveMap::new(dm, None);
        let mut m = FileModel::new(&fsys, &[]);
        let e = m.simulate(
            &fsys,
            &[
                RawOp::Create {
                    path: "C:\\a.txt".into(),
                    access: GENERIC_WRITE,
                    disp: 2,
                },
                RawOp::Create {
                    path: "Z:\\x".into(),
                    access: GENERIC_WRITE,
                    disp: 2,
                },
                RawOp::Write { len: 1 },
            ],
        );
        assert!(e[0].valid);
        assert!(!e[1].valid);
        // Handle anterior destruído pelo protocolo: write falha.
        assert_eq!(e[2], FileExpect::count_ok(false, 0, false));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_model_rejects() {
        let fsys = nt_file::DriveMap::empty();
        let mut m = FileModel::new(&fsys, &[]);
        // Sem handle: write/read/close falham (guest recebe FALSE + 0).
        let e = m.simulate(
            &fsys,
            &[
                RawOp::Write { len: 1 },
                RawOp::Read { len: 1 },
                RawOp::Close,
            ],
        );
        assert_eq!(e[0], FileExpect::count_ok(false, 0, false));
        assert_eq!(e[1], FileExpect::count_ok(false, 0, false));
        assert!(!e[2].ok);
        // Disp inválida + drive ausente.
        let e = m.simulate(
            &fsys,
            &[
                RawOp::Create {
                    path: "Z:\\x".into(),
                    access: GENERIC_READ,
                    disp: 3,
                },
                RawOp::Create {
                    path: "a".into(),
                    access: GENERIC_READ,
                    disp: 9,
                },
            ],
        );
        assert!(!e[0].valid && !e[1].valid);
    }

    #[test]
    fn handle_model_stale() {
        let mut m = HandleModel::default();
        assert_eq!(m.step(&HandleOp::Insert), Ok(Some(0)));
        assert!(m.step(&HandleOp::Lookup { id: 0 }).is_ok());
        assert!(m.step(&HandleOp::Close { id: 0 }).is_ok());
        assert!(m.step(&HandleOp::Lookup { id: 0 }).is_err());
        assert!(m.step(&HandleOp::Close { id: 0 }).is_err());
        assert!(m.step(&HandleOp::Lookup { id: 9 }).is_err());
    }

    #[test]
    fn memory_model_lifecycle() {
        let mut m = MemoryModel::default();
        assert!(m
            .step(&MemOp::Reserve {
                id: 0,
                size: 5000,
                protect: 2
            })
            .is_ok());
        assert!(m.step(&MemOp::Protect { id: 0, protect: 1 }).is_err()); // reserved!
        assert!(m.step(&MemOp::Commit { id: 0, protect: 2 }).is_ok());
        assert!(m.step(&MemOp::Protect { id: 0, protect: 1 }).is_ok());
        assert!(m.step(&MemOp::Commit { id: 0, protect: 2 }).is_err()); // já committed
        assert!(m.step(&MemOp::Release { id: 0 }).is_ok());
        assert!(m.step(&MemOp::Release { id: 0 }).is_err());
        assert!(m
            .step(&MemOp::Reserve {
                id: 0,
                size: 0,
                protect: 2
            })
            .is_err());
        assert!(m
            .step(&MemOp::Reserve {
                id: 1,
                size: 99,
                protect: 9
            })
            .is_err());
        assert_eq!(m.live_count(), 0);
    }
}
