//! Teste end-to-end v0.1: `hello.exe` executa no CPU real e imprime.
//!
//! Estratégia: gera o PE via `pe::builder` (sem toolchain Windows),
//! escreve em temp, executa o binário `rine` como processo filho
//! (o guest termina com `ExitProcess` -> exit do filho, nunca do teste),
//! e verifica stdout + exit code. É o primeiro differential harness:
//! o mesmo `hello.exe` deve produzir byte-identico no Windows real.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn hello_exe_prints_hello_world() {
    let bytes = pe::builder::build_minimal_hello();
    let dir = std::env::temp_dir().join(format!("rine-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("hello.exe");
    std::fs::write(&exe, &bytes).unwrap();

    let rine = env!("CARGO_BIN_EXE_rine");
    let child = Command::new(rine)
        .arg(&exe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rine");
    let out = child.wait_with_output().expect("wait");

    let mut log = std::fs::File::create(dir.join("result.txt")).unwrap();
    let _ = writeln!(
        log,
        "status={} stdout={:?}",
        out.status,
        String::from_utf8_lossy(&out.stdout)
    );

    assert!(out.status.success(), "rine falhou: {out:?}");
    assert_eq!(
        out.stdout, b"Hello World\n",
        "stdout divergiu do oracle Windows"
    );
    assert_eq!(out.status.code(), Some(0));

    // Limpeza best-effort.
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hello_pe_imports_are_minimal_and_known() {
    let bytes = pe::builder::build_minimal_hello();
    let img = pe::Image::parse(&bytes).unwrap();
    let (_descs, syms) = img.imports().unwrap();
    let mut names: Vec<String> = syms.iter().filter_map(|s| s.name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["ExitProcess", "GetStdHandle", "WriteFile"]);
}
