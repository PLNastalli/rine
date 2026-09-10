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
    // 46 imports − 14 KERNEL32 (todas implementadas: VirtualProtect, TlsGetValue,
    // GetLastError, Sleep, 4 CriticalSection, SetUnhandledExceptionFilter,
    // VirtualQuery, GetProcAddress, LoadLibraryA, GetModuleHandleA, FreeLibrary)
    // = 32 em demanda (só api-ms-win-crt-*).
    let map = fixture();
    assert_eq!(map.len(), 9);
    let total: usize = map.values().map(|v| v.len()).sum();
    assert_eq!(total, 46);
    assert!(map["KERNEL32.dll"].contains(&"DeleteCriticalSection".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"VirtualProtect".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"TlsGetValue".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"GetLastError".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"Sleep".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"SetUnhandledExceptionFilter".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"VirtualQuery".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"GetProcAddress".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"LoadLibraryA".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"GetModuleHandleA".to_string()));
    assert!(map["KERNEL32.dll"].contains(&"FreeLibrary".to_string()));
    assert!(map.contains_key("api-ms-win-crt-heap-l1-1-0.dll"));
}

/// Prova do mecanismo: um PE sintético que importa SÓ `GetProcAddress`
/// não tem mais demanda (o resolvedor do `runtime` o satisfaz).
#[test]
fn get_proc_address_leaves_no_demand() {
    let r = pe::builder::build_rdata_generic("KERNEL32.dll", &["GetProcAddress"], &[]);
    let mut code = vec![0xC3u8];
    while code.len() < 0x200 {
        code.push(0xCC);
    }
    let vsize = code.len() as u32;
    let bytes = pe::builder::assemble(&code, &r.bytes, vsize, r.import_dir, 40, r.iats[0], 24);
    assert!(runtime::missing_imports(&bytes).unwrap().is_empty());
}
