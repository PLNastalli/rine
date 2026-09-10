//! Bench de parse PE: `cargo run -p pe --example bench_parse`.
//! Baseline em `bench/baselines/`. Compatibilidade > performance;
//! isto existe para detectar regressões absurdas, não para otimizar cedo.

fn main() {
    let exes = [
        ("hello", pe::builder::build_minimal_hello()),
        ("suite", pe::builder::build_suite_exe()),
    ];
    for (name, bytes) in &exes {
        let n = 2000;
        let t = std::time::Instant::now();
        for _ in 0..n {
            let img = pe::Image::parse(bytes).unwrap();
            let _ = img.imports().unwrap();
            let _ = img.exports().unwrap();
        }
        println!(
            "parse+imports+exports[{name}]: {} ns/op",
            t.elapsed().as_nanos() / n as u128
        );
    }
}
