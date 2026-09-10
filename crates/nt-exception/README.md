# nt-exception

Modelo SEH. v0.1: tipos + códigos; tradução signal→SEH em v0.3.

- Não é: handler instalado ainda.
- API: `ExceptionRecord`, `code::*`. Deps: `winabi`. Consumers: `ntdll`,
  `runtime` (futuro). Testes: (v0.3: `seh.exe`). ADR: 0001 (riscos).
