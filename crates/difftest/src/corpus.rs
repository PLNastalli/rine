//! Corpus de regressão: `tests/regression/<área>/*.json`.
//!
//! Cada arquivo = caso minimizado que JÁ falhou um dia (campo `history`),
//! com expectativa travada. Replay roda no pipeline normal: se voltar a
//! divergir, o teste quebra com o id do caso. Promoção via
//! `rine-test minimize --promote <dir>` (sempre a partir de falha real,
//! nunca inventado).

use crate::compare::Expectation;
use crate::scenario::Scenario;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusCase {
    pub scenario: Scenario,
    pub expect: Expectation,
    /// Bug original: o quê, quando, onde foi corrigido.
    pub history: String,
}

/// Carrega todos os `*.json` sob `dir` RECURSIVAMENTE. Só diretórios com
/// conteúdo (a missão proíbe dirs vazios de enfeite — e aqui também não
/// os criamos).
pub fn load_dir(dir: &std::path::Path) -> Result<Vec<(String, CorpusCase)>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = std::fs::read_dir(&d).map_err(|e| e.to_string())?;
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().is_some_and(|x| x == "json") {
                let text = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
                let case: CorpusCase =
                    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))?;
                out.push((p.display().to_string(), case));
            }
        }
    }
    Ok(out)
}

/// Serializa um caso promovido (minimizado + expectativa do modelo).
pub fn serialize_promote(
    scenario: &Scenario,
    expect: &Expectation,
    history: &str,
) -> Result<String, String> {
    let case = CorpusCase {
        scenario: scenario.clone(),
        expect: expect.clone(),
        history: history.into(),
    };
    serde_json::to_string_pretty(&case).map_err(|e| e.to_string())
}

/// Reexecuta UM caso fileops (guest isolado em tmp próprio) e compara com
/// `expect`. Caminho único usado pelo CLI (`corpus`) e pelos testes.
pub fn replay_fileops_case(
    case: &CorpusCase,
    rine_bin: &str,
    timeout: std::time::Duration,
) -> crate::compare::Verdict {
    use crate::compare::Verdict;
    let ops: Vec<crate::model::RawOp> =
        serde_json::from_value(case.scenario.params["ops"].clone()).unwrap_or_default();
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
    // Drive mínimo a partir dos paths (C: se mencionado, senão tmp puro).
    let tmp = std::env::temp_dir().join(format!("rine-replay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let fsys = nt_file::DriveMap::new(
        [('C', tmp.to_string_lossy().into_owned())]
            .into_iter()
            .collect(),
        Some(tmp.to_string_lossy().into_owned()),
    );
    let mut model = crate::model::FileModel::new(&fsys, &setup_b);
    let expects = model.simulate(&fsys, &ops);
    let steps = expects_to_driver(&ops, &expects);
    let version = crate::exec::rine_version(rine_bin);
    let r = crate::exec::run_fileops_guest(
        rine_bin,
        &version,
        &tmp,
        &case.scenario.scenario_id,
        &steps,
        &setup_b,
        timeout,
    );
    let verdict = match r {
        Err(_) => Verdict::InfrastructureFailure,
        Ok(r) => {
            let n = crate::normalize::normalize(&r, &tmp);
            case.expect.check(
                &n,
                &tmp,
                &[],
                &case.scenario.scenario_id,
                &case.scenario.api,
            )
        }
    };
    let _ = std::fs::remove_dir_all(&tmp);
    verdict
}

/// Converte (op + expectativa do modelo) em passos do driver guest.
pub fn expects_to_driver(
    ops: &[crate::model::RawOp],
    expects: &[crate::model::FileExpect],
) -> Vec<pe::driver::FileOp> {
    use crate::model::RawOp;
    ops.iter()
        .zip(expects.iter())
        .map(|(op, e)| match op {
            RawOp::Create { path, access, disp } => pe::driver::FileOp::Create {
                path: path.clone(),
                access: *access,
                disp: *disp,
                expect_valid: e.valid,
            },
            RawOp::Write { len } => pe::driver::FileOp::Write {
                len: *len,
                expect_ok: e.ok,
                expect_written: e.count,
            },
            RawOp::Read { len } => pe::driver::FileOp::Read {
                len: *len,
                expect_ok: e.ok,
                expect_count: e.count,
                verify: e.verify,
            },
            RawOp::Close => pe::driver::FileOp::Close { expect_ok: e.ok },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips() {
        let s = Scenario::new(
            "fileops",
            "CreateFileA",
            1,
            serde_json::json!({}),
            serde_json::json!({}),
        );
        let e = Expectation {
            exit_code: Some(0),
            stdout_contains: vec![],
            files: Default::default(),
        };
        let text = serialize_promote(&s, &e, "demo").unwrap();
        let back: CorpusCase = serde_json::from_str(&text).unwrap();
        assert_eq!(back.scenario.scenario_id, s.scenario_id);
    }

    #[test]
    fn missing_dir_errors() {
        assert!(load_dir(std::path::Path::new("/rine-inexistente-xyz")).is_err());
    }
}
