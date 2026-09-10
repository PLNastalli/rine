//! Motor de comparação puro: baseline × medição → veredito por benchmark.
//!
//! Determinístico e sem I/O: toda a lógica de decisão testável sem executar
//! nada. Coleta (`rine-bench run`) e decisão nunca se misturam.

use crate::schema::{Baseline, RunResults};
use serde::{Deserialize, Serialize};

/// Política de thresholds. `alpha()` = a vigente (aviso, sem bloqueio);
/// `stable()` = a futura (5% injustificado = FAIL). Trocar de política é
/// trocar uma constante + documentar — arquitetura pronta desde o dia 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Policy {
    /// Acima disso (e até `regression_pct`): WARNING.
    pub warn_pct: f64,
    /// Acima disso: REGRESSION.
    pub regression_pct: f64,
    /// Se `true`, REGRESSION vira FAIL (exit nonzero no CI).
    pub strict: bool,
}

impl Policy {
    pub fn alpha() -> Self {
        Self {
            warn_pct: 10.0,
            regression_pct: 15.0,
            strict: false,
        }
    }

    pub fn stable() -> Self {
        Self {
            warn_pct: 5.0,
            regression_pct: 5.0,
            strict: true,
        }
    }
}

/// Veredito por benchmark. Melhora também é PASS (com nota no relatório).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Pass,
    Warning,
    Regression,
    /// No resultado atual, mas na baseline: reportado, nunca bloqueia
    /// (benchmark novo não pode quebrar CI).
    Missing,
    /// Schema/metodologia/ID incompatíveis: não comparar (ver `detail`).
    /// Também nunca bloqueia sozinho — mas exige ação humana documentada.
    Incompatible,
    /// REGRESSION sob `Policy { strict: true }`.
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Verdict {
    pub id: String,
    pub status: Status,
    /// `((current - baseline) / baseline) * 100` sobre medianas.
    /// `None` quando incalculável (ver `detail`).
    pub change_pct: Option<f64>,
    pub baseline_median_ns: Option<f64>,
    pub current_median_ns: Option<f64>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Comparison {
    pub policy: String,
    pub env_match: bool,
    pub env_note: String,
    pub verdicts: Vec<Verdict>,
}

/// Compara tudo. Pura: sem fs, sem tempo, sem aleatoriedade.
pub fn compare(baseline: &Baseline, current: &RunResults, policy: Policy) -> Comparison {
    if baseline.schema != crate::schema::SCHEMA || current.schema != crate::schema::SCHEMA {
        return Comparison {
            policy: policy_name(policy),
            env_match: false,
            env_note: "schema incompatível: atualize baseline e runner juntos".into(),
            verdicts: vec![],
        };
    }
    let (env_match, env_note) = check_env(&baseline.env, &current.env);
    let mut verdicts = Vec::new();
    for (id, base) in &baseline.benchmarks {
        let Some(cur) = current.benchmarks.get(id) else {
            verdicts.push(Verdict {
                id: id.clone(),
                status: Status::Missing,
                change_pct: None,
                baseline_median_ns: Some(base.median_ns),
                current_median_ns: None,
                detail: "ausente no resultado atual (benchmark novo/removido?)".into(),
            });
            continue;
        };
        if base.method_version != cur.method_version {
            verdicts.push(Verdict {
                id: id.clone(),
                status: Status::Incompatible,
                change_pct: None,
                baseline_median_ns: Some(base.median_ns),
                current_median_ns: Some(cur.median_ns),
                detail: format!(
                    "metodologia divergiu (baseline v{} vs atual v{}): não comparar, versionar ID ou re-baselinar",
                    base.method_version, cur.method_version
                ),
            });
            continue;
        }
        if base.median_ns <= 0.0 {
            verdicts.push(Verdict {
                id: id.clone(),
                status: Status::Incompatible,
                change_pct: None,
                baseline_median_ns: Some(base.median_ns),
                current_median_ns: Some(cur.median_ns),
                detail: "baseline degenerada (mediana <= 0)".into(),
            });
            continue;
        }
        let pct = (cur.median_ns - base.median_ns) / base.median_ns * 100.0;
        let mut status = if pct <= policy.warn_pct {
            Status::Pass
        } else if pct <= policy.regression_pct {
            Status::Warning
        } else {
            Status::Regression
        };
        if status == Status::Regression && policy.strict {
            status = Status::Fail;
        }
        let detail = if pct < 0.0 {
            format!("melhora de {pct:.1}% (bem-vinda, mas confira que nada foi pulado)")
        } else {
            String::new()
        };
        verdicts.push(Verdict {
            id: id.clone(),
            status,
            change_pct: Some(pct),
            baseline_median_ns: Some(base.median_ns),
            current_median_ns: Some(cur.median_ns),
            detail,
        });
    }
    verdicts.sort_by(|a, b| a.id.cmp(&b.id));
    Comparison {
        policy: policy_name(policy),
        env_match,
        env_note,
        verdicts,
    }
}

fn policy_name(p: Policy) -> String {
    if p.strict {
        format!(
            "strict(warn>{:.0}%,fail>{:.0}%)",
            p.warn_pct, p.regression_pct
        )
    } else {
        format!(
            "alpha(warn>{:.0}%,regression>{:.0}%,warn-only)",
            p.warn_pct, p.regression_pct
        )
    }
}

/// Compara ambientes campo a campo. Qualquer diferença → `env_match=false`
/// com nota (nunca falha por isso: máquinas diferentes não são equivalentes,
/// mas o relatório deve gritar).
fn check_env(
    base: &crate::schema::Environment,
    cur: &crate::schema::Environment,
) -> (bool, String) {
    let mut diffs: Vec<String> = Vec::new();
    if base.cpu != cur.cpu {
        diffs.push("cpu".into());
    }
    if base.kernel != cur.kernel {
        diffs.push("kernel".into());
    }
    if base.toolchain != cur.toolchain {
        diffs.push("toolchain".into());
    }
    if base.profile != cur.profile {
        diffs.push("profile".into());
    }
    if base.commit != cur.commit {
        diffs.push(format!(
            "commit({}→{})",
            short(&base.commit),
            short(&cur.commit)
        ));
    }
    if diffs.is_empty() {
        (true, "mesmo ambiente da baseline".into())
    } else {
        (
            false,
            format!(
                "ambiente difere ({}): comparar com cautela",
                diffs.join(", ")
            ),
        )
    }
}

fn short(s: &str) -> String {
    s.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Environment, Measurement};
    use std::collections::BTreeMap;

    fn env() -> Environment {
        Environment {
            commit: "abc".into(),
            rine_version: "0.2.0-alpha.1".into(),
            cpu: "c".into(),
            kernel: "k".into(),
            toolchain: "t".into(),
            profile: "debug".into(),
            rustflags: "default".into(),
            timestamp: 0,
        }
    }

    fn meas(median: f64) -> Measurement {
        Measurement {
            id: "b".into(),
            scope: "s".into(),
            method_version: 1,
            samples_ns: vec![median as u64],
            mean_ns: median,
            median_ns: median,
            stddev_ns: 0.0,
        }
    }

    fn cmp(base: f64, cur: f64) -> Status {
        let mut b = BTreeMap::new();
        b.insert("b".into(), meas(base));
        let mut c = BTreeMap::new();
        c.insert("b".into(), meas(cur));
        let baseline = Baseline {
            schema: 1,
            env: env(),
            benchmarks: b,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let current = RunResults {
            schema: 1,
            env: env(),
            benchmarks: c,
        };
        let r = compare(&baseline, &current, Policy::alpha());
        assert!(r.env_match);
        assert_eq!(r.verdicts.len(), 1);
        r.verdicts[0].status
    }

    #[test]
    fn melhora_e_igualdade_passam() {
        assert_eq!(cmp(100.0, 80.0), Status::Pass); // −20%
        assert_eq!(cmp(100.0, 100.0), Status::Pass); // 0%
    }

    #[test]
    fn faixas_alpha() {
        assert_eq!(cmp(100.0, 105.0), Status::Pass); // +5%
        assert_eq!(cmp(100.0, 110.0), Status::Pass); // +10% (borda incluída)
        assert_eq!(cmp(100.0, 112.0), Status::Warning); // +12%
        assert_eq!(cmp(100.0, 115.0), Status::Warning); // +15% (borda incluída)
        assert_eq!(cmp(100.0, 120.0), Status::Regression); // +20%
    }

    #[test]
    fn strict_transforma_regression_em_fail() {
        let mut b = BTreeMap::new();
        b.insert("b".into(), meas(100.0));
        let mut c = BTreeMap::new();
        c.insert("b".into(), meas(120.0));
        let baseline = Baseline {
            schema: 1,
            env: env(),
            benchmarks: b,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let current = RunResults {
            schema: 1,
            env: env(),
            benchmarks: c,
        };
        let r = compare(&baseline, &current, Policy::stable());
        assert_eq!(r.verdicts[0].status, Status::Fail);
        // Sob stable, +6% já falha (política futura pronta).
        let mut c6 = BTreeMap::new();
        c6.insert("b".into(), meas(106.0));
        let current6 = RunResults {
            schema: 1,
            env: env(),
            benchmarks: c6,
        };
        let r6 = compare(&baseline, &current6, Policy::stable());
        assert_eq!(r6.verdicts[0].status, Status::Fail);
        let r2 = compare(&baseline, &current, Policy::alpha());
        assert_eq!(r2.verdicts[0].status, Status::Regression);
    }

    #[test]
    fn benchmark_ausente_nao_bloqueia() {
        let mut b = BTreeMap::new();
        b.insert("b".into(), meas(100.0));
        b.insert("novo".into(), meas(50.0));
        let mut c = BTreeMap::new();
        c.insert("b".into(), meas(100.0));
        let baseline = Baseline {
            schema: 1,
            env: env(),
            benchmarks: b,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let current = RunResults {
            schema: 1,
            env: env(),
            benchmarks: c,
        };
        let r = compare(&baseline, &current, Policy::alpha());
        let v = r.verdicts.iter().find(|v| v.id == "novo").unwrap();
        assert_eq!(v.status, Status::Missing);
        assert_eq!(v.change_pct, None);
    }

    #[test]
    fn baseline_incompativel() {
        // Schema divergente: zero vereditos + nota (nunca crash).
        let mut b = BTreeMap::new();
        b.insert("b".into(), meas(100.0));
        let baseline = Baseline {
            schema: 999,
            env: env(),
            benchmarks: b,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let current = RunResults {
            schema: 1,
            env: env(),
            benchmarks: BTreeMap::new(),
        };
        let r = compare(&baseline, &current, Policy::alpha());
        assert!(r.verdicts.is_empty());
        assert!(!r.env_match);
        // Metodologia divergente: Incompatible por benchmark.
        let mut b2 = BTreeMap::new();
        let mut m = meas(100.0);
        m.method_version = 2;
        b2.insert("b".into(), m);
        let baseline2 = Baseline {
            schema: 1,
            env: env(),
            benchmarks: b2,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let mut c2 = BTreeMap::new();
        c2.insert("b".into(), meas(100.0));
        let current2 = RunResults {
            schema: 1,
            env: env(),
            benchmarks: c2,
        };
        let r2 = compare(&baseline2, &current2, Policy::alpha());
        assert_eq!(r2.verdicts[0].status, Status::Incompatible);
        // Baseline degenerada: Incompatible, sem divisão por zero.
        let mut b3 = BTreeMap::new();
        b3.insert("b".into(), meas(0.0));
        let baseline3 = Baseline {
            schema: 1,
            env: env(),
            benchmarks: b3,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let r3 = compare(&baseline3, &current2, Policy::alpha());
        assert_eq!(r3.verdicts[0].status, Status::Incompatible);
    }

    #[test]
    fn ambiente_diferente_avisa_sem_falhar() {
        let mut b = BTreeMap::new();
        b.insert("b".into(), meas(100.0));
        let mut e = env();
        e.cpu = "outra".into();
        let baseline = Baseline {
            schema: 1,
            env: e,
            benchmarks: b,
            promoted_by: "".into(),
            promoted_at_unix: 0,
            reason: "".into(),
            supersedes: None,
        };
        let mut c = BTreeMap::new();
        c.insert("b".into(), meas(100.0));
        let current = RunResults {
            schema: 1,
            env: env(),
            benchmarks: c,
        };
        let r = compare(&baseline, &current, Policy::alpha());
        assert!(!r.env_match);
        assert!(r.env_note.contains("cpu"));
        assert_eq!(r.verdicts[0].status, Status::Pass);
    }
}
