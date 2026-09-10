//! RNG determinístico da plataforma (SplitMix64).
//!
//! Um único algoritmo em todo o `difftest` (não misturar com `rand`/OS):
//! mesma seed → mesma sequência, em qualquer máquina, para sempre.
//! Workers derivam sub-streams (`seed ^ worker`), casos derivam
//! `seed ^ index` — ordem de conclusão nunca afeta resultados.

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E3779B97F4A7C15))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut z = self.0.wrapping_add(0x9E3779B97F4A7C15);
        self.0 = z;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// `[0, bound)`. `bound == 0` retorna 0 (evita divisão por zero em
    /// geradores — documentado, não silencioso: chamador decide).
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        // Rejeição parcial é aceitável aqui (geração, não cripto).
        self.next_u64() % bound
    }

    pub fn below_usize(&mut self, bound: usize) -> usize {
        self.below(bound as u64) as usize
    }

    pub fn choose<'a, T>(&mut self, xs: &'a [T]) -> Option<&'a T> {
        if xs.is_empty() {
            return None;
        }
        Some(&xs[self.below_usize(xs.len())])
    }

    /// Bool com probabilidade `num/den` de `true`.
    pub fn biased(&mut self, num: u64, den: u64) -> bool {
        den > 0 && self.below(den) < num
    }

    /// Stream independente para um worker/caso.
    pub fn derive(&self, salt: u64) -> Rng {
        Rng(self.0 ^ salt.wrapping_mul(0x9E3779B97F4A7C15))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_stream() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Rng::new(42).next_u64(), Rng::new(43).next_u64());
    }

    #[test]
    fn below_respects_bound() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            assert!(r.below(10) < 10);
        }
        assert_eq!(r.below(0), 0);
        assert_eq!(r.below(1), 0);
    }

    #[test]
    fn derive_is_independent() {
        let base = Rng::new(99);
        assert_ne!(base.derive(1).next_u64(), base.derive(2).next_u64());
        assert_eq!(base.derive(1).next_u64(), base.derive(1).next_u64());
    }
}
