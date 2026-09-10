//! Demanda MinGW: fixture real + mecanismo.
//!
//! O binário `hello_mingw.exe` (145K, CRT) NÃO vai para o git; sua lista de
//! imports vai (`tests/windows/hello_mingw.imports.json`, gerada pelo nosso
//! parser — ver `tests/windows/README.md`). O mecanismo (`missing_imports`)
//! é testado de forma determinística em `runtime` (PE sintético).

use std::collections::BTreeMap;

fn fixture() -> BTreeMap<String, Vec<String>> {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/windows/hello_mingw.imports.json");
    let text = std::fs::read_to_string(&p).expect("fixture hello_mingw.imports.json");
    serde_json::from_str(&text).expect("fixture JSON válido")
}

#[test]
fn mingw_demand_is_known() {
    // Contagem travada: qualquer mudança no fixture ou no resolve() aparece aqui.
    // 46 imports − VirtualProtect − TlsGetValue − GetLastError − Sleep
    // − 4 CriticalSection = 38 em demanda.
    let map = fixture();
    assert_eq!(map.len(), 9);
    let total: usize = map.values().map(|v| v.len()).sum();
    assert_eq!(total, 46);
    assert!(map["KERNEL32.dll"].contains(&"DeleteCriticalSection".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"VirtualProtect".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"TlsGetValue".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"GetLastError".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"Sleep".to_string()));
    assert!(map.contains_key("api-ms-win-crt-heap-l1-1-0.dll"));
}
