# ADR-0005 — Abstração do host Linux (`host-linux`)

- Status: aceito (2026-09-10).
- Context: como acessar o kernel sem espalhar `libc` pelo projeto?
- Decision: UMA crate (`host-linux`) detém todos os syscalls
  (`mmap/mprotect/munmap/arch_prctl/write/...`); demais crates chamam wrappers
  safe tipados (`Prot`, `HostError`). Auditoria: `grep libc::` fora dela deve
  ser vazio.
- Alternatives: cada subsistema chamar `libc` (rápido no início, impensável
  para portabilidade/testes — ex.: backend `seccomp`/mock futuro).
- Why: fronteira `unsafe` única e auditável; troca de backend (ex. `rustix`
  puro, `io_uring`) num só lugar; mapeamentos Windows→Linux documentados
  (nunca equivalência cega — ex. `GUARD` sem par direto).
- Consequences: `host-linux` não pode depender do workspace (DAG).
- Risks: API mínima demais (estender por necessidade, com teste).
