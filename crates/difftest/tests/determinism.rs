//! Determinismo fim-a-fim: mesmos vereditos com 1 ou N workers.
//!
//! Regressão direta da classe de bug "estado compartilhado" (ex.: drive
//! relativo resolvido contra CWD). Barato (12 casos) e roda no CI.
//! Binários localizados via `current_exe` (sem `CARGO_BIN_EXE` cruzado).

fn debug_dir() -> std::path::PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // deps/
    p.pop(); // debug/
    p
}

fn run_workers(workers: usize, dir: &std::path::Path) -> Vec<(String, String)> {
    let debug = debug_dir();
    let out = std::process::Command::new(debug.join("rine-test"))
        .args([
            "differential",
            "fileops",
            "--cases",
            "12",
            "--seed",
            "99",
            "--ops-per-case",
            "5",
        ])
        .args(["--workers", &workers.to_string()])
        .args(["--out", dir.to_str().unwrap()])
        .args(["--rine-bin", debug.join("rine").to_str().unwrap()])
        .output()
        .expect("rine-test");
    assert!(out.status.success(), "workers={workers}: {out:?}");
    // Vereditos por (scenario_id → verdict): igualdade forte, ordem livre.
    let text = std::fs::read_to_string(dir.join("report.json")).expect("report.json");
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut pairs: Vec<(String, String)> = v["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["scenario_id"].as_str().unwrap().into(),
                format!("{:?}", r["verdict"]),
            )
        })
        .collect();
    pairs.sort();
    pairs
}

#[test]
fn fileops_same_verdicts_1_vs_3_workers() {
    let base = std::env::temp_dir().join(format!("rine-det-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let d1 = base.join("w1");
    let d3 = base.join("w3");
    let a = run_workers(1, &d1);
    let b = run_workers(3, &d3);
    assert_eq!(a.len(), 12, "campanha executou menos casos?");
    assert_eq!(a, b);
    std::fs::remove_dir_all(&base).unwrap();
}
