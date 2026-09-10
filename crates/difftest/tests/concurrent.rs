//! Concorrência: tabela compartilhada sob contenção real.
//!
//! N threads × K ops com seeds independentes, largada sincronizada por
//! `Barrier` (interleavings reais, não sequenciais). Cada thread é dona dos
//! próprios handles → resultado determinístico apesar da contenção.
//! Estratégia: scheduling aleatório (barreira + SO), stress repetido,
//! sem brute force de interleavings (ver `docs/fuzzing.md`).

use difftest::rng::Rng;
use nt_object::{FileObject, HandleTable, KernelObject, ObjectPayload, ObjectType};
use std::sync::{Arc, Barrier};

fn test_object() -> KernelObject {
    KernelObject {
        typ: ObjectType::File,
        name: None,
        rights: 0,
        payload: std::sync::Mutex::new(ObjectPayload::File(FileObject {
            fd: -1,
            path: "concurrent".into(),
            writable: true,
            readable: true,
            owns_fd: false,
        })),
    }
}

fn worker_ops(table: &Arc<HandleTable>, seed: u64, n: usize) {
    let mut rng = Rng::new(seed);
    let mut mine = Vec::new();
    for _ in 0..n {
        match rng.below(100) {
            0..=49 => mine.push(table.insert(Arc::new(test_object()))),
            50..=79 => {
                if let Some(h) = mine.get(rng.below_usize(mine.len().max(1))) {
                    let _ = table.lookup(*h);
                }
            }
            _ => {
                if !mine.is_empty() {
                    let i = rng.below_usize(mine.len());
                    let h = mine.swap_remove(i);
                    let _ = table.close(h);
                }
            }
        }
    }
    // Fecha tudo que restou: ao fim, nada desta thread pode ser válido.
    for h in mine {
        let _ = table.close(h);
    }
}

#[test]
fn concurrent_handles_stay_consistent() {
    for round in 0..3u64 {
        let table = Arc::new(HandleTable::new());
        let barrier = Arc::new(Barrier::new(8));
        std::thread::scope(|s| {
            for t in 0..8 {
                let (table, barrier) = (Arc::clone(&table), Arc::clone(&barrier));
                s.spawn(move || {
                    barrier.wait();
                    worker_ops(&table, round * 100 + t as u64, 500);
                });
            }
        });
        // Todos os handles criados foram fechados; lookups restantes falham.
        // (A tabela em si não expõe contagem — consistência = sem panic,
        // sem deadlock, e inserts seguem funcionando.)
        let h = table.insert(Arc::new(test_object()));
        assert!(table.lookup(h).is_ok());
        assert!(table.close(h).is_ok());
    }
}
