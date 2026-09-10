//! Execução: spawn estruturado do `rine` + classificação tipada.
//!
//! NUNCA `shell=true`, nunca concatenação de comando: argv separado via
//! `std::process::Command`. O ÚNICO lugar que interpreta stderr do runtime
//! (adaptação temporária até formato estruturado futuro).

use crate::error::{io_err, ManagerError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Pedido de execução (vem da página Run).
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// Binário `rine` validado (`detect_runtime`).
    pub rine_bin: PathBuf,
    /// `.exe` Windows.
    pub exe: PathBuf,
    /// `capsule.toml` (opcional; `None` = default do runtime).
    pub capsule: Option<PathBuf>,
    /// Argumentos do guest (vão para a cmdline Win32; nunca para shell).
    pub args: Vec<String>,
    /// CWD do filho (default: dir do exe).
    pub workdir: Option<PathBuf>,
    /// Variáveis extras (`KEY=value`, somadas ao ambiente herdado).
    pub env_extra: Vec<(String, String)>,
    /// Timeout em ms (0 = sem limite; expirado mata o filho).
    pub timeout_ms: u64,
}

/// Estado final (domínio de execução — não misturar com estados de API).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    /// Load falhou com demanda (imports em falta).
    Blocked,
    /// `RINE-CRASH` em stderr (ou timeout com kill).
    Crashed,
    /// Processo saiu (código em `exit_code`).
    Exited,
}

/// `RINE-CRASH schema=1` parseado (`docs/crash-format.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashInfo {
    /// Versão do `rine` que crashou.
    pub version: String,
    /// argv do guest.
    pub executable: String,
    /// `load` | `enter`.
    pub phase: String,
    /// Mensagem do panic/sinal.
    pub reason: String,
    /// `arquivo:linha` (ou `-`).
    pub location: String,
    /// Segundos Unix.
    pub timestamp: u64,
}

/// Relatório de uma execução (vai para Diagnostics + `last_run`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    /// Estado classificado.
    pub status: RunStatus,
    /// Exit code do `rine` (`None` = morto por sinal/timeout).
    pub exit_code: Option<i32>,
    /// stdout do guest (truncado em 64 KiB; completo só em arquivo).
    pub stdout: String,
    /// stderr do runtime (truncado em 64 KiB).
    pub stderr: String,
    /// Crash estruturado (quando `Crashed` por `RINE-CRASH`).
    pub crash: Option<CrashInfo>,
    /// Demanda (`RINE-DEMAND`) quando `Blocked`.
    pub missing: Vec<(String, String)>,
    /// Duração em ms.
    pub duration_ms: u64,
    /// Versão do runtime usado.
    pub rine_version: String,
}

/// Truncamento honesto (UI nunca carrega log gigante no DOM).
const LOG_CAP: usize = 64 * 1024;

fn trunc(s: &str) -> String {
    if s.len() <= LOG_CAP {
        s.to_string()
    } else {
        format!("{}…[truncado: {} bytes no total]", &s[..LOG_CAP], s.len())
    }
}

/// Extrai `RINE-CRASH schema=1` do stderr (único parser de texto do Manager).
pub fn parse_crash(stderr: &str) -> Option<CrashInfo> {
    let mut lines = stderr.lines();
    let first = lines.next()?;
    if first.trim() != "RINE-CRASH schema=1" {
        // Pode estar após logs `tracing`: procura o marcador em qualquer linha.
        return stderr
            .lines()
            .position(|l| l.trim() == "RINE-CRASH schema=1")
            .and_then(|i| parse_crash(&stderr.lines().skip(i).collect::<Vec<_>>().join("\n")));
    }
    let get = |key: &str| -> Option<String> {
        lines.clone().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
    };
    Some(CrashInfo {
        version: get("version")?,
        executable: get("executable")?,
        phase: get("phase")?,
        reason: get("reason")?,
        location: get("location").unwrap_or_else(|| "-".into()),
        timestamp: get("timestamp").and_then(|t| t.parse().ok()).unwrap_or(0),
    })
}

/// Extrai a lista `RINE-DEMAND` (`  DLL!nome` por linha) do stderr.
pub fn parse_demand(stderr: &str) -> Vec<(String, String)> {
    stderr
        .lines()
        .filter_map(|l| {
            let t = l.strip_prefix("  ")?;
            let (dll, name) = t.split_once('!')?;
            (!dll.is_empty() && !name.is_empty()).then(|| (dll.to_string(), name.to_string()))
        })
        .collect()
}

/// Executa o pedido de forma síncrona (o chamador isola da thread da UI).
/// Erros de spawn/config viram `ManagerError`; saída do guest vira `RunReport`
/// (mesmo com exit != 0 — isso é dado, não erro do Manager).
pub fn run_app(req: &RunRequest) -> Result<RunReport, ManagerError> {
    if !req.exe.is_file() {
        return Err(ManagerError::UnsupportedPe(format!(
            "{} não existe ou não é arquivo",
            req.exe.display()
        )));
    }
    if let Some(cap) = &req.capsule {
        if !cap.is_file() {
            return Err(ManagerError::InvalidCapsule {
                path: cap.display().to_string(),
                reason: "arquivo não encontrado".into(),
            });
        }
        // Valida o TOML agora (capsule inválida = falha antes do spawn).
        runtime::Capsule::load_toml(&cap.to_string_lossy()).map_err(|e| {
            ManagerError::InvalidCapsule {
                path: cap.display().to_string(),
                reason: e.to_string(),
            }
        })?;
    }
    let workdir: PathBuf = match &req.workdir {
        Some(d) => d.clone(),
        None => req
            .exe
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let rine_version = super::runtime::EXPECTED_RUNTIME_VERSION.to_string();
    let t0 = std::time::Instant::now();
    let mut cmd = std::process::Command::new(&req.rine_bin);
    if let Some(cap) = &req.capsule {
        cmd.arg("--capsule").arg(cap);
    }
    cmd.arg(&req.exe).args(&req.args);
    cmd.current_dir(&workdir);
    for (k, v) in &req.env_extra {
        cmd.env(k, v);
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| io_err("spawn rine", e))?;
    // Espera com timeout (poll; sem thread extra, sem dependência nova).
    let out = if req.timeout_ms == 0 {
        child
            .wait_with_output()
            .map_err(|e| io_err("esperar rine", e))?
    } else {
        let step = std::time::Duration::from_millis(10);
        let mut waited = 0u64;
        loop {
            match child.try_wait().map_err(|e| io_err("esperar rine", e))? {
                Some(_) => {
                    break child
                        .wait_with_output()
                        .map_err(|e| io_err("ler saída", e))?;
                }
                None if waited >= req.timeout_ms => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(RunReport {
                        status: RunStatus::Crashed,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        crash: Some(CrashInfo {
                            version: rine_version,
                            executable: req.exe.display().to_string(),
                            phase: "enter".into(),
                            reason: format!("timeout ({} ms)", req.timeout_ms),
                            location: "-".into(),
                            timestamp: 0,
                        }),
                        missing: Vec::new(),
                        duration_ms: t0.elapsed().as_millis() as u64,
                        rine_version: super::runtime::EXPECTED_RUNTIME_VERSION.into(),
                    });
                }
                None => {
                    std::thread::sleep(step);
                    waited += step.as_millis() as u64;
                }
            }
        }
    };
    let duration_ms = t0.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let crash = parse_crash(&stderr);
    let missing = parse_demand(&stderr);
    let status = if crash.is_some() {
        RunStatus::Crashed
    } else if !missing.is_empty() && !out.status.success() {
        RunStatus::Blocked
    } else {
        RunStatus::Exited
    };
    Ok(RunReport {
        status,
        exit_code: out.status.code(),
        stdout: trunc(&stdout),
        stderr: trunc(&stderr),
        crash,
        missing,
        duration_ms,
        rine_version: super::runtime::EXPECTED_RUNTIME_VERSION.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_shape_parses() {
        let err = "RINE-CRASH schema=1\ntimestamp: 1700000000\nversion: 0.2.0-alpha.2\n\
                   executable: foo.exe\nphase: load\nreason: boom\nlocation: x.rs:1\n";
        let c = parse_crash(err).unwrap();
        assert_eq!(c.phase, "load");
        assert_eq!(c.reason, "boom");
        assert_eq!(c.timestamp, 1700000000);
        assert!(parse_crash("nada aqui").is_none());
    }

    #[test]
    fn demand_lines_parse() {
        let err = "rine: demanda total (2 imports sem implementação):\n  KERNEL32.dll!LoadLibraryA\n  NTDLL.dll!X\n";
        assert_eq!(
            parse_demand(err),
            vec![
                ("KERNEL32.dll".to_string(), "LoadLibraryA".to_string()),
                ("NTDLL.dll".to_string(), "X".to_string()),
            ]
        );
    }

    #[test]
    fn missing_exe_is_error_not_report() {
        let req = RunRequest {
            rine_bin: PathBuf::from("rine"),
            exe: PathBuf::from("/nao/existe/app.exe"),
            capsule: None,
            args: Vec::new(),
            workdir: None,
            env_extra: Vec::new(),
            timeout_ms: 5000,
        };
        assert!(matches!(run_app(&req), Err(ManagerError::UnsupportedPe(_))));
    }
}
