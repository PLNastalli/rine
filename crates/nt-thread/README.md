# nt-thread

Modelo de thread NT (TEB, LastError TLS). Thread inicial única em v0.1.

- Não é: scheduler (threads extras em v0.3).
- API: `Thread::new`, `set/get_last_error`. Deps: `winabi`.
  Consumers: `kernel32` (LastError).
- Threads: TLS Rust hoje; GS/TEB instalado pelo `runtime`. Unsafe: nenhum.
  Testes: `nt-thread` unit. ADRs: 0004. Futuro: `Clone`+TEB/thread (v0.3).
