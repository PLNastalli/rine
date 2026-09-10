//! Vínculo matriz↔testes: `compatibility/matrix.json` referencia apenas
//! testes que existem e valores já cobertos pelos E2E (sem re-executar nada).

use std::collections::HashSet;

#[test]
fn matrix_links_to_real_tests() {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../compatibility/matrix.json");
    let text = std::fs::read_to_string(&p).expect("matrix.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("matrix JSON válido");
    assert_eq!(v["schema"], 1);
    // Toda entrada RINE_PASS tem sujeito em tests/.
    let known: HashSet<&str> = [
        "hello.exe",
        "file.exe",
        "alloc.exe",
        "args.exe",
        "suite.exe",
        "evil.exe",
    ]
    .into_iter()
    .collect();
    for e in v["entries"].as_array().unwrap() {
        let test = e["test"].as_str().unwrap();
        let subject = test.split_whitespace().next().unwrap();
        assert!(
            known.contains(subject),
            "teste desconhecido na matriz: {test}"
        );
        assert_eq!(e["status"], "RINE_PASS");
        assert!(
            e["windows_build"].is_null(),
            "oracle Windows ainda pendente"
        );
    }
}
