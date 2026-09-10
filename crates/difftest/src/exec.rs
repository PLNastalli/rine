//! Executores de casos: guest (via `rine`+timeout) e in-process.
//!
//! Guest = fidelidade total (ABI real); in-process = velocidade (milhões de
//! casos) para alvos puros (handles, memória, parser). Ambos convergem para
//! os mesmos tipos de veredito em `compare`.

use oracle::format::OracleResult;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("driver build: {0}")]
    Driver(String),
    #[error("io: {0}")]
    Io(String),
    #[error("runner: {0}")]
    Runner(String),
}

/// Resultado bruto de um passo in-process (um op).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub ok: bool,
    pub detail: String,
}

/// Executa um guest `fileops` no sandbox `tmp` (com `C:` → tmp).
/// Retorna o `OracleResult` (normalizar/comparar é com o chamador).
pub fn run_fileops_guest(
    rine_bin: &str,
    rine_version: &str,
    tmp: &std::path::Path,
    scenario_id: &str,
    steps: &[pe::driver::FileOp],
    setup: &[(String, Vec<u8>)],
    timeout: std::time::Duration,
) -> Result<OracleResult, ExecError> {
    std::fs::create_dir_all(tmp).map_err(|e| ExecError::Io(e.to_string()))?;
    // Sandbox SEMPRE absoluto: drive C: relativo resolveria contra o CWD do
    // filho (bug real pego na primeira campanha — ENOENT fantasma).
    let tmp_abs: std::path::PathBuf =
        std::fs::canonicalize(tmp).map_err(|e| ExecError::Io(e.to_string()))?;
    let tmp = tmp_abs.as_path();
    // Setup EXATAMENTE onde o modelo enxerga (via translate): um único
    // escritor (aqui), nunca duplicado no chamador.
    let fsys = nt_file::DriveMap::new(
        [('C', tmp.to_string_lossy().into_owned())]
            .into_iter()
            .collect(),
        Some(tmp.to_string_lossy().into_owned()),
    );
    for (name, bytes) in setup {
        if let Ok(p) = fsys.translate(name) {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&p, bytes);
        }
    }
    let exe = pe::driver::build(steps).map_err(|e| ExecError::Driver(e.to_string()))?;
    std::fs::write(tmp.join("case.exe"), &exe).map_err(|e| ExecError::Io(e.to_string()))?;
    let cap = format!(
        "[app]\nname = \"dt\"\n[drives]\nC = \"{}\"\n",
        tmp.display()
    );
    std::fs::write(tmp.join("capsule.toml"), cap).map_err(|e| ExecError::Io(e.to_string()))?;
    let case = oracle::runner::TestCase {
        name: scenario_id,
        rine_bin,
        capsule: Some("capsule.toml"),
        exe: "case.exe",
        guest_args: &[],
        workdir: tmp,
        timeout: Some(timeout),
    };
    oracle::runner::run_case(&case, rine_version).map_err(|e| ExecError::Runner(e.to_string()))
}

/// Mede a versão do `rine` uma vez (nunca inventar: erro vira "unknown").
pub fn rine_version(rine_bin: &str) -> String {
    std::process::Command::new(rine_bin)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().trim_start_matches("rine ").to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}
