# runtime

Orquestração: `Capsule` + `Emulator::load/enter` (map→relocs→imports→
protect→TEB/GS→call entry).

- Não é: parse, mmap, syscalls, semântica Win32 (delega tudo).
- API: `Emulator::{load,enter}`, `Capsule::load_toml`,
  `missing_imports`/`demand_report` (loop de demanda).
  Deps: `winabi`, `pe`, `loader`, `nt-loader`, `nt-object`, `nt-memory`,
  `nt-file`, `ntdll`, `kernel32`, `host-linux`, `tracing`, `toml`.
  (Sem `kernelbase/nt-process/nt-thread`: orquestra via façades.)
  Consumers: `launcher`.
- Invariantes: contexto instalado antes do entry; GS restaurado no retorno;
  `Box<TEB/PEB>` vivos durante o guest. Threads: single-thread v0.1.
  Erros: `RuntimeError`. Unsafe: `transmute` do entry + GS (documentados).
- Testes: IAT resolvida (unit) + hello E2E. ADRs: 0001, 0003.
  Futuro: `capsule.toml`, isolamento (v0.2–v0.4). Exemplos: `check_iat`
  (ferramenta de dev, não API).
