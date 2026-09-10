//! Motor paralelo determinístico de campanhas.
//!
//! Workers puxam índices de um contador atômico; cada caso é independente
//! (tmpdir próprio, seed própria `master ^ index`). Resultados ordenados por
//! índice no final: **1 worker ou 32 dão a mesma saída** (testado).
//! Sem estado mutável compartilhado além do contador e do coletor.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Executa `f(index)` para `0..n` em `workers` threads; retorna ordenado.
pub fn run_parallel<T, F>(n: usize, workers: usize, f: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync,
{
    if n == 0 {
        return Vec::new();
    }
    let workers = workers.max(1).min(n.max(1));
    let next = AtomicUsize::new(0);
    let out: std::sync::Mutex<Vec<(usize, T)>> = std::sync::Mutex::new(Vec::with_capacity(n));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= n {
                    break;
                }
                let r = f(i);
                out.lock()
                    .expect("coletor envenenado (bug da plataforma)")
                    .push((i, r));
            });
        }
    });
    let mut v = out
        .into_inner()
        .expect("coletor envenenado (bug da plataforma)");
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, r)| r).collect()
}

/// Diretório isolado por caso (`run/worker_{w}/case_{i}`).
pub fn case_dir(run_dir: &std::path::Path, worker: usize, index: usize) -> std::path::PathBuf {
    run_dir
        .join(format!("worker_{worker}"))
        .join(format!("case_{index}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_across_worker_counts() {
        let f = |i: usize| (i * 2654435761) % 1000;
        let a = run_parallel(500, 1, f);
        let b = run_parallel(500, 8, f);
        assert_eq!(a, b);
    }

    #[test]
    fn empty_and_single() {
        let v: Vec<u32> = run_parallel(0, 4, |_| 1);
        assert!(v.is_empty());
        assert_eq!(run_parallel(1, 8, |i| i * 2), vec![0]);
    }
}
