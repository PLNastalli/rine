//! Replay do corpus de regressão in-process (memory/handles).
//! Casos fileops (guest) rodam em `launcher/tests/corpus_files.rs`
//! (precisam do binário `rine`). Aqui: modelo×real direto.

use difftest::{campaigns, compare, corpus};

fn replay_inprocess(path: &str, case: &corpus::CorpusCase) {
    let ops_v = case.scenario.params["ops"].clone();
    let (verdict, detail) = match case.scenario.target.as_str() {
        "memory" => {
            let ops: Vec<difftest::model::MemOp> =
                serde_json::from_value(ops_v).expect("ops de memória");
            let (v, d, _) = campaigns::run_memory_once(&ops);
            (v, d)
        }
        "handles" => {
            let ops: Vec<difftest::model::HandleOp> =
                serde_json::from_value(ops_v).expect("ops de handles");
            // Reexecuta a sequência com o mesmo runner da campanha.
            let (v, d, _) = campaigns::run_handles_once(0, case.scenario.seed, &ops);
            (v, d)
        }
        other => panic!("alvo in-process desconhecido: {other}"),
    };
    assert_eq!(verdict, compare::Verdict::Match, "{path}: {detail}");
    assert_eq!(
        case.expect.exit_code,
        Some(0),
        "{path}: corpus guarda casos que passam"
    );
}

#[test]
fn regression_corpus_stays_green() {
    let cases = corpus::load_dir(std::path::Path::new("../../tests/regression")).expect("corpus");
    let mut n = 0;
    for (path, case) in &cases {
        match case.scenario.target.as_str() {
            "memory" | "handles" => {
                replay_inprocess(path, case);
                n += 1;
            }
            "fileops" => {} // launcher/tests/corpus_files.rs
            other => panic!("alvo desconhecido no corpus: {other} em {path}"),
        }
    }
    assert!(n >= 2, "corpus in-process vazio?");
}
