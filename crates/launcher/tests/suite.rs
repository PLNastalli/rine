//! Testes standing v0.2+: `suite.exe` (tudo aplicado) e `evil.exe` (abusos).
//!
//! REGRA (testing-strategy): toda API nova entra no `suite.exe`; todoinput
//! inválido que deva falhar limpo entra no `evil.exe`. Filho isolado sempre.

use std::process::{Command, Stdio};

fn tmpdir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rine-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn capsule_with_c(dir: &std::path::Path) {
    let cap = format!(
        "[app]\nname = \"suitetest\"\n[drives]\nC = \"{}\"\n",
        dir.display()
    );
    std::fs::write(dir.join("capsule.toml"), &cap).unwrap();
}

#[test]
fn suite_exercises_everything_and_passes() {
    let dir = tmpdir("suite");
    std::fs::write(dir.join("suite.exe"), pe::builder::build_suite_exe()).unwrap();
    capsule_with_c(&dir);

    let out = Command::new(env!("CARGO_BIN_EXE_rine"))
        .args(["--capsule", "capsule.toml", "suite.exe"])
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn rine");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("RINE-SUITE-START"),
        "stdout={stdout:?} {out:?}"
    );
    assert!(
        stdout.contains("RINE-SUITE-OK"),
        "stdout={stdout:?} {out:?}"
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    // Efeito colateral verificado no host.
    let content = std::fs::read(dir.join("suitetest.txt")).expect("arquivo do guest");
    assert_eq!(content, pe::builder::file_exe_msg());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn evil_inputs_are_all_contained() {
    let dir = tmpdir("evil");
    std::fs::write(dir.join("evil.exe"), pe::builder::build_evil_exe()).unwrap();
    capsule_with_c(&dir);

    let out = Command::new(env!("CARGO_BIN_EXE_rine"))
        .args(["--capsule", "capsule.toml", "evil.exe"])
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn rine");

    // Exit 0 = os 12 abusos foram recusados com erro limpo, sem crash.
    // Qualquer outro valor (51–62, 139 SIGSEGV…) indica falha de contenção.
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    // Nada pode ter sido criado: recusas vêm antes de qualquer efeito.
    assert!(!dir.join("evil_disp.txt").exists());
    assert!(!dir.join("nope.txt").exists());

    std::fs::remove_dir_all(&dir).unwrap();
}
