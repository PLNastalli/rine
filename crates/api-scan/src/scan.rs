//! Varredura de diretório → `PeRecord`s (+ pulados com motivo).

use crate::record::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("io: {0}")]
    Io(String),
    #[error("parse: {0}")]
    Parse(String),
}

/// Um arquivo pulado (32 bits, .mui sem exports? não — .mui parseia;
/// aqui: não-PE32+, não-AMD64, truncado, ilegível).
#[derive(Debug, Clone)]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct ScanOutput {
    pub records: Vec<PeRecord>,
    pub skipped: Vec<Skipped>,
}

/// Extensões PE escaneadas (case-insensitive — a referência varia a caixa).
fn pe_extension(name_lower: &str) -> bool {
    ["dll", "exe", "sys", "ocx", "cpl", "scr", "mui", "ax"]
        .iter()
        .any(|e| name_lower.ends_with(&format!(".{e}")))
}

/// Escaneia UM arquivo. Erro de parse NÃO aborta a varredura (vira `Skipped`).
pub fn scan_one(full_path: &std::path::Path, rel: &str) -> Result<PeRecord, ScanError> {
    let bytes = std::fs::read(full_path).map_err(|e| ScanError::Io(format!("{rel}: {e}")))?;
    let img = pe::Image::parse(&bytes).map_err(|e| ScanError::Parse(format!("{rel}: {e}")))?;
    let (_descs, imports_raw) = img
        .imports()
        .map_err(|e| ScanError::Parse(format!("{rel}: {e}")))?;
    let exports_raw = img
        .exports()
        .map_err(|e| ScanError::Parse(format!("{rel}: {e}")))?;

    // Agrupa imports por DLL preservando ordem.
    let mut imports: Vec<ImportRec> = Vec::new();
    for s in &imports_raw {
        match imports
            .last_mut()
            .filter(|d: &&mut ImportRec| d.dll == s.dll)
        {
            Some(d) => d.symbols.push(ImportSym {
                name: s.name.clone(),
                ordinal: s.ordinal,
            }),
            None => imports.push(ImportRec {
                dll: s.dll.clone(),
                symbols: vec![ImportSym {
                    name: s.name.clone(),
                    ordinal: s.ordinal,
                }],
            }),
        }
    }

    let filename = full_path
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    Ok(PeRecord {
        schema: SCHEMA,
        filename: filename.clone(),
        path: rel.to_string(),
        machine: img.machine,
        arch: arch_name(img.machine),
        pe32plus: true,
        image_base: img.image_base,
        entry_rva: img.entry_point_rva,
        subsystem: img.subsystem,
        dll_characteristics: img.dll_characteristics,
        sections: img
            .sections
            .iter()
            .map(|s| SectionRec {
                name: s.name.clone(),
                va: s.virtual_address,
                vsize: s.virtual_size,
                raw_size: s.raw_size,
                characteristics: s.characteristics,
            })
            .collect(),
        imports,
        exports: exports_raw
            .into_iter()
            .map(|e| ExportRec {
                name: e.name,
                ordinal: e.ordinal,
                rva: e.rva,
                forwarder: e.forwarder.map(|f| ForwarderRec {
                    dll: f.dll,
                    symbol: match f.symbol {
                        pe::ForwardedSymbol::Name(n) => n,
                        pe::ForwardedSymbol::Ordinal(o) => format!("#{o}"),
                    },
                }),
            })
            .collect(),
        is_api_set: is_api_set_name(&filename),
    })
}

/// Varre `root` recursivamente (top-down, ordem estável).
pub fn scan_tree(root: &std::path::Path) -> Result<ScanOutput, ScanError> {
    let mut out = ScanOutput::default();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| ScanError::Io(format!("{}: {e}", dir.display())))?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let name_lower = p
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !pe_extension(&name_lower) {
                continue;
            }
            let rel = p
                .strip_prefix(root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or(name_lower.clone());
            match scan_one(&p, &rel) {
                Ok(r) => out.records.push(r),
                Err(e) => out.skipped.push(Skipped {
                    path: rel,
                    reason: e.to_string(),
                }),
            }
        }
    }
    out.records.sort_by(|a, b| a.path.cmp(&b.path));
    out.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_own_test_exe() {
        let bytes = pe::builder::build_minimal_hello();
        let dir = std::env::temp_dir().join(format!("rine-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("HELLO.EXE"), &bytes).unwrap(); // caixa alta de propósito
        let out = scan_tree(&dir).unwrap();
        assert_eq!(out.records.len(), 1);
        assert_eq!(out.records[0].filename, "hello.exe"); // normalizado
        assert_eq!(out.records[0].arch, "x86_64");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn malformed_becomes_skipped_not_panic() {
        let dir = std::env::temp_dir().join(format!("rine-scan-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.dll"), b"MZ-truncado").unwrap();
        let out = scan_tree(&dir).unwrap();
        assert!(out.records.is_empty());
        assert_eq!(out.skipped.len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
