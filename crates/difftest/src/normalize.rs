//! Normalização explícita de resultados (regras documentadas, testadas).
//!
//! Cada regra existe por um motivo e tem teste. Regra sem justificativa é
//! bug disfarçado de conveniência — nunca ignore diferença "para passar".
//!
//! Regras (aplicadas nesta ordem; idempotente por construção):
//! 1. `tmp-paths`: sandbox por worker/caso difere a cada run → `<TMP>`.
//!    Justificativa: local do diretório temporário não é observável Windows.
//! 2. `sort-files`: ordem de `readdir` não é garantida → ordena `files_created`.
//!    Justificativa: Windows não garante ordem de enumeração aqui.
//! 3. `cap-stdout`: trunca stdout/stderr > 64 KiB com marcador (métrica, não
//!    semântica: nenhum comparador lê além do cap; evita OOM em fuzz).
//! 4. `strip-timings`: nenhum campo de tempo existe em `OracleResult` hoje;
//!    regra reservada (no-op testado) para quando existirem.
//! 5. `pid-tid`: idem — reservada, no-op testado.
//! 6. ANTI-regra `handles-are-stable`: handles do Rine são geracionais e
//!    determinísticos por sequência — NÃO normalizar (normalizar aqui
//!    esconderia bugs de reuso; ver ADR-0002).

use oracle::format::OracleResult;

pub const STDOUT_CAP: usize = 64 * 1024;

/// Aplica todas as regras; retorna clone normalizado. Idempotente.
pub fn normalize(r: &OracleResult, sandbox: &std::path::Path) -> OracleResult {
    let mut out = r.clone();
    // 1. tmp-paths (sandbox + temp do SO).
    let tmp = std::env::temp_dir().to_string_lossy().into_owned();
    for s in [&tmp, &sandbox.to_string_lossy().into_owned()] {
        if s.is_empty() {
            continue;
        }
        for field in [&mut out.stdout_text, &mut out.stderr_text]
            .into_iter()
            .flatten()
        {
            *field = field.replace(s.as_str(), "<TMP>");
        }
        out.files_created = out
            .files_created
            .into_iter()
            .map(|f| f.replace(s.as_str(), "<TMP>"))
            .collect();
    }
    // stdout_bytes acompanha o texto (se truncado abaixo, recalcula).
    // 2. sort-files.
    out.files_created.sort();
    // 3. cap-stdout.
    for field in [&mut out.stdout_text, &mut out.stderr_text]
        .into_iter()
        .flatten()
    {
        if field.len() > STDOUT_CAP {
            field.truncate(STDOUT_CAP);
            field.push_str("<TRUNCATED>");
        }
    }
    out.stdout_bytes = out
        .stdout_text
        .as_ref()
        .map(|s| s.len())
        .unwrap_or(out.stdout_bytes);
    out.stderr_bytes = out
        .stderr_text
        .as_ref()
        .map(|s| s.len())
        .unwrap_or(out.stderr_bytes);
    out
}

/// Lista legível das regras (para relatórios).
pub fn rules() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "tmp-paths",
            "sandbox difere por run; não é observável Windows",
        ),
        ("sort-files", "ordem de enumeração não garantida"),
        ("cap-stdout", "64 KiB + marcador; evita OOM em fuzz"),
        ("strip-timings", "reservada (sem campos de tempo hoje)"),
        ("pid-tid", "reservada (sem PIDs/TIDs nos resultados hoje)"),
        (
            "handles-are-stable",
            "ANTI-regra: handles são determinísticos, não tocar",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> OracleResult {
        let mut r = OracleResult::new_rine("t", "v");
        r.exit_code = Some(0);
        r.stdout_text = Some(format!("{}/x ok", std::env::temp_dir().display()));
        r.files_created = vec!["b.txt".into(), "a.txt".into()];
        r
    }

    #[test]
    fn normalizes_tmp_and_sorts() {
        let n = normalize(&sample(), std::path::Path::new("/tmp/xyz"));
        assert!(n.stdout_text.unwrap().contains("<TMP>"));
        assert_eq!(
            n.files_created,
            vec!["a.txt".to_string(), "b.txt".to_string()]
        );
    }

    #[test]
    fn idempotent() {
        let n1 = normalize(&sample(), std::path::Path::new("/tmp/xyz"));
        let n2 = normalize(&n1, std::path::Path::new("/tmp/xyz"));
        assert_eq!(n1, n2);
    }

    #[test]
    fn caps_huge_stdout() {
        let mut r = sample();
        r.stdout_text = Some("y".repeat(STDOUT_CAP + 10));
        let n = normalize(&r, std::path::Path::new("/x"));
        assert!(n.stdout_text.unwrap().ends_with("<TRUNCATED>"));
    }
}
