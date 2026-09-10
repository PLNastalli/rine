//! Comparador diferencial: `Expectation` × `OracleResult` → `Verdict`.
//!
//! Taxonomia fechada (nunca reduzir a PASS/FAIL internamente):
//! - `Match`: tudo equivalente após normalização.
//! - `KnownDifference`: bate com entrada documentada (motivo + issue).
//! - `SemanticMismatch`: divergência real — candidata a bug.
//! - `Crash`: processo morreu por sinal/exceção.
//! - `Timeout`: morto por timeout (distinto de crash! loop ≠ segfault).
//! - `OracleFailure`: o oracle não produziu resultado (ex.: baseline ausente).
//! - `RineFailure`: o Rine falhou antes de produzir (spawn/load error).
//! - `InfrastructureFailure`: disco cheio, OOM do runner, bug da plataforma.

use oracle::format::OracleResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Match,
    KnownDifference,
    SemanticMismatch,
    Crash,
    Timeout,
    OracleFailure,
    RineFailure,
    InfrastructureFailure,
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Match | Verdict::KnownDifference)
    }
}

/// O que o modelo/oracle gravado ESPERA (construído pelo modelo, nunca à mão
/// para campanhas; à mão só em `tests/regression/` com revisão).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Expectation {
    pub exit_code: Option<i32>,
    /// Substrings que o stdout deve conter (todas).
    pub stdout_contains: Vec<String>,
    /// Arquivos finais `{nome: bytes}`. Mapa ordenado (determinístico).
    pub files: std::collections::BTreeMap<String, Vec<u8>>,
}

impl Expectation {
    /// Compara um resultado NORMALIZADO + conteúdo real dos arquivos.
    pub fn check(
        &self,
        r: &OracleResult,
        workdir: &std::path::Path,
        known: &[KnownDifference],
        scenario_id: &str,
        api: &str,
    ) -> Verdict {
        if r.timed_out {
            return Verdict::Timeout;
        }
        if r.crash.is_some() {
            return Verdict::Crash;
        }
        if r.exit_code != self.exit_code {
            return known_or_mismatch(known, scenario_id, api, "exit_code");
        }
        let stdout = r.stdout_text.as_deref().unwrap_or("");
        if !self.stdout_contains.iter().all(|s| stdout.contains(s)) {
            return known_or_mismatch(known, scenario_id, api, "stdout");
        }
        for (name, want) in &self.files {
            match std::fs::read(workdir.join(name)) {
                Ok(got) if &got == want => {}
                _ => return known_or_mismatch(known, scenario_id, api, "files"),
            }
        }
        // Arquivos extras inesperados também divergem (efeito colateral).
        let mut listed: Vec<String> = r.files_created.clone();
        listed.sort();
        let mut expected: Vec<String> = self.files.keys().cloned().collect();
        expected.sort();
        // `files_created` do runner inclui o próprio .exe/capsule do harness;
        // compara só interseção relevante: todo esperado existe; extras que
        // parecem teste (`.txt`) e não estão no esperado divergem.
        for name in &listed {
            if name.ends_with(".txt") && !self.files.contains_key(name) {
                return known_or_mismatch(known, scenario_id, api, "extra-files");
            }
        }
        let _ = expected;
        Verdict::Match
    }
}

fn known_or_mismatch(
    known: &[KnownDifference],
    scenario_id: &str,
    api: &str,
    aspect: &str,
) -> Verdict {
    if known.iter().any(|k| k.matches(scenario_id, api, aspect)) {
        Verdict::KnownDifference
    } else {
        Verdict::SemanticMismatch
    }
}

/// Diferença conhecida e documentada (nunca "ignorar para passar").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownDifference {
    /// Prefixo de `scenario_id` ou `"*"` (qualquer).
    pub scenario_prefix: String,
    /// API ou `"*"`; aspecto (`"exit_code"`, `"stdout"`, `"files"`, …) ou `"*"`.
    pub api: String,
    pub aspect: String,
    /// Por que difere (semântica real) + condição de remoção.
    pub reason: String,
}

impl KnownDifference {
    pub fn matches(&self, scenario_id: &str, api: &str, aspect: &str) -> bool {
        (self.scenario_prefix == "*" || scenario_id.starts_with(&self.scenario_prefix))
            && (self.api == "*" || self.api == api)
            && (self.aspect == "*" || self.aspect == aspect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_with(exit: Option<i32>, stdout: &str) -> OracleResult {
        let mut r = OracleResult::new_rine("t", "v");
        r.exit_code = exit;
        r.stdout_text = Some(stdout.into());
        r
    }

    fn expect(code: i32) -> Expectation {
        Expectation {
            exit_code: Some(code),
            stdout_contains: vec![],
            files: Default::default(),
        }
    }

    #[test]
    fn match_and_mismatch() {
        let dir = std::env::temp_dir();
        assert_eq!(
            expect(0).check(&result_with(Some(0), ""), &dir, &[], "s", "a"),
            Verdict::Match
        );
        assert_eq!(
            expect(0).check(&result_with(Some(1), ""), &dir, &[], "s", "a"),
            Verdict::SemanticMismatch
        );
    }

    #[test]
    fn timeout_crash_distinct() {
        let dir = std::env::temp_dir();
        let mut t = result_with(None, "");
        t.timed_out = true;
        assert_eq!(expect(0).check(&t, &dir, &[], "s", "a"), Verdict::Timeout);
        let mut c = result_with(None, "");
        c.crash = Some("SIGSEGV".into());
        assert_eq!(expect(0).check(&c, &dir, &[], "s", "a"), Verdict::Crash);
        // known_difference absorve mismatch documentado
        let known = vec![KnownDifference {
            scenario_prefix: "s".into(),
            api: "*".into(),
            aspect: "*".into(),
            reason: "demo".into(),
        }];
        assert_eq!(
            expect(0).check(&result_with(Some(9), ""), &dir, &known, "s-1", "a"),
            Verdict::KnownDifference
        );
        assert!(Verdict::Match.is_pass() && Verdict::KnownDifference.is_pass());
        assert!(!Verdict::SemanticMismatch.is_pass());
    }
}
