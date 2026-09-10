//! Relatórios: agregação por veredito, top mismatches, cobertura
//! comportamental e merge com `compatibility/matrix.json`.
//!
//! Duas saídas sempre: humana (stdout) e máquina (`report.json` no run dir).
//! Números nunca fictícios: tudo contado a partir dos resultados reais.

use crate::compare::Verdict;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseRecord {
    pub index: usize,
    pub scenario_id: String,
    pub api: String,
    pub seed: u64,
    pub verdict: Verdict,
    pub detail: String,
    pub seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCell {
    pub dimension: String,
    pub value: String,
    pub seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignReport {
    pub schema: u32,
    pub target: String,
    pub api: String,
    pub seed: u64,
    pub cases: usize,
    pub workers: usize,
    pub rine_version: String,
    pub per_verdict: HashMap<String, usize>,
    pub top_mismatches: Vec<TopMismatch>,
    pub coverage: Vec<CoverageCell>,
    pub cases_per_sec: f64,
    pub generated_at_unix: u64,
    /// Vereditos por caso (ordenados por índice): torna o relatório
    /// autocontido e o determinismo verificável sem reexecutar.
    pub records: Vec<CaseRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopMismatch {
    pub api: String,
    pub detail: String,
    pub count: usize,
    pub example_scenario: String,
}

/// Agrega `records` em relatório. `coverage_dims`: valores (dim, valor) vistos.
/// Parâmetros de agregação (struct em vez de 8 args — `too_many_arguments`
/// se resolve com tipos, não com `allow`).
pub struct AggregateCfg<'a> {
    pub target: &'a str,
    pub api: &'a str,
    pub seed: u64,
    pub workers: usize,
    pub rine_version: &'a str,
    pub elapsed_secs: f64,
}

pub fn aggregate(
    cfg: &AggregateCfg,
    records: &[CaseRecord],
    coverage_dims: &[(String, String)],
) -> CampaignReport {
    let mut per_verdict: HashMap<String, usize> = HashMap::new();
    for r in records {
        *per_verdict.entry(format!("{:?}", r.verdict)).or_insert(0) += 1;
    }
    // Agrupa mismatches por (api, detail).
    let mut groups: HashMap<(String, String), (usize, String)> = HashMap::new();
    for r in records
        .iter()
        .filter(|r| r.verdict == Verdict::SemanticMismatch)
    {
        let e = groups
            .entry((r.api.clone(), r.detail.clone()))
            .or_insert((0, r.scenario_id.clone()));
        e.0 += 1;
    }
    let mut top: Vec<TopMismatch> = groups
        .into_iter()
        .map(|((api, detail), (count, example_scenario))| TopMismatch {
            api,
            detail,
            count,
            example_scenario,
        })
        .collect();
    top.sort_by(|a, b| b.count.cmp(&a.count).then(a.detail.cmp(&b.detail)));
    top.truncate(10);

    let mut cov: HashMap<(String, String), u64> = HashMap::new();
    for (d, v) in coverage_dims {
        *cov.entry((d.clone(), v.clone())).or_insert(0) += 1;
    }
    let mut coverage: Vec<CoverageCell> = cov
        .into_iter()
        .map(|((dimension, value), seen)| CoverageCell {
            dimension,
            value,
            seen,
        })
        .collect();
    coverage.sort_by(|a, b| (&a.dimension, &a.value).cmp(&(&b.dimension, &b.value)));

    CampaignReport {
        schema: 1,
        target: cfg.target.into(),
        api: cfg.api.into(),
        seed: cfg.seed,
        cases: records.len(),
        workers: cfg.workers,
        rine_version: cfg.rine_version.into(),
        per_verdict,
        top_mismatches: top,
        coverage,
        cases_per_sec: if cfg.elapsed_secs > 0.0 {
            records.len() as f64 / cfg.elapsed_secs
        } else {
            0.0
        },
        generated_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        records: records.to_vec(),
    }
}

/// Resumo humano (espelha o exemplo da missão).
pub fn render_human(rep: &CampaignReport) -> String {
    let mut s = format!("API: {}\nCases: {}\n", rep.api, rep.cases);
    for v in [
        Verdict::Match,
        Verdict::KnownDifference,
        Verdict::SemanticMismatch,
        Verdict::Crash,
        Verdict::Timeout,
        Verdict::OracleFailure,
        Verdict::RineFailure,
        Verdict::InfrastructureFailure,
    ] {
        let n = rep.per_verdict.get(&format!("{v:?}")).copied().unwrap_or(0);
        s.push_str(&format!("{v:?}: {n}\n"));
    }
    if !rep.top_mismatches.is_empty() {
        s.push_str("\nTop mismatches:\n");
        for m in &rep.top_mismatches {
            s.push_str(&format!(
                "- {} ({}×, ex. {})\n",
                m.detail, m.count, m.example_scenario
            ));
        }
    }
    s.push_str(&format!("{:.1} cases/sec\n", rep.cases_per_sec));
    s
}

/// Funde estatísticas no `compatibility/matrix.json` (aditivo: nunca apaga
/// entradas existentes; cria `campaigns[]` se ausente).
pub fn merge_matrix(matrix_path: &std::path::Path, rep: &CampaignReport) -> Result<(), String> {
    let text = std::fs::read_to_string(matrix_path).map_err(|e| e.to_string())?;
    let mut v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let entry = serde_json::json!({
        "api": rep.api,
        "target": rep.target,
        "scenarios": rep.cases,
        "matches": rep.per_verdict.get("Match").copied().unwrap_or(0),
        "known_differences": rep.per_verdict.get("KnownDifference").copied().unwrap_or(0),
        "mismatches": rep.per_verdict.get("SemanticMismatch").copied().unwrap_or(0),
        "crashes": rep.per_verdict.get("Crash").copied().unwrap_or(0),
        "timeouts": rep.per_verdict.get("Timeout").copied().unwrap_or(0),
        "rine_version": rep.rine_version,
        "seed": rep.seed,
        "generated_at_unix": rep.generated_at_unix,
    });
    let arr = v
        .get_mut("campaigns")
        .and_then(|c| c.as_array_mut())
        .ok_or_else(|| "matrix.json sem array campaigns (atualizar formato)".to_string())?;
    arr.retain(|e| !(e["api"] == rep.api.as_str() && e["target"] == rep.target.as_str()));
    arr.push(entry);
    std::fs::write(
        matrix_path,
        serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(v: Verdict, detail: &str) -> CaseRecord {
        CaseRecord {
            index: 0,
            scenario_id: "s".into(),
            api: "CreateFileA".into(),
            seed: 1,
            verdict: v,
            detail: detail.into(),
            seconds: 0.01,
        }
    }

    #[test]
    fn aggregates_and_renders() {
        let records = vec![
            rec(Verdict::Match, ""),
            rec(Verdict::Match, ""),
            rec(Verdict::SemanticMismatch, "exit_code"),
        ];
        let rep = aggregate(
            &AggregateCfg {
                target: "fileops",
                api: "CreateFileA",
                seed: 1,
                workers: 2,
                rine_version: "v",
                elapsed_secs: 1.0,
            },
            &records,
            &[],
        );
        assert_eq!(rep.per_verdict["Match"], 2);
        assert_eq!(rep.top_mismatches.len(), 1);
        assert_eq!(rep.top_mismatches[0].count, 1);
        let h = render_human(&rep);
        assert!(h.contains("Cases: 3") && h.contains("SemanticMismatch: 1"));
    }

    #[test]
    fn merge_matrix_is_additive() {
        let dir = std::env::temp_dir().join(format!("rine-matrix-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("matrix.json");
        std::fs::write(&p, r#"{"schema":1,"entries":[],"campaigns":[]}"#).unwrap();
        let records = vec![rec(Verdict::Match, "")];
        let rep = aggregate(
            &AggregateCfg {
                target: "fileops",
                api: "CreateFileA",
                seed: 1,
                workers: 1,
                rine_version: "v",
                elapsed_secs: 1.0,
            },
            &records,
            &[],
        );
        merge_matrix(&p, &rep).unwrap();
        merge_matrix(&p, &rep).unwrap(); // idempotente (substitui, não duplica)
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(v["campaigns"].as_array().unwrap().len(), 1);
        assert_eq!(v["entries"].as_array().unwrap().len(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
