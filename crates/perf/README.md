# perf

Regression gate de performance: schema versionado, estatística robusta
(mediana de batches pós-warmup), comparação pura e relatório humano+JSON.

- Não é: otimizador, profiler, nem decisor acoplado à coleta.
- API: `schema` (RunResults/Baseline/Environment), `stats`
  (median/mean/stddev), `compare` (`Policy::alpha` vigente, `::stable`
  futura; `Status::{Pass,Warning,Regression,Missing,Incompatible,Fail}`),
  `report` (render + exit codes). Bin `rine-bench`: `run`, `compare`,
  `check` (CI), `promote` (manual, com `--reason` obrigatório).
- Deps: `serde`, `serde_json`, `thiserror` (+ `pe`, `runtime` SÓ no bin,
  para medir o escopo real; o lib nunca as importa).
  Consumers: CI (`check`), dev (`run/compare/promote`).
- Invariantes: mesma seed metodológica → mesmos vereditos; baseline sem
  `--reason` nunca é escrita; regressão nunca promove sozinha; máquinas
  diferentes nunca são comparadas como iguais (nota `env difere`).
  Unsafe: nenhum.
- Testes: `stats` (mediana/outlier/stddev populacional), `compare`
  (melhora, igualdade, +5/+10/+12/+15/+20%, ausente, incompatível ×3,
  env), `report` (formato + exit codes). ADRs: 0012.
  Futuro: `--strict` no CI dedicado; mais benchmarks por demanda.
