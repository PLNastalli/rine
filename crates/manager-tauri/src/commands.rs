//! Tauri commands: thin adapters React → manager-core.
//!
//! REGRA (§35 da missão): nenhum command reinventa lógica — cada um resolve
//! paths, delega a `manager-core` e converte o erro. Lógica aqui = bug
//! arquitetural (o mesmo teste deve passar sem abrir a GUI).

use crate::error::CmdError;
use manager_core::{capsules, pe_inspect, preflight, registry, run, runtime, settings};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

// ---------------------------------------------------------------------------
// DTOs (payloads do frontend; tipos do core viajam por serde quando dá)
// ---------------------------------------------------------------------------

/// Runtime para a Home (`version` + `binary`).
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeDto {
    /// Versão (`0.2.0-alpha.2`).
    pub version: String,
    /// Binário validado.
    pub binary: String,
    /// `true` quando bate com o workspace (sempre aqui: mismatch = erro).
    pub compatible: bool,
}

/// Pedido da página Run (ad-hoc, sem cadastro).
#[derive(Debug, Clone, Deserialize)]
pub struct RunPayload {
    /// `.exe` Windows.
    pub exe: String,
    /// `capsule.toml` (ou `null`).
    pub capsule: Option<String>,
    /// Args do guest (cmdline Win32; nunca shell).
    pub args: Vec<String>,
    /// CWD (ou `null` = dir do exe).
    pub workdir: Option<String>,
    /// Extras `KEY=value`.
    pub env_extra: Vec<(String, String)>,
    /// Timeout ms (0 = sem limite).
    pub timeout_ms: u64,
}

/// Pedido de criação de Capsule (wizard; só chaves do schema v0.2).
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCapsulePayload {
    /// Nome.
    pub name: String,
    /// Diretório pai.
    pub parent_dir: String,
    /// `(LETRA, dir host)`.
    pub drives: Vec<(char, String)>,
    /// `current_dir` Win32 (ou `null`).
    pub current_dir: Option<String>,
}

/// Evento de lifecycle de run (a UI reage a eventos, §33).
#[derive(Debug, Clone, Serialize)]
struct RunEvent {
    /// `started` | `finished`.
    kind: String,
    /// Id do app (Library) ou `null` (Run ad-hoc).
    app_id: Option<String>,
    /// Relatório (só em `finished`).
    report: Option<run::RunReport>,
}

fn registry_path() -> Result<PathBuf, CmdError> {
    registry::AppRegistry::default_path().map_err(CmdError::from)
}

fn settings_path() -> Result<PathBuf, CmdError> {
    settings::Settings::default_path().map_err(CmdError::from)
}

// ---------------------------------------------------------------------------
// Runtime / PE / pre-flight
// ---------------------------------------------------------------------------

/// Home: detecta e valida o runtime (`ProtocolMismatch` vira erro explícito).
#[tauri::command]
pub fn detect_runtime() -> Result<RuntimeDto, CmdError> {
    let info = runtime::detect_runtime(None).map_err(CmdError::from)?;
    Ok(RuntimeDto {
        version: info.version,
        binary: info.binary.display().to_string(),
        compatible: true,
    })
}

/// Run: inspeciona o PE sem executar (arquitetura, imports, entry).
#[tauri::command]
pub fn inspect_pe(path: String) -> Result<pe_inspect::PeReport, CmdError> {
    pe_inspect::inspect_pe(PathBuf::from(path).as_path()).map_err(CmdError::from)
}

/// Run: pre-check de compatibilidade estática (nunca "Compatible").
#[tauri::command]
pub fn preflight(path: String) -> Result<preflight::PreflightReport, CmdError> {
    let bytes = std::fs::read(&path).map_err(|e| {
        CmdError::from(manager_core::ManagerError::Io {
            op: "ler exe".into(),
            reason: e.to_string(),
        })
    })?;
    preflight::preflight_bytes(&bytes).map_err(CmdError::from)
}

// ---------------------------------------------------------------------------
// Execução (fora da thread da UI: `spawn_blocking`; eventos tipados)
// ---------------------------------------------------------------------------

fn to_request(
    rine_bin: PathBuf,
    exe: String,
    capsule: Option<String>,
    args: Vec<String>,
    workdir: Option<String>,
    env_extra: Vec<(String, String)>,
    timeout_ms: u64,
) -> run::RunRequest {
    run::RunRequest {
        rine_bin,
        exe: PathBuf::from(exe),
        capsule: capsule.map(PathBuf::from),
        args,
        workdir: workdir.map(PathBuf::from),
        env_extra,
        timeout_ms,
    }
}

/// Run ad-hoc (página Run): executa e devolve o `RunReport` completo.
#[tauri::command]
pub async fn run_exe(app: AppHandle, payload: RunPayload) -> Result<run::RunReport, CmdError> {
    let info = runtime::detect_runtime(None).map_err(CmdError::from)?;
    let req = to_request(
        info.binary,
        payload.exe,
        payload.capsule,
        payload.args,
        payload.workdir,
        payload.env_extra,
        payload.timeout_ms,
    );
    let _ = app.emit(
        "rine-run",
        RunEvent {
            kind: "started".into(),
            app_id: None,
            report: None,
        },
    );
    let report = tauri::async_runtime::spawn_blocking(move || run::run_app(&req))
        .await
        .map_err(|e| {
            CmdError::from(manager_core::ManagerError::Io {
                op: "executar".into(),
                reason: e.to_string(),
            })
        })??;
    let _ = app.emit(
        "rine-run",
        RunEvent {
            kind: "finished".into(),
            app_id: None,
            report: Some(report.clone()),
        },
    );
    Ok(report)
}

/// Library: executa app cadastrado e registra `last_run` (dois cliques).
#[tauri::command]
pub async fn run_registered_app(
    app: AppHandle,
    id: String,
    args: Vec<String>,
    timeout_ms: u64,
) -> Result<run::RunReport, CmdError> {
    let path = registry_path()?;
    let mut reg = registry::AppRegistry::open(path.clone()).map_err(CmdError::from)?;
    let entry = reg
        .get(&id)
        .ok_or_else(|| {
            CmdError::from(manager_core::ManagerError::InvalidRegistry(format!(
                "app {id} desconhecido"
            )))
        })?
        .clone();
    let info = runtime::detect_runtime(None).map_err(CmdError::from)?;
    let req = to_request(
        info.binary,
        entry.exe_path.display().to_string(),
        entry.capsule_path.map(|p| p.display().to_string()),
        args,
        None,
        Vec::new(),
        timeout_ms,
    );
    let _ = app.emit(
        "rine-run",
        RunEvent {
            kind: "started".into(),
            app_id: Some(id.clone()),
            report: None,
        },
    );
    let report = tauri::async_runtime::spawn_blocking(move || run::run_app(&req))
        .await
        .map_err(|e| {
            CmdError::from(manager_core::ManagerError::Io {
                op: "executar".into(),
                reason: e.to_string(),
            })
        })??;
    reg.record_run(&id, &report).map_err(CmdError::from)?;
    reg.save().map_err(CmdError::from)?;
    let _ = app.emit(
        "rine-run",
        RunEvent {
            kind: "finished".into(),
            app_id: Some(id),
            report: Some(report.clone()),
        },
    );
    Ok(report)
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

/// Library: lista apps cadastrados.
#[tauri::command]
pub fn list_apps() -> Result<Vec<registry::AppEntry>, CmdError> {
    let reg = registry::AppRegistry::open(registry_path()?).map_err(CmdError::from)?;
    Ok(reg.list().to_vec())
}

/// Library: cadastra um `.exe` (só inspeciona; nunca executa sozinho).
#[tauri::command]
pub fn add_app(exe_path: String, name: Option<String>) -> Result<registry::AppEntry, CmdError> {
    let path = registry_path()?;
    let mut reg = registry::AppRegistry::open(path).map_err(CmdError::from)?;
    let entry = reg
        .add(PathBuf::from(exe_path), name)
        .map_err(CmdError::from)?
        .clone();
    reg.save().map_err(CmdError::from)?;
    Ok(entry)
}

/// Library: remove SÓ o cadastro (o `.exe` permanece no disco, sempre).
#[tauri::command]
pub fn remove_app(id: String) -> Result<(), CmdError> {
    let path = registry_path()?;
    let mut reg = registry::AppRegistry::open(path).map_err(CmdError::from)?;
    reg.remove(&id).map_err(CmdError::from)?;
    reg.save().map_err(CmdError::from)?;
    Ok(())
}

/// Library: associa uma Capsule ao app.
#[tauri::command]
pub fn set_app_capsule(id: String, capsule: Option<String>) -> Result<(), CmdError> {
    let path = registry_path()?;
    let mut reg = registry::AppRegistry::open(path).map_err(CmdError::from)?;
    reg.set_capsule(&id, capsule.map(PathBuf::from))
        .map_err(CmdError::from)?;
    reg.save().map_err(CmdError::from)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Capsules
// ---------------------------------------------------------------------------

/// Capsules: lista `*.toml` de um diretório (inválidas vêm marcadas).
#[tauri::command]
pub fn list_capsules(dir: String) -> Result<Vec<capsules::CapsuleSummary>, CmdError> {
    capsules::list_capsules(PathBuf::from(dir).as_path()).map_err(CmdError::from)
}

/// Capsules: cria no schema v0.2 (o próprio runtime valida em seguida).
#[tauri::command]
pub fn create_capsule(payload: CreateCapsulePayload) -> Result<String, CmdError> {
    let path = capsules::create_capsule(&capsules::CreateCapsule {
        name: payload.name,
        parent_dir: PathBuf::from(payload.parent_dir),
        drives: payload
            .drives
            .into_iter()
            .map(|(l, d)| (l, PathBuf::from(d)))
            .collect(),
        current_dir: payload.current_dir,
    })
    .map_err(CmdError::from)?;
    Ok(path.display().to_string())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Settings: carrega (ausente = default).
#[tauri::command]
pub fn get_settings() -> Result<settings::Settings, CmdError> {
    settings::Settings::load(&settings_path()?).map_err(CmdError::from)
}

/// Settings: salva.
#[tauri::command]
pub fn save_settings(value: settings::Settings) -> Result<(), CmdError> {
    value.save(&settings_path()?).map_err(CmdError::from)
}
