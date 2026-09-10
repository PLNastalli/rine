//! Fuzzing: mutação determinística + alvos por superfície.
//!
//! Sem libfuzzer (stable por reprodutibilidade): seeds fixas, `Rng` próprio,
//! propriedades verificadas em escala via `rine-test fuzz`. A mesma `mutate`
//! alimenta `crates/pe/tests/fuzz_corpus.rs` (fonte única — sem duplicar).
//!
//! Superfícies (missão): parser PE, loader (via parse+views), imports,
//! exports, relocs, paths NT, registry/TOML. API Sets: quando o parser
//! existir (hoje: fixture `apisets.json` apenas).

use crate::rng::Rng;

/// Aplica mutações destrutivas determinísticas (headers truncados, flips,
/// splices, `section count` absurdo via bytes, RVAs selvagens...).
pub fn mutate(seed: u64, base: &[u8]) -> Vec<u8> {
    let mut rng = Rng::new(seed | 1);
    let mut b = base.to_vec();
    let cut = rng.below(b.len() as u64 + 1) as usize;
    if seed % 3 == 0 {
        b.truncate(cut.min(b.len()));
        return b;
    }
    let n = 1 + rng.below(8) as usize;
    for _ in 0..n {
        if b.is_empty() {
            break;
        }
        let i = rng.below(b.len() as u64) as usize;
        match rng.below(4) {
            0 => b[i] ^= 1 << rng.below(8),
            1 => b[i] = (rng.next_u64() & 0xFF) as u8,
            2 => b.truncate(i),
            _ => b.extend_from_slice(&[0xFF, 0x00, 0x4D, 0x5A]),
        }
    }
    b
}

/// Resultado estruturado de UMA entrada (para campanhas e corpus).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutcome {
    pub ok: bool,
    pub imports: usize,
    pub exports: usize,
    pub relocs: usize,
}

/// Parse + views totais: nunca panic (propriedade central do fuzzing PE).
/// Qualquer panic aqui é bug do parser — a campanha o captura como Crash.
pub fn check_parse(bytes: &[u8]) -> ParseOutcome {
    match pe::Image::parse(bytes) {
        Err(_) => ParseOutcome {
            ok: false,
            imports: 0,
            exports: 0,
            relocs: 0,
        },
        Ok(img) => {
            let imports = img.imports().map(|(_, s)| s.len()).unwrap_or(0);
            let exports = img.exports().map(|e| e.len()).unwrap_or(0);
            let relocs = img
                .relocs()
                .map(|rs| rs.iter().map(|b| b.entries.len()).sum())
                .unwrap_or(0);
            ParseOutcome {
                ok: true,
                imports,
                exports,
                relocs,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutate_is_deterministic() {
        let base = pe::builder::build_minimal_hello();
        assert_eq!(mutate(5, &base), mutate(5, &base));
        assert_ne!(mutate(5, &base), mutate(6, &base));
    }

    #[test]
    fn valid_pe_checks_ok() {
        let o = check_parse(&pe::builder::build_minimal_hello());
        assert!(o.ok && o.imports == 3);
        let bad = check_parse(b"MZ-truncado");
        assert!(!bad.ok);
    }
}
