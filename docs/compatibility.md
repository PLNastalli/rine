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
| kernel32 | `TlsAlloc/Free` | ✅ (bitmap 64 slots; exaustão → `TLS_OUT_OF_INDEXES`) | v03 (suite 63/66) |
| kernel32 | `TlsGetValue/SetValue` | ✅ (valor 64-bit; inválido → NULL/FALSE) | v03 (suite 64/65/67) |
| kernel32 | `GetLastError` | ✅ (TLS da thread; provado após erro forçado) | v03 (suite 68) |
| kernel32 | `Sleep` | ✅ (0 cede; ms dorme; INFINITE fiel — sem teste) | v03 (retorno) |
| kernel32 | `Initialize/DeleteCriticalSection` | ✅ (estado canônico; delete zera p/ visibilidade) | v03 (suite 69) |
| kernel32 | `Enter/LeaveCriticalSection` | ✅ (posse + recursão; contenção → `Err` p/ futex futuro; **forwarder → `ntdll!Rtl*`** como no Windows) | v03 (suite 70–73) |
| kernel32 | `SetUnhandledExceptionFilter` | ✅ (troca atômica, retorna anterior; opaco até SEH) | v03 (suite 74–75) |
| kernel32 | `VirtualQuery` | ✅ (48 bytes ou 0; BaseAddress/Size/State/Protect/Type; não-mapeado→0) | v03 (suite 76–83, evil 66–67) |
| kernel32 | `GetProcAddress` | ✅ (nome case-sensitive + ordinal real; == IAT; NULL+126/127) | v03 (suite 84–90, evil 68–69; unit `kernel32::modules`) |
| kernel32 | `GetModuleHandleA/W` | ✅ (NULL = base da imagem; normaliza caminho/case/`.dll`; ApiSet → 126) | v03 (suite 91–94/101, evil 72; unit `modules`) |
| kernel32 | `LoadLibraryA/W` | ✅ (conjunto carregado; imagem nova → 126 honesto, loader real em v0.4) | v03 (suite 95–98, evil 70) |
| kernel32 | `FreeLibrary` | ✅ (estático nunca descarrega = TRUE; estranho → 126) | v03 (suite 99–100, evil 71) |
| kernel32 | `CreateFileW` | ✅ (UTF-16 estrito; mesma semântica/dispositions da A) | v03 (suite 102–103, evil 73) |
| kernel32 | `GetFileAttributesW` | ✅ (DIRECTORY/NORMAL + READONLY; ausente → -1) | v03 (suite 109–110, evil 74) |
| kernel32 | `SetFilePointerEx` | ✅ (BEGIN/CURRENT/END via lseek; `lp` opcional) | v03 (suite 104–105, evil 75) |
| kernel32 | `GetFileSizeEx` | ✅ (SEEK_END + restore, sem mover o cursor) | v03 (suite 106–108, evil 76) |
| kernel32 | `DeleteFileW` | ✅ (diretório nega sozinho, como no Windows) | v03 (suite 117–118, evil 77) |
| kernel32 | `MoveFileExW` | ✅ (REPLACE honesto; cross-device = copy+remove) | v03 (suite 115–116, evil 78) |
| kernel32 | `CreateDirectoryW` | ✅ (sem recursão; existente = 80) | v03 (suite 111–112, evil 79) |
| kernel32 | `RemoveDirectoryW` | ✅ (só vazio; dup = erro) | v03 (suite 119–120, evil 80) |
| kernel32 | `ExitProcess` | ✅ (nunca retorna; exit code preservado) | hello (exit 0) |

Cobertura standing: `suite.exe` (todas acima em cadeia, exit 0) e `evil.exe`
(30 recusas limpas: handle inválido/stale, drive inexistente, disposition
ruim, alloc size-0/tipo-0, free solto, stdhandle ruim, TLS índice inválido,
VirtualQuery ruim, GetProcAddress ruim, LoadLibrary/FreeLibrary/GetModuleHandle ruins,
CreateFileW NULL, attrs ausente, seek/size em handle ruim,
delete/move/mkdir/rmdir ruins)
— `tests/suite.rs`. CS sem ponteiro inválido no evil (fronteira SEH).
| ntdll | `RtlExitUserProcess` | ✅ | via `ExitProcess` |
| ntdll | `Rtl*CriticalSection` (4) | ✅ (alvos dos forwarders kernel32; lógica única em `nt-sync`) | v03 (suite 69–73 via chase) |
| ntdll | `NtTerminateProcess` | ✅ (processo atual) | compilado, sem teste E2E ainda |
| ntdll | `NtWriteFile` (interno) | ✅ | idem `WriteFile` |
| todo resto | — | ❌ (retornar endereço nulo = bug do resolvedor, não silent-stub) | — |

Política de stubs: o resolvedor **falha o load** (`Unresolved`) em vez de
instalar stub silencioso. Stub que finge sucesso é bug pior que ausência.

Desvio documentado (até `LoadLibrary`, v0.4): os `HMODULE`s aceitos por
`GetProcAddress` são tokens opacos (`winabi::{KERNEL32,NTDLL}_PSEUDO_BASE`),
não bases mapeadas — `VirtualQuery` sobre eles retorna 0. Endereços
devolvidos são reais e chamáveis (iguais aos da IAT).

## ApiSet namespace (item 9) e forwarders (item 10)

- ApiSet: `pe::apiset` parseia o `API_SET_NAMESPACE` v6 de
  `apisetschema.dll`; snapshot em `api-db/.../apiset-map.json` (843 rotas);
  tabela gerada `kernel32/src/apiset_table.rs` (175 entradas p/ hosts
  implementados; `kernelbase.dll` roteia p/ `Kernel32` — aproximação
  documentada); teste de conformidade trava tabela↔snapshot. Regra v0:
  primeiro valor; vazio/`ext-ms-*`/seladas = sem rota (`MOD_NOT_FOUND`).
- Forwarders: `kernel32!*CriticalSection → ntdll!Rtl*` (ordinais/símbolos do
  oracle); `resolve()` persegue 1 salto. `kernelbase` perdeu as 4 fns CS
  (lógica única em `nt-sync` via ntdll) e as deps `nt-sync`/`tracing`.

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
