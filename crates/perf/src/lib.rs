//! `perf`: regression gate de performance do Rine.
//!
//! Pipeline separado por construção:
//!
//! ```text
//! rine-bench run   (mede; depende de pe/runtime)
//!       ↓ results.json (schema versionado)
//! comparação pura (este lib: schema/stats/compare/report)
//!       ↓ relatório humano + decisão CI
//! ```
//!
//! `src/*.rs` (lib) NUNCA importa `pe`/`runtime`: coleta e decisão não se
//! misturam. Filosofia: correção primeiro, compatibilidade em segundo,
//! regressão sempre visível, otimização sem mudar comportamento Windows.

pub mod compare;
pub mod report;
pub mod schema;
pub mod stats;
