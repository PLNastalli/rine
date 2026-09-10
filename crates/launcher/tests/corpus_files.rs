//! Replay do corpus `files/*` (guest real via `rine` isolado).

#[test]
fn regression_files_stay_green() {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/regression/files");
    let cases = difftest::corpus::load_dir(&p).expect("corpus files");
    assert!(!cases.is_empty(), "corpus files vazio?");
    for (path, case) in &cases {
        let v = difftest::corpus::replay_fileops_case(
            case,
            env!("CARGO_BIN_EXE_rine"),
            std::time::Duration::from_secs(10),
        );
        assert_eq!(v, difftest::compare::Verdict::Match, "{path}");
    }
}
