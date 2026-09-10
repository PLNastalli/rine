# nt-thread

Modelo de thread NT (TEB, LastError TLS, slots TLS). Thread inicial única.

- Não é: scheduler (threads extras em v0.3).
- API: `Thread::new`, `set/get_last_error`, `TlsBitmap::{alloc,free,
  is_allocated}`, `teb_tls_slot` (`# Safety`). Deps: `winabi`.
  Consumers: `kernel32` (LastError), `kernelbase` (TLS), `ntdll`/`runtime`
  (bitmap/TEB do contexto).
- Threads: TLS Rust hoje; GS/TEB instalado pelo `runtime`. Unsafe: só
  `teb_tls_slot` (fronteira TEB).
- Testes: `nt-thread` unit (bitmap, slots). ADRs: 0004, 0011. Futuro:
  `Clone`+TEB/thread, TLS de imagem (v0.3).
