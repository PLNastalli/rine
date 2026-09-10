//! Pre-flight: superfície estática de imports (nunca "Compatible").
//!
//! Usa `runtime::missing_imports` (o mesmo resolvedor do load real), então
//! o pre-flight nunca discorda do que o `rine` fará em seguida.

use serde::{Deserialize, Serialize};

/// Veredito honesto do pre-check (ver `docs/manager/architecture.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreflightVerdict {
    /// Todos os imports estáticos conhecidos resolvem.
    /// (Compatibilidade real exige execução/workflow — nunca "Compatible".)
    ImportSurfaceSatisfied,
    /// Há imports sem implementação: execução quase certamente bloqueada.
    LikelyBlocked,
}

/// Relatório do pre-flight para a página Run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    /// Total de símbolos importados (únicos).
    pub total_imports: usize,
    /// Resolvidos pelo runtime atual.
    pub supported: usize,
    /// Sem implementação, em ordem (KERNEL32 primeiro, depois alfabético).
    pub missing: Vec<(String, String)>,
    /// Veredito (terminologia travada).
    pub verdict: PreflightVerdict,
}

/// Analisa bytes já lidos (o chamador leu o arquivo uma vez só).
pub fn preflight_bytes(bytes: &[u8]) -> Result<PreflightReport, crate::error::ManagerError> {
    let img = pe::Image::parse(bytes)
        .map_err(|e| crate::error::ManagerError::UnsupportedPe(format!("parse PE: {e}")))?;
    let (_descs, symbols) = img
        .imports()
        .map_err(|e| crate::error::ManagerError::UnsupportedPe(format!("imports: {e}")))?;
    let total_imports = {
        let mut seen = Vec::new();
        for s in &symbols {
            let key = (
                s.dll.clone(),
                s.name
                    .clone()
                    .unwrap_or_else(|| format!("#{}", s.ordinal.unwrap_or(0))),
            );
            if !seen.contains(&key) {
                seen.push(key);
            }
        }
        seen.len()
    };
    let mut missing = runtime::missing_imports(bytes)
        .map_err(|e| crate::error::ManagerError::UnsupportedPe(format!("imports: {e}")))?;
    missing.sort_by(|a, b| {
        let ka = (!a.0.eq_ignore_ascii_case("kernel32.dll"), &a.0, &a.1);
        let kb = (!b.0.eq_ignore_ascii_case("kernel32.dll"), &b.0, &b.1);
        ka.cmp(&kb)
    });
    let supported = total_imports.saturating_sub(missing.len());
    let verdict = if missing.is_empty() {
        PreflightVerdict::ImportSurfaceSatisfied
    } else {
        PreflightVerdict::LikelyBlocked
    };
    Ok(PreflightReport {
        total_imports,
        supported,
        missing,
        verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_surface_is_satisfied() {
        let bytes = pe::builder::build_suite_exe();
        let r = preflight_bytes(&bytes).unwrap();
        assert_eq!(r.verdict, PreflightVerdict::ImportSurfaceSatisfied);
        assert!(r.missing.is_empty());
        assert!(r.total_imports >= 23);
        assert_eq!(r.supported, r.total_imports);
    }

    #[test]
    fn unknown_import_blocks_with_list() {
        // (`CreateFileW` já foi esse exemplo — virou implementado em v0.3;
        // `FindFirstFileW` segue em demanda. A troca é intencional.)
        let r = pe::builder::build_rdata_generic("KERNEL32.dll", &["FindFirstFileW"], &[]);
        let mut code = vec![0xC3u8];
        while code.len() < 0x200 {
            code.push(0xCC);
        }
        let vsize = code.len() as u32;
        let bytes = pe::builder::assemble(&code, &r.bytes, vsize, r.import_dir, 40, r.iats[0], 24);
        let rep = preflight_bytes(&bytes).unwrap();
        assert_eq!(rep.verdict, PreflightVerdict::LikelyBlocked);
        assert_eq!(
            rep.missing,
            vec![("KERNEL32.dll".to_string(), "FindFirstFileW".to_string())]
        );
    }
}
