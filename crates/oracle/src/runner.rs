//! Runner lado-Rine: executa `test.exe [args]` via binário `rine` isolado
//! (filho — o guest pode terminar o processo) e produz `OracleResult`.
//! Mesmo formato do runner Windows → `normalized diff` futuro.

use crate::format::OracleResult;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("spawn: {0}")]
    Spawn(String),
    #[error("io: {0}")]
    Io(String),
}

/// Um caso: executável + argv do guest + arquivos-base esperados.
pub struct TestCase<'a> {
    pub name: &'a str,
    /// Binário `rine` (normalmente `env!("CARGO_BIN_EXE_rine")` nos testes).
    pub rine_bin: &'a str,
    pub capsule: Option<&'a str>,
    pub exe: &'a str,
    pub guest_args: &'a [&'a str],
    pub workdir: &'a std::path::Path,
    /// Timeout por execução. `None` = esperar para sempre (legado; campanhas
    /// devem sempre fixar timeout — guest em loop não pode travar a suíte).
    pub timeout: Option<std::time::Duration>,
}

fn signal_name(status: std::process::ExitStatus) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal().map(|s| match s {
            11 => "SIGSEGV".to_string(),
            8 => "SIGFPE".to_string(),
            4 => "SIGILL".to_string(),
            6 => "SIGABRT".to_string(),
            n => format!("SIG{n}"),
        })
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

/// Executa o caso e preenche o resultado (nunca inventa: ausente = `None`).
/// Com `timeout`, processo excedente é morto e `timed_out = true`.
pub fn run_case(case: &TestCase, rine_version: &str) -> Result<OracleResult, RunnerError> {
    let mut cmd = std::process::Command::new(case.rine_bin);
    if let Some(cap) = case.capsule {
        cmd.args(["--capsule", cap]);
    }
    cmd.arg(case.exe)
        .args(case.guest_args)
        .current_dir(case.workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let before: Vec<String> = ls_files(case.workdir);
    let mut r = OracleResult::new_rine(case.name, rine_version);
    match case.timeout {
        None => {
            let out = cmd
                .output()
                .map_err(|e| RunnerError::Spawn(e.to_string()))?;
            fill(&mut r, &out, &before, case.workdir);
        }
        Some(limit) => {
            let mut child = cmd.spawn().map_err(|e| RunnerError::Spawn(e.to_string()))?;
            // Drena pipes em threads: sem isso, guest prolífico trava no
            // buffer cheio enquanto aguardamos o deadline (deadlock clássico).
            let mut stdout_take = child.stdout.take();
            let mut stderr_take = child.stderr.take();
            let out_h = std::thread::spawn(move || {
                let mut v = Vec::new();
                if let Some(ref mut p) = stdout_take {
                    use std::io::Read as _;
                    let _ = p.read_to_end(&mut v);
                }
                v
            });
            let err_h = std::thread::spawn(move || {
                let mut v = Vec::new();
                if let Some(ref mut p) = stderr_take {
                    use std::io::Read as _;
                    let _ = p.read_to_end(&mut v);
                }
                v
            });
            let start = std::time::Instant::now();
            let status = loop {
                match child
                    .try_wait()
                    .map_err(|e| RunnerError::Io(e.to_string()))?
                {
                    Some(st) => break Some(st),
                    None if start.elapsed() >= limit => {
                        // Timeout: kill + wait (zumbi não vaza) + drena.
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                    None => std::thread::sleep(std::time::Duration::from_millis(1)),
                }
            };
            let stdout = out_h.join().unwrap_or_default();
            let stderr = err_h.join().unwrap_or_default();
            match status {
                Some(st) => {
                    r.exit_code = st.code();
                    r.crash = signal_name(st);
                    r.stdout_bytes = stdout.len();
                    r.stderr_bytes = stderr.len();
                    r.stdout_text = String::from_utf8(stdout).ok();
                    r.stderr_text = String::from_utf8(stderr).ok();
                    r.files_created = created_since(&before, case.workdir);
                }
                None => {
                    r.timed_out = true;
                    r.stdout_bytes = stdout.len();
                    r.stderr_bytes = stderr.len();
                    r.files_created = created_since(&before, case.workdir);
                }
            }
        }
    }
    Ok(r)
}

fn fill(
    r: &mut OracleResult,
    out: &std::process::Output,
    before: &[String],
    workdir: &std::path::Path,
) {
    r.exit_code = out.status.code();
    r.crash = signal_name(out.status);
    r.stdout_bytes = out.stdout.len();
    r.stderr_bytes = out.stderr.len();
    r.stdout_text = String::from_utf8(out.stdout.clone()).ok();
    r.stderr_text = String::from_utf8(out.stderr.clone()).ok();
    r.files_created = created_since(before, workdir);
}

fn created_since(before: &[String], workdir: &std::path::Path) -> Vec<String> {
    ls_files(workdir)
        .into_iter()
        .filter(|f| !before.contains(f))
        .collect()
}

fn ls_files(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_produces_schema_v1() {
        // Usa /bin/true como "rine" fake? Não — testa só o envoltório com
        // um binário real mínimo (/bin/true ignora args e sai 0).
        let dir = std::env::temp_dir();
        let case = TestCase {
            name: "smoke",
            rine_bin: "/bin/true",
            capsule: None,
            exe: "x.exe",
            guest_args: &[],
            workdir: &dir,
            timeout: None,
        };
        let r = run_case(&case, "test").unwrap();
        assert_eq!(r.schema, crate::format::SCHEMA);
        assert_eq!(r.exit_code, Some(0));
        assert_eq!(r.side, "rine");
        assert!(!r.timed_out);
    }

    #[test]
    fn timeout_kills_and_marks() {
        // /bin/sleep com timeout curto: deve voltar timed_out, sem travar.
        let dir = std::env::temp_dir();
        let case = TestCase {
            name: "hang",
            rine_bin: "/bin/sleep",
            capsule: None,
            exe: "30",
            guest_args: &[],
            workdir: &dir,
            timeout: Some(std::time::Duration::from_millis(200)),
        };
        let r = run_case(&case, "test").unwrap();
        assert!(r.timed_out);
        assert_eq!(r.exit_code, None);
    }
}
