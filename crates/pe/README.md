# pe

Parser PE32+ x86_64 (safe, só leitura) + builder de PEs de teste.

- Não é: mapeador (sem mmap), resolvedor (sem IAT patch), executor.
- API: `Image::parse`, `rva_to_offset`, `imports/exports/relocs`,
  `builder::build_minimal_hello`.
- Deps: `winabi`, `tracing`, `thiserror`. Consumers:
  `nt-loader`, `loader`, `runtime`, testes E2E.
- Builders: `hello` (congelado) + `file/alloc/args` via `assemble()` e
  `.rdata` genérico; helpers `emit_*` com regressões documentadas.
- Invariantes: nunca panica em bytes arbitrários (só `PeError`); RVAs
  validadas por seção. Threads: imutável após parse (`&[u8]` borrow).
  Erros: `PeError`. Unsafe: nenhum.
- Testes: `cargo test -p pe` (+ `launcher/tests/hello.rs` como oracle).
  ADRs: 0001. Futuro: TLS callbacks parse completo, `.pdata` (v0.3–v0.4).
