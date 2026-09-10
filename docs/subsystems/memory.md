# Memória virtual

`VirtualAlloc` ≠ `mmap`. Máquina de estados por região:

```text
        RESERVE             COMMIT (integral, v0.1)
livre ─────────► Reserved ─────────► Committed
                     │                   │ ╲
                     │ RELEASE           │ DECOMMIT / RELEASE
                     ▼                   ▼  ▼
                   livre               Reserved / livre
```

Proteções: `PAGE_*` → `host_linux::Prot` via `host_protect` (`NOACCESS`→NONE;
`GUARD` = committed normal, quirk QUI-0001). Seções PE → `PAGE_*` via
`section_protect` (fonte única; `loader::protect_sections` consolidado nela
na auditoria 2026-09-10). Gerente por processo (`ProcessContext.mem`, v0.2):
`VirtualAlloc/Free/Protect` públicos; imagem rastreada por seção
(`track_external`) para o alocador nunca colidir. Commit parcial e
`VirtualQuery` em v0.3. Testes: ciclos + `section_protect` + `alloc.exe` (E2E).
