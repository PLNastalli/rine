//! `difftest`: plataforma de testes diferenciais do Rine.
//!
//! Pipeline: gerar cenário → executar (Rine/modelo/gravado) → normalizar →
//! comparar → minimizar → corpus de regressão. Windows real pluga como oracle
//! via `Oracle` quando houver host (ver `docs/windows-oracle.md`).
//!
//! Regras: seeds sempre registrados; nada fictício; vereditos ricos
//! (nunca só PASS/FAIL); determinismo por seed em qualquer nº de workers.

pub mod campaigns;
pub mod compare;
pub mod corpus;
pub mod exec;
pub mod fuzz;
pub mod gen;
pub mod model;
pub mod normalize;
pub mod report;
pub mod rng;
pub mod runner;
pub mod scenario;
pub mod shrink;
