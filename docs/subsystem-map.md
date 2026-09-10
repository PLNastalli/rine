# Mapa de subsistemas (quem implementa o quê)

| Conceito | Dono | Expõe via | Testes |
|---|---|---|---|
| Tipos ABI, PEB/TEB, NTSTATUS | `winabi` | tipos | `winabi` unit |
| Parse PE, builder hello | `pe` | `Image`, `builder` | `pe` unit + `launcher/tests/hello.rs` |
| Relocs/imports (lógica) | `nt-loader` | `apply_relocs`, `resolve_imports` | `nt-loader` unit |
| mmap/mapeamento | `loader` | `map_image`, `protect_sections` | `loader` unit |
| Syscalls Linux | `host-linux` | `reserve_*`, `protect`, `write_all`, `set_gs_base` | `host-linux` unit |
| Objetos + handles | `nt-object` | `HandleTable`, `KernelObject` | `nt-object` unit |
| Memória virtual NT | `nt-memory` | `MemoryManager` | `nt-memory` unit |
| Processo/PEB | `nt-process` | `Process` | (v0.2 E2E) |
| Thread/TEB/LastError | `nt-thread` | `Thread`, `set_last_error` | indireto via hello |
| Arquivos/console | `nt-file` | `write/read/create/close_file`, `DriveMap` | v02 E2E + unit |
| Sync/Event | `nt-sync` | `Event` | (v0.3) |
| Exceções/SEH | `nt-exception` | `ExceptionRecord` | (v0.3) |
| Registry | `nt-registry` | `Registry::{set,get,save,load}` | unit (roundtrip arquivo) |
| Segurança/token | `nt-security` | `Token` (stub) | — |
| Façade NT | `ntdll` | `RtlExitUserProcess_impl`, contexto | via hello |
| Win32 interna | `kernelbase` | `get_std_handle`, `write_file` | via hello |
| Win32 pública | `kernel32` | 21 exports (UEF novo) | hello + v02/v03 E2E |
| Orquestração/entry | `runtime` | `Emulator`, `Capsule::load_toml` | unit + hello/v02 |
| CLI | `launcher` | bin `rine` | hello E2E |
| Oracle estrutural | `api-scan` | `rine-api-scan scan/coverage`, `api-db/` | scan/coverage unit |
| Oracle comportamental | `oracle` | formato v1 + runner Rine + probe C | format/runner unit |
| Plataforma diferencial | `difftest` | cenários, modelos, shrink, corpus, `rine-test` | properties/concurrent/determinism/regression |
| Driver guest | `pe::driver` | `FileOp` + expectativas embutidas | driver unit + campanhas |
| Demanda | `runtime` | `missing_imports`, `demand_report` | `mingw_demand` |
| Performance gate | `perf` | schema/stats/compare/report, `rine-bench`, `bench/` | unit por módulo |

Lógica no lugar errado = bug arquitetural. Em dúvida, ver `architecture.md`.
