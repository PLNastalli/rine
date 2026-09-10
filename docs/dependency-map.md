# Mapa de dependências (sem ciclos; podado na auditoria 2026-09-10)

```text
launcher ──► runtime ──┬──► loader ──► nt-loader ──► pe ──► winabi
                       │       ├──────► nt-memory ──► host-linux
                       │       └──────► host-linux
                       ├──► kernel32 ──► kernelbase ──► ntdll ──┬──► nt-file ──► host-linux
                       │       ├──────► nt-thread              └──► nt-object ──► winabi
                       │       └──────► ntdll
                       ├──► ntdll, nt-object, nt-memory, nt-file, nt-loader, loader
                       ├──► host-linux, pe, winabi, toml

winabi: bitflags (folha)               host-linux: libc + thiserror (folha)
pe: winabi + tracing + thiserror        nt-*: só winabi (+toml em nt-registry)
api-scan: pe + winabi + kernel32 + ntdll + serde/serde_json (ferramenta)
oracle: serde/serde_json (formato; runner usa `rine` via processo)
difftest: oracle + pe + winabi + nt-object + nt-memory + nt-file + serde (+proptest em dev)
launcher: runtime + tracing-subscriber (+pe/serde_json/difftest em dev)
```

Regras: `host-linux` não depende do workspace. `winabi` é vocabulário.
Façades nunca são deps de `nt-*`. Deps sem consumidor são removidas
(auditoria deletou ~40; `cargo tree` deve continuar DAG).
`runtime` orquestra via façades + `nt-file`/`nt-loader`/`loader`/`nt-memory`
(memória do processo é criada no load e instalada no contexto).
