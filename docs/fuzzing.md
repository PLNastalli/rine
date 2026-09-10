# Fuzzing no Rine

Sem libfuzzer (stable por reprodutibilidade — ADR-0010). Mutação
determinística + seeds fixas: mesma seed, mesmos casos, para sempre.

## Alvos e propriedades

| Superfície | Propriedade | Onde |
|---|---|---|
| Parser PE | nunca panic/UB; `Ok`/`Err` | `pe/tests/fuzz_corpus.rs` → `difftest::fuzz` |
| Loader/imports/exports/relocs | idem, via views totais | idem (`check_parse`) |
| Paths NT | nunca panic; sem fuga do drive | `nt-file/tests/robust.rs` + proptest |
| Registry/TOML | `Err` limpo, nunca panic | `nt-file/tests/robust.rs` |
| API Sets | fixture apenas (parser futuro; gap documentado) | `api-db/.../apisets.json` |

PEs malformados (headers truncados, section count absurdo, overflow,
RVA fora, reloc inválida, import circular, string sem NUL, overlap)
devem gerar erro controlado — nunca corrupção, UB ou acesso arbitrário.

## Escala

```bash
rine-test fuzz pe --cases 1000000 --seed 99     # ~1M casos/s
rine-test fuzz paths --cases 20000 --seed 11
```

Mutação mora em `difftest::fuzz::mutate` (fonte única; o teste do `pe`
consome de lá). Propriedades com shrinking: proptest
(`difftest/tests/properties.rs`).

## Achou falha?

`seed → reproduce → minimize → corrija → caso mínimo em
tests/regression/` (nunca volta silenciosamente). Concorrência: seeds +
threads + barreiras registradas (`difftest/tests/concurrent.rs`).
