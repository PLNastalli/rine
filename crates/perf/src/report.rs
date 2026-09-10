//! Relatório humano + JSON da comparação.
//!
//! Formato espelha o exemplo da missão. Regressão imprime também o bloco
//! "Possible performance regression detected." por benchmark afetado.

use crate::compare::{Comparison, Status};

/// Renderiza o relatório legível por humanos.
pub fn render_human(cmp: &Comparison) -> String {
    let mut s = String::from("RINE PERFORMANCE REPORT\n\n");
    s.push_str(&format!("policy: {}\n", cmp.policy));
    s.push_str(&format!("env: {}\n\n", cmp.env_note));
    s.push_str(&format!(
        "{:<36}{:>12}{:>12}{:>10}  {}\n",
        "Benchmark", "Baseline", "Current", "Change", "Status"
    ));
    for v in &cmp.verdicts {
        s.push_str(&format!(
            "{:<36}{:>12}{:>12}{:>10}  {}\n",
            v.id,
            fmt_ns(v.baseline_median_ns),
            fmt_ns(v.current_median_ns),
            v.change_pct
                .map(|p| format!("{p:+.1}%"))
                .unwrap_or_else(|| "-".into()),
            status_str(v.status),
        ));
    }
    let regs: Vec<_> = cmp
        .verdicts
        .iter()
        .filter(|v| v.status == Status::Regression)
        .collect();
    if !regs.is_empty() {
        s.push_str("\nPossible performance regression detected.\n");
        for v in regs {
            s.push_str(&format!(
                "\nBenchmark: {}\nBaseline: {}\nCurrent: {}\nRegression: {}\n",
                v.id,
                fmt_ns(v.baseline_median_ns),
                fmt_ns(v.current_median_ns),
                v.change_pct
                    .map(|p| format!("{p:+.1}%"))
                    .unwrap_or_else(|| "-".into()),
            ));
        }
    }
    s
}

fn fmt_ns(v: Option<f64>) -> String {
    match v {
        None => "-".into(),
        Some(ns) if ns >= 1000.0 => format!("{:.3} us", ns / 1000.0),
        Some(ns) => format!("{ns:.0} ns"),
    }
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Pass => "PASS",
        Status::Warning => "WARNING",
        Status::Regression => "REGRESSION",
        Status::Missing => "MISSING",
        Status::Incompatible => "INCOMPATIBLE",
        Status::Fail => "FAIL",
    }
}

/// exit code para CI: 0 sempre em modo aviso; 1 se houver FAIL (strict),
/// 2 em erro de infraestrutura (comparação impossível de avaliar).
/// Missing/Incompatible/Regression(warn-only) nunca bloqueiam sozinhos.
pub fn exit_code(cmp: &Comparison, had_infra_error: bool) -> i32 {
    if had_infra_error {
        return 2;
    }
    if cmp.verdicts.iter().any(|v| v.status == Status::Fail) {
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::Verdict;

    fn verdict(id: &str, status: Status, pct: Option<f64>) -> Verdict {
        Verdict {
            id: id.into(),
            status,
            change_pct: pct,
            baseline_median_ns: Some(1000.0),
            current_median_ns: Some(1100.0),
            detail: String::new(),
        }
    }

    #[test]
    fn render_matches_mission_shape() {
        let cmp = Comparison {
            policy: "alpha".into(),
            env_match: true,
            env_note: "ok".into(),
            verdicts: vec![
                verdict("pe.parse.hello", Status::Pass, Some(1.9)),
                verdict("pe.parse.suite", Status::Warning, Some(12.6)),
                verdict("loader.full.suite", Status::Regression, Some(22.1)),
            ],
        };
        let h = render_human(&cmp);
        assert!(h.contains("RINE PERFORMANCE REPORT"));
        assert!(h.contains("pe.parse.hello"));
        assert!(h.contains("PASS"));
        assert!(h.contains("WARNING"));
        assert!(h.contains("REGRESSION"));
        assert!(h.contains("Possible performance regression detected."));
        assert!(h.contains("Benchmark: loader.full.suite"));
        assert_eq!(exit_code(&cmp, false), 0); // warn-only: não bloqueia
    }

    #[test]
    fn strict_fail_blocks() {
        let cmp = Comparison {
            policy: "strict".into(),
            env_match: true,
            env_note: String::new(),
            verdicts: vec![verdict("x", Status::Fail, Some(30.0))],
        };
        assert_eq!(exit_code(&cmp, false), 1);
        assert_eq!(exit_code(&cmp, true), 2);
    }
}
