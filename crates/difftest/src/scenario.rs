//! Cenário versionado e reproduzível (`schema = 1`).
//!
//! Todo cenário aleatório registra seu seed; `id` é hash determinístico do
//! conteúdo canônico (mesmo seed+params → mesmo id, em qualquer máquina).

use serde::{Deserialize, Serialize};

/// Versão do formato de cenário.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scenario {
    pub schema: u32,
    /// `"<target>-<12 hex fnv1a>"`. Determinístico, estável entre versões.
    pub scenario_id: String,
    /// Alvo: `"fileops"`, `"handles"`, `"memory"`, `"pe"`, ...
    pub target: String,
    /// API Windows principal exercitada (ex.: `"CreateFileA"`).
    pub api: String,
    /// Seed que gerou este cenário. Repetir = reproduzir exatamente.
    pub seed: u64,
    /// Parâmetros do cenário (forma livre por alvo, sempre serializável).
    pub params: serde_json::Value,
    /// Ambiente de geração (versão do gerador, dimensões, etc.).
    pub environment: serde_json::Value,
}

impl Scenario {
    pub fn new(
        target: &str,
        api: &str,
        seed: u64,
        params: serde_json::Value,
        environment: serde_json::Value,
    ) -> Self {
        let mut s = Self {
            schema: SCHEMA,
            scenario_id: String::new(),
            target: target.into(),
            api: api.into(),
            seed,
            params,
            environment,
        };
        s.scenario_id = s.compute_id();
        s
    }

    fn compute_id(&self) -> String {
        // Canônico: target + seed + params em JSON estável (BTreeMap ordena).
        let canon = format!("{}:{}:{}", self.target, self.seed, self.params);
        format!(
            "{}-{:012x}",
            short_target(&self.target),
            fnv1a_64(canon.as_bytes()) & 0xFFFFFFFFFFFF
        )
    }
}

fn short_target(t: &str) -> String {
    t.chars().take(8).collect()
}

/// FNV-1a 64 bits (estável entre plataformas/compiladores — ao contrário de
/// `DefaultHasher`, cuja estabilidade não é garantida; ver doc de `std`).
/// Implementação local de 5 linhas para não depender de crate de hash.
pub fn fnv1a_64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn id_is_deterministic_and_versioned() {
        let a = Scenario::new("fileops", "CreateFileA", 123, json!({"x": 1}), json!({}));
        let b = Scenario::new("fileops", "CreateFileA", 123, json!({"x": 1}), json!({}));
        assert_eq!(a.scenario_id, b.scenario_id);
        assert_eq!(a.schema, SCHEMA);
        let c = Scenario::new("fileops", "CreateFileA", 124, json!({"x": 1}), json!({}));
        assert_ne!(a.scenario_id, c.scenario_id);
    }

    #[test]
    fn scenario_roundtrips_json() {
        let a = Scenario::new("memory", "VirtualAlloc", 7, json!([1, 2]), json!({}));
        let s = serde_json::to_string(&a).unwrap();
        let back: Scenario = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn fnv_is_stable_vector() {
        // Vetor conhecido do FNV-1a 64 (garante estabilidade futura).
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63dc4c8601ec8c);
    }
}
