# Mapa de dependências (sem ciclos; podado na auditoria 2026-09-10)

```text
launcher ──► runtime ──┬──► loader ──► nt-loader ──► pe ──► winabi
                       │       ├──────► nt-memory ──► host-linux
                       │       └──────► host-linux
                       ├──► kernel32 ──┬──► kernelbase ──► ntdll ──┬──► nt-file ──► host-linux
                       │               │                          ├──► nt-thread
                       │               │                          ├──► nt-sync (Rtl*CriticalSection)
                       │               │                          └──► nt-object ──► winabi
                       │               ├──► nt-thread
                       │               └──► ntdll
                       ├──► ntdll, nt-object, nt-memory, nt-file, nt-loader, loader
                       ├──► nt-thread, host-linux, pe, winabi, toml

winabi: bitflags (folha)               host-linux: libc + thiserror (folha)
pe: winabi + tracing + thiserror        nt-*: só winabi (+toml em nt-registry)
api-scan: pe + winabi + kernel32 + ntdll + serde/serde_json (ferramenta)
oracle: serde/serde_json (formato; runner usa `rine` via processo)
difftest: oracle + pe + winabi + nt-object + nt-memory + nt-file + serde (+proptest em dev)
perf: serde/serde_json/thiserror (+pe/runtime SÓ no bin rine-bench; lib puro)
launcher: runtime + tracing-subscriber (+pe/serde_json/difftest em dev)
manager-core: pe + runtime + serde/serde_json + thiserror (SEM tauri/react;
  nada do core depende dele — fronteira de frontend)
manager-tauri: manager-core + tauri/tauri-plugin-dialog (shell fino;
  NENHUM crate do core depende de manager-*)
```

Regras: `host-linux` não depende do workspace. `winabi` é vocabulário.
Façades nunca são deps de `nt-*`. Núcleo nunca depende de `manager-*`
(o Manager consome o runtime; o inverso é proibido). Deps sem consumidor
são removidas
(auditoria deletou ~40; `cargo tree` deve continuar DAG).
`runtime` orquestra via façades + `nt-file`/`nt-loader`/`loader`/`nt-memory`
(memória do processo é criada no load e instalada no contexto).
TLS (v0.3): `kernelbase` → `nt-thread` (`teb_tls_slot`), `ntdll` → `nt-thread`
(`TlsBitmap`), `runtime` → `nt-thread` (bitmap do contexto).
