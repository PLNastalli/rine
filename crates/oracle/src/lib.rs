//! `oracle`: Windows Oracle — formato versionado + runner lado-Rine.
//!
//! O oracle COMPLETO exige um host Windows (runner de lá produz o mesmo
//! formato; ver `tests/windows/oracle_probe.c`). Aqui vive o formato e o
//! produtor lado-Rine, que permite `normalized diff` futuro.

pub mod format;
pub mod runner;
