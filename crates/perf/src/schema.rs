//! Formatos versionados: resultados medidos e baselines promovidas.
//!
//! `schema = 1`. Leitores rejeitam schema desconhecido. Baselines carregam
//! ambiente completo (comparar máquinas diferentes como equivalentes é bug).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Versão do formato de resultados/baselines.
pub const SCHEMA: u32 = 1;

/// Ambiente de medição. Tudo que invalida comparação direta fica aqui.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Environment {
    /// Commit do Rine medido (`git rev-parse --short HEAD`; `"unknown"` fora de repo).
    pub commit: String,
    /// `rine --version` (aqui: versão do workspace `perf`, mesma do release).
    pub rine_version: String,
    /// CPU (`model name` de `/proc/cpuinfo`; `"unknown"` fora do Linux).
    pub cpu: String,
    /// Kernel (`uname -sr`; `"unknown"` se falhar).
    pub kernel: String,
    /// Toolchain (`rustc --version`; `"unknown"` se falhar).
    pub toolchain: String,
    /// `"debug"` ou `"release"` (via `cfg!(debug_assertions)`).
    pub profile: String,
    /// `RUSTFLAGS` no momento da medição, ou `"default"`.
    pub rustflags: String,
    /// Segundos Unix da medição.
    pub timestamp: u64,
}

/// Uma medição: amostras em ns/op + estatísticas derivadas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Measurement {
    /// ID estável do benchmark (`pe.parse.hello`). Nunca renomear sem
    /// versionar: ID novo = benchmark novo (anti-drift).
    pub id: String,
    /// Escopo conceitual em linguagem humana. Mudou o escopo → mude o ID.
    pub scope: String,
    /// Versão da metodologia (bump quando o procedimento muda).
    pub method_version: u32,
    /// Uma amostra = ns/op de um batch (após warmup descartado).
    pub samples_ns: Vec<u64>,
    pub mean_ns: f64,
    pub median_ns: f64,
    /// Desvio padrão populacional das amostras (ruído visível, não escondido).
    pub stddev_ns: f64,
}

/// Resultado de uma rodada: várias medições + ambiente.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResults {
    pub schema: u32,
    pub env: Environment,
    pub benchmarks: BTreeMap<String, Measurement>,
}

/// Baseline promovida: mesmos campos + auditoria da promoção.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Baseline {
    pub schema: u32,
    pub env: Environment,
    pub benchmarks: BTreeMap<String, Measurement>,
    /// Quem promoveu, quando e por quê. Promoção é manual e auditável;
    /// nunca automática (ver `rine-bench promote`).
    pub promoted_by: String,
    pub promoted_at_unix: u64,
    pub reason: String,
    /// Baseline anterior substituída (cadeia auditável; `null` na primeira).
    pub supersedes: Option<String>,
}

impl RunResults {
    pub fn new(env: Environment) -> Self {
        Self {
            schema: SCHEMA,
            env,
            benchmarks: BTreeMap::new(),
        }
    }
}
