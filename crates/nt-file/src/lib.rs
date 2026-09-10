//! `nt-file`: semântica `NtCreateFile`/Win32 sobre I/O Linux.
//!
//! Tradução de paths (`C:\` → Capsule, `\??\` → strip) em `DriveMap`;
//! abertura via `std::fs` (que emite `openat`/`openat2` no kernel moderno —
//! wrapper `openat2` direto em `host-linux` chega em v0.3 com `dirfd`/`flags`
//! explícitos). Leitura/escrita sempre via `host-linux` (fronteira auditável).

use nt_object::{FileObject, HandleTable, KernelObject, ObjectPayload, ObjectType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use winabi::{NtStatus, WindowsHandle};

/// Visão resolvida do filesystem da Capsule (snapshot construído no load;
/// ver `runtime::Emulator`). Fonte única da tradução de paths.
#[derive(Debug, Clone, Default)]
pub struct DriveMap {
    drives: HashMap<char, PathBuf>,
    current_dir: Option<PathBuf>,
}

impl DriveMap {
    pub fn new(drives: HashMap<char, String>, current_dir: Option<String>) -> Self {
        Self {
            drives: drives
                .into_iter()
                .map(|(k, v)| (k.to_ascii_uppercase(), PathBuf::from(v)))
                .collect(),
            current_dir: current_dir.map(PathBuf::from),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Raízes conhecidas (valores dos drives + current_dir): âncoras de
    /// existência para modelos (um path aninhado só abre se os pais existem).
    pub fn roots(&self) -> Vec<std::path::PathBuf> {
        let mut v: Vec<std::path::PathBuf> = self.drives.values().cloned().collect();
        if let Some(d) = &self.current_dir {
            if !v.contains(d) {
                v.push(d.clone());
            }
        }
        v
    }

    /// Traduz path Win32/NT para path do host.
    ///
    /// Regras (testadas abaixo; lacunas retornam erro, nunca chute):
    /// - prefixo `\??\` removido;
    /// - `X:\...` → `drives[X]`; letra case-insensitive; drive ausente →
    ///   `OBJECT_PATH_INVALID`;
    /// - `..` acima da raiz do drive é grampeado na raiz (Windows não deixa
    ///   escapar do drive; sem isso, `C:\..\x` fugia do sandbox — regression);
    ///   em paths RELATIVOS, `..` segue semântica Unix normal (pai do CWD);
    /// - separador FINAL além da raiz nua = path de diretório → erro;
    /// - `\\.\`, UNC (`\\srv\...`) e `X:relativo` → `NOT_IMPLEMENTED`;
    /// - relativo → `current_dir` ou CWD do host.
    pub fn translate(&self, win: &str) -> Result<PathBuf, NtStatus> {
        let mut p = win;
        if let Some(rest) = p.strip_prefix("\\\\??\\") {
            p = rest;
        } else if let Some(rest) = p.strip_prefix("\\??\\") {
            p = rest;
        }
        if p.starts_with("\\\\.\\") || (p.starts_with("\\\\") && !p.starts_with("\\\\?\\")) {
            return Err(NtStatus::NOT_IMPLEMENTED);
        }
        // Separador final além da raiz nua = path de diretório (ver doc).
        if (p.ends_with('\\') || p.ends_with('/')) && p.len() > 3 {
            return Err(NtStatus::OBJECT_PATH_INVALID);
        }
        let bytes = p.as_bytes();
        if bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
            let letter = (bytes[0] as char).to_ascii_uppercase();
            let base = self
                .drives
                .get(&letter)
                .ok_or(NtStatus::OBJECT_PATH_INVALID)?;
            let mut out = base.clone();
            for comp in p[3..].split(['\\', '/']).filter(|s| !s.is_empty()) {
                join_sandbox(&mut out, base, comp);
            }
            return Ok(out);
        }
        if bytes.len() >= 2 && bytes[1] == b':' {
            return Err(NtStatus::NOT_IMPLEMENTED); // `X:rel` drive-relative (v0.3)
        }
        let mut out = match &self.current_dir {
            Some(d) => d.clone(),
            None => std::env::current_dir().map_err(|_| NtStatus::OBJECT_PATH_INVALID)?,
        };
        for comp in p.split(['\\', '/']).filter(|s| !s.is_empty()) {
            out.push(comp);
        }
        Ok(out)
    }
}

/// Opções de abertura (espelho mínimo de `CreateFile`).
pub struct CreateOptions {
    pub read: bool,
    pub write: bool,
    /// 1=CREATE_NEW 2=CREATE_ALWAYS 3=OPEN_EXISTING 4=OPEN_ALWAYS 5=TRUNCATE_EXISTING.
    pub disposition: u32,
}

impl CreateOptions {
    /// Mapeia `(GENERIC_READ/WRITE, disposition)` Win32. `access==0`
    /// (query-only) abre read-only — aproximação documentada v0.2.
    pub fn from_win32(access: u32, disposition: u32) -> Result<Self, NtStatus> {
        if !matches!(disposition, 1..=5) {
            return Err(NtStatus::INVALID_PARAMETER);
        }
        let read = access & 0x8000_0000 != 0;
        let write = access & 0x4000_0000 != 0;
        Ok(Self {
            read: read || (!read && !write),
            write,
            disposition,
        })
    }
}

/// Cria/abre arquivo do host e registra como objeto `File` (fd possuído).
pub fn create_file(
    table: &HandleTable,
    fsys: &DriveMap,
    win_path: &str,
    opts: &CreateOptions,
) -> Result<WindowsHandle, NtStatus> {
    let path = fsys.translate(win_path)?;
    // access==0 abre read-only (aproximação); mas O_RDONLY|O_CREAT é EINVAL
    // no Linux, e o Windows permite criar com handle sem acesso. Two-step:
    // garante existência com escrita e reabre read-only. Achado pela
    // campanha `fileops` (exit 10 em CREATE_NEW+access 0); regression abaixo.
    if !opts.write && matches!(opts.disposition, 1 | 2 | 4) {
        let mut c = std::fs::OpenOptions::new();
        c.write(true);
        match opts.disposition {
            1 => {
                c.create_new(true);
            }
            2 => {
                c.create(true).truncate(true);
            }
            _ => {
                c.create(true);
            } // 4
        }
        c.open(&path)
            .map_err(|e| io_to_status(e, opts.disposition))?;
        // (`File` descartado aqui: fd fechado por RAII, sem vazamento.)
    }
    // Truncar sem escrita não faz sentido (Windows negaria); explícito aqui
    // em vez de depender do capricho do kernel para O_RDONLY|O_TRUNC.
    if opts.disposition == 5 && !opts.write {
        return Err(NtStatus::ACCESS_DENIED);
    }
    let mut o = std::fs::OpenOptions::new();
    o.read(opts.read).write(opts.write);
    match opts.disposition {
        1 if opts.write => {
            o.create_new(true);
        }
        2 if opts.write => {
            o.create(true).truncate(true);
        }
        4 if opts.write => {
            o.create(true);
        }
        // 5 com escrita trunca (sem escrita já negado acima).
        5 => {
            o.truncate(true);
        }
        // 3: deve existir; 1/2/4 read-only já garantidos acima.
        _ => {}
    }
    let file = o
        .open(&path)
        .map_err(|e| io_to_status(e, opts.disposition))?;
    // SAFETY de ownership: `into_raw_fd` transfere; `owns_fd=true` garante
    // `close_fd` exatamente uma vez em `close_handle`.
    use std::os::unix::io::IntoRawFd;
    let fd = file.into_raw_fd();
    let obj = Arc::new(KernelObject {
        typ: ObjectType::File,
        name: None,
        rights: 0x120089,
        payload: std::sync::Mutex::new(ObjectPayload::File(FileObject {
            fd,
            path: path.to_string_lossy().into_owned(),
            writable: opts.write,
            readable: opts.read,
            owns_fd: true,
        })),
    });
    Ok(table.insert(obj))
}

fn io_to_status(e: std::io::Error, disposition: u32) -> NtStatus {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => NtStatus::OBJECT_NAME_NOT_FOUND,
        ErrorKind::AlreadyExists => NtStatus::OBJECT_NAME_COLLISION,
        ErrorKind::PermissionDenied => NtStatus::ACCESS_DENIED,
        ErrorKind::InvalidInput => NtStatus::INVALID_PARAMETER,
        _ if disposition == 1 => NtStatus::OBJECT_NAME_COLLISION,
        _ => NtStatus::UNSUCCESSFUL,
    }
}

/// Escreve `buf` no objeto `File` de `handle`. Retorna bytes escritos.
pub fn write_file(table: &HandleTable, handle: WindowsHandle, buf: &[u8]) -> Result<u32, NtStatus> {
    let obj: Arc<KernelObject> = table.lookup(handle)?;
    if obj.typ != ObjectType::File {
        return Err(NtStatus::INVALID_HANDLE);
    }
    let guard = obj.payload.lock().unwrap();
    let ObjectPayload::File(f) = &*guard;
    let (fd, writable) = (f.fd, f.writable);
    drop(guard);
    if !writable {
        return Err(NtStatus::ACCESS_DENIED);
    }
    host_linux::write_all(fd, buf)
        .map(|n| n as u32)
        .map_err(|_| NtStatus::UNSUCCESSFUL)
}

/// Lê até `buf.len()` bytes (short count em EOF = `Ok(n)`, como Win32).
pub fn read_file(
    table: &HandleTable,
    handle: WindowsHandle,
    buf: &mut [u8],
) -> Result<u32, NtStatus> {
    let obj: Arc<KernelObject> = table.lookup(handle)?;
    if obj.typ != ObjectType::File {
        return Err(NtStatus::INVALID_HANDLE);
    }
    let guard = obj.payload.lock().unwrap();
    let ObjectPayload::File(f) = &*guard;
    let (fd, readable) = (f.fd, f.readable);
    drop(guard);
    // Windows nega leitura sem GENERIC_READ (o Rine deixava passar — bug
    // encontrado pelo modelo `difftest::model::FileModel`; regression: teste abaixo).
    if !readable {
        return Err(NtStatus::ACCESS_DENIED);
    }
    host_linux::read_upto(fd, buf)
        .map(|n| n as u32)
        .map_err(|_| NtStatus::UNSUCCESSFUL)
}

/// Fecha handle; se o objeto for `File` com fd possuído, fecha o fd.
/// fds 1/2 do console (`owns_fd=false`) nunca são fechados (são do host).
pub fn close_handle(table: &HandleTable, handle: WindowsHandle) -> Result<(), NtStatus> {
    let obj = table.lookup(handle)?;
    let owned_fd = if obj.typ == ObjectType::File {
        let guard = obj.payload.lock().unwrap();
        let ObjectPayload::File(f) = &*guard;
        if f.owns_fd {
            Some(f.fd)
        } else {
            None
        }
    } else {
        None
    };
    table.close(handle)?;
    if let Some(fd) = owned_fd {
        let _ = host_linux::close_fd(fd);
    }
    Ok(())
}

/// Junta componente sob `base` sem nunca subir acima dela (`..` na raiz
/// gruda na raiz, como no Windows). `comp` nunca contém separador aqui.
fn join_sandbox(out: &mut PathBuf, base: &PathBuf, comp: &str) {
    match comp {
        "" | "." => {}
        ".." => {
            if *out != *base {
                out.pop();
            }
        }
        _ => out.push(comp),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive_map() -> DriveMap {
        let mut d = HashMap::new();
        d.insert('C', "/tmp".to_string());
        DriveMap::new(d, None)
    }

    #[test]
    fn translate_drive_absolute() {
        let m = drive_map();
        assert_eq!(
            m.translate("C:\\a\\b.txt").unwrap(),
            PathBuf::from("/tmp/a/b.txt")
        );
        assert_eq!(m.translate("c:/a").unwrap(), PathBuf::from("/tmp/a"));
        assert_eq!(m.translate("\\??\\C:\\a").unwrap(), PathBuf::from("/tmp/a"));
    }

    /// Regression: leitura sem GENERIC_READ era aceita (Windows nega).
    /// Encontrado pelo modelo `difftest::model::FileModel`.
    #[test]
    fn read_without_read_access_is_denied() {
        let dir = std::env::temp_dir().join(format!("rine-ntfile-ro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = HashMap::new();
        d.insert('T', dir.to_string_lossy().into_owned());
        let fsys = DriveMap::new(d, None);
        let table = HandleTable::new();
        let opts_w = CreateOptions::from_win32(0x4000_0000, 2).unwrap();
        let h = create_file(&table, &fsys, "T:\\ro.txt", &opts_w).unwrap();
        let mut buf = [0u8; 8];
        assert_eq!(read_file(&table, h, &mut buf), Err(NtStatus::ACCESS_DENIED));
        close_handle(&table, h).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Regression: `access==0` + disposição que cria falhava com EINVAL
    /// (O_RDONLY|O_CREAT inválido). Encontrado pela campanha `fileops`.
    #[test]
    fn query_only_create_succeeds_readonly() {
        let dir = std::env::temp_dir().join(format!("rine-ntfile-q0-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = HashMap::new();
        d.insert('T', dir.to_string_lossy().into_owned());
        let fsys = DriveMap::new(d, None);
        let table = HandleTable::new();
        let opts = CreateOptions::from_win32(0, 2).unwrap();
        let h = create_file(&table, &fsys, "T:\\q0.txt", &opts).unwrap();
        assert!(dir.join("q0.txt").exists());
        let mut buf = [0u8; 8];
        assert_eq!(read_file(&table, h, &mut buf).unwrap(), 0); // vazio: EOF limpo
        assert_eq!(write_file(&table, h, b"x"), Err(NtStatus::ACCESS_DENIED));
        close_handle(&table, h).unwrap();
        // Truncar sem escrita: negado explícito.
        let opts_t = CreateOptions::from_win32(0, 5).unwrap();
        assert!(create_file(&table, &fsys, "T:\\q0.txt", &opts_t).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Regression: TRUNCATE_EXISTING parou de truncar numa reescrita
    /// (campanha `fileops` pegou: modelo 0B vs disco 11B).
    #[test]
    fn truncate_existing_truncates() {
        let dir = std::env::temp_dir().join(format!("rine-ntfile-tr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.txt"), b"Hello File\n").unwrap();
        let mut d = HashMap::new();
        d.insert('T', dir.to_string_lossy().into_owned());
        let fsys = DriveMap::new(d, None);
        let table = HandleTable::new();
        let opts = CreateOptions::from_win32(0x4000_0000, 5).unwrap();
        let h = create_file(&table, &fsys, "T:\\t.txt", &opts).unwrap();
        close_handle(&table, h).unwrap();
        assert_eq!(std::fs::read(dir.join("t.txt")).unwrap(), b"");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn translate_clamps_dotdot_to_drive() {
        // `C:\..\x` NÃO pode escapar para /tmp/x (fuga de sandbox).
        let m = drive_map();
        assert_eq!(m.translate("C:\\..\\x").unwrap(), PathBuf::from("/tmp/x"));
        assert_eq!(
            m.translate("C:\\a\\..\\..\\x").unwrap(),
            PathBuf::from("/tmp/x")
        );
        assert_eq!(
            m.translate("C:\\a\\.\\b").unwrap(),
            PathBuf::from("/tmp/a/b")
        );
    }

    #[test]
    fn translate_trailing_is_dir_not_file() {
        let m = drive_map();
        assert!(m.translate("C:\\a\\").is_err());
        assert!(m.translate("rel\\").is_err());
        assert!(m.translate("C:\\").is_ok()); // raiz nua: diretório legítimo
    }

    #[test]
    fn translate_rejects_lacunas() {
        let m = drive_map();
        assert!(m.translate("Z:\\a").is_err()); // drive ausente
        assert!(m.translate("\\\\.\\C:").is_err());
        assert!(m.translate("\\\\srv\\x").is_err());
        assert!(m.translate("C:rel").is_err());
    }

    #[test]
    fn create_write_read_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rine-ntfile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = HashMap::new();
        d.insert('T', dir.to_string_lossy().into_owned());
        let fsys = DriveMap::new(d, None);
        let table = HandleTable::new();
        let opts = CreateOptions::from_win32(0x4000_0000, 2).unwrap();
        let h = create_file(&table, &fsys, "T:\\rt.txt", &opts).unwrap();
        assert_eq!(write_file(&table, h, b"abc").unwrap(), 3);
        close_handle(&table, h).unwrap();
        assert!(table.lookup(h).is_err());
        let opts_r = CreateOptions::from_win32(0x8000_0000, 3).unwrap();
        let h2 = create_file(&table, &fsys, "T:\\rt.txt", &opts_r).unwrap();
        let mut buf = [0u8; 16];
        assert_eq!(read_file(&table, h2, &mut buf).unwrap(), 3);
        assert_eq!(&buf[..3], b"abc");
        close_handle(&table, h2).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn open_missing_is_not_found() {
        let table = HandleTable::new();
        let opts = CreateOptions::from_win32(0x8000_0000, 3).unwrap();
        let r = create_file(&table, &DriveMap::empty(), "C:\\nope\\x.txt", &opts);
        assert!(r.is_err());
    }
}
