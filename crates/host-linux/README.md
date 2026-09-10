# host-linux

ÚNICA fronteira com o kernel Linux. Todo syscall passa aqui.

- Não é: política Windows (só mecanismo: `reserve_anonymous/fixed`,
  `protect`, `release`, `write_all`, `set/restore_gs_base`, `page_size`).
- API: ver `src/lib.rs`. Deps: `libc`, `thiserror`.
  Consumers: `nt-memory`, `nt-file`, `loader`, `runtime` (via subsistemas).
- Invariantes: cada função documenta `SAFETY`; `MAP_FIXED_NOREPLACE` nunca
  destrói mapeamento alheio. Threads: por-thread onde importa (GS/errno).
  Erros: `HostError` (errno preservado). Unsafe: sim — só aqui + trampolins
  documentados (ver `safety-model.md`).
- Testes: roundtrip reserve/protect/release. ADRs: 0003, 0005.
  Futuro: backend `rustix`-puro, `io_uring` (só se diferencial pedir).
