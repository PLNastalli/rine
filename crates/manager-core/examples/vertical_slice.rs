//! Vertical slice GUI 0 (manual): detecta o runtime, inspeciona o
//! `hello.exe`, roda o pre-flight, executa e mostra o relatório.
//!
//! ```bash
//! cargo build -p launcher
//! RINE_BIN=$PWD/target/debug/rine cargo run -p manager-core --example vertical_slice
//! ```

use manager_core::{pe_inspect, preflight, run, runtime};
use std::path::PathBuf;

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    // 1. Runtime detection.
    let rt = runtime::detect_runtime(Some(&base)).expect("runtime detectado");
    println!("runtime: {} @ {}", rt.version, rt.binary.display());
    // 2. hello.exe real (builder) no disco.
    let dir = std::env::temp_dir().join(format!("rine-vs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("hello.exe");
    std::fs::write(&exe, pe::builder::build_minimal_hello()).unwrap();
    // 3. Inspeção (sem executar).
    let rep = pe_inspect::inspect_pe(&exe).expect("inspeção");
    println!(
        "pe: {} arch={} subsystem={} entry=0x{:x} imports={} dlls={:?}",
        rep.filename,
        pe_inspect::machine_name(rep.machine),
        pe_inspect::subsystem_name(rep.subsystem),
        rep.entry_point_rva,
        rep.imports.len(),
        rep.dll_dependencies
    );
    // 4. Pre-flight.
    let bytes = std::fs::read(&exe).unwrap();
    let pf = preflight::preflight_bytes(&bytes).expect("pre-flight");
    println!(
        "preflight: {:?} ({}/{} supported)",
        pf.verdict, pf.supported, pf.total_imports
    );
    // 5. Run real + relatório.
    let report = run::run_app(&run::RunRequest {
        rine_bin: rt.binary,
        exe,
        capsule: None,
        args: Vec::new(),
        workdir: Some(dir.clone()),
        env_extra: Vec::new(),
        timeout_ms: 10_000,
    })
    .expect("run");
    println!(
        "run: {:?} exit={:?} em {}ms stdout={:?}",
        report.status,
        report.exit_code,
        report.duration_ms,
        report.stdout.trim()
    );
    assert_eq!(report.status, run::RunStatus::Exited);
    assert_eq!(report.exit_code, Some(0));
    assert!(report.stdout.contains("Hello World"));
    std::fs::remove_dir_all(&dir).unwrap();
    println!("VERTICAL-SLICE-OK");
}
