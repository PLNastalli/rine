//! Erros tipados do Manager (nunca "something went wrong").
//!
//! Cada variante vira `{human, code, details?}` no React. Nenhum
//! `unwrap/expect` em caminho de input: arquivo, path, Capsule, runtime
//! e config do frontend podem falhar e falham de forma explícita.

use thiserror::Error;

/// Erro de gestão (fronteira Manager ↔ usuário/runtime).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManagerError {
    /// Binário `rine` não encontrado ou inválido.
    #[error("runtime Rine não encontrado: {0}")]
    RuntimeNotFound(String),
    /// Versão do runtime incompatível com o Manager (nunca segue silencioso).
    #[error("incompatibilidade de versão (manager {manager}, runtime {runtime})")]
    ProtocolMismatch {
        /// Versão esperada pelo Manager (workspace atual).
        manager: String,
        /// Versão reportada pelo binário.
        runtime: String,
    },
    /// Bytes não são um PE x86_64 suportado.
    #[error("PE não suportado: {0}")]
    UnsupportedPe(String),
    /// Capsule ilegível/inválida (nunca default silencioso).
    #[error("capsule inválida ({path}): {reason}")]
    InvalidCapsule {
        /// Arquivo que falhou.
        path: String,
        /// Motivo (do `runtime`, sem adivinhação).
        reason: String,
    },
    /// Execução recusada ou falhada no load (com a demanda real anexada).
    #[error("bloqueado por imports ({missing} em falta)")]
    ImportBlocked {
        /// Nº de imports sem implementação.
        missing: usize,
    },
    /// Runtime executou e saiu com código != 0 (sem crash estruturado).
    #[error("runtime saiu com código {code}")]
    RuntimeFailed {
        /// Exit code do processo `rine`.
        code: i32,
    },
    /// Registro/biblioteca inconsistente.
    #[error("registro inválido: {0}")]
    InvalidRegistry(String),
    /// I/O (disco, spawn) com contexto.
    #[error("io ({op}): {reason}")]
    Io {
        /// Operação (`ler exe`, `spawn rine`, …).
        op: String,
        /// Motivo do SO.
        reason: String,
    },
}

/// Código estável para o frontend (nunca só a mensagem humana).
impl ManagerError {
    /// `RUNTIME_NOT_FOUND | PROTOCOL_MISMATCH | UNSUPPORTED_PE | …`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::RuntimeNotFound(_) => "RUNTIME_NOT_FOUND",
            Self::ProtocolMismatch { .. } => "PROTOCOL_MISMATCH",
            Self::UnsupportedPe(_) => "UNSUPPORTED_PE",
            Self::InvalidCapsule { .. } => "INVALID_CAPSULE",
            Self::ImportBlocked { .. } => "IMPORT_BLOCKED",
            Self::RuntimeFailed { .. } => "RUNTIME_FAILED",
            Self::InvalidRegistry(_) => "INVALID_REGISTRY",
            Self::Io { .. } => "IO",
        }
    }
}

pub(crate) fn io_err(op: &str, e: std::io::Error) -> ManagerError {
    ManagerError::Io {
        op: op.into(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(
            ManagerError::RuntimeNotFound("x".into()).code(),
            "RUNTIME_NOT_FOUND"
        );
        assert_eq!(
            ManagerError::ProtocolMismatch {
                manager: "a".into(),
                runtime: "b".into(),
            }
            .code(),
            "PROTOCOL_MISMATCH"
        );
    }
}
