//! `api-scan`: oracle estrutural da referência Windows.
//!
//! Reutiliza o ÚNICO parser PE (`pe`); nunca duplica parsing.
//! Saídas em `api-db/` (versionadas, regeneráveis — ver `api-db/README.md`).
//! Conhecimento, não stubs: nada aqui gera código do runtime.

pub mod coverage;
pub mod db;
pub mod record;
pub mod scan;
