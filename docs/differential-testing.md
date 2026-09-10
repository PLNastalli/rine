# Teste diferencial no Rine

Compara o MESMO cenário em dois lados (Rine × modelo, Rine × baseline
gravada, futuramente Rine × Windows) após normalização explícita.

## Pipeline (implementação: `crates/difftest`)

```text
Scenario (schema 1, seed registrada)
  → gerar (gen: combinatório/pairwise/aleatório/stateful)
  → executar (guest via `rine`+timeout, ou in-process)
  → OracleResult (schema 1, `crates/oracle`)
  → normalizar (`normalize`: regras documentadas, testadas)
  → comparar vs Expectation/modelo (`compare`)
  → Verdict: Match | KnownDifference | SemanticMismatch | Crash
           | Timeout | OracleFailure | RineFailure | InfrastructureFailure
```

## Alvos atuais

| Alvo | API | Execução | Velocidade |
|---|---|---|---|
| `fileops` | CreateFileA (+R/W/Close) | guest (`pe::driver`) | ~3k casos/s |
| `handles` | HandleTable vs modelo | in-process | ~200k casos/s |
| `memory` | MemoryManager vs modelo | in-process | ~90k casos/s |
| `pe` | parser (fuzz) | in-process | ~1M casos/s |

## CLI

```bash
rine-test differential fileops --cases 100 --seed 42 --workers 4
rine-test reproduce <cases/X.json>   # reexecuta exato pelo seed+params
rine-test minimize <cases/X.json>    # ddmin preservando a divergência
```

`reproduce` antes de investigar; `minimize` antes de corrigir; caso mínimo
vai para `tests/regression/` (ver `docs/fuzzing.md`).

## Regras

- Veredito nunca é só PASS/FAIL (ver taxonomia acima).
- Divergência real → workflow de bug (abaixo), nunca hack no teste.
- Sem host Windows aqui: lado Windows = baselines gravadas + probe C;
  `DifferentiallyVerified` só com medição real (nunca fictício).
- Determinismo: mesma seed, 1 ou N workers → mesmos vereditos
  (`difftest/tests/determinism.rs` prova para fileops).

## Workflow de bug

`cenário aleatório → reproduce (seed) → confirme → minimize → reproducer
mínimo → regression test em tests/regression/ → identifique a camada →
corrija → rode regression → rode campanha maior`
