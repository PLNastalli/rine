# launcher

Binário `rine`: `rine <programa.exe>` → exit code do guest.

- Não é: runtime (só CLI + leitura do arquivo + `process::exit`).
- API: binário; testes E2E em `tests/hello.rs` e `tests/v02.rs`
  (filho isolado, stdout+exit).
  Deps: `runtime`. Unsafe: nenhum.
- Futuro: flags (`--capsule`, `--dump-imports`), modo `--check` sem executar.
