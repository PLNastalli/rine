#[test]
fn find_file_exports_resolve() {
    for name in ["FindFirstFileW", "FindNextFileW", "FindClose"] {
        assert!(
            kernel32::resolve("KERNEL32.dll", name).is_some(),
            "{name} must resolve from kernel32"
        );
    }
}
