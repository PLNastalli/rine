//! Minimização de falhas (ddmin): do caso que falha ao menor reproducer.
//!
//! Quando um cenário gerado falha, tenta remover pedaços preservando a
//! falha (mesmo predicado). Uso: sequências de ops, listas de flags, bytes.
//! Determinístico; orçamento de tentativas para não rodar para sempre.

/// Minimiza `items` preservando `fails(&subset) == true`.
/// Retorna subconjunto (ordem preservada) que ainda falha, ou `[]` se nem o
/// original falha (erro do chamador — documentado, não panic).
pub fn minimize<T: Clone>(items: &[T], fails: &dyn Fn(&[T]) -> bool, budget: usize) -> Vec<T> {
    if !fails(items) {
        return Vec::new();
    }
    let mut current: Vec<T> = items.to_vec();
    let mut spent = 0usize;
    let mut gran = 2usize;
    while current.len() >= 2 && spent < budget {
        let mut reduced = false;
        let chunk = (current.len() + gran - 1) / gran.max(1);
        let mut i = 0;
        while i < current.len() && spent < budget {
            let end = (i + chunk).min(current.len());
            let mut candidate = Vec::with_capacity(current.len() - (end - i));
            candidate.extend_from_slice(&current[..i]);
            candidate.extend_from_slice(&current[end..]);
            spent += 1;
            if !candidate.is_empty() && fails(&candidate) {
                current = candidate;
                reduced = true;
                gran = 2;
                break;
            }
            i = end;
        }
        if !reduced {
            if gran >= current.len().max(2) {
                break;
            }
            gran *= 2;
        }
    }
    // Passada final: tenta remover 1-a-1 (pequeno, exato).
    let mut i = 0;
    while i < current.len() && spent < budget {
        let mut candidate = current.clone();
        candidate.remove(i);
        spent += 1;
        if !candidate.is_empty() && fails(&candidate) {
            current = candidate;
        } else {
            i += 1;
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fails_if_contains_3_and_7(items: &[u32]) -> bool {
        items.contains(&3) && items.contains(&7)
    }

    #[test]
    fn shrinks_to_minimal() {
        let big: Vec<u32> = (0..50).collect();
        let min = minimize(&big, &fails_if_contains_3_and_7, 10000);
        assert_eq!(min, vec![3, 7]);
    }

    #[test]
    fn non_failing_returns_empty() {
        let v = vec![1, 2, 3];
        assert!(minimize(&v, &|_: &[u32]| false, 100).is_empty());
    }

    #[test]
    fn single_culprit() {
        let v = vec![1, 2, 3, 4, 5];
        let min = minimize(&v, &|s: &[u32]| s.contains(&4), 100);
        assert_eq!(min, vec![4]);
    }
}
