//! `rine-api-scan`: CLI do oracle estrutural.
//!
//! ```text
//! rine-api-scan scan <system32-dir> <api-db-out>
//! rine-api-scan coverage <api-db> [--out <coverage.json>]
//! ```

use api_scan::{coverage, db, scan};

fn usage() -> ! {
    eprintln!("uso:");
    eprintln!("  rine-api-scan scan <system32-dir> <api-db-out>");
    eprintln!("  rine-api-scan coverage <api-db> [--out <coverage.json>]");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    match args[1].as_str() {
        "scan" => {
            if args.len() != 4 {
                usage();
            }
            let root = std::path::Path::new(&args[2]);
            let out = std::path::Path::new(&args[3]);
            let scanned = scan::scan_tree(root).unwrap_or_else(|e| {
                eprintln!("rine-api-scan: falha no scan: {e}");
                std::process::exit(1);
            });
            match db::write_db(&scanned, out, &args[2]) {
                Ok(meta) => {
                    println!(
                        "dlls={} exports={} forwarders={} apisets={} skipped={}",
                        meta.dll_count,
                        meta.export_count,
                        meta.forwarder_count,
                        meta.apiset_files,
                        meta.skipped_count
                    );
                    if !scanned.skipped.is_empty() {
                        println!("skipped (primeiros 10):");
                        for s in scanned.skipped.iter().take(10) {
                            println!("  {} — {}", s.path, s.reason);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("rine-api-scan: falha ao escrever db: {e}");
                    std::process::exit(1);
                }
            }
        }
        "coverage" => {
            if args.len() < 3 {
                usage();
            }
            let db_dir = std::path::Path::new(&args[2]);
            let mut out_path = db_dir.join("coverage.json");
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--out" && i + 1 < args.len() {
                    out_path = std::path::PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    usage();
                }
            }
            // Lê os records das DLLs do escopo do runtime.
            let mut records = Vec::new();
            for dll in ["KERNEL32.json", "NTDLL.json"] {
                let p = db_dir.join("dlls").join(dll);
                let text = std::fs::read_to_string(&p).unwrap_or_else(|_| {
                    eprintln!("rine-api-scan: sem {} (rode scan primeiro)", p.display());
                    std::process::exit(1);
                });
                let doc: db::DllFile = serde_json::from_str(&text).unwrap_or_else(|_| {
                    eprintln!("rine-api-scan: {} inválido", p.display());
                    std::process::exit(1);
                });
                records.push(doc.record);
            }
            let rep = coverage::report(
                &records,
                &coverage::RineImpls::current(),
                &coverage::behavior_tested(),
            );
            std::fs::write(&out_path, serde_json::to_string_pretty(&rep).unwrap()).unwrap_or_else(
                |e| {
                    eprintln!("rine-api-scan: escrita: {e}");
                    std::process::exit(1);
                },
            );
            println!("{} apis -> {}", rep.apis.len(), out_path.display());
            let mut keys: Vec<_> = rep.totals.iter().collect();
            keys.sort();
            for (k, v) in keys {
                println!("  {k}={v}");
            }
        }
        _ => usage(),
    }
}
