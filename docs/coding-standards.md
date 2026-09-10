# Padrões de código

- Rust moderno, workspace, edition 2021, `resolver = "2"`. Formatação `cargo fmt`.
- Clippy com `-D warnings`: zero warnings é gate, não aspiração.
- Erros internos: `Result<T, NtStatus>` / enums `thiserror` convertíveis em
  `NtStatus`. Nada de `String` como erro em `nt-*`.
- Newtypes para não confundir domínios: `WindowsHandle ≠ fd`, `Rva ≠ VA`,
  `NtStatus ≠ Win32Error`. Conversões explícitas (`ntstatus_to_win32`).
- `bitflags` para flags Windows; `tracing` para diagnóstico (sem `println!`
  no runtime; `eprintln!` só no `launcher` para erros fatais).
- Sem `unwrap`/`expect` em caminhos do runtime; testes podem.
- Arquivos pequenos; uma responsabilidade por crate (ver README da crate).
- `TODO` rastreável: `// TODO(<id|issue>): o quê + condição de remoção`.
  Nunca `// TODO fix later`.
- Código gerado separado (`generated/` ou `codegen/`), com fonte/versão/como
  regenerar no cabeçalho. Não conta como complexidade do runtime.
- Commits pequenos, mensagens com intenção; referenciar ADR (`Refs ADR-0003`).
