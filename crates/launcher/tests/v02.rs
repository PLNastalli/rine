//! Testes end-to-end v0.2: `file.exe`, `alloc.exe`, `args.exe`.
//!
//! Mesmo harness do hello (filho isolado via `CARGO_BIN_EXE_rine`):
//! exit codes observáveis + efeitos no host (arquivo criado).

use std::process::{Command, Stdio};

fn tmpdir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rine-{}-{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_rine(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rine"))
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn rine")
}

#[test]
fn file_exe_roundtrips_through_capsule_drive() {
    let dir = tmpdir("v02file");
    let bytes = pe::builder::build_file_exe();
    std::fs::write(dir.join("file.exe"), &bytes).unwrap();
    let cap = format!(
        "[app]\nname = \"filetest\"\n[drives]\nC = \"{}\"\n",
        dir.display()
    );
    std::fs::write(dir.join("capsule.toml"), &cap).unwrap();

    let out = run_rine(&dir, &["--capsule", "capsule.toml", "file.exe"]);
    // Exit code É o dado (bytes lidos); morte por sinal daria `code() == None`.
    assert_eq!(
        out.status.code(),
        Some(pe::builder::file_exe_msg().len() as i32),
        "{out:?}"
    );

    // Efeito observável no host, via drive C: da Capsule.
    let content = std::fs::read(dir.join("v02test.txt")).expect("arquivo do guest");
    assert_eq!(content, pe::builder::file_exe_msg());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn alloc_exe_proves_virtual_memory_roundtrip() {
    let dir = tmpdir("v02alloc");
    std::fs::write(dir.join("alloc.exe"), pe::builder::build_alloc_exe()).unwrap();

    let out = run_rine(&dir, &["alloc.exe"]);
    assert_eq!(out.status.code(), Some(66), "{out:?}"); // b'B' escrito, protegido, lido, liberado

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn args_exe_proves_cmdline_path() {
    let dir = tmpdir("v02args");
    std::fs::write(dir.join("args.exe"), pe::builder::build_args_exe()).unwrap();

    let out = run_rine(&dir, &["args.exe", "foo", "bar"]);
    let expected = "args.exe foo bar".len() as i32;
    assert_eq!(out.status.code(), Some(expected), "{out:?}");

    std::fs::remove_dir_all(&dir).unwrap();
}
