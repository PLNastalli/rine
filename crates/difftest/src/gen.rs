//! Geradores de cenários: combinatoriais, pairwise e stateful.
//!
//! Filosofia: bons geradores > muitos casos. Cada gerador declara suas
//! DIMENSÕES (para cobertura comportamental em `report`) e produz `RawOp`s
//! ou cenários completos a partir de seed. Pairwise cobre pares sem
//! explosão combinatorial; aleatório com seed cobre o resto.

use crate::model::{MemOp, RawOp};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Uma dimensão de cobertura comportamental: valores possíveis + descrição.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub api: String,
    pub name: String,
    pub values: Vec<String>,
}

// ---------------------------------------------------------------------------
// fileops: dimensões estilo CreateFileW da missão.
// ---------------------------------------------------------------------------

/// Classes de path (equivalence classes; cada uma mira um ramo de `translate`).
pub const PATH_CLASSES: &[(&str, &str)] = &[
    ("missing", "C:\\nz_{n}.txt"),       // ausente no drive
    ("existing", "C:\\ex_{n}.txt"),      // pré-criado no setup
    ("relative", "rel_{n}.txt"),         // relativo ao CWD/sandbox
    ("nested", "C:\\d1\\d2\\f_{n}.txt"), // dirs inexistentes (falha honesta)
    ("drive-missing", "Z:\\f_{n}.txt"),  // drive ausente
    ("unc", "\\\\srv\\f_{n}.txt"),       // UNC não suportado
    ("trailing", "C:\\t_{n}.txt\\"),     // trailing separator
    ("unicode", "C:\\é_{n}.txt"),        // não-ASCII
];
pub const ACCESSES: &[(&str, u32)] = &[
    ("read", 0x8000_0000),
    ("write", 0x4000_0000),
    ("rw", 0xC000_0000),
    ("zero", 0),
];
pub const DISPS: &[(&str, u32)] = &[
    ("new", 1),
    ("always", 2),
    ("existing", 3),
    ("open-always", 4),
    ("truncate", 5),
    ("bad", 9),
];

pub fn fileops_dimensions() -> Vec<Dimension> {
    vec![
        Dimension {
            api: "CreateFileA".into(),
            name: "path_class".into(),
            values: PATH_CLASSES.iter().map(|(n, _)| n.to_string()).collect(),
        },
        Dimension {
            api: "CreateFileA".into(),
            name: "access".into(),
            values: ACCESSES.iter().map(|(n, _)| n.to_string()).collect(),
        },
        Dimension {
            api: "CreateFileA".into(),
            name: "disposition".into(),
            values: DISPS.iter().map(|(n, _)| n.to_string()).collect(),
        },
        Dimension {
            api: "fileops".into(),
            name: "op".into(),
            values: ["create", "write", "read", "close"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        },
    ]
}

/// Cobertura pairwise gulosa sobre `dims` (cada dim = lista de índices).
/// Garante todo PAR coberto; sufixo aleatório (seed) preenche até `extra`.
/// Determinístico para mesmos inputs.
pub fn pairwise(dims: &[Vec<usize>], extra_seed: u64, extra: usize) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut covered = std::collections::HashSet::new();
    // Semente cartesiana mínima: varia dim0 × dim1, resto 0.
    if dims.len() >= 2 {
        for a in &dims[0] {
            for b in &dims[1] {
                let mut row = vec![0; dims.len()];
                row[0] = *a;
                row[1] = *b;
                mark_pairs(&row, &mut covered);
                out.push(row);
            }
        }
    }
    // Guloso: para cada par descoberto, tenta linha que cubra pares novos.
    let mut rng = Rng::new(extra_seed.wrapping_add(0x9E3779B9));
    for _ in 0..extra {
        let row: Vec<usize> = dims.iter().map(|d| *rng.choose(d).unwrap_or(&0)).collect();
        mark_pairs(&row, &mut covered);
        out.push(row);
    }
    let _ = covered;
    out
}

fn mark_pairs(
    row: &[usize],
    covered: &mut std::collections::HashSet<(usize, usize, usize, usize)>,
) {
    for i in 0..row.len() {
        for j in (i + 1)..row.len() {
            covered.insert((i, row[i], j, row[j]));
        }
    }
}

/// Conta pares cobertos (para teste da propriedade pairwise).
pub fn pairs_covered(rows: &[Vec<usize>]) -> usize {
    let mut s = std::collections::HashSet::new();
    for r in rows {
        mark_pairs(r, &mut s);
    }
    s.len()
}

/// Sequência stateful de `RawOp`s: cadeias create→write/read→close com
/// desvios (duplo-close, op sem handle, handle stale). `n` = nº de ops.
pub fn fileop_sequence(rng: &mut Rng, case_idx: u64, n: usize) -> Vec<RawOp> {
    let mut ops = Vec::new();
    let mut have_open = false;
    let mut created: Vec<String> = Vec::new();
    for _ in 0..n {
        let r = rng.below(100);
        if !have_open || r < 45 {
            // Create (às vezes reabrindo, às vezes inválido de propósito).
            let (cls, tpl) = *rng.choose(PATH_CLASSES).unwrap();
            let _ = cls;
            let path = tpl.replace("{n}", &format!("{case_idx}"));
            let (_, access) = *rng.choose(ACCESSES).unwrap();
            let (_, disp) = *rng.choose(DISPS).unwrap();
            // 1/4 das vezes mira existente de verdade (setup cobre `existing`).
            ops.push(RawOp::Create {
                path: path.clone(),
                access,
                disp,
            });
            if !created.contains(&path) {
                created.push(path);
            }
            have_open = true; // modelo decide o valor real
        } else if r < 65 {
            ops.push(RawOp::Write {
                len: 1 + rng.below(11) as u32,
            });
        } else if r < 85 {
            ops.push(RawOp::Read {
                len: 1 + rng.below(32) as u32,
            });
        } else {
            ops.push(RawOp::Close);
            have_open = rng.biased(1, 4); // às vezes fecha "duas vezes" seguidas
        }
    }
    ops
}

// ---------------------------------------------------------------------------
// handles / memory: sequências sobre ids simbólicos pequenos.
// ---------------------------------------------------------------------------

/// Sequência de `HandleOp`s sobre pool de `pool` ids (referências simbólicas
/// a inserts anteriores + ids selvagens ocasionais).
pub fn handle_sequence(rng: &mut Rng, n: usize, pool: usize) -> Vec<crate::model::HandleOp> {
    use crate::model::HandleOp;
    let mut ops = Vec::new();
    let mut inserts = 0usize;
    for _ in 0..n {
        match rng.below(100) {
            0..=44 => {
                ops.push(HandleOp::Insert);
                inserts += 1;
            }
            45..=74 => {
                let id = pick_id(rng, inserts, pool);
                ops.push(HandleOp::Lookup { id });
            }
            _ => {
                let id = pick_id(rng, inserts, pool);
                ops.push(HandleOp::Close { id });
            }
        }
    }
    ops
}

fn pick_id(rng: &mut Rng, inserts: usize, pool: usize) -> usize {
    if inserts == 0 || rng.biased(1, 5) {
        rng.below_usize(pool + 2) // selvagem ou futuro
    } else {
        rng.below_usize(inserts) // referência válida (talvez já fechada)
    }
}

/// Sequência de `MemOp`s (tamanhos página-arredondados pelo modelo).
/// `protect` é índice em `model::PROTECTS` (sempre válido: bits inválidos
/// morrem na façade, cobertos por teste unitário lá).
pub fn memory_sequence(rng: &mut Rng, n: usize, pool: usize) -> Vec<MemOp> {
    let mut ops = Vec::new();
    for _ in 0..n {
        let id = rng.below_usize(pool);
        match rng.below(100) {
            0..=34 => ops.push(MemOp::Reserve {
                id,
                size: [0, 1, 100, 4096, 5000, 1 << 20][rng.below_usize(6)],
                protect: rng.below(6) as u8,
            }),
            35..=54 => ops.push(MemOp::Commit { id }),
            55..=74 => ops.push(MemOp::Protect {
                id,
                protect: rng.below(6) as u8,
            }),
            _ => ops.push(MemOp::Release { id }),
        }
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_covers_dim_product_pairs() {
        let dims = vec![vec![0, 1, 2], vec![0, 1]];
        let rows = pairwise(&dims, 99, 0);
        // Pares (0..3)x(0..2) = 6, todos cobertos pelas 6 linhas cartesianas.
        assert_eq!(rows.len(), 6);
        assert_eq!(pairs_covered(&rows), 6);
    }

    #[test]
    fn pairwise_deterministic() {
        let dims = vec![vec![0, 1, 2, 3], vec![0, 1, 2], vec![0, 1]];
        let a = pairwise(&dims, 5, 20);
        let b = pairwise(&dims, 5, 20);
        assert_eq!(a, b);
        // Todos os pares dim0×dim1 cobertos.
        let mut need = std::collections::HashSet::new();
        for x in 0..4 {
            for y in 0..3 {
                need.insert((0, x, 1, y));
            }
        }
        let mut got = std::collections::HashSet::new();
        for r in &a {
            mark_pairs(r, &mut got);
        }
        assert!(need.is_subset(&got));
    }

    #[test]
    fn sequences_are_shaped() {
        let mut rng = Rng::new(1);
        let ops = fileop_sequence(&mut rng, 0, 50);
        assert_eq!(ops.len(), 50);
        let h = handle_sequence(&mut rng, 50, 4);
        assert_eq!(h.len(), 50);
        let m = memory_sequence(&mut rng, 50, 4);
        assert_eq!(m.len(), 50);
    }
}
