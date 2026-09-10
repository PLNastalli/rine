# nt-process

Modelo de processo NT (PEB, parâmetros, exit code). v0.1: um emulado/host.

- Não é: isolamento real (fork/namespaces em v0.4).
- API: `Process::{new,terminate,exit_code}`, `ProcessParameters`.
  Deps: `winabi`. Consumers: (construção plena com isolamento v0.4).
- Threads/unsafe: nenhum. Erros: `NtStatus` (via callers). Testes:
  `nt-process` unit (lifecycle).
  ADRs: 0003. Futuro: isolamento, job objects.
