# winabi

Vocabulário ABI Windows (tipos, layouts, constantes). Sem lógica, sem I/O.

- Não é: implementação de nada (só define `NtStatus`, `WindowsHandle`,
  `Rva/VA`, `AllocType/PageProtect`, `Tib/TebMinimal/PebMinimal`, convenção x64).
- API: ver `src/lib.rs` (tudo documentado). Conversão `ntstatus_to_win32`.
- Deps: `bitflags`. Consumers: todas as crates.
- Invariantes: valores/constantes == Windows; layouts `repr(C/transparent)`.
- Threads: tipos `Copy`; sem estado. Erros: n/a. Unsafe: nenhum.
- Testes: `cargo test -p winabi`. ADRs: 0004, 0006. Futuro: tabela gerada
  `NTSTATUS↔Win32` completa (v0.3).
