# nt-object

NT Object Manager + `HandleTable` geracional.

- Não é: I/O ou syscalls (só lifetime/tipos/nomes).
- API: `HandleTable::{insert,lookup,close,init_console}`, `KernelObject`,
  `ObjectType`, `ObjectPayload::File`, `FileObject` (+`owns_fd`).
- Deps: `winabi`. Consumers: `nt-file`, `ntdll`, `runtime`.
- Invariantes: stale → `INVALID_HANDLE` (geração); `Arc` = refcount NT.
  Threads: `Mutex` interno; `Arc` compartilhável. Erros: `NtStatus`.
  Unsafe: nenhum.
- Testes: insert/lookup/close, anti-stale, null/invalid. ADRs: 0002.
  Futuro: namespaces hierárquicos, herança, duplicação (v0.2–v0.3).
