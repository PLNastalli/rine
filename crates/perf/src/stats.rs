//! Estatística robusta sobre amostras: mediana primeiro, média depois.
//!
//! Uma única execução nunca decide regressão: cada benchmark produz VÁRIAS
//! amostras (batches após warmup descartado) e a decisão usa a mediana.

use crate::schema::Measurement;

/// Ordena cópia e retorna mediana (média dos dois centrais se par).
/// Vetor vazio → `None` (chamador decide; nunca inventar zero).
pub fn median(sorted: &mut [u64]) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_unstable();
    let n = sorted.len();
    Some(if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] as f64 + sorted[n / 2] as f64) / 2.0
    })
}

pub fn mean(samples: &[u64]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    Some(samples.iter().sum::<u64>() as f64 / samples.len() as f64)
}

/// Desvio padrão populacional (denominador N, não N−1: descreve AS amostras,
/// não estima população — documentado para não gerar debate falso).
pub fn stddev(samples: &[u64], mean: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let var = samples
        .iter()
        .map(|&x| (x as f64 - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    var.sqrt()
}

/// Constrói `Measurement` a partir das amostras brutas.
pub fn summarize(
    id: &str,
    scope: &str,
    method_version: u32,
    samples_ns: Vec<u64>,
) -> Option<Measurement> {
    let mean = mean(&samples_ns)?;
    let median = median(&mut samples_ns.clone())?;
    let stddev_ns = stddev(&samples_ns, mean);
    Some(Measurement {
        id: id.into(),
        scope: scope.into(),
        method_version,
        samples_ns,
        mean_ns: mean,
        median_ns: median,
        stddev_ns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_even_empty() {
        assert_eq!(median(&mut [3, 1, 2]), Some(2.0));
        assert_eq!(median(&mut [4, 1, 3, 2]), Some(2.5));
        assert_eq!(median(&mut Vec::<u64>::new()), None);
        // Outlier desloca média, quase não toca a mediana (o ponto todo).
        let mut v = vec![10, 10, 10, 10, 1000];
        assert_eq!(median(&mut v), Some(10.0));
        assert!(mean(&[10, 10, 10, 10, 1000]).unwrap() > 100.0);
    }

    #[test]
    fn stddev_population() {
        assert_eq!(stddev(&[], 0.0), 0.0);
        assert_eq!(stddev(&[5], 5.0), 0.0);
        // [2,4,4,4,5,5,7,9]: média 5, var populacional 4, stddev 2.
        assert!((stddev(&[2, 4, 4, 4, 5, 5, 7, 9], 5.0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn summarize_needs_samples() {
        assert!(summarize("x", "y", 1, vec![]).is_none());
        let m = summarize("x", "y", 1, vec![10, 20, 30]).unwrap();
        assert_eq!(m.median_ns, 20.0);
        assert_eq!(m.mean_ns, 20.0);
    }
}
