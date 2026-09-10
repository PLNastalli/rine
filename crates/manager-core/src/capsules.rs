//! Capsules: leitura do schema v0.2 + criação sem inventar chaves.
//!
//! A GUI nunca escreve propriedade que o `runtime` não lê (ver
//! `docs/manager/runtime-interface.md`). Perfis futuros aparecem
//! `disabled/coming soon` na UI até o runtime suportá-los.

use crate::error::{io_err, ManagerError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Resumo para os cards da página Capsules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleSummary {
    /// Arquivo `capsule.toml`.
    pub path: PathBuf,
    /// `[app] name`.
    pub name: String,
    /// Versão Windows declarada.
    pub windows_version: (u32, u32),
    /// Letras de drive (`C`, `D`, …).
    pub drives: Vec<char>,
    /// Válida (se `false`, `error` explica e a UI oferece correção).
    pub valid: bool,
    /// Motivo da invalidez (quando `!valid`).
    pub error: Option<String>,
}

/// Diretório é perigoso para mapear (`/` ou `$HOME` direto)?
/// A UI mostra warning + exige confirmação (nunca silencioso).
pub fn is_risky_drive_target(host_path: &str) -> bool {
    let p = host_path.trim();
    if p == "/" {
        return true;
    }
    if let Ok(home) = std::env::var("HOME") {
        if p == home || p == "$HOME" {
            return true;
        }
    }
    false
}

/// Lê uma capsule (TOML válido do `runtime` ou entrada inválida explícita).
pub fn load_summary(path: &Path) -> CapsuleSummary {
    let text = path.to_string_lossy().into_owned();
    match runtime::Capsule::load_toml(&text) {
        Ok(cap) => {
            let mut drives: Vec<char> = cap.drives.keys().copied().collect();
            drives.sort();
            CapsuleSummary {
                path: path.to_path_buf(),
                name: cap.app_name,
                windows_version: cap.windows_version,
                drives,
                valid: true,
                error: None,
            }
        }
        Err(e) => CapsuleSummary {
            path: path.to_path_buf(),
            name: path
                .parent()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".into()),
            windows_version: (10, 0),
            drives: Vec::new(),
            valid: false,
            error: Some(e.to_string()),
        },
    }
}

/// Lista `*.toml`/`capsule.toml` diretos em `dir` (um nível; sem recursão).
/// Arquivos inválidos vêm marcados (nunca somem silenciosamente).
pub fn list_capsules(dir: &Path) -> Result<Vec<CapsuleSummary>, ManagerError> {
    let read = std::fs::read_dir(dir).map_err(|e| io_err("listar capsules", e))?;
    let mut out = Vec::new();
    for entry in read.flatten() {
        let p = entry.path();
        let is_capsule = p.is_file()
            && (p.file_name().map(|n| n == "capsule.toml").unwrap_or(false)
                || p.extension().map(|e| e == "toml").unwrap_or(false));
        if is_capsule {
            out.push(load_summary(&p));
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Pedido de criação (wizard: só chaves do schema v0.2).
#[derive(Debug, Clone)]
pub struct CreateCapsule {
    /// Nome (`[app] name`).
    pub name: String,
    /// Diretório pai (a capsule vive em `<parent>/<name>/capsule.toml`).
    pub parent_dir: PathBuf,
    /// Drives (`LETRA` → dir host; dirs criados se ausentes).
    pub drives: Vec<(char, PathBuf)>,
    /// `current_dir` Win32 (opcional).
    pub current_dir: Option<String>,
}

/// Cria a capsule no disco (dirs + `capsule.toml` no schema do runtime).
/// Falha se o destino já existe (nunca sobrescreve silenciosamente).
pub fn create_capsule(req: &CreateCapsule) -> Result<PathBuf, ManagerError> {
    if req.name.trim().is_empty() {
        return Err(ManagerError::InvalidCapsule {
            path: req.parent_dir.display().to_string(),
            reason: "nome vazio".into(),
        });
    }
    let dir = req.parent_dir.join(&req.name);
    if dir.exists() {
        return Err(ManagerError::InvalidCapsule {
            path: dir.display().to_string(),
            reason: "já existe (recuse sobrescrever sem confirmação)".into(),
        });
    }
    std::fs::create_dir_all(&dir).map_err(|e| io_err("criar capsule", e))?;
    let mut drives_toml = String::new();
    for (letter, host) in &req.drives {
        let letter = letter.to_ascii_uppercase();
        if !letter.is_ascii_uppercase() {
            return Err(ManagerError::InvalidCapsule {
                path: dir.display().to_string(),
                reason: format!("letra de drive inválida: {letter}"),
            });
        }
        std::fs::create_dir_all(host).map_err(|e| io_err("criar drive", e))?;
        drives_toml.push_str(&format!("{letter} = \"{}\"\n", host.display()));
    }
    let current = match &req.current_dir {
        Some(c) => format!("current_dir = \"{c}\"\n"),
        None => String::new(),
    };
    let toml = format!(
        "[app]\nname = \"{}\"\nwindows_version = [10, 0]\narch = \"x86_64\"\n{current}[drives]\n{drives_toml}",
        req.name
    );
    let path = dir.join("capsule.toml");
    std::fs::write(&path, toml).map_err(|e| io_err("escrever capsule", e))?;
    // Prova imediata: o próprio runtime lê o que escrevemos.
    runtime::Capsule::load_toml(&path.to_string_lossy()).map_err(|e| {
        ManagerError::InvalidCapsule {
            path: path.display().to_string(),
            reason: format!("recém-criada ilegível (bug): {e}"),
        }
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_then_list_roundtrip() {
        let d = std::env::temp_dir().join(format!("rine-cap-{}-roundtrip", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let req = CreateCapsule {
            name: "game".into(),
            parent_dir: d.clone(),
            drives: vec![('c', d.join("game/drive_c"))],
            current_dir: None,
        };
        let path = create_capsule(&req).unwrap();
        let list = list_capsules(&d.join("game")).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].valid);
        assert_eq!(list[0].name, "game");
        assert_eq!(list[0].drives, vec!['C']);
        assert_eq!(path, d.join("game/capsule.toml"));
        // Criar de novo no mesmo lugar falha (sem sobrescrita silenciosa).
        assert!(create_capsule(&req).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn risky_targets_flagged() {
        assert!(is_risky_drive_target("/"));
        assert!(!is_risky_drive_target("/home/u/Rine/capsules/game/drive_c"));
    }
}
