//! `rine-test` — campanhas diferenciais, fuzz, shrink e corpus.
//!
//! ```text
//! rine-test differential <fileops|handles|memory> --cases N --seed S [--workers W] [--timeout MS] [--out DIR] [--ops-per-case K] [--merge-matrix]
//! rine-test fuzz <pe|paths> --cases N --seed S [--workers W] [--out DIR]
//! rine-test reproduce <case.json> [--rine-bin P] [--timeout MS]
//! rine-test minimize <case.json> [--budget N] [--promote DIR] [--history TXT]
//! rine-test corpus [--dir tests/regression] [--rine-bin P]
//! rine-test coverage <fileops|handles|memory|pe>
//! rine-test report <run-dir>
//! ```
//!
//! Nenhum comando inventa resultado: tudo medido, seeds registradas, saídas
//! em `<out>` (default `./target/rine-test/<cmd>-<ts>`).

use difftest::{campaigns, compare, corpus, gen, model, report, scenario, shrink};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn usage() -> ! {
    eprintln!("uso: rine-test <differential|fuzz|reproduce|minimize|corpus|coverage|report> ...");
    eprintln!("  differential <fileops|handles|memory> --cases N --seed S [--workers W] [--timeout MS] [--out DIR] [--ops-per-case K] [--merge-matrix] [--rine-bin P]");
    eprintln!("  fuzz <pe|paths> --cases N --seed S [--workers W] [--out DIR]");
    eprintln!("  reproduce <case.json> [--rine-bin P] [--timeout MS]");
    eprintln!("  minimize <case.json> [--budget N] [--promote DIR] [--history TXT]");
    eprintln!("  corpus [--dir DIR] [--rine-bin P] [--timeout MS]");
    eprintln!("  coverage <fileops|handles|memory|pe>");
    eprintln!("  report <run-dir>");
    std::process::exit(2);
}

struct Flags {
    cases: usize,
    seed: u64,
    workers: usize,
    timeout_ms: u64,
    out: Option<PathBuf>,
    ops_per_case: usize,
    merge_matrix: bool,
    rine_bin: String,
    budget: usize,
    promote: Option<PathBuf>,
    history: String,
    dir: Option<PathBuf>,
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            cases: 200,
            seed: 1,
            workers: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            timeout_ms: 10_000,
            out: None,
            ops_per_case: 6,
            merge_matrix: false,
            rine_bin: "./target/debug/rine".into(),
            budget: 500,
            promote: None,
            history: String::new(),
            dir: None,
        }
    }
}

fn parse_u64(args: &[String], i: &mut usize, flag: &str) -> u64 {
    *i += 1;
    args.get(*i)
        .unwrap_or_else(|| usage())
        .parse()
        .unwrap_or_else(|_| {
            eprintln!("rine-test: {flag} exige número");
            std::process::exit(2);
        })
}

fn parse_flags(args: &[String]) -> Flags {
    let mut f = Flags::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cases" => f.cases = parse_u64(args, &mut i, "--cases") as usize,
            "--seed" => f.seed = parse_u64(args, &mut i, "--seed"),
            "--workers" => f.workers = parse_u64(args, &mut i, "--workers") as usize,
            "--timeout" => f.timeout_ms = parse_u64(args, &mut i, "--timeout"),
            "--out" => {
                i += 1;
                f.out = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--ops-per-case" => f.ops_per_case = parse_u64(args, &mut i, "--ops-per-case") as usize,
            "--merge-matrix" => f.merge_matrix = true,
            "--rine-bin" => {
                i += 1;
                f.rine_bin = args.get(i).unwrap_or_else(|| usage()).clone();
            }
            "--budget" => f.budget = parse_u64(args, &mut i, "--budget") as usize,
            "--promote" => {
                i += 1;
                f.promote = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            "--history" => {
                i += 1;
                f.history = args.get(i).unwrap_or_else(|| usage()).clone();
            }
            "--dir" => {
                i += 1;
                f.dir = Some(PathBuf::from(args.get(i).unwrap_or_else(|| usage())));
            }
            other => {
                eprintln!("rine-test: flag desconhecida {other}");
                usage();
            }
        }
        i += 1;
    }
    f
}

fn default_out(cmd: &str, f: &Flags) -> PathBuf {
    f.out.clone().unwrap_or_else(|| {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        PathBuf::from(format!("./target/rine-test/{cmd}-{ts}"))
    })
}

fn write_json(path: &std::path::Path, v: &serde_json::Value) {
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(path, serde_json::to_string_pretty(v).unwrap()).expect("escrever json");
}

/// Caminho absoluto do binário `rine` (casos rodam com CWD próprio; relativo
/// quebraria o spawn — bug real pego na primeira campanha).
fn abs_rine_bin(f: &Flags) -> String {
    std::fs::canonicalize(&f.rine_bin)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            eprintln!("rine-test: rine-bin inexistente: {}", f.rine_bin);
            std::process::exit(2);
        })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    match args[0].as_str() {
        "differential" => cmd_differential(&args[1..]),
        "fuzz" => cmd_fuzz(&args[1..]),
        "reproduce" => cmd_reproduce(&args[1..]),
        "minimize" => cmd_minimize(&args[1..]),
        "corpus" => cmd_corpus(&args[1..]),
        "coverage" => cmd_coverage(&args[1..]),
        "report" => cmd_report(&args[1..]),
        _ => usage(),
    }
}

// ---------------------------------------------------------------------------
// differential
// ---------------------------------------------------------------------------

fn cmd_differential(args: &[String]) {
    if args.is_empty() {
        usage();
    }
    let target = args[0].clone();
    let mut f = parse_flags(&args[1..]);
    f.rine_bin = abs_rine_bin(&f);
    let out = default_out(&format!("differential-{target}"), &f);
    std::fs::create_dir_all(&out).expect("out dir");
    // Absoluto: drives da campanha derivam daqui (relativo quebraria o guest).
    let out = std::fs::canonicalize(&out).expect("out dir canônico");
    let version = difftest::exec::rine_version(&f.rine_bin);
    let t = Instant::now();
    let (records, coverage, api) = match target.as_str() {
        "fileops" => {
            let cfg = campaigns::CampaignCfg {
                seed: f.seed,
                workers: f.workers,
                timeout: Duration::from_millis(f.timeout_ms),
                out_dir: out.clone(),
                rine_bin: &f.rine_bin,
                rine_version: version.clone(),
            };
            let (records, cov) = campaigns::fileops_campaign(&cfg, f.cases, f.ops_per_case);
            (records, cov, "CreateFileA".to_string())
        }
        "handles" => {
            let (records, cov) =
                campaigns::handles_campaign(f.seed, f.cases, f.ops_per_case, f.workers);
            (records, cov, "NtCreateFile".to_string())
        }
        "memory" => {
            let (records, cov) =
                campaigns::memory_campaign(f.seed, f.cases, f.ops_per_case, f.workers);
            (records, cov, "VirtualAlloc".to_string())
        }
        _ => {
            eprintln!("rine-test: alvo desconhecido {target} (fileops|handles|memory)");
            std::process::exit(2);
        }
    };
    let elapsed = t.elapsed().as_secs_f64();
    let rep = report::aggregate(
        &report::AggregateCfg {
            target: &target,
            api: &api,
            seed: f.seed,
            workers: f.workers,
            rine_version: &version,
            elapsed_secs: elapsed,
        },
        &records,
        &coverage,
    );
    write_json(
        &out.join("report.json"),
        &serde_json::to_value(&rep).unwrap(),
    );
    // Falhas salvas (todos os alvos): reproduce/minimize sem re-gerar nada.
    let cases_dir = out.join("cases");
    let mut saved = 0;
    for r in &records {
        if r.verdict != compare::Verdict::Match {
            let _ = std::fs::create_dir_all(&cases_dir);
            let doc = serde_json::json!({"scenario": r.scenario});
            if std::fs::write(
                cases_dir.join(format!("{}.json", r.scenario_id)),
                doc.to_string(),
            )
            .is_ok()
            {
                saved += 1;
            }
        }
    }
    if saved > 0 {
        println!("{saved} casos de falha em {}", cases_dir.display());
    }
    print!("{}", report::render_human(&rep));
    if f.merge_matrix {
        let matrix = PathBuf::from("compatibility/matrix.json");
        match report::merge_matrix(&matrix, &rep) {
            Ok(()) => {
                println!("matrix.json atualizada.");
            }
            Err(e) => {
                eprintln!("rine-test: merge-matrix falhou: {e}");
                std::process::exit(1);
            }
        }
    }
    let bad = rep
        .per_verdict
        .get("SemanticMismatch")
        .copied()
        .unwrap_or(0)
        + rep.per_verdict.get("Crash").copied().unwrap_or(0)
        + rep.per_verdict.get("Timeout").copied().unwrap_or(0)
        + rep.per_verdict.get("RineFailure").copied().unwrap_or(0)
        + rep
            .per_verdict
            .get("InfrastructureFailure")
            .copied()
            .unwrap_or(0);
    if bad > 0 {
        std::process::exit(1);
    }
}

fn cmd_fuzz(args: &[String]) {
    if args.is_empty() {
        usage();
    }
    let target = args[0].clone();
    let f = parse_flags(&args[1..]);
    let out = default_out(&format!("fuzz-{target}"), &f);
    std::fs::create_dir_all(&out).expect("out dir");
    let version = difftest::exec::rine_version(&f.rine_bin);
    let t = Instant::now();
    let (records, api): (Vec<report::CaseRecord>, String) = match target.as_str() {
        "pe" => {
            let recs = campaigns::pe_fuzz_campaign(f.seed, f.cases, f.workers);
            (recs, "parse".to_string())
        }
        "paths" => {
            let recs = runner_paths_fuzz(f.seed, f.cases, f.workers);
            (recs, "translate".to_string())
        }
        _ => {
            eprintln!("rine-test: alvo fuzz desconhecido {target} (pe|paths)");
            std::process::exit(2);
        }
    };
    let elapsed = t.elapsed().as_secs_f64();
    let rep = report::aggregate(
        &report::AggregateCfg {
            target: &format!("fuzz-{target}"),
            api: &api,
            seed: f.seed,
            workers: f.workers,
            rine_version: &version,
            elapsed_secs: elapsed,
        },
        &records,
        &[],
    );
    write_json(
        &out.join("report.json"),
        &serde_json::to_value(&rep).unwrap(),
    );
    print!("{}", report::render_human(&rep));
    if rep.per_verdict.get("Crash").copied().unwrap_or(0) > 0 {
        std::process::exit(1);
    }
}

/// Fuzz de paths NT: translate nunca pode panicar (Mismatch/Crash só via panic).
fn runner_paths_fuzz(seed: u64, n: usize, workers: usize) -> Vec<report::CaseRecord> {
    difftest::runner::run_parallel(n, workers, |index| {
        use difftest::compare::Verdict;
        let case_seed = seed ^ (index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut rng = difftest::rng::Rng::new(case_seed);
        let scenario = scenario::Scenario::new(
            "paths",
            "translate",
            case_seed,
            serde_json::json!({}),
            serde_json::json!({"generator": "paths-fuzz-v1"}),
        );
        let t = Instant::now();
        let pieces = [
            "C:", "D:", "\\\\srv", "\\\\??\\", "\\\\.\\", "rel", "..", "é", "\u{0}", "con", " ",
        ];
        let mut path = String::new();
        for _ in 0..(1 + rng.below_usize(6)) {
            if !path.is_empty() && rng.biased(1, 2) {
                path.push('\\');
            }
            path.push_str(rng.choose(&pieces).unwrap_or(&"x"));
            if rng.biased(1, 4) {
                path.push_str(&"A".repeat(rng.below_usize(300)));
            }
        }
        let m = nt_file::DriveMap::empty();
        let r = std::panic::catch_unwind(|| {
            let _ = m.translate(&path);
        });
        let (verdict, detail) = match r {
            Ok(_) => (Verdict::Match, String::new()),
            Err(_) => (Verdict::Crash, format!("panic em translate({path:?})")),
        };
        report::CaseRecord {
            index,
            scenario_id: scenario.scenario_id.clone(),
            api: scenario.api.clone(),
            seed: case_seed,
            verdict,
            detail,
            seconds: t.elapsed().as_secs_f64(),
            scenario,
        }
    })
}

// ---------------------------------------------------------------------------
// reproduce / minimize / corpus / coverage / report
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct CaseFile {
    scenario: scenario::Scenario,
}

fn cmd_reproduce(args: &[String]) {
    if args.is_empty() {
        usage();
    }
    let mut f = parse_flags(&args[1..]);
    f.rine_bin = abs_rine_bin(&f);
    let text = std::fs::read_to_string(&args[0]).unwrap_or_else(|_| {
        eprintln!("rine-test: sem leitura de {}", args[0]);
        std::process::exit(2);
    });
    let case: CaseFile = serde_json::from_str(&text).unwrap_or_else(|_| {
        eprintln!("rine-test: case.json inválido");
        std::process::exit(2);
    });
    println!(
        "reproduzindo {} (seed {})",
        case.scenario.scenario_id, case.scenario.seed
    );
    match case.scenario.target.as_str() {
        "memory" | "handles" => cmd_reproduce_inprocess(&case.scenario),
        "fileops" => {
            let ops: Vec<model::RawOp> =
                serde_json::from_value(case.scenario.params["ops"].clone()).unwrap_or_else(|_| {
                    eprintln!("rine-test: ops inválidos no cenário");
                    std::process::exit(2);
                });
            let setup: Vec<(String, String)> = serde_json::from_value(
                case.scenario
                    .params
                    .get("setup")
                    .cloned()
                    .unwrap_or_default(),
            )
            .unwrap_or_default();
            let setup_b: Vec<(String, Vec<u8>)> = setup
                .into_iter()
                .map(|(n, c)| (n, c.into_bytes()))
                .collect();
            let tmp = std::env::temp_dir().join(format!("rine-repro-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp);
            let fsys = nt_file::DriveMap::new(
                [('C', tmp.to_string_lossy().into_owned())]
                    .into_iter()
                    .collect(),
                Some(tmp.to_string_lossy().into_owned()),
            );
            let mut model = model::FileModel::new(&fsys, &setup_b);
            let expects = model.simulate(&fsys, &ops);
            let steps = steps_from_expects(&ops, &expects);
            let version = difftest::exec::rine_version(&f.rine_bin);
            match difftest::exec::run_fileops_guest(
                &f.rine_bin,
                &version,
                &tmp,
                &case.scenario.scenario_id,
                &steps,
                &setup_b,
                Duration::from_millis(f.timeout_ms),
            ) {
                Ok(r) => {
                    let n = difftest::normalize::normalize(&r, &tmp);
                    println!(
                        "exit={:?} crash={:?} timed_out={}",
                        n.exit_code, n.crash, n.timed_out
                    );
                    println!("{}", serde_json::to_string_pretty(&n).unwrap());
                }
                Err(e) => {
                    eprintln!("rine-test: execução falhou: {e}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!(
                "rine-test: reproduce de {other}: use a campanha do alvo (in-process é direto)"
            );
            std::process::exit(2);
        }
    }
}

fn cmd_reproduce_inprocess(scenario: &scenario::Scenario) {
    let ops_v = scenario.params["ops"].clone();
    let (verdict, detail) = match scenario.target.as_str() {
        "memory" => {
            let ops: Vec<model::MemOp> = serde_json::from_value(ops_v).unwrap_or_else(|_| {
                eprintln!("rine-test: ops inválidas no cenário");
                std::process::exit(2);
            });
            let (v, d, _) = campaigns::run_memory_once(&ops);
            (v, d)
        }
        "handles" => {
            let ops: Vec<model::HandleOp> = serde_json::from_value(ops_v).unwrap_or_else(|_| {
                eprintln!("rine-test: ops inválidas no cenário");
                std::process::exit(2);
            });
            let (v, d, _) = campaigns::run_handles_once(0, scenario.seed, &ops);
            (v, d)
        }
        other => {
            eprintln!("rine-test: reproduce in-process desconhecido: {other}");
            std::process::exit(2);
        }
    };
    println!("verdict={verdict:?} detail={detail}");
}

fn steps_from_expects(
    ops: &[model::RawOp],
    expects: &[model::FileExpect],
) -> Vec<pe::driver::FileOp> {
    ops.iter()
        .zip(expects.iter())
        .map(|(op, e)| match op {
            model::RawOp::Create { path, access, disp } => pe::driver::FileOp::Create {
                path: path.clone(),
                access: *access,
                disp: *disp,
                expect_valid: e.valid,
            },
            model::RawOp::Write { len } => pe::driver::FileOp::Write {
                len: *len,
                expect_ok: e.ok,
                expect_written: e.count,
            },
            model::RawOp::Read { len } => pe::driver::FileOp::Read {
                len: *len,
                expect_ok: e.ok,
                expect_count: e.count,
                verify: e.verify,
            },
            model::RawOp::Close => pe::driver::FileOp::Close { expect_ok: e.ok },
        })
        .collect()
}

fn cmd_minimize(args: &[String]) {
    if args.is_empty() {
        usage();
    }
    let mut f = parse_flags(&args[1..]);
    f.rine_bin = abs_rine_bin(&f);
    let text = std::fs::read_to_string(&args[0]).unwrap_or_else(|_| {
        eprintln!("rine-test: sem leitura de {}", args[0]);
        std::process::exit(2);
    });
    // Aceita `cases/<id>.json` ({scenario, model_expects}) ou corpus ({scenario, expect}).
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| {
        eprintln!("rine-test: json inválido");
        std::process::exit(2);
    });
    let scenario: scenario::Scenario = serde_json::from_value(v["scenario"].clone())
        .unwrap_or_else(|_| {
            eprintln!("rine-test: sem scenario no arquivo");
            std::process::exit(2);
        });
    if scenario.target == "memory" || scenario.target == "handles" {
        return cmd_minimize_inprocess(&f, &scenario, &v);
    }
    if scenario.target != "fileops" {
        eprintln!("rine-test: minimize cobre fileops/handles/memory");
        std::process::exit(2);
    }
    let ops: Vec<model::RawOp> = serde_json::from_value(scenario.params["ops"].clone())
        .unwrap_or_else(|_| {
            eprintln!("rine-test: ops inválidas");
            std::process::exit(2);
        });
    let setup: Vec<(String, String)> =
        serde_json::from_value(scenario.params.get("setup").cloned().unwrap_or_default())
            .unwrap_or_default();
    let setup_b: Vec<(String, Vec<u8>)> = setup
        .into_iter()
        .map(|(n, c)| (n, c.into_bytes()))
        .collect();
    let version = difftest::exec::rine_version(&f.rine_bin);
    // Minimiza preservando DIVERGÊNCIA (guest vs modelo): exit inesperado
    // ou arquivos finais diferentes. Original sem divergência = nada a fazer.
    let first = run_divergence(&f, &scenario, &ops, &setup_b, &version);
    match &first {
        None => {
            eprintln!("rine-test: caso original nem executa");
            std::process::exit(2);
        }
        Some(d) if d.is_empty() => {
            eprintln!("rine-test: caso original sem divergência (nada a minimizar)");
            std::process::exit(2);
        }
        Some(d) => println!("divergência original: {d}"),
    }
    println!("minimizando (budget {})...", f.budget);
    let min = shrink::minimize(
        &ops,
        &|sub: &[model::RawOp]| matches!(run_divergence(&f, &scenario, sub, &setup_b, &version), Some(d) if !d.is_empty()),
        f.budget,
    );
    println!("{} ops → {} ops:", ops.len(), min.len());
    println!("{}", serde_json::to_string_pretty(&min).unwrap());
    if let Some(dir) = f.promote {
        if f.history.is_empty() {
            eprintln!("rine-test: --promote exige --history \"descrição do bug\"");
            std::process::exit(2);
        }
        let tmp = std::env::temp_dir().join(format!("rine-promote-{}", std::process::id()));
        let fsys = nt_file::DriveMap::new(
            [('C', tmp.to_string_lossy().into_owned())]
                .into_iter()
                .collect(),
            Some(tmp.to_string_lossy().into_owned()),
        );
        let mut model = model::FileModel::new(&fsys, &setup_b);
        let expects = model.simulate(&fsys, &min);
        // Expectativa do corpus: exit 0 + arquivos finais do modelo.
        let mut files = std::collections::BTreeMap::new();
        for (k, v) in model.files_snapshot() {
            if let Ok(rel) = std::path::Path::new(&k).strip_prefix(&tmp) {
                files.insert(rel.to_string_lossy().into_owned(), v);
            }
        }
        let min_scenario = scenario::Scenario::new(
            "fileops",
            &scenario.api,
            scenario.seed,
            serde_json::json!({"ops": min, "setup": setup_b.iter().map(|(n, c)| (n, String::from_utf8_lossy(c).into_owned())).collect::<Vec<_>>()}),
            serde_json::json!({"generator": "minimize-v1", "from": scenario.scenario_id}),
        );
        let expect = compare::Expectation {
            exit_code: Some(0),
            stdout_contains: vec![],
            files,
        };
        let _ = expects;
        let text =
            corpus::serialize_promote(&min_scenario, &expect, &f.history).unwrap_or_else(|e| {
                eprintln!("rine-test: promote: {e}");
                std::process::exit(1);
            });
        let dest = dir.join(format!("files/{}.json", min_scenario.scenario_id));
        if let Some(p) = dest.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(&dest, text).expect("escrever corpus");
        println!("promovido: {}", dest.display());
    }
}

/// Minimize in-process (handles/memory): ddmin sobre ops com predicado
/// "ainda diverge". Sem guest, sem tmpdir — direto modelo×real.
fn cmd_minimize_inprocess(f: &Flags, scenario: &scenario::Scenario, v: &serde_json::Value) {
    let _ = v;
    let ops_v = scenario.params["ops"].clone();
    match scenario.target.as_str() {
        "memory" => {
            let ops: Vec<model::MemOp> = serde_json::from_value(ops_v).unwrap_or_else(|_| {
                eprintln!("rine-test: ops inválidas");
                std::process::exit(2);
            });
            let first = campaigns::run_memory_once(&ops);
            if first.0 == compare::Verdict::Match {
                eprintln!("rine-test: caso original sem divergência (nada a minimizar)");
                std::process::exit(2);
            }
            println!("divergência original: {}", first.1);
            let min = shrink::minimize(
                &ops,
                &|sub: &[model::MemOp]| {
                    campaigns::run_memory_once(sub).0 != compare::Verdict::Match
                },
                f.budget,
            );
            finish_minimize_inprocess(f, scenario, "memory", "VirtualAlloc", min);
        }
        "handles" => {
            let ops: Vec<model::HandleOp> = serde_json::from_value(ops_v).unwrap_or_else(|_| {
                eprintln!("rine-test: ops inválidas");
                std::process::exit(2);
            });
            let first = campaigns::run_handles_once(0, scenario.seed, &ops);
            if first.0 == compare::Verdict::Match {
                eprintln!("rine-test: caso original sem divergência (nada a minimizar)");
                std::process::exit(2);
            }
            println!("divergência original: {}", first.1);
            let min = shrink::minimize(
                &ops,
                &|sub: &[model::HandleOp]| {
                    campaigns::run_handles_once(0, scenario.seed, sub).0 != compare::Verdict::Match
                },
                f.budget,
            );
            finish_minimize_inprocess(f, scenario, "handles", "NtCreateFile", min);
        }
        other => {
            eprintln!("rine-test: alvo desconhecido: {other}");
            std::process::exit(2);
        }
    }
}

fn finish_minimize_inprocess<T: serde::Serialize>(
    f: &Flags,
    scenario: &scenario::Scenario,
    target: &str,
    api: &str,
    min: Vec<T>,
) {
    // Conta ops via JSON (genérico sobre os dois tipos de op).
    let n = serde_json::to_value(&min)
        .unwrap()
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    println!("minimizado para {n} ops:");
    println!("{}", serde_json::to_string_pretty(&min).unwrap());
    if let Some(dir) = &f.promote {
        if f.history.is_empty() {
            eprintln!("rine-test: --promote exige --history \"descrição do bug\"");
            std::process::exit(2);
        }
        let min_scenario = scenario::Scenario::new(
            target,
            api,
            scenario.seed,
            serde_json::json!({"ops": min}),
            serde_json::json!({"generator": "minimize-v1", "from": scenario.scenario_id}),
        );
        let expect = compare::Expectation {
            exit_code: Some(0),
            stdout_contains: vec![],
            files: Default::default(),
        };
        let text =
            corpus::serialize_promote(&min_scenario, &expect, &f.history).unwrap_or_else(|e| {
                eprintln!("rine-test: promote: {e}");
                std::process::exit(1);
            });
        let dest = dir.join(format!("{target}/{}.json", min_scenario.scenario_id));
        if let Some(p) = dest.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(&dest, text).expect("escrever corpus");
        println!("promovido: {}", dest.display());
    }
}

/// Divergência guest×modelo: `None` = infra falhou; `Some("")` = match;
/// `Some(detalhe)` = mismatch (exit inesperado ou arquivos finais).
fn run_divergence(
    f: &Flags,
    scenario: &scenario::Scenario,
    ops: &[model::RawOp],
    setup_b: &[(String, Vec<u8>)],
    version: &str,
) -> Option<String> {
    if ops.is_empty() {
        return None;
    }
    let tmp = std::env::temp_dir().join(format!(
        "rine-min-{}-{}",
        std::process::id(),
        difftest::scenario::fnv1a_64(format!("{ops:?}").as_bytes())
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let fsys = nt_file::DriveMap::new(
        [('C', tmp.to_string_lossy().into_owned())]
            .into_iter()
            .collect(),
        Some(tmp.to_string_lossy().into_owned()),
    );
    let mut model = model::FileModel::new(&fsys, setup_b);
    let expects = model.simulate(&fsys, ops);
    let steps = steps_from_expects(ops, &expects);
    let r = difftest::exec::run_fileops_guest(
        &f.rine_bin,
        version,
        &tmp,
        &scenario.scenario_id,
        &steps,
        setup_b,
        Duration::from_millis(f.timeout_ms.min(5000)),
    )
    .ok()?;
    let n = difftest::normalize::normalize(&r, &tmp);
    // Exit esperado: 0 (checks embutidos vêm do próprio modelo).
    if n.exit_code != Some(0) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Some(format!("exit={:?}", n.exit_code));
    }
    // Arquivos finais vs modelo.
    let mut model_files: std::collections::BTreeMap<String, Vec<u8>> = Default::default();
    for (k, v) in model.files_snapshot() {
        if let Ok(rel) = std::path::Path::new(&k).strip_prefix(&tmp) {
            model_files.insert(rel.to_string_lossy().into_owned(), v);
        }
    }
    let on_disk = snapshot_files(&tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    if on_disk == model_files {
        Some(String::new())
    } else {
        Some(format!("files: disco={on_disk:?} modelo={model_files:?}"))
    }
}

/// Snapshot `{relname: bytes}` do sandbox (sem harness).
fn snapshot_files(tmp: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(tmp) {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "case.exe" && n != "capsule.toml")
            .collect();
        names.sort();
        for name in names {
            if let Ok(b) = std::fs::read(tmp.join(&name)) {
                out.insert(name, b);
            }
        }
    }
    out
}

fn cmd_corpus(args: &[String]) {
    let mut f = parse_flags(args);
    f.rine_bin = abs_rine_bin(&f);
    let dir = f
        .dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("tests/regression"));
    let cases = corpus::load_dir(&dir).unwrap_or_else(|e| {
        eprintln!("rine-test: corpus: {e}");
        std::process::exit(1);
    });
    if cases.is_empty() {
        println!("corpus vazio em {}", dir.display());
        return;
    }
    let mut fail = 0;
    for (path, case) in &cases {
        let (verdict, origin) = match case.scenario.target.as_str() {
            "fileops" => {
                let v = corpus::replay_fileops_case(
                    case,
                    &f.rine_bin,
                    Duration::from_millis(f.timeout_ms),
                );
                (v, "guest")
            }
            "memory" | "handles" => {
                let (v, _) = corpus::replay_inprocess_case(case);
                (v, "in-process")
            }
            other => {
                println!("SKIP (alvo desconhecido: {other}) {path}");
                continue;
            }
        };
        if verdict != compare::Verdict::Match {
            fail += 1;
        }
        println!(
            "{verdict:16?} [{origin}] {path}  [{}]",
            case.history.lines().next().unwrap_or("")
        );
    }
    println!("{} casos, {fail} falhas", cases.len());
    if fail > 0 {
        std::process::exit(1);
    }
}

fn cmd_coverage(args: &[String]) {
    if args.is_empty() {
        usage();
    }
    match args[0].as_str() {
        "fileops" => {
            for d in gen::fileops_dimensions() {
                println!("{}::{}:", d.api, d.name);
                for v in &d.values {
                    println!("  - {v}");
                }
            }
            println!("\nMedido por campanha em report.json: campo `coverage`.");
        }
        "handles" => println!("handles::op: insert, lookup, close"),
        "memory" => println!("memory::op: reserve, commit, protect, release"),
        "pe" => println!("pe: mutações trunc/flip/splice x bases hello/suite/evil"),
        _ => usage(),
    }
}

fn cmd_report(args: &[String]) {
    if args.is_empty() {
        usage();
    }
    let rep_path = std::path::Path::new(&args[0]).join("report.json");
    let text = std::fs::read_to_string(&rep_path).unwrap_or_else(|_| {
        eprintln!("rine-test: sem report.json em {}", args[0]);
        std::process::exit(2);
    });
    let rep: report::CampaignReport = serde_json::from_str(&text).unwrap_or_else(|_| {
        eprintln!("rine-test: report.json inválido");
        std::process::exit(2);
    });
    print!("{}", report::render_human(&rep));
}
