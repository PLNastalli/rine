fn main() {
    let what = std::env::args().nth(1).unwrap_or("hello".to_string());
    let path = std::env::args()
        .nth(2)
        .unwrap_or("/tmp/out.exe".to_string());
    let bytes = match what.as_str() {
        "hello" => pe::builder::build_minimal_hello(),
        "file" => pe::builder::build_file_exe(),
        "alloc" => pe::builder::build_alloc_exe(),
        "args" => pe::builder::build_args_exe(),
        "suite" => pe::builder::build_suite_exe(),
        "evil" => pe::builder::build_evil_exe(),
        _ => {
            eprintln!("desconhecido: {what}");
            std::process::exit(2);
        }
    };
    std::fs::write(&path, &bytes).unwrap();
    eprintln!("wrote {path} ({} bytes)", bytes.len());
}
