//! `manager-core`: management API do Rine Manager (sem nenhuma dep de UI).
//!
//! Camada entre o frontend (Tauri/React, futuro) e o runtime:
//!
//! ```text
//! detect_runtime → inspect_pe → preflight → run_app → RunReport
//! registry (Library) · capsules (Capsules) · ManagerError (tipado)
//! ```
//!
//! Regras: nunca depende de Tauri; nunca toca `kernel32`/`nt-*` direto
//! (só via `runtime`/`pe` públicos); nunca `unwrap/expect` em input;
//! nunca finge capacidade que o runtime não tem (ver `docs/manager/`).

pub mod capsules;
pub mod error;
pub mod pe_inspect;
pub mod preflight;
pub mod registry;
pub mod run;
pub mod runtime;
pub mod settings;

pub use error::ManagerError;
