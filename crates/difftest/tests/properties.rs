//! Propriedades (proptest): modelo×real em centenas de entradas aleatórias.
//!
//! Por que proptest e não quickcheck (ADR-0010): seeds determinísticas
//! configuráveis (`PROPTEST_CASES`, `TestRunner` com seed fixa em CI),
//! shrinking integrado (minimiza contra-exemplo sozinho) e manutenção ativa.
//! Casos: default 256 (rápido, in-process); campanhas CLI escalam além.

use difftest::{campaigns, model};
use proptest::prelude::*;

fn arb_handle_op() -> impl Strategy<Value = model::HandleOp> {
    use model::HandleOp;
    prop_oneof![
        Just(HandleOp::Insert),
        (0usize..8).prop_map(|id| HandleOp::Lookup { id }),
        (0usize..8).prop_map(|id| HandleOp::Close { id }),
    ]
}

fn arb_mem_op() -> impl Strategy<Value = model::MemOp> {
    use model::MemOp;
    prop_oneof![
        (0usize..5, prop::option::of(0usize..7000), 0u8..6).prop_map(|(id, size, p)| {
            MemOp::Reserve {
                id,
                size: size.unwrap_or(0),
                protect: p,
            }
        }),
        (0usize..5).prop_map(|id| MemOp::Commit { id }),
        (0usize..5, 0u8..6).prop_map(|(id, protect)| MemOp::Protect { id, protect }),
        (0usize..5).prop_map(|id| MemOp::Release { id }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn handles_agree_with_model(ops in prop::collection::vec(arb_handle_op(), 0..30)) {
        let (v, d, _) = campaigns::run_handles_once(0, 0, &ops);
        prop_assert_eq!(v, difftest::compare::Verdict::Match, "{}", d);
    }

    #[test]
    fn memory_agrees_with_model(ops in prop::collection::vec(arb_mem_op(), 0..30)) {
        let (v, d, _) = campaigns::run_memory_once(&ops);
        prop_assert_eq!(v, difftest::compare::Verdict::Match, "{}", d);
    }

    #[test]
    fn translate_never_panics_nor_escapes_drive(s in "[A-Za-z0-9_.\\\\:/\\- é]{0,40}") {
        use std::collections::HashMap;
        use std::path::Component;
        let mut d = HashMap::new();
        d.insert('C', "/tmp/xyz".to_string());
        let m = nt_file::DriveMap::new(d, None);
        let path = format!("C:\\{s}");
        if let Ok(p) = m.translate(&path) {
            // Sob o drive (sem escape por `..`: nenhum ParentDir restante).
            prop_assert!(p.starts_with("/tmp/xyz"), "{p:?}");
            prop_assert!(
                !p.components().any(|c| matches!(c, Component::ParentDir)),
                "escape: {p:?}"
            );
        }
    }
}
