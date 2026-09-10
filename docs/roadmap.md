# Roadmap

## Versionamento (semver real; updates pequenos não sobem números grandes)

Versão atual: `0.2.0-alpha.1` (`rine --version`). Enquanto `0.x`:
pré-releases `-alpha.N` durante o milestone; ao fechar, `0.MINOR.0`;
fixes viram `0.MINOR.PATCH` (sobe só o último número). `1.0` só com ABI
estável. Regra: número grande (minor em 0.x) só por milestone fechado —
nunca por commit avulso.

Ordem de milestones é obrigatória: não pular porque uma feature posterior
é tentadora.

## v0.1 — Hello World direto na CPU ✅ (concluído 2026-09-10)

Critérios: PE32+ x86_64 próprio; código original na CPU; ntdll/kernel32 mínimas;
`hello.exe → "Hello World"` sem Wine/Proton/VM; testes + docs base.
APIs: `GetStdHandle`, `WriteFile`, `ExitProcess`, `RtlExitUserProcess`,
`NtTerminateProcess`.

## v0.2 — Processo e filesystem (em andamento, ~90%)

Goal: 3+1 binários reais passando (file/alloc/args + suite/evil) + infra
de conhecimento (scanner, oracle, demanda MinGW).

Requirements: `NtCreateFile/Read`, drives, `VirtualAlloc` público,
registry em arquivo, `capsule.toml`, `suite/evil.exe`, api-db, cobertura.

Tests: `tests/{hello,v02,suite,mingw_demand,matrix}.rs`, units, ABI, fuzz.

Exit criteria: gate verde + matriz RINE_PASS + docs (falta: diferencial
Windows — sem host — e fechamento formal).

Explicitly out of scope: threads extras, SEH, DLL loading, GUI (v0.3+).

## v0.3 — Threads, sync, exceptions (+ demanda MinGW)

Goal: `hello_mingw.exe` (CRT) passar a carregar — demanda travada em
`tests/windows/hello_mingw.imports.json` (46 imports, 38 em aberto).

Requirements (feito ✅ / falta): TLS slots ✅, `Sleep` ✅, `GetLastError` ✅,
critical sections ✅, `SetUnhandledExceptionFilter` mínimo,
`VirtualQuery`, `LoadLibraryA/GetProcAddress` (loader de DLLs próprias primeiro),
`__try/__except` mínimo, API Sets resolvidos via `apisets.json`,
NTSTATUS↔Win32 gerada.

Tests: `threads.exe`, `seh.exe`, `dll.exe` + MinGW demand zerada.

Exit criteria: `hello_mingw.exe → "Hello MinGW"`, exit 0 + gate + docs.

Explicitly out of scope: GUI, rede, áudio (v0.4+).

## v0.4 — Isolamento e loader completo

- Processo emulado isolado (fork/namespaces por Capsule); IPC versionado para
  coordenação estritamente necessária (nada de servidor central por chamada).
- DLL loading (`LoadLibrary`/`GetProcAddress`), forwarded exports, TLS
  callbacks, `.pdata`/exceções por imagem, ASLR.
- Critério: `dll.exe` com DLL própria carregada pelo nosso loader.

## v0.5+ — Win32 pleno e além (NÃO implementar antes)

User32/Wayland, GDI, Winsock, COM, áudio/PipeWire, DXGI/D3D→Vulkan, WoW64,
serviços, .NET. Cada um com milestone próprio quando v0.4 estiver verde.

## Explicitamente NÃO fazer agora

GUI, DirectX, COM, .NET, jogos, servidor central obrigatório, hacks
app-específicos no núcleo (`if exe == ...` é proibido; usar `compat/quirks/`).
