//! Campanhas: geração → modelo → execução → veredito, por alvo.
//!
//! Três sabores, um formato de registro (`CaseRecord`):
//! - `fileops`: guest real (fidelidade total; lento — processo por caso).
//! - `handles`/`memory`: in-process contra modelo (milhões de casos).
//! - `pe`: fuzz do parser (mutação + `check_parse`; panic = Crash).

use crate::compare::{Expectation, Verdict};
use crate::gen;
use crate::model::{self};
use crate::report::CaseRecord;
use crate::rng::Rng;
use crate::runner;
use crate::scenario::Scenario;
use nt_object::{FileObject, HandleTable, KernelObject, ObjectPayload, ObjectType};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct CampaignCfg<'a> {
    pub seed: u64,
    pub workers: usize,
    pub timeout: Duration,
    pub out_dir: PathBuf,
    pub rine_bin: &'a str,
    pub rine_version: String,
}

/// Classifica path nos termos de `gen::PATH_CLASSES` (para cobertura).
fn classify_path(path: &str) -> &'static str {
    if path.starts_with("Z:") {
        return "drive-missing";
    }
    if path.starts_with("\\\\") {
        return "unc";
    }
    if path.ends_with(['\\', '/']) {
        return "trailing";
    }
    if !path.is_ascii() {
        return "unicode";
    }
    if path.contains("d1") {
        return "nested";
    }
    if path.contains(':') {
        return "missing";
    }
    "relative"
}

// ---------------------------------------------------------------------------
// fileops (guest)
// ---------------------------------------------------------------------------

/// Executa UM cenário fileops completo. Retorna registro + dims de cobertura.
#[allow(clippy::too_many_arguments)]
pub fn run_fileops_once(
    cfg: &CampaignCfg,
    index: usize,
    scenario: &Scenario,
    ops: &[model::RawOp],
    setup: &[(String, Vec<u8>)],
    tmp: &Path,
    coverage: &mut Vec<(String, String)>,
) -> CaseRecord {
    let t = Instant::now();
    let fsys = nt_file::DriveMap::new(
        [('C', tmp.to_string_lossy().into_owned())]
            .into_iter()
            .collect(),
        Some(tmp.to_string_lossy().into_owned()),
    );
    // Setup no disco EXATAMENTE onde o modelo enxerga (via translate) —
    // senão setup e modelo divergem antes do guest existir.
    for (name, bytes) in setup {
        if let Ok(p) = fsys.translate(name) {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&p, bytes);
        }
    }
    let mut model = model::FileModel::new(&fsys, setup);
    let expects = model.simulate(&fsys, ops);
    // Driver guest a partir de (op + expectativa).
    let steps = crate::corpus::expects_to_driver(ops, &expects);
    // Cobertura comportamental deste cenário.
    for op in ops {
        match op {
            model::RawOp::Create { path, access, disp } => {
                coverage.push(("path_class".into(), classify_path(path).into()));
                coverage.push((
                    "access".into(),
                    match *access {
                        0x8000_0000 => "read",
                        0x4000_0000 => "write",
                        0xC000_0000 => "rw",
                        _ => "other",
                    }
                    .into(),
                ));
                coverage.push((
                    "disposition".into(),
                    match *disp {
                        1 => "new",
                        2 => "always",
                        3 => "existing",
                        4 => "open-always",
                        5 => "truncate",
                        _ => "bad",
                    }
                    .into(),
                ));
            }
            model::RawOp::Write { .. } => coverage.push(("op".into(), "write".into())),
            model::RawOp::Read { .. } => coverage.push(("op".into(), "read".into())),
            model::RawOp::Close => coverage.push(("op".into(), "close".into())),
        }
    }
    let outcome = crate::exec::run_fileops_guest(
        cfg.rine_bin,
        &cfg.rine_version,
        tmp,
        &scenario.scenario_id,
        &steps,
        setup,
        cfg.timeout,
    );
    let (verdict, detail) = match outcome {
        Err(e) => (Verdict::InfrastructureFailure, format!("exec: {e}")),
        Ok(r) if r.timed_out => (Verdict::Timeout, "guest excedeu timeout".into()),
        Ok(r) if r.crash.is_some() => (Verdict::Crash, format!("crash={:?}", r.crash)),
        Ok(r) => {
            // Exit do guest: 0 = modelo confirmado; 10+i = passo i divergiu.
            let norm = crate::normalize::normalize(&r, tmp);
            match norm.exit_code {
                Some(0) => {
                    // Confirma efeitos no disco contra o modelo final.
                    let mut files = BTreeMap::new();
                    for (k, v) in &model_files_for_compare(&model, tmp) {
                        files.insert(k.clone(), v.clone());
                    }
                    let exp = Expectation {
                        exit_code: Some(0),
                        stdout_contains: vec![],
                        files,
                    };
                    let v = exp.check(&norm, tmp, &[], &scenario.scenario_id, &scenario.api);
                    let detail = if v == Verdict::Match {
                        String::new()
                    } else {
                        files_detail(&exp, &norm, tmp)
                    };
                    (v, detail)
                }
                Some(code) => (Verdict::SemanticMismatch, format!("guest exit {code}")),
                None => (Verdict::RineFailure, "sem exit code nem crash".into()),
            }
        }
    };
    // Detalhe com passo divergente quando houver.
    CaseRecord {
        index,
        scenario_id: scenario.scenario_id.clone(),
        api: scenario.api.clone(),
        seed: scenario.seed,
        verdict,
        detail,
        seconds: t.elapsed().as_secs_f64(),
        scenario: scenario.clone(),
    }
}

/// Detalhe da primeira divergência em arquivos (para top-mismatches).
fn files_detail(exp: &Expectation, norm: &oracle::format::OracleResult, tmp: &Path) -> String {
    for (name, want) in &exp.files {
        match std::fs::read(tmp.join(name)) {
            Ok(got) if &got == want => {}
            Ok(got) => return format!("files:{name} diverge ({}B vs {}B)", got.len(), want.len()),
            Err(_) => return format!("files:{name} ausente"),
        }
    }
    for name in &norm.files_created {
        if name.ends_with(".txt") && !exp.files.contains_key(name) {
            return format!("files:extra {name}");
        }
    }
    "exit/stdout diverge".into()
}
fn model_files_for_compare(model: &model::FileModel, tmp: &Path) -> Vec<(String, Vec<u8>)> {
    model
        .files_snapshot()
        .into_iter()
        .filter_map(|(host_path, bytes)| {
            std::path::Path::new(&host_path)
                .strip_prefix(tmp)
                .ok()
                .map(|rel| (rel.to_string_lossy().into_owned(), bytes))
        })
        .collect()
}

/// Campanha fileops completa (paralela, determinística).
pub fn fileops_campaign(
    cfg: &CampaignCfg,
    n_cases: usize,
    ops_per_case: usize,
) -> (Vec<CaseRecord>, Vec<(String, String)>) {
    use std::sync::Mutex;
    let coverage = Mutex::new(Vec::new());
    let records = runner::run_parallel(n_cases, cfg.workers, |index| {
        let seed = cfg.seed ^ (index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut rng = Rng::new(seed);
        let ops = gen::fileop_sequence(&mut rng, index as u64, ops_per_case);
        let setup = gen_setup(&mut rng, index as u64);
        // Setup serializado COM conteúdo (texto; setups da campanha são
        // textuais por construção — binário é limitação documentada).
        let setup_json: Vec<(String, String)> = setup
            .iter()
            .map(|(n, b)| (n.clone(), String::from_utf8_lossy(b).into_owned()))
            .collect();
        let scenario = Scenario::new(
            "fileops",
            "CreateFileA",
            seed,
            serde_json::json!({"ops": ops, "setup": setup_json}),
            serde_json::json!({"generator": "fileops-v1"}),
        );
        let tmp = cfg.out_dir.join(format!("case_{index:06}"));
        let mut cov = Vec::new();
        let rec = run_fileops_once(cfg, index, &scenario, &ops, &setup, &tmp, &mut cov);
        // Sandbox some: mantém só falhas (disco) — sucesso limpa.
        if rec.verdict != Verdict::Match {
            // (tmp preservado para inspeção)
        } else {
            let _ = std::fs::remove_dir_all(&tmp);
        }
        coverage.lock().expect("coverage").extend(cov);
        rec
    });
    let cov = coverage.into_inner().expect("coverage");
    (records, cov)
}

// ---------------------------------------------------------------------------
// handles (in-process, milhões de ops)
// ---------------------------------------------------------------------------

fn test_file_object() -> KernelObject {
    KernelObject {
        typ: ObjectType::File,
        name: None,
        rights: 0,
        payload: std::sync::Mutex::new(ObjectPayload::File(FileObject {
            fd: -1,
            path: "difftest".into(),
            writable: true,
            readable: true,
            owns_fd: false,
        })),
    }
}

/// Um caso handles: modelo prevê, real executa, compara passo a passo.
pub fn run_handles_once(
    index: usize,
    seed: u64,
    ops: &[model::HandleOp],
) -> (crate::compare::Verdict, String, Vec<(String, String)>) {
    use crate::compare::Verdict;
    use std::sync::Arc;
    let mut coverage = Vec::new();
    let table = HandleTable::new();
    let mut model = model::HandleModel::default();
    let mut sym: Vec<Option<winabi::WindowsHandle>> = Vec::new();
    for op in ops {
        let want = model.step(op);
        let got = match op {
            model::HandleOp::Insert => {
                sym.push(None);
                let h = table.insert(Arc::new(test_file_object()));
                *sym.last_mut().unwrap() = Some(h);
                Ok(())
            }
            model::HandleOp::Lookup { id } => match sym.get(*id).copied().flatten() {
                Some(h) => table.lookup(h).map(|_| ()).map_err(|_| ()),
                None => Err(()),
            },
            model::HandleOp::Close { id } => match sym.get(*id).copied().flatten() {
                Some(h) => table.close(h).map_err(|_| ()),
                None => Err(()),
            },
        };
        // Normaliza NtStatus→bool para comparar com o modelo.
        let want_ok = want.is_ok();
        let got_ok = got.is_ok();
        coverage.push((
            "op".to_string(),
            match op {
                model::HandleOp::Insert => "insert",
                model::HandleOp::Lookup { .. } => "lookup",
                model::HandleOp::Close { .. } => "close",
            }
            .to_string(),
        ));
        if want_ok != got_ok {
            return (
                Verdict::SemanticMismatch,
                format!("op {op:?}: modelo={want_ok} rine={got_ok}"),
                coverage,
            );
        }
    }
    let _ = (index, seed);
    (Verdict::Match, String::new(), coverage)
}

/// Campanha handles paralela (tabela por caso → isolamento total).
pub fn handles_campaign(
    seed: u64,
    n_cases: usize,
    ops_per_case: usize,
    workers: usize,
) -> (Vec<CaseRecord>, Vec<(String, String)>) {
    use std::sync::Mutex;
    let coverage = Mutex::new(Vec::new());
    let records = runner::run_parallel(n_cases, workers, |index| {
        let case_seed = seed ^ (index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut rng = Rng::new(case_seed);
        let ops = gen::handle_sequence(&mut rng, ops_per_case, 6);
        let scenario = Scenario::new(
            "handles",
            "NtCreateFile",
            case_seed,
            serde_json::json!({"ops": ops}),
            serde_json::json!({"generator": "handles-v1"}),
        );
        let t = Instant::now();
        let (verdict, detail, cov) = run_handles_once(index, case_seed, &ops);
        coverage.lock().expect("coverage").extend(cov);
        CaseRecord {
            index,
            scenario_id: scenario.scenario_id.clone(),
            api: scenario.api.clone(),
            seed: case_seed,
            verdict,
            detail,
            seconds: t.elapsed().as_secs_f64(),
            scenario,
        }
    });
    let cov = coverage.into_inner().expect("coverage");
    (records, cov)
}

// ---------------------------------------------------------------------------
// memory (in-process)
// ---------------------------------------------------------------------------

/// Um caso memory: modelo prevê, `MemoryManager` real executa.
/// Diferencial verdadeiro (sem exceções): Reserve aloca atômico, Commit
/// chama o caminho standalone do gerente, Protect/Release diretos.
/// Bases reais (ASLR) nunca saem daqui (regra `aslr`).
pub fn run_memory_once(
    ops: &[model::MemOp],
) -> (crate::compare::Verdict, String, Vec<(String, String)>) {
    use crate::compare::Verdict;
    use winabi::AllocType;
    let mut coverage = Vec::new();
    let mgr = nt_memory::MemoryManager::new();
    let mut model = model::MemoryModel::default();
    // id → (base real, size pedido). Bases opacas, nunca comparadas.
    let mut sym: std::collections::HashMap<usize, (usize, usize)> = Default::default();
    for op in ops {
        // Query tem caminho próprio (compara descritores, não Ok/Err):
        // concordar na ausência também é Match.
        if let model::MemOp::Query { id, offset } = op {
            let want = model.query_desc(*id);
            let addr = sym
                .get(id)
                .map(|(b, _)| b + (*offset as usize % 4096))
                .unwrap_or(0);
            let got = mgr.query(addr);
            let agree = match (want, got) {
                (Some((mlen, mstate, mprot, malloc)), Some(info)) => {
                    info.base == sym.get(id).unwrap().0
                        && info.len == mlen
                        && states_eq(info.state, mstate)
                        && info.protect.bits() == mprot
                        && info.alloc_protect.bits() == malloc
                        && info.kind == nt_memory::RegionKind::Private
                }
                (None, None) => true,
                _ => false,
            };
            coverage.push(("op".to_string(), "query".to_string()));
            if !agree {
                return (
                    Verdict::SemanticMismatch,
                    format!("query {id} diverge"),
                    coverage,
                );
            }
            continue;
        }
        let want = model.step(op);
        // Reserva prevê disciplina de ids: id duplicado/tamanho 0/índice
        // ruim nunca chegam ao gerente (o modelo já recusou sem efeitos).
        let got = match op {
            model::MemOp::Query { .. } => {
                unreachable!("Query sai no caminho próprio acima (continue)")
            }
            model::MemOp::Reserve { id, size, protect } => {
                if sym.contains_key(id)
                    || *size == 0
                    || (*protect as usize) >= model::PROTECTS.len()
                {
                    Err(())
                } else {
                    match mgr.allocate(0, *size, AllocType::RESERVE, protect_from_idx(*protect)) {
                        Ok(base) => {
                            sym.insert(*id, (base, *size));
                            Ok(())
                        }
                        Err(_) => Err(()),
                    }
                }
            }
            // Caminho standalone de commit do gerente (base+size reais).
            // `protect` vem do op como no Windows real (flProtect do COMMIT
            // define a proteção atual; índice inválido nunca toca o gerente).
            model::MemOp::Commit { id, protect } => {
                if (*protect as usize) >= model::PROTECTS.len() {
                    Err(())
                } else {
                    match sym.get(id) {
                        Some((base, size)) => mgr
                            .allocate(*base, *size, AllocType::COMMIT, protect_from_idx(*protect))
                            .map(|_| ())
                            .map_err(|_| ()),
                        // Id desconhecido: base sintética fora de qualquer região.
                        None => mgr
                            .allocate(
                                id.wrapping_mul(0x1_0000).max(0x1000),
                                4096,
                                AllocType::COMMIT,
                                protect_from_idx(*protect),
                            )
                            .map(|_| ())
                            .map_err(|_| ()),
                    }
                }
            }
            model::MemOp::Protect { id, protect } => match sym.get(id) {
                Some((base, _)) if (*protect as usize) < model::PROTECTS.len() => mgr
                    .protect_region(*base, protect_from_idx(*protect))
                    .map(|_| ())
                    .map_err(|_| ()),
                _ => Err(()),
            },
            model::MemOp::Release { id } => match sym.remove(id) {
                Some((base, _)) => mgr.free(base, AllocType::RELEASE).map_err(|_| ()),
                None => Err(()),
            },
        };
        coverage.push((
            "op".to_string(),
            match op {
                model::MemOp::Reserve { .. } => "reserve",
                model::MemOp::Commit { .. } => "commit",
                model::MemOp::Protect { .. } => "protect",
                model::MemOp::Query { .. } => "query",
                model::MemOp::Release { .. } => "release",
            }
            .to_string(),
        ));
        let want_ok = want.is_ok();
        let got_ok = got.is_ok();
        if want_ok != got_ok {
            return (
                Verdict::SemanticMismatch,
                format!("op {op:?}: modelo={want_ok} rine={got_ok}"),
                coverage,
            );
        }
    }
    // Vazamento: tudo liberado ao fim? (checa ANTES de limpar)
    if mgr.region_count() != model.live_count() {
        return (
            Verdict::SemanticMismatch,
            format!(
                "leak: rine={} modelo={}",
                mgr.region_count(),
                model.live_count()
            ),
            coverage,
        );
    }
    // Higiene: libera sobras no host (o Drop do gerente só solta o mapa;
    // sem isso, 1M de casos = OOM do runner, não do Rine).
    for (base, _) in sym.values() {
        let _ = mgr.free(*base, AllocType::RELEASE);
    }
    (Verdict::Match, String::new(), coverage)
}

fn protect_from_idx(i: u8) -> winabi::PageProtect {
    use winabi::PageProtect;
    match model::PROTECTS.get(i as usize).copied().unwrap_or(0x04) {
        0x01 => PageProtect::NOACCESS,
        0x02 => PageProtect::READONLY,
        0x04 => PageProtect::READWRITE,
        0x10 => PageProtect::EXECUTE,
        0x20 => PageProtect::EXECUTE_READ,
        0x40 => PageProtect::EXECUTE_READWRITE,
        _ => PageProtect::READWRITE,
    }
}

/// Igualdade entre estado do modelo e do gerente (tipos distintos,
/// mesmas variantes — comparação explícita, sem `as` mágico).
fn states_eq(a: nt_memory::RegionState, b: model::RegionState) -> bool {
    use model::RegionState as M;
    use nt_memory::RegionState as R;
    matches!(
        (a, b),
        (R::Reserved, M::Reserved) | (R::Committed, M::Committed)
    )
}

/// Campanha memory paralela.
pub fn memory_campaign(
    seed: u64,
    n_cases: usize,
    ops_per_case: usize,
    workers: usize,
) -> (Vec<CaseRecord>, Vec<(String, String)>) {
    use std::sync::Mutex;
    let coverage = Mutex::new(Vec::new());
    let records = runner::run_parallel(n_cases, workers, |index| {
        let case_seed = seed ^ (index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut rng = Rng::new(case_seed);
        let ops = gen::memory_sequence(&mut rng, ops_per_case, 5);
        let scenario = Scenario::new(
            "memory",
            "VirtualAlloc",
            case_seed,
            serde_json::json!({"ops": ops}),
            serde_json::json!({"generator": "memory-v1"}),
        );
        let t = Instant::now();
        let (verdict, detail, cov) = run_memory_once(&ops);
        coverage.lock().expect("coverage").extend(cov);
        CaseRecord {
            index,
            scenario_id: scenario.scenario_id.clone(),
            api: scenario.api.clone(),
            seed: case_seed,
            verdict,
            detail,
            seconds: t.elapsed().as_secs_f64(),
            scenario,
        }
    });
    let cov = coverage.into_inner().expect("coverage");
    (records, cov)
}

// ---------------------------------------------------------------------------
// pe fuzz (in-process, escala)
// ---------------------------------------------------------------------------

/// Campanha fuzz: mutações em escala; panic = Crash (capturado), resto Match.
pub fn pe_fuzz_campaign(seed: u64, n_cases: usize, workers: usize) -> Vec<CaseRecord> {
    let bases: Vec<(&str, Vec<u8>)> = vec![
        ("hello", pe::builder::build_minimal_hello()),
        ("suite", pe::builder::build_suite_exe()),
        ("evil", pe::builder::build_evil_exe()),
    ];
    runner::run_parallel(n_cases, workers, |index| {
        let case_seed = seed ^ (index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut rng = Rng::new(case_seed);
        let (bname, base) = &bases[rng.below_usize(bases.len())];
        let bytes = crate::fuzz::mutate(case_seed, base);
        let scenario = Scenario::new(
            "pe",
            "parse",
            case_seed,
            serde_json::json!({"base": bname, "len": bytes.len()}),
            serde_json::json!({"generator": "pe-fuzz-v1"}),
        );
        let t = Instant::now();
        // Panics do parser viram Crash em vez de matar o worker.
        let r = std::panic::catch_unwind(|| crate::fuzz::check_parse(&bytes));
        let (verdict, detail) = match r {
            Err(_) => (crate::compare::Verdict::Crash, "panic no parser".into()),
            Ok(_) => (crate::compare::Verdict::Match, String::new()),
        };
        CaseRecord {
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

/// Setup determinístico: às vezes pré-cria `C:\ex_<i>.txt` com MSG.
fn gen_setup(rng: &mut Rng, case_idx: u64) -> Vec<(String, Vec<u8>)> {
    if rng.biased(1, 2) {
        vec![(format!("C:\\ex_{case_idx}.txt"), model::MSG.to_vec())]
    } else {
        vec![]
    }
}
