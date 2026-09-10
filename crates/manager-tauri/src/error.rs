//! Erro de command: `{code, message}` para o React (nunca string solta).

use manager_core::ManagerError;
use serde::Serialize;

/// Erro serializável dos Tauri commands.
#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    /// Código estável (`RUNTIME_NOT_FOUND`, …).
    pub code: String,
    /// Mensagem humana (do `ManagerError`, sem vazar paths internos além do útil).
    pub message: String,
}

impl From<ManagerError> for CmdError {
    fn from(e: ManagerError) -> Self {
        Self {
            code: e.code().to_string(),
            message: e.to_string(),
        }
    }
}
