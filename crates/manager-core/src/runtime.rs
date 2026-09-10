//! Detecção do runtime (`rine --version`, sem adivinhação).

use crate::error::ManagerError;
use std::path::{Path, PathBuf};

/// Versão do workspace que este Manager sabe operar (protocolo =
/// argv + arquivos + `RINE-CRASH schema=1`; ver `docs/manager/`).
pub const EXPECTED_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runtime localizado e validado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInfo {
    /// Binário executado.
    pub binary: PathBuf,
    /// Versão reportada por `--version` (sem o prefixo `rine `).
    pub version: String,
}

/// Binário `rine` ao lado do executável atual (distribuição sidecar:
/// instalador entrega `rine` + `rine-manager` juntos; em dev, ambos vivem
/// em `target/debug/`). `None` se o caminho do executável for desconhecido.
pub fn sibling_binary() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|dir| dir.join("rine"))
}

/// Localiza o binário `rine` (ordem em `docs/manager/runtime-interface.md`)
/// e valida versão. `base_dir` = dir do Manager (para `<exe>/rine` e
/// `./target/debug/rine` em dev); `None` pula candidatos relativos.
/// Em qualquer caso, o sidecar (ao lado do executável atual) e o `PATH`
/// são sempre tentados — o Manager nunca exige configuração para o caso
/// padrão (rine instalado junto).
pub fn detect_runtime(base_dir: Option<&Path>) -> Result<RuntimeInfo, ManagerError> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(env) = std::env::var("RINE_BIN") {
        if !env.is_empty() {
            candidates.push(PathBuf::from(env));
        }
    }
    // Sidecar primeiro: é o layout oficial (instalador) e o de dev
    // (`target/debug/rine` ao lado de `target/debug/rine-manager`).
    if let Some(sib) = sibling_binary() {
        if !candidates.contains(&sib) {
            candidates.push(sib);
        }
    }
    if let Some(base) = base_dir {
        candidates.push(base.join("rine"));
        // Em dev, o Manager roda do workspace: `target/debug/rine` existe.
        if let Some(parent) = base.parent() {
            candidates.push(parent.join("target/debug/rine"));
        }
        candidates.push(base.join("target/debug/rine"));
    }
    candidates.push(PathBuf::from("rine")); // PATH
    let mut tried: Vec<String> = Vec::new();
    for bin in &candidates {
        match query_version(bin) {
            Ok(version) => {
                if version == EXPECTED_RUNTIME_VERSION {
                    return Ok(RuntimeInfo {
                        binary: bin.clone(),
                        version,
                    });
                }
                return Err(ManagerError::ProtocolMismatch {
                    manager: EXPECTED_RUNTIME_VERSION.into(),
                    runtime: version,
                });
            }
            Err(e) => tried.push(format!("{} ({e})", bin.display())),
        }
    }
    Err(ManagerError::RuntimeNotFound(format!(
        "{} — instale `rine` ao lado do Manager, rode `cargo build -p launcher`, \
         ou aponte $RINE_BIN para o binário",
        tried.join("; ")
    )))
}

/// Roda `<bin> --version`, espera `rine <semver>` em stdout e exit 0.
fn query_version(bin: &Path) -> Result<String, String> {
    let out = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("exit {}", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    line.strip_prefix("rine ")
        .map(str::to_string)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("saída inesperada: {line:?}"))
}

/// Caminho de dados do Manager (XDG; criado sob demanda pelo registry).
pub fn data_dir() -> Result<PathBuf, ManagerError> {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        });
    Ok(base.join("rine-manager"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::io_err;

    #[test]
    fn rejects_garbage_binary() {
        // `/bin/false` existe e sai != 0 → não é runtime.
        let r = query_version(Path::new("/bin/false"));
        assert!(r.is_err());
    }

    #[test]
    fn sibling_points_next_to_current_exe() {
        // Lógica pura de path (sem tocar no disco): o sidecar mora ao lado
        // de quem está rodando — layout do instalador e de target/debug/.
        let sib = sibling_binary().expect("current_exe conhecido em teste");
        assert_eq!(sib.file_name().unwrap(), "rine");
        assert_eq!(
            sib.parent().unwrap(),
            std::env::current_exe().unwrap().parent().unwrap()
        );
    }

    #[test]
    fn not_found_error_tells_what_to_do() {
        // Sem RINE_BIN e sem sidecar/PATH válidos: erro acionável, não vago.
        // PATH esvaziado de propósito (hermético: passa mesmo com rine
        // instalado na máquina). RINE_BIN/PATH só são lidos aqui; os demais
        // testes não fazem spawn — seguro em paralelo com save/restore.
        let prev_bin = std::env::var("RINE_BIN").ok();
        let prev_path = std::env::var("PATH").ok();
        std::env::remove_var("RINE_BIN");
        std::env::set_var("PATH", "/nao/existe-xyz");
        let err = detect_runtime(Some(Path::new("/nao/existe/dir"))).unwrap_err();
        if let Some(v) = prev_bin {
            std::env::set_var("RINE_BIN", v);
        } else {
            std::env::remove_var("RINE_BIN");
        }
        if let Some(v) = prev_path {
            std::env::set_var("PATH", v);
        }
        let msg = err.to_string();
        assert!(msg.contains("cargo build -p launcher"), "{msg}");
        assert!(msg.contains("RINE_BIN"), "{msg}");
    }

    #[test]
    fn version_mismatch_is_explicit() {
        // Binário válido mas versão errada: ProtocolMismatch, nunca "ok".
        let dir = std::env::temp_dir().join(format!("rine-mgr-{}-mismatch", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .unwrap();
        let fake = dir.join("rine");
        std::fs::write(&fake, "#!/bin/sh\necho 'rine 0.0.0-fake'\n")
            .map_err(|e| e.to_string())
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = std::fs::metadata(&fake).unwrap().permissions();
            p.set_mode(0o755);
            std::fs::set_permissions(&fake, p).unwrap();
        }
        let v = query_version(&fake).unwrap();
        assert_eq!(v, "0.0.0-fake");
        assert_ne!(v, EXPECTED_RUNTIME_VERSION);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn io_helper_carries_op() {
        let e = io_err(
            "ler exe",
            std::io::Error::new(std::io::ErrorKind::NotFound, "x"),
        );
        assert_eq!(e.code(), "IO");
    }
}
