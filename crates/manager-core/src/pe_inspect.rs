//! Inspeção PE sem executar (para a página Run e o pre-flight).

use crate::error::ManagerError;
use serde::{Deserialize, Serialize};

/// Um import estático (`dll!nome` ou `dll!#ordinal`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRef {
    /// `KERNEL32.dll` (grafia original do PE).
    pub dll: String,
    /// `Some("WriteFile")` ou `None` (import por ordinal).
    pub name: Option<String>,
    /// Ordinal (quando por ordinal).
    pub ordinal: Option<u16>,
}

/// Relatório de inspeção (nunca executa um byte do guest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeReport {
    /// Nome do arquivo.
    pub filename: String,
    /// Tamanho em bytes.
    pub size_bytes: u64,
    /// `IMAGE_FILE_MACHINE_*` (0x8664 = x86_64).
    pub machine: u16,
    /// `IMAGE_SUBSYSTEM_*` (2 = GUI, 3 = CUI).
    pub subsystem: u16,
    /// RVA do entry point.
    pub entry_point_rva: u32,
    /// `ImageBase` preferencial.
    pub image_base: u64,
    /// Imports estáticos (ordem do PE, com duplicatas removidas).
    pub imports: Vec<ImportRef>,
    /// DLLs referenciadas (únicas, ordem de aparição).
    pub dll_dependencies: Vec<String>,
}

/// Verdade legível para `machine`.
pub fn machine_name(machine: u16) -> &'static str {
    match machine {
        0x8664 => "x86_64",
        0x14c => "i386 (não suportado)",
        0xaa64 => "ARM64 (não suportado)",
        _ => "desconhecida",
    }
}

/// Verdade legível para `subsystem`.
pub fn subsystem_name(subsystem: u16) -> &'static str {
    match subsystem {
        2 => "Windows GUI",
        3 => "Windows CUI",
        1 => "Native",
        _ => "desconhecido",
    }
}

/// Lê `path`, valida que é arquivo e devolve o relatório.
/// Bytes arbitrários do usuário: parse falho = `UnsupportedPe`, nunca panic.
pub fn inspect_pe(path: &std::path::Path) -> Result<PeReport, ManagerError> {
    let meta = std::fs::metadata(path).map_err(|e| crate::error::io_err("ler exe", e))?;
    if !meta.is_file() {
        return Err(ManagerError::UnsupportedPe(format!(
            "{} não é um arquivo",
            path.display()
        )));
    }
    let bytes = std::fs::read(path).map_err(|e| crate::error::io_err("ler exe", e))?;
    inspect_bytes(
        &bytes,
        path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
        meta.len(),
    )
}

/// Núcleo puro (testável sem disco).
pub fn inspect_bytes(
    bytes: &[u8],
    filename: String,
    size_bytes: u64,
) -> Result<PeReport, ManagerError> {
    let img = pe::Image::parse(bytes)
        .map_err(|e| ManagerError::UnsupportedPe(format!("parse PE: {e}")))?;
    if img.machine != 0x8664 {
        return Err(ManagerError::UnsupportedPe(format!(
            "machine {} ({}) — só x86_64",
            img.machine,
            machine_name(img.machine)
        )));
    }
    let (_descs, symbols) = img
        .imports()
        .map_err(|e| ManagerError::UnsupportedPe(format!("imports: {e}")))?;
    let mut imports = Vec::new();
    let mut dll_dependencies = Vec::new();
    for s in &symbols {
        let entry = ImportRef {
            dll: s.dll.clone(),
            name: s.name.clone(),
            ordinal: s.ordinal,
        };
        if !imports.contains(&entry) {
            imports.push(entry);
        }
        if !dll_dependencies.iter().any(|d| d == &s.dll) {
            dll_dependencies.push(s.dll.clone());
        }
    }
    Ok(PeReport {
        filename,
        size_bytes,
        machine: img.machine,
        subsystem: img.subsystem,
        entry_point_rva: img.entry_point_rva,
        image_base: img.image_base,
        imports,
        dll_dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_report_shape() {
        let bytes = pe::builder::build_suite_exe();
        let r = inspect_bytes(&bytes, "suite.exe".into(), bytes.len() as u64).unwrap();
        assert_eq!(r.machine, 0x8664);
        assert_eq!(machine_name(r.machine), "x86_64");
        assert!(r
            .imports
            .iter()
            .any(|i| i.name.as_deref() == Some("GetProcAddress")));
        assert!(r
            .dll_dependencies
            .iter()
            .any(|d| d.eq_ignore_ascii_case("kernel32.dll")));
        assert!(r.entry_point_rva != 0);
    }

    #[test]
    fn garbage_is_unsupported_not_panic() {
        assert!(inspect_bytes(b"nem um pe", "x.exe".into(), 9).is_err());
        assert!(inspect_bytes(&[], "vazio.exe".into(), 0).is_err());
    }
}
