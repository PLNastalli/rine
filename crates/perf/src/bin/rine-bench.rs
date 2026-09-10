//! `rine-bench`: coleta, compara e promove baselines de performance.
//!
//! ```text
//! rine-bench run [--out results.json] [--batches N]
//! rine-bench compare <results.json> <baseline.json> [--strict] [--out report.json]
//! rine-bench check [--baseline PATH] [--strict] [--out report.json]   # run + compare (CI)
//! rine-bench promote <results.json> --baseline PATH --reason "..." [--by WHO] [--accept-regression]
//! ```
//!
//! Medição preserva o escopo dos benchmarks originais (nada pulado para
//! melhorar número — ver IDs/scopes abaixo).

use perf::{compare, report, schema, stats};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
enum CliError {
    #[error("uso: {0}")]
    Usage(String),
    #[error("io: {0}")]
    Io(String),
    #[error("json: {0}")]
    Json(String),
    #[error("recusado: {0}")]
    Refused(String),
}

const METHOD_VERSION: u32 = 1;
const DEFAULT_BASELINE: &str = "bench/baselines/v0.2.0-alpha.1.json";

fn main() {
    if let Err(e) = real_main() {
        eprintln!("rine-bench: erro: {e}");
        std::process::exit(2);
    }
}

fn real_main() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("compare") => cmd_compare(&args[1..]),
        Some("check") => cmd_check(&args[1..]),
        Some("promote") => cmd_promote(&args[1..]),
        _ => Err(CliError::Usage(
            "rine-bench <run|compare|check|promote> ...".into(),
        )),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

// ---------------------------------------------------------------------------
// ambiente
// ---------------------------------------------------------------------------

fn read_cmd(prog: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(prog)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().into())
        .filter(|s: &String| !s.is_empty())
}

fn cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|t| {
                t.lines()
                    .find(|l| l.starts_with("model name"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_else(|| "unknown".into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        "unknown".into()
    }
}

fn collect_env() -> schema::Environment {
    schema::Environment {
        commit: read_cmd("git", &["rev-parse", "--short", "HEAD"])
            .unwrap_or_else(|| "unknown".into()),
        rine_version: env!("CARGO_PKG_VERSION").into(),
        cpu: cpu_model(),
        kernel: read_cmd("uname", &["-sr"]).unwrap_or_else(|| "unknown".into()),
        toolchain: read_cmd("rustc", &["--version"]).unwrap_or_else(|| "unknown".into()),
        profile: if cfg!(debug_assertions) {
            "debug".into()
        } else {
            "release".into()
        },
        rustflags: std::env::var("RUSTFLAGS").unwrap_or_else(|_| "default".into()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    }
}

// ---------------------------------------------------------------------------
// medição (mesmo escopo dos examples antigos; warmup descartado)
// ---------------------------------------------------------------------------

/// Mede um batch: executa `op` N vezes, retorna ns/op. `black_box` por
/// iteração impede DCE e hoisting (sem ele o compilador poderia eliminar
/// o trabalho medido — benchmark que mede nada é pior que nenhum).
fn batch<T, F: FnMut() -> T>(mut op: F, iters: usize) -> u64 {
    let t = std::time::Instant::now();
    for _ in 0..iters {
        std::hint::black_box(op());
    }
    t.elapsed().as_nanos() as u64 / iters as u64
}

fn measure_parse(bytes: &[u8], batches: usize, iters: usize) -> Vec<u64> {
    // Warmup: cold cache e alocação inicial fora das amostras.
    batch(|| parse_once(bytes), iters);
    (0..batches)
        .map(|_| batch(|| parse_once(bytes), iters))
        .collect()
}

fn parse_once(bytes: &[u8]) -> usize {
    let img = pe::Image::parse(bytes).expect("PE de teste sempre parseia");
    let (_, imports) = img.imports().expect("imports");
    let exports = img.exports().expect("exports");
    // Toca os dados para o compilador não eliminar o trabalho.
    imports.len() + exports.len()
}

fn measure_load(bytes: &[u8], batches: usize, iters: usize) -> Vec<u64> {
    batch(|| load_once(bytes), iters);
    (0..batches)
        .map(|_| batch(|| load_once(bytes), iters))
        .collect()
}

fn load_once(bytes: &[u8]) -> u64 {
    // Mesmo escopo de sempre: map+relocs+imports+protect, sem enter
    // (enter terminaria o processo). Cada load substitui o contexto global,
    // como no uso real single-process.
    let emu =
        runtime::Emulator::load(runtime::Capsule::default(), bytes, "suite.exe").expect("load");
    emu.image_base()
}

fn cmd_run(args: &[String]) -> Result<(), CliError> {
    let batches: usize = flag(args, "--batches")
        .map(|s| s.parse().unwrap_or(0))
        .unwrap_or(10);
    if batches == 0 {
        return Err(CliError::Usage(
            "rine-bench run [--out F] [--batches N>=1]".into(),
        ));
    }
    let out = flag(args, "--out").map(PathBuf::from);
    let hello = pe::builder::build_minimal_hello();
    let suite = pe::builder::build_suite_exe();
    type BenchFn = Box<dyn Fn() -> Vec<u64>>;
    let mut benchmarks = BTreeMap::new();
    // `move` + clone: cada case é dono dos seus bytes ('static p/ o Box).
    let cases: Vec<(&str, &str, BenchFn)> = vec![
        (
            "pe.parse.hello",
            "parse+imports+exports de hello.exe",
            Box::new({
                let hello = hello.clone();
                move || measure_parse(&hello, batches, 2000)
            }),
        ),
        (
            "pe.parse.suite",
            "parse+imports+exports de suite.exe",
            Box::new({
                let suite = suite.clone();
                move || measure_parse(&suite, batches, 2000)
            }),
        ),
        (
            "loader.full.suite",
            "map+relocs+imports+protect de suite.exe (sem enter)",
            Box::new(move || measure_load(&suite, batches, 50)),
        ),
    ];
    for (id, scope, run) in &cases {
        let m = stats::summarize(id, scope, METHOD_VERSION, run())
            .ok_or_else(|| CliError::Io(format!("sem amostras para {id}")))?;
        println!(
            "{id}: mediana {} ns/op ({} amostras)",
            m.median_ns as u64,
            m.samples_ns.len()
        );
        benchmarks.insert(id.to_string(), m);
    }
    let results = schema::RunResults {
        schema: schema::SCHEMA,
        env: collect_env(),
        benchmarks,
    };
    let text = serde_json::to_string_pretty(&results).map_err(|e| CliError::Json(e.to_string()))?;
    if let Some(p) = out {
        std::fs::write(&p, &text).map_err(|e| CliError::Io(e.to_string()))?;
        println!("resultados em {}", p.display());
    } else {
        println!("{text}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// compare / check / promote
// ---------------------------------------------------------------------------

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, CliError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CliError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| CliError::Json(format!("{}: {e}", path.display())))
}

fn cmd_compare(args: &[String]) -> Result<(), CliError> {
    if args.len() < 2 {
        return Err(CliError::Usage(
            "rine-bench compare <results.json> <baseline.json> [--strict]".into(),
        ));
    }
    let results: schema::RunResults = read_json(Path::new(&args[0]))?;
    let baseline: schema::Baseline = read_json(Path::new(&args[1]))?;
    let policy = if has_flag(args, "--strict") {
        compare::Policy::stable()
    } else {
        compare::Policy::alpha()
    };
    let cmp = compare::compare(&baseline, &results, policy);
    print!("{}", report::render_human(&cmp));
    maybe_write_report(args, &cmp)?;
    finish(&cmp, &baseline);
}

/// Salva o relatório em JSON se `--out` foi passado (machine-readable).
fn maybe_write_report(args: &[String], cmp: &compare::Comparison) -> Result<(), CliError> {
    if let Some(out) = flag(args, "--out") {
        #[derive(serde::Serialize)]
        struct Report<'a> {
            schema: u32,
            comparison: &'a compare::Comparison,
        }
        let doc = Report {
            schema: schema::SCHEMA,
            comparison: cmp,
        };
        let text = serde_json::to_string_pretty(&doc).map_err(|e| CliError::Json(e.to_string()))?;
        std::fs::write(&out, text).map_err(|e| CliError::Io(e.to_string()))?;
        println!("relatório em {out}");
    }
    Ok(())
}

fn cmd_check(args: &[String]) -> Result<(), CliError> {
    let baseline = flag(args, "--baseline").unwrap_or_else(|| DEFAULT_BASELINE.into());
    let strict = has_flag(args, "--strict");
    // Roda em diretório temporário e compara em seguida (um passo no CI).
    let tmp = std::env::temp_dir().join(format!("rine-bench-check-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let results_path = tmp.join("results.json");
    // Reusa cmd_run escrevendo no tmp (sem duplicar a medição).
    cmd_run(&[
        "--out".into(),
        results_path.to_string_lossy().into_owned(),
        "--batches".into(),
        flag(args, "--batches").unwrap_or_else(|| "10".into()),
    ])?;
    let results: schema::RunResults = read_json(&results_path)?;
    let bl: schema::Baseline = read_json(Path::new(&baseline))?;
    let policy = if strict {
        compare::Policy::stable()
    } else {
        compare::Policy::alpha()
    };
    let cmp = compare::compare(&bl, &results, policy);
    print!("{}", report::render_human(&cmp));
    maybe_write_report(args, &cmp)?;
    let _ = std::fs::remove_dir_all(&tmp);
    finish(&cmp, &bl);
}

/// Saída com flush (sem ele, `process::exit` pode perder o relatório quando
/// stdout não é tty, i.e. exatamente no CI) + guarda contra comparação vazia:
/// zero vereditos com baseline não-vazia = nada avaliado (exit 2, nunca 0).
fn finish(cmp: &compare::Comparison, baseline: &schema::Baseline) -> ! {
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    if cmp.verdicts.is_empty() && !baseline.benchmarks.is_empty() {
        eprintln!("rine-bench: comparação vazia (schema/metodologia incompatíveis?)");
        std::process::exit(2);
    }
    std::process::exit(report::exit_code(cmp, false));
}

fn cmd_promote(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() {
        return Err(CliError::Usage(
            "rine-bench promote <results.json> --baseline PATH --reason \"...\" [--by WHO] [--accept-regression]".into(),
        ));
    }
    let reason = flag(args, "--reason").ok_or_else(|| {
        CliError::Refused("promoção exige --reason explícito (nunca automática)".into())
    })?;
    let baseline_path = flag(args, "--baseline").ok_or_else(|| {
        CliError::Usage("rine-bench promote <results.json> --baseline PATH --reason ...".into())
    })?;
    let by = flag(args, "--by")
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "unknown".into());
    let results: schema::RunResults = read_json(Path::new(&args[0]))?;
    // Guarda: se existe baseline anterior e o novo é PIOR em algo, exige
    // --accept-regression explícito. Regressão nunca sobrescreve sozinha.
    if Path::new(&baseline_path).exists() {
        let old: schema::Baseline = read_json(Path::new(&baseline_path))?;
        let cmp = compare::compare(&old, &results, compare::Policy::alpha());
        let regressed: Vec<_> = cmp
            .verdicts
            .iter()
            .filter(|v| v.status == compare::Status::Regression)
            .collect();
        if !regressed.is_empty() && !has_flag(args, "--accept-regression") {
            eprintln!("rine-bench: nova baseline PIORA em:");
            for v in &regressed {
                eprintln!(
                    "  {} {}",
                    v.id,
                    v.change_pct
                        .map(|p| format!("{p:+.1}%"))
                        .unwrap_or_default()
                );
            }
            return Err(CliError::Refused(
                "piora detectada: revise a causa ou repita com --accept-regression (auditável)"
                    .into(),
            ));
        }
        let old_id = format!("{}@{}", old.env.rine_version, old.env.commit);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bl = schema::Baseline {
            schema: schema::SCHEMA,
            env: results.env.clone(),
            benchmarks: results.benchmarks.clone(),
            promoted_by: by,
            promoted_at_unix: now,
            reason,
            supersedes: Some(old_id),
        };
        write_baseline(&baseline_path, &bl)?;
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bl = schema::Baseline {
            schema: schema::SCHEMA,
            env: results.env.clone(),
            benchmarks: results.benchmarks.clone(),
            promoted_by: by,
            promoted_at_unix: now,
            reason,
            supersedes: None,
        };
        write_baseline(&baseline_path, &bl)?;
    }
    println!("baseline promovida em {baseline_path}");
    Ok(())
}

fn write_baseline(path: &str, bl: &schema::Baseline) -> Result<(), CliError> {
    let text = serde_json::to_string_pretty(bl).map_err(|e| CliError::Json(e.to_string()))?;
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::Io(e.to_string()))?;
        }
    }
    std::fs::write(path, text).map_err(|e| CliError::Io(e.to_string()))
}
