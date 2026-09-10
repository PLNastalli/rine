# oracle

Windows Oracle comportamental: formato + produtor lado-Rine.

- API: `format::OracleResult` (schema 1), `runner::{TestCase,run_case}`.
- Runner Windows: template C em `tests/windows/oracle_probe.c` (compila com
  MinGW, executa SÓ no Windows real; mesmo formato → diff futuro).
- Deps: `serde`, `serde_json`. Unsafe: nenhum.
- Testes: roundtrip do formato + smoke do runner. ADRs: 0009.
- Regra: resultado Windows fictício é bug — só medido.
