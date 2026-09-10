# difftest

Plataforma de testes diferenciais: cenários com seed → execução → normalização
→ veredito rico → shrink → corpus. CLI `rine-test`.

- Não é: oracle Windows (isso é `crates/oracle` + probe); parser PE (isso é
  `crates/pe`); implementação do runtime (só observa).
- API: `scenario` (schema+seed+id), `gen` (combinatório/pairwise/stateful),
  `model` (File/Handle/Memory executáveis), `normalize` (regras explícitas),
  `compare` (8 vereditos), `shrink` (ddmin), `runner` (paralelo
  determinístico), `exec` (guest+timeout), `fuzz` (mutação), `report`
  (+merge na matriz), `corpus` (replay/promote).
- Deps: `oracle`, `pe`, `winabi`, `nt-object`, `nt-memory`, `nt-file`,
  `serde`, `serde_json`. Consumers: `rine-test`, `launcher/tests/*`.
- Invariantes: mesma seed → mesmos vereditos (1 ou N workers); nada fictício;
  tmpdir por caso; timeout sempre em guest. Unsafe: nenhum.
- Testes: units por módulo + `tests/{properties,concurrent,determinism,
  regression}.rs`. ADRs: 0010. Futuro: oracle remoto, mais alvos.
