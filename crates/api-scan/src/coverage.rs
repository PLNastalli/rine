//! Cobertura: cruza exports reais com o estado do Rine.
//!
//! Estados (definições exatas — `docs/compatibility.md`):
//! - `Missing`: existe no Windows, nada no Rine.
//! - `Stub`: existe façade marcada `Stub` (HOJE: zero — proibido stub mentiroso).
//! - `Partial`: funciona em subset documentado.
//! - `Implemented`: resolve + semântica completa no subset alegado.
//! - `BehaviorTested`: + teste E2E/comportamental no Rine.
//! - `DifferentiallyVerified`: + mesmo teste no Windows real com diff limpo
//!   (HOJE: zero — sem host Windows; nunca alegar sem oracle).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiStatus {
    Missing,
    Stub,
    Partial,
    Implemented,
    BehaviorTested,
    DifferentiallyVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCoverage {
    pub dll: String,
    pub name: String,
    pub ordinal: Option<u32>,
    pub forwarder: Option<String>,
    pub status: ApiStatus,
    /// De onde vem a alegação (teste, doc). Vazio = sem evidência.
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub provenance: String,
    pub totals: std::collections::HashMap<String, usize>,
    pub apis: Vec<ApiCoverage>,
}

/// Implementação do Rine por DLL (lê das tabelas autoritativas).
pub struct RineImpls {
    pub kernel32: Vec<String>,
    pub ntdll: Vec<String>,
}

impl RineImpls {
    pub fn current() -> Self {
        Self {
            kernel32: kernel32::EXPORTS.iter().map(|s| s.to_string()).collect(),
            ntdll: ntdll::EXPORTS.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn contains(&self, dll: &str, name: &str) -> bool {
        let l = dll.to_ascii_lowercase();
        let list = if l == "kernel32.dll" || l == "kernel32" {
            &self.kernel32
        } else if l == "ntdll.dll" || l == "ntdll" {
            &self.ntdll
        } else {
            return false;
        };
        list.iter().any(|n| n == name)
    }
}

/// `(dll, name)` cobertos por teste comportamental/E2E no Rine.
/// Fonte: `crates/launcher/tests/{hello,v02,suite}.rs` (+ `evil.rs` p/ recusas).
pub fn behavior_tested() -> Vec<(String, String)> {
    let k = |n: &str| ("kernel32.dll".to_string(), n.to_string());
    let mut v = vec![
        k("GetStdHandle"),
        k("WriteFile"),
        k("ExitProcess"),
        k("CreateFileA"),
        k("ReadFile"),
        k("CloseHandle"),
        k("VirtualAlloc"),
        k("VirtualFree"),
        k("VirtualProtect"),
        k("GetCommandLineW"),
        k("TlsAlloc"),
        k("TlsFree"),
        k("TlsGetValue"),
        k("TlsSetValue"),
        k("GetLastError"),
        k("Sleep"),
    ];
    v.push(("ntdll.dll".to_string(), "RtlExitUserProcess".to_string()));
    v.push(("ntdll.dll".to_string(), "NtTerminateProcess".to_string()));
    v
}

/// Gera o relatório a partir dos records reais (kernel32+ntdll bastam;
/// escopo do runtime hoje; demais DLLs entram quando houver demanda).
pub fn report(
    real: &[crate::record::PeRecord],
    rine: &RineImpls,
    behavior: &[(String, String)],
) -> CoverageReport {
    let mut apis = Vec::new();
    for r in real {
        for e in &r.exports {
            let name = match &e.name {
                Some(n) => n.clone(),
                None => format!("#{}", e.ordinal),
            };
            let implemented = e
                .name
                .as_ref()
                .is_some_and(|n| rine.contains(&r.filename, n));
            let tested = e
                .name
                .as_ref()
                .is_some_and(|n| behavior.contains(&(r.filename.clone(), n.clone())));
            let status = if tested && implemented {
                ApiStatus::BehaviorTested
            } else if implemented {
                ApiStatus::Implemented
            } else {
                ApiStatus::Missing
            };
            apis.push(ApiCoverage {
                dll: r.filename.clone(),
                name,
                ordinal: Some(e.ordinal),
                forwarder: e
                    .forwarder
                    .as_ref()
                    .map(|f| format!("{}!{}", f.dll, f.symbol)),
                status,
                evidence: if tested {
                    "launcher/tests/{hello,v02,suite}.rs".into()
                } else {
                    String::new()
                },
            });
        }
    }
    apis.sort_by(|a, b| (&a.dll, &a.name).cmp(&(&b.dll, &b.name)));
    let mut totals = std::collections::HashMap::new();
    for a in &apis {
        *totals.entry(format!("{:?}", a.status)).or_insert(0) += 1;
    }
    CoverageReport {
        provenance: "rine-api-scan coverage (ver api-db/README.md)".into(),
        totals,
        apis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_are_honest() {
        let rine = RineImpls::current();
        assert!(rine.kernel32.contains(&"WriteFile".to_string()));
        let rep = report(
            &[crate::record::PeRecord {
                schema: 1,
                filename: "kernel32.dll".into(),
                path: "x".into(),
                machine: 0x8664,
                arch: "x86_64".into(),
                pe32plus: true,
                image_base: 0,
                entry_rva: 0,
                subsystem: 0,
                dll_characteristics: 0,
                sections: vec![],
                imports: vec![],
                exports: vec![
                    crate::record::ExportRec {
                        name: Some("WriteFile".into()),
                        ordinal: 1,
                        rva: 2,
                        forwarder: None,
                    },
                    crate::record::ExportRec {
                        name: Some("Nope".into()),
                        ordinal: 3,
                        rva: 4,
                        forwarder: None,
                    },
                ],
                is_api_set: false,
            }],
            &rine,
            &behavior_tested(),
        );
        let w = rep.apis.iter().find(|a| a.name == "WriteFile").unwrap();
        assert_eq!(w.status, ApiStatus::BehaviorTested);
        let n = rep.apis.iter().find(|a| a.name == "Nope").unwrap();
        assert_eq!(n.status, ApiStatus::Missing);
        assert!(rep.totals.contains_key("Missing"));
    }
}
