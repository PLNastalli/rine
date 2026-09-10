//! `launcher`: binário `rine` — carrega e executa um PE x86_64 no Linux.
//!
//! Uso: `rine [--capsule <capsule.toml>] <programa.exe> [args-do-guest...]`.
//! Retorna com o exit code do guest. Argumentos após o `.exe` vão para a
//! linha de comando Win32 do guest (`GetCommandLineW`), como no Windows.

use runtime::{Capsule, Emulator};

/// Relatório `RINE-CRASH` (puro e testado; ver `docs/crash-format.md`).
fn format_crash_report(
    version: &str,
    executable: &str,
    phase: &str,
    reason: &str,
    location: Option<String>,
    timestamp_secs: u64,
) -> String {
    format!(
        "RINE-CRASH schema=1\ntimestamp: {timestamp_secs}\nversion: {version}\n\
         executable: {executable}\nphase: {phase}\nreason: {reason}\n\
         location: {}\n",
        location.unwrap_or_else(|| "-".into())
    )
}

/// Instala o hook uma vez por execução (fase `load`; `enter` reinstala).
fn install_crash_hook(executable: String, phase: &'static str) {
    let version = env!("CARGO_PKG_VERSION").to_string();
    std::panic::set_hook(Box::new(move |info| {
        let reason = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic sem mensagem".into());
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()));
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        eprint!(
            "{}",
            format_crash_report(&version, &executable, phase, &reason, location, ts)
        );
    }));
}

fn main() {
    // Observabilidade: `RUST_LOG=rine=debug ./rine app.exe`. Saída normal limpa.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();
    let raw: Vec<String> = std::env::args().collect();
    if raw.iter().any(|a| a == "--version" || a == "-V") {
        println!("rine {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    // Parse mínimo: `--capsule F` opcional, depois `<exe> [guest-args...]`.
    let mut capsule_path: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut it = raw.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--capsule" {
            match it.next() {
                Some(f) => capsule_path = Some(f.clone()),
                None => {
                    eprintln!("rine: --capsule exige um arquivo");
                    std::process::exit(2);
                }
            }
        } else {
            rest.push(a.clone());
        }
    }
    if rest.is_empty() {
        eprintln!("uso: rine [--capsule <capsule.toml>] <programa.exe> [args...]");
        std::process::exit(2);
    }
    let exe_path = &rest[0];
    install_crash_hook(exe_path.clone(), "load");
    let bytes = match std::fs::read(exe_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("rine: não foi possível ler {exe_path}: {e}");
            std::process::exit(2);
        }
    };
    let mut capsule = match capsule_path {
        Some(f) => match Capsule::load_toml(&f) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("rine: capsule inválida: {e}");
                std::process::exit(2);
            }
        },
        None => Capsule::default(),
    };
    if capsule.app_name == "hello" {
        capsule.app_name = exe_path.clone();
    }
    // Linha de comando Win32: `nome.exe arg1 arg2...` (argv[0] = exe).
    let guest_cmdline = rest.join(" ");
    let mut emu = match Emulator::load(capsule, &bytes, &guest_cmdline) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("rine: falha ao carregar: {e}");
            // Loop de demanda: mostra TUDO que falta, não só o primeiro.
            if matches!(e, runtime::RuntimeError::Loader(_)) {
                eprint!("{}", runtime::demand_report(&bytes));
            }
            std::process::exit(1);
        }
    };
    install_crash_hook(exe_path.clone(), "enter");
    match emu.enter() {
        Ok(code) => std::process::exit(code as i32),
        Err(e) => {
            eprintln!("rine: falha ao executar: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_report_shape() {
        let s = format_crash_report(
            "0.2.0-alpha.1",
            "foo.exe",
            "load",
            "boom",
            Some("x.rs:1".into()),
            1700000000,
        );
        assert!(s.starts_with("RINE-CRASH schema=1\n"));
        assert!(s.contains("version: 0.2.0-alpha.1"));
        assert!(s.contains("executable: foo.exe"));
        assert!(s.contains("phase: load"));
        assert!(s.contains("reason: boom"));
    }
}
