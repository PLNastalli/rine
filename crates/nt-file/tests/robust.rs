//! Robustez de superfície: paths NT e registry nunca panicam.
//!
//! `DriveMap::translate` e `Registry::{set,get,save,load}` sobre inputs
//! hostis (unicode esquisito, `..`, NULs, TOML quebrado) retornam `Err`.

use nt_file::DriveMap;

#[test]
fn translate_hostile_paths() {
    let m = DriveMap::empty();
    for p in [
        "",
        "\\",
        "\\\\",
        "\\\\??\\",
        "C:",
        "C:\\",
        "c:/..\\..\\x",
        "\\??\\C:\\a",
        "\\\\.\\x",
        "\\\\srv\\x",
        "X:rel",
        "rel\\..\\..\\..\\x",
        "C:\\a\u{0}b",
        "C:\\é\\ü\\日",
        &"A".repeat(5000),
        "C:\\con",
        "C:\\a\\",
    ] {
        let _ = m.translate(p); // Ok ou Err — nunca panic
    }
}

#[test]
fn registry_hostile_inputs() {
    let r = nt_registry::Registry::new();
    for (path, name) in [("", ""), ("\\", "\\"), ("HKLM", "v"), ("A\\B\\C", "x")] {
        let _ = r.set(path, name, nt_registry::RegValue::Binary(vec![0; 300]));
        let _ = r.get(path, name);
    }
    // TOML quebrado e inexistente: Err, nunca panic.
    let dir = std::env::temp_dir().join(format!("rine-regfuzz-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.toml");
    std::fs::write(&bad, "[[[inválido").unwrap();
    let path = bad.to_string_lossy().into_owned();
    assert!(r.load(&path).is_err());
    assert!(r.load(dir.join("nope.toml").to_str().unwrap()).is_err());
    assert!(r.save("/proc/rine-impossivel/x.toml").is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}
