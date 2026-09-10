# api-scan

Oracle estrutural da referência Windows. Reutiliza o parser `pe`
(nunca duplica). Gera conhecimento (`api-db/`), não stubs, não código.

- API: `scan::{scan_tree,scan_one}`, `db::write_db`, `coverage::{report,
  RineImpls,behavior_tested,ApiStatus}`, `record::PeRecord` (schema 1).
- Bin `rine-api-scan`: `scan <dir> <out>` + `coverage <db> [--out]`.
- Deps: `pe`, `winabi`, `kernel32`, `ntdll` (tabelas EXPORTS), `serde`,
  `serde_json`. Consumers: dev (CLI) + `api-db/`.
- Invariantes: parse-error vira `skipped` com motivo (nunca aborta, nunca
  panic); nomes de arquivo normalizados (referência é case-insensitiva).
- Testes: scan de PE próprio + malformado→skipped + honestidade da cobertura.
  ADRs: 0009. Futuro: mais DLLs no escopo de cobertura por demanda.
