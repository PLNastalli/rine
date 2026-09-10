//! Bench de load (map+relocs+imports+protect, sem `enter` — que terminaria
//! o processo): `cargo run -p runtime --example bench_load`.

fn main() {
    let bytes = pe::builder::build_suite_exe();
    let n = 50;
    let t = std::time::Instant::now();
    for i in 0..n {
        // NOTE: cada load substitui o contexto global (single-process v0.2);
        // medir carrega+descarrega sequencial é exatamente o uso real.
        let emu = runtime::Emulator::load(runtime::Capsule::default(), &bytes, "suite.exe")
            .expect("load");
        std::hint::black_box(emu.image_base());
        drop(emu);
        let _ = i;
    }
    println!("load[suite]: {} us/op", t.elapsed().as_micros() / n);
}
