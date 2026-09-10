# nt-sync

Sincronização NT. `Event` em memória (v0.1); critical sections
single-threaded (v0.3); futex/eventfd com threads (v0.3+).

- Não é: espera bloqueante real ainda.
- API: `Event::{set,reset,is_signaled}`, `nt_set_event`,
  `initialize/enter/leave/delete_cs` (critical sections, single-threaded).
  Deps: `winabi`. Consumers: `kernelbase` (CS).
- Testes: unit (ciclo/recursão/contenção recusada); E2E via `suite.exe`
  (saídas 69–73). ADRs: 0003 (futex quando semântica permitir), 0013 (CS).
