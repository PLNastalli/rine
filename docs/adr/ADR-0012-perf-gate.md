# ADR-0012 — Performance regression gate (visível, sem bloquear no alpha)

- Status: aceito (2026-09-10).
- Context: detectar lentidão relevante sem transformar ruído de medição em
  bloqueio falso; benchmarks antigos (média de batch único) insuficientes.
- Decision:
  1. Nova crate `perf`: lib pura (schema/stats/compare/report) + bin
     `rine-bench` (`run/compare/check/promote`). Coleta e decisão separadas.
  2. Metodologia: warmup descartado + N batches, amostra = ns/op por batch,
     decisão pela mediana; amostras + ambiente gravados (schema 1).
  3. Política alpha (10/15, warn-only; exit sempre 0 em Regression) com
     `Policy::stable` (5/5 strict) pronta. Trocar = constante + documento.
  4. Baseline versionada com auditoria (`--reason` obrigatório; piora exige
     `--accept-regression`); comparação vazia/incompatível nunca passa
     silenciosa (exit 2); máquina diferente gera nota, nunca fail.
  5. Benchmarks com ID estável + `method_version`; mudou escopo → novo ID.
     Escopos atuais preservam os examples antigos (nada pulado por número).
  6. CI mínimo (`.github/workflows/ci.yml`: fmt/clippy/test + `check`
     warn-only). Runners compartilhados publicam e avisam; dedicado futuro
     pode usar `--strict`.
- Alternatives: (a) criterion/iai (deps externas + harness próprio; std e
  mediana bastam); (b) bloquear em >15% já (falso-positivo em runner
  compartilhado); (c) baseline em .txt (sem ambienteamostras/auditoria).
- Why: regressão visível sem travar velocity do alpha; arquitetura pronta
  para endurecer.
- Consequences: `bench/baselines/*.json` versionados; `bench/results/`
  ignorado; examples antigos removidos (substituídos pelo runner).
- Risks: variância residual mesmo com mediana (mitigado: stddev visível +
  thresholds tolerantes); tentação de otimizar número (proibido em doc).
- Supersedes: baseline `.txt` de 2026-09-10 (metodologia antiga, removida).
