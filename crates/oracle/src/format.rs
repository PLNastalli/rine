//! Formato versionado de resultado do oracle (`schema = 1`).
//!
//! Produtores: runner Windows (C, `tests/windows/oracle_probe.c`) e runner
//! Rine (`runner::run_case`). Leitores devem rejeitar `schema` desconhecido.
//! NUNCA commitar resultados Windows fictícios — só medidos.

use serde::{Deserialize, Serialize};

/// Versão do formato.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OracleResult {
    pub schema: u32,
    /// Nome do teste (ex.: `"createfile-invalid-path"`).
    pub test: String,
    /// Lado que produziu: `"rine"` ou `"windows"`.
    pub side: String,
    /// Versão do produtor (`rine --version` / build do Windows, ex. `"26100"`).
    /// `None` = desconhecido (nunca inventar).
    pub producer_version: Option<String>,
    /// Arquitetura do guest (`"x86_64"`).
    pub architecture: String,
    /// Exit code do processo; `None` = morto por sinal/crash.
    pub exit_code: Option<i32>,
    /// Sinal Linux (`"SIGSEGV"`) ou código de crash Windows, se houve.
    pub crash: Option<String>,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    /// stdout/stderr como string quando UTF-8 (auxílio; bytes mandam).
    pub stdout_text: Option<String>,
    pub stderr_text: Option<String>,
    pub observations: Observations,
    /// Arquivos criados/modificados durante o teste (relativos ao CWD).
    pub files_created: Vec<String>,
    /// `true` se o runner matou o processo por timeout (distinto de crash:
    /// exit_code/crash ficam `None`). Aditivo e opcional: JSON schema 1
    /// antigo (sem o campo) parseia como `false`.
    #[serde(default)]
    pub timed_out: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Observations {
    /// Valor de retorno da API sob teste, em hex (`"0x0"`, `"0xFFFFFFFF"`…).
    pub return_value: Option<String>,
    pub get_last_error: Option<u32>,
    pub ntstatus: Option<String>,
}

impl OracleResult {
    pub fn new_rine(test: &str, producer_version: &str) -> Self {
        Self {
            schema: SCHEMA,
            test: test.into(),
            side: "rine".into(),
            producer_version: Some(producer_version.into()),
            architecture: "x86_64".into(),
            exit_code: None,
            crash: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_text: None,
            stderr_text: None,
            observations: Observations::default(),
            files_created: Vec::new(),
            timed_out: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_roundtrips() {
        let mut r = OracleResult::new_rine("x", "0.2.0-alpha.1");
        r.exit_code = Some(11);
        r.observations.get_last_error = Some(2);
        let s = serde_json::to_string(&r).unwrap();
        let back: OracleResult = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
        assert_eq!(back.schema, SCHEMA);
    }

    #[test]
    fn schema_is_checked_by_readers() {
        // Um resultado v1 parseia e carimba schema 1.
        let r = OracleResult::new_rine("x", "0.2.0-alpha.1");
        let s = serde_json::to_string(&r).unwrap();
        let back: OracleResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back.schema, SCHEMA);
        // JSON sem os campos obrigatórios falha em vez de inventar valores.
        assert!(serde_json::from_str::<OracleResult>(r#"{"schema":99}"#).is_err());
    }
}
