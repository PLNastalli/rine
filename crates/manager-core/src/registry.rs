//! Biblioteca de apps (cadastro; `remove` nunca apaga o `.exe`).

use crate::error::{io_err, ManagerError};
use crate::run::RunStatus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Versão do schema de `registry.json` (migração explícita, nunca silenciosa).
pub const REGISTRY_SCHEMA: u32 = 1;

/// Última execução conhecida (para `last_run` e estados da Library).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastRun {
    /// Segundos Unix.
    pub at_unix: u64,
    /// Estado classificado (`run::RunStatus`).
    pub status: RunStatus,
    /// Exit code (quando `Exited`).
    pub exit_code: Option<i32>,
}

/// App cadastrado (metadata NOSSA; o programa continua sendo do usuário).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    /// id estável (milissegundos + pid + contador — único por máquina).
    pub id: String,
    /// Nome de exibição.
    pub name: String,
    /// Caminho do `.exe` Windows.
    pub exe_path: PathBuf,
    /// Capsule associada (opcional).
    pub capsule_path: Option<PathBuf>,
    /// Cadastro (segundos Unix).
    pub added_at_unix: u64,
    /// Última execução (opcional).
    pub last_run: Option<LastRun>,
}

/// Estado de Library derivado de dados reais (nunca inferido do nada).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppStatus {
    /// Sem pre-flight nem run ainda.
    Unknown,
    /// Último pre-flight/run apontou imports em falta.
    ImportBlocked,
    /// Saiu com 0 ao menos uma vez (único estado "positivo" automático).
    Working,
    /// Já saiu 0 e agora falha (possível regressão — investigar).
    Regression,
    /// Último run crashou (`RINE-CRASH`).
    Crashed,
}

/// Registro persistido (arquivo único, escrita atômica via temporário).
#[derive(Debug, Default, Serialize, Deserialize)]
struct RegistryFile {
    schema: u32,
    apps: Vec<AppEntry>,
}

/// Biblioteca (carregada do disco, salva sob demanda).
pub struct AppRegistry {
    path: PathBuf,
    apps: Vec<AppEntry>,
    counter: u64,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl AppRegistry {
    /// Abre (ou cria vazio) em `path`. Schema diferente = erro explícito.
    pub fn open(path: PathBuf) -> Result<Self, ManagerError> {
        let apps = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let file: RegistryFile = serde_json::from_str(&text)
                    .map_err(|e| ManagerError::InvalidRegistry(e.to_string()))?;
                if file.schema != REGISTRY_SCHEMA {
                    return Err(ManagerError::InvalidRegistry(format!(
                        "schema {} (esperado {REGISTRY_SCHEMA})",
                        file.schema
                    )));
                }
                file.apps
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(io_err("abrir registry", e)),
        };
        Ok(Self {
            path,
            apps,
            counter: 0,
        })
    }

    /// Caminho padrão (`~/.local/share/rine-manager/registry.json`).
    pub fn default_path() -> Result<PathBuf, ManagerError> {
        Ok(super::runtime::data_dir()?.join("registry.json"))
    }

    /// Persiste (escrita atômica: temporário + rename).
    pub fn save(&self) -> Result<(), ManagerError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_err("criar datadir", e))?;
        }
        let file = RegistryFile {
            schema: REGISTRY_SCHEMA,
            apps: self.apps.clone(),
        };
        let text = serde_json::to_string_pretty(&file)
            .map_err(|e| ManagerError::InvalidRegistry(e.to_string()))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| io_err("escrever registry", e))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| io_err("publicar registry", e))?;
        Ok(())
    }

    /// Cadastra um `.exe` (deve existir e ser arquivo; nome default = arquivo).
    pub fn add(
        &mut self,
        exe_path: PathBuf,
        name: Option<String>,
    ) -> Result<&AppEntry, ManagerError> {
        let meta = std::fs::metadata(&exe_path).map_err(|e| io_err("ler exe", e))?;
        if !meta.is_file() {
            return Err(ManagerError::UnsupportedPe(format!(
                "{} não é um arquivo",
                exe_path.display()
            )));
        }
        self.counter += 1;
        let id = format!("{}-{}-{}", now_unix(), std::process::id(), self.counter);
        let name = name.unwrap_or_else(|| {
            exe_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "app".into())
        });
        self.apps.push(AppEntry {
            id,
            name,
            exe_path,
            capsule_path: None,
            added_at_unix: now_unix(),
            last_run: None,
        });
        self.apps
            .last()
            .ok_or_else(|| ManagerError::InvalidRegistry("push falhou".into()))
    }

    /// Remove SÓ o cadastro (o `.exe` permanece no disco, sempre).
    pub fn remove(&mut self, id: &str) -> Result<AppEntry, ManagerError> {
        let pos = self
            .apps
            .iter()
            .position(|a| a.id == id)
            .ok_or_else(|| ManagerError::InvalidRegistry(format!("app {id} desconhecido")))?;
        Ok(self.apps.remove(pos))
    }

    /// Lista todos (ordem de cadastro).
    pub fn list(&self) -> &[AppEntry] {
        &self.apps
    }

    /// Busca por id.
    pub fn get(&self, id: &str) -> Option<&AppEntry> {
        self.apps.iter().find(|a| a.id == id)
    }

    /// Associa capsule (validada na execução, não aqui).
    pub fn set_capsule(&mut self, id: &str, capsule: Option<PathBuf>) -> Result<(), ManagerError> {
        let app = self
            .apps
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| ManagerError::InvalidRegistry(format!("app {id} desconhecido")))?;
        app.capsule_path = capsule;
        Ok(())
    }

    /// Registra o resultado de um run (para `last_run` + estados).
    pub fn record_run(
        &mut self,
        id: &str,
        report: &crate::run::RunReport,
    ) -> Result<(), ManagerError> {
        let app = self
            .apps
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| ManagerError::InvalidRegistry(format!("app {id} desconhecido")))?;
        app.last_run = Some(LastRun {
            at_unix: now_unix(),
            status: report.status,
            exit_code: report.exit_code,
        });
        Ok(())
    }

    /// Estado derivado: último run manda; sem run = `Unknown`.
    /// (`ImportBlocked` vem do pre-flight na UI; `Working` exige exit 0.)
    pub fn status_of(entry: &AppEntry, preflight_blocked: bool) -> AppStatus {
        if preflight_blocked {
            return AppStatus::ImportBlocked;
        }
        match &entry.last_run {
            None => AppStatus::Unknown,
            Some(r) => match (r.status, r.exit_code) {
                (RunStatus::Crashed, _) => AppStatus::Crashed,
                (RunStatus::Blocked, _) => AppStatus::ImportBlocked,
                (RunStatus::Exited, Some(0)) => AppStatus::Working,
                (RunStatus::Exited, _) => AppStatus::Regression,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rine-reg-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn add_list_remove_roundtrip() {
        let d = tmp("roundtrip");
        let exe = d.join("app.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let mut reg = AppRegistry::open(d.join("registry.json")).unwrap();
        let id = reg.add(exe.clone(), None).unwrap().id.clone();
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.get(&id).unwrap().name, "app");
        reg.save().unwrap();
        let reg2 = AppRegistry::open(d.join("registry.json")).unwrap();
        assert_eq!(reg2.list().len(), 1);
        // Remove apaga o cadastro, NÃO o arquivo.
        let mut reg2 = reg2;
        reg2.remove(&id).unwrap();
        assert!(reg2.list().is_empty());
        assert!(exe.is_file());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn schema_mismatch_is_explicit() {
        let d = tmp("schema");
        let p = d.join("registry.json");
        std::fs::write(&p, r#"{"schema":99,"apps":[]}"#).unwrap();
        assert!(matches!(
            AppRegistry::open(p),
            Err(ManagerError::InvalidRegistry(_))
        ));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
