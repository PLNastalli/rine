# nt-memory

Memória virtual NT sobre mmap (RESERVE/COMMIT/DECOMMIT/RELEASE + `PAGE_*`).

- Não é: `mmap` direto (transição de estado primeiro, efeito depois).
- API: `MemoryManager::{allocate,free,protect_region,track_external}`.
- Deps: `winabi`, `host-linux`. Consumers: `ntdll` (mem do processo),
  `loader` (via `section_protect`/`host_protect`), `runtime` (rastreio).
- API extra: `section_protect` (fonte única PE→PAGE), `host_protect`,
  `track_external` (imagem rastreada no load).
- Invariantes: região rastreada em `BTreeMap`; `release` uma vez; tamanhos em
  páginas. Threads: lock único (granularidade fina em v0.4). Erros:
  `MemError→NtStatus`. Unsafe: nenhum (chama `host-linux` safe).
- Testes: ciclo reserve/commit/release. ADRs: 0003, 0005.
  Futuro: commit parcial, `VirtualQuery`, guard real (v0.2–v0.3).
