//! Registro estrutural de um PE (`schema = 1`).

use serde::{Deserialize, Serialize};

/// Versão do formato. Leitores devem rejeitar `schema` desconhecido.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeRecord {
    pub schema: u32,
    /// Nome do arquivo em minúsculas (FS da referência é case-insensitivo).
    pub filename: String,
    /// Caminho relativo à raiz escaneada, com `/`.
    pub path: String,
    pub machine: u16,
    /// `"x86_64"`, `"i386"`, `"arm64"` ou `"other(<n>)"`.
    pub arch: String,
    /// Sempre `true` (o parser só aceita PE32+; 32 bits caem em `skipped`).
    pub pe32plus: bool,
    pub image_base: u64,
    pub entry_rva: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub sections: Vec<SectionRec>,
    pub imports: Vec<ImportRec>,
    pub exports: Vec<ExportRec>,
    /// `api-ms-win-*` / `ext-ms-win-*` (nunca DLL física).
    pub is_api_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRec {
    pub name: String,
    pub va: u32,
    pub vsize: u32,
    pub raw_size: u32,
    pub characteristics: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRec {
    pub dll: String,
    pub symbols: Vec<ImportSym>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSym {
    pub name: Option<String>,
    pub ordinal: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRec {
    pub name: Option<String>,
    pub ordinal: u32,
    pub rva: u32,
    pub forwarder: Option<ForwarderRec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwarderRec {
    pub dll: String,
    pub symbol: String,
}

pub fn arch_name(machine: u16) -> String {
    match machine {
        winabi::machine::AMD64 => "x86_64".into(),
        winabi::machine::I386 => "i386".into(),
        winabi::machine::ARM64 => "arm64".into(),
        n => format!("other({n:#06X})"),
    }
}

pub fn is_api_set_name(filename_lower: &str) -> bool {
    filename_lower.starts_with("api-ms-win-") || filename_lower.starts_with("ext-ms-win-")
}
