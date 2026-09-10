//! Preferências da GUI (só UI; nada do runtime mora aqui).

use crate::error::{io_err, ManagerError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Versão do schema de `settings.json`.
pub const SETTINGS_SCHEMA: u32 = 1;

/// Aparência (`system` = segue o desktop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Segue o desktop.
    System,
    /// Escuro (default do Rine Manager).
    Dark,
    /// Claro.
    Light,
}

/// Nível de log exibido na UI (`Info+` default; Debug/Trace no developer mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Erros.
    Error,
    /// Avisos e acima.
    Warn,
    /// Informativo e acima (default).
    Info,
    /// Depuração (developer mode).
    Debug,
    /// Rastro (developer mode).
    Trace,
}

/// Preferências persistidas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Schema (migração explícita).
    pub schema: u32,
    /// Aparência.
    pub appearance: Appearance,
    /// Diretório pai default para novas capsules.
    pub default_capsule_dir: PathBuf,
    /// Confirmar ações destrutivas.
    pub confirm_destructive: bool,
    /// Nível de log da UI.
    pub log_level: LogLevel,
    /// Nº de runs retidos em Diagnostics.
    pub retain_runs: usize,
    /// Ferramentas de desenvolvedor (difftest/bench/oracle).
    pub developer_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        Self {
            schema: SETTINGS_SCHEMA,
            appearance: Appearance::Dark,
            default_capsule_dir: PathBuf::from(home).join("Rine/capsules"),
            confirm_destructive: true,
            log_level: LogLevel::Info,
            retain_runs: 50,
            developer_mode: false,
        }
    }
}

impl Settings {
    /// Caminho padrão (`~/.local/share/rine-manager/settings.json`).
    pub fn default_path() -> Result<PathBuf, ManagerError> {
        Ok(super::runtime::data_dir()?.join("settings.json"))
    }

    /// Carrega (ausente = default; schema diferente = erro explícito).
    pub fn load(path: &std::path::Path) -> Result<Self, ManagerError> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let s: Settings = serde_json::from_str(&text)
                    .map_err(|e| ManagerError::InvalidRegistry(format!("settings: {e}")))?;
                if s.schema != SETTINGS_SCHEMA {
                    return Err(ManagerError::InvalidRegistry(format!(
                        "settings schema {} (esperado {SETTINGS_SCHEMA})",
                        s.schema
                    )));
                }
                Ok(s)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(io_err("ler settings", e)),
        }
    }

    /// Salva (escrita atômica via temporário + rename).
    pub fn save(&self, path: &std::path::Path) -> Result<(), ManagerError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_err("criar datadir", e))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| ManagerError::InvalidRegistry(format!("settings: {e}")))?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| io_err("escrever settings", e))?;
        std::fs::rename(&tmp, path).map_err(|e| io_err("publicar settings", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrip() {
        let d = std::env::temp_dir().join(format!("rine-set-{}-rt", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("settings.json");
        let s = Settings::load(&p).unwrap(); // ausente = default
        assert_eq!(s.appearance, Appearance::Dark);
        assert!(!s.developer_mode);
        let mut s2 = s;
        s2.appearance = Appearance::Light;
        s2.save(&p).unwrap();
        assert_eq!(Settings::load(&p).unwrap().appearance, Appearance::Light);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
