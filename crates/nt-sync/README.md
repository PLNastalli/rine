# nt-sync

Sincronização NT. v0.1: `Event` em memória; futex/eventfd em v0.3.

- Não é: espera bloqueante real ainda.
- API: `Event::{set,reset,is_signaled}`, `nt_set_event`. Deps: `winabi`.
  Consumers: `ntdll` (futuro).
- Testes: (v0.3: `sync.exe`). ADRs: 0003 (futex quando semântica permitir).
