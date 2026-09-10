//! Corpus de robustez do parser PE: mutações determinísticas, sem pânico.
//!
//! Propriedade: `Image::parse` (+ views) sobre bytes arbitrários retorna
//! `Ok` ou `Err` — NUNCA panic, OOM ou loop. A mutação vive em
//! `difftest::fuzz::mutate` (fonte única; campanhas `rine-test fuzz`
//! usam a mesma). Sementes fixas.

fn exercise(bytes: &[u8]) {
    // `check_parse` já exercita parse + todas as views.
    let _ = difftest::fuzz::check_parse(bytes);
}

#[test]
fn malformed_never_panics() {
    let seeds: Vec<Vec<u8>> = vec![
        pe::builder::build_minimal_hello(),
        pe::builder::build_suite_exe(),
        pe::builder::build_evil_exe(),
    ];
    for (si, seed_bytes) in seeds.iter().enumerate() {
        for m in 0..300u64 {
            exercise(&difftest::fuzz::mutate((si as u64) << 32 | m, seed_bytes));
        }
    }
}

#[test]
fn empty_and_garbage_never_panics() {
    for b in [&[][..], &[0u8; 1][..], b"MZ", b"PE\0\0", &[0xFF; 4096][..]] {
        exercise(b);
    }
}
