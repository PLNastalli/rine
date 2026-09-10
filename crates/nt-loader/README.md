# nt-loader

Lógica de imagem: relocs + imports + entry (sobre memória já mapeada).

- Não é: mmap (isso é `loader`), parse (isso é `pe`).
- API: `apply_relocs`, `resolve_imports`. Deps: `winabi`, `pe`.
  Consumers: `loader`, `runtime`.
- Invariantes: toda escrita com guard `rva+8 ≤ size`; `ADDR64` com delta.
  Erros: `LoaderError→NtStatus`. Unsafe: sim, documentado por função.
- Testes: delta-zero, ADDR64, via E2E. ADRs: 0001. Futuro: TLS callbacks,
  `.pdata`, forwarded exports (v0.4).
