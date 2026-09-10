# Compatibilidade (v0.1)

Compatibilidade = comportamento observável idêntico ao Windows real, não mera
existência de símbolos.

## Cobertura de APIs

| DLL | API | Estado | Teste |
|---|---|---|---|
| kernel32 | `GetStdHandle` | ✅ (-11/-12; -10 → `FILE_NOT_FOUND`) | `launcher/tests/hello.rs` |
| kernel32 | `WriteFile` | ✅ (console + arquivos; `*written` sempre) | hello + v02 |
| kernel32 | `ReadFile` | ✅ (short count em EOF) | v02 (`file.exe`, exit 11) |
| kernel32 | `CreateFileA` | ✅ (`C:\`, `\??\`, relativo; disp 1–5) | v02 + `nt-file` unit |
| kernel32 | `CloseHandle` | ✅ (fecha fd possuído; nunca 1/2) | v02 |
| kernel32 | `VirtualAlloc/Free/Protect` | ✅ (região integral; sem `VirtualQuery` ainda) | v02 (`alloc.exe`, exit 66) |
| kernel32 | `GetCommandLineW` | ✅ (UTF-16 da Capsule/argv) | v02 (`args.exe`) |
| kernel32 | `ExitProcess` | ✅ (nunca retorna; exit code preservado) | hello (exit 0) |

Cobertura standing: `suite.exe` (todas acima em cadeia, exit 0) e `evil.exe`
(12 recusas limpas: handle inválido/stale, drive inexistente, disposition
ruim, alloc size-0/tipo-0, free solto, stdhandle ruim) — `tests/suite.rs`.
| ntdll | `RtlExitUserProcess` | ✅ | via `ExitProcess` |
| ntdll | `NtTerminateProcess` | ✅ (processo atual) | compilado, sem teste E2E ainda |
| ntdll | `NtWriteFile` (interno) | ✅ | idem `WriteFile` |
| todo resto | — | ❌ (retornar endereço nulo = bug do resolvedor, não silent-stub) | — |

Política de stubs: o resolvedor **falha o load** (`Unresolved`) em vez de
instalar stub silencioso. Stub que finge sucesso é bug pior que ausência.

## Método de medição

Diferencial vs Windows real (oracle). Para cada teste: mesmos bytes do PE,
mesmos args/stdin; comparar exit code, stdout/stderr bytes, `GetLastError`
quando observável. Harness em `crates/launcher/tests/` (filho isolado) +
futuro `tests/differential/` com corpus versionado e relatório CSV.

## Como adicionar cobertura

1. Primeiro o teste (`tests/behavior/<api>.exe` via builder ou compilador
   Windows futuro) demonstrando o comportamento real.
2. Implementar na camada certa (ver `subsystem-map.md`).
3. Atualizar esta tabela + `current-state.md`.
