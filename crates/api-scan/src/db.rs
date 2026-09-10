//! Escrita do `api-db/` (sharded, regenerável, com proveniência).

use crate::record::PeRecord;
use crate::scan::ScanOutput;
use serde::{Deserialize, Serialize};

/// Cabeçalho de proveniência gravado em todo artefato gerado.
/// NUNCA editar gerados à mão (regenerar com `rine-api-scan`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub generator: String,
    pub generator_version: String,
    pub schema: u32,
    pub reference: String,
    pub note: String,
}

pub fn provenance(reference: &str) -> Provenance {
    Provenance {
        generator: "rine-api-scan".into(),
        generator_version: env!("CARGO_PKG_VERSION").into(),
        schema: crate::record::SCHEMA,
        reference: reference.into(),
        note: "GERADO — não editar; regenerar com `cargo run -p api-scan -- scan`".into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub provenance: Provenance,
    pub dll_count: usize,
    pub export_count: usize,
    pub import_edge_count: usize,
    pub forwarder_count: usize,
    pub apiset_files: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DllFile {
    pub provenance: Provenance,
    pub record: PeRecord,
}

/// Escreve o db em `out_dir`:
/// `metadata.json`, `dlls/<UPPER>.dll.json`, `forwarders.json`, `apisets.json`.
pub fn write_db(
    out: &ScanOutput,
    out_dir: &std::path::Path,
    reference: &str,
) -> Result<Metadata, String> {
    let dlls_dir = out_dir.join("dlls");
    std::fs::create_dir_all(&dlls_dir).map_err(|e| e.to_string())?;
    let prov = provenance(reference);

    let mut export_count = 0;
    let mut import_edges = 0;
    let mut forwarder_count = 0;
    let mut apiset_files = 0;
    let mut forwarders: Vec<serde_json::Value> = Vec::new();
    let mut apisets: Vec<serde_json::Value> = Vec::new();

    for r in &out.records {
        export_count += r.exports.len();
        import_edges += r.imports.iter().map(|d| d.symbols.len()).sum::<usize>();
        for e in &r.exports {
            if let Some(f) = &e.forwarder {
                forwarder_count += 1;
                forwarders.push(serde_json::json!({
                    "from": {"dll": r.filename, "name": e.name, "ordinal": e.ordinal},
                    "to": {"dll": f.dll, "symbol": f.symbol},
                }));
            }
        }
        if r.is_api_set {
            apiset_files += 1;
            apisets.push(serde_json::json!({"file": r.filename, "path": r.path}));
        }
        let stem = r
            .filename
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(&r.filename);
        let fname = format!("{}.json", stem.to_ascii_uppercase());
        let doc = DllFile {
            provenance: prov.clone(),
            record: r.clone(),
        };
        let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
        std::fs::write(dlls_dir.join(fname), text).map_err(|e| e.to_string())?;
    }

    forwarders.sort_by_key(|a| a.to_string());
    let meta = Metadata {
        provenance: prov,
        dll_count: out.records.len(),
        export_count,
        import_edge_count: import_edges,
        forwarder_count,
        apiset_files,
        skipped_count: out.skipped.len(),
    };
    std::fs::write(
        out_dir.join("metadata.json"),
        serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join("forwarders.json"),
        serde_json::to_string_pretty(&forwarders).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join("apisets.json"),
        serde_json::to_string_pretty(&apisets).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(meta)
}
