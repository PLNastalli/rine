# Estado atual

_Data: 2026-09-10 (quinta sessão). Versão: **0.2.0-alpha.3**.
Milestone: **v0.2 ~90%** + plataforma de testes diferenciais operacional._

## Milestone atual

v0.2 (processo+filesystem) + transversal: plataforma `difftest`/`rine-test`
pronta e caçando bugs de verdade. Próximo: v0.3 = `hello_mingw.exe` CRT
(demanda travada: 46 imports, 45 em aberto — `tests/windows/`).

## O que funciona

- E2E (filho isolado): hello/file/alloc/args/suite/evil + matrix RINE_PASS.
- `rine --capsule`, `--version`, relatório `RINE-DEMAND` (32 faltantes do MinGW).
- Infra: `rine-api-scan` (5578 DLLs/183k exports em 3s), `api-db/`
   (KERNEL32/NTDLL + coverage 25/4209 BehaviorTested), oracle schema 1
  (runner Rine + probe C compilável), ABI suite, fuzz determinístico,
  `RINE-CRASH`, `RUST_LOG` observável, benches + baseline, security-model.
- `hello.exe` âncora bit-idêntico (`c9ba94…`).
- Plataforma diferencial (`crates/difftest`, `rine-test`): fileops guest
  (~3k/s), handles/memory in-process (90–220k/s), fuzz PE (~1M/s);
  campanhas 200–500/500 Match; 4 bugs reais achados+corrigidos+regredidos
  (ver última tarefa). Baseline: `compatibility/baselines/fileops-seed42-200.json`.
- Política de não-regressão adotada (`docs/non-regression-policy.md`,
  regra 11 do AGENTS.md); `smoke-apps/` mapeia apps reais (sem binários).

## O que NÃO funciona (e por quê)

- `hello_mingw.exe` (CRT): 42 imports sem implementação (critical sections,
  LoadLibrary, SEH, api-sets…) = **escopo v0.3**, não bug.
- Diferencial Windows: sem host Windows (formato + probe prontos).
- Ponteiros selvagens do guest derrubam o host (sem SEH até v0.3).
- 32 bits/EFI: scanner pula com motivo (WoW64 futuro).

## Última tarefa concluída

v0.3 (parte 8): itens 15–18 — `DeleteFileW`, `MoveFileExW` (REPLACE honesto;
cross-device = copy+remove com `is_cross_device` em `host-linux`, sem libc
fora da fronteira), `CreateDirectoryW` (sem recursão), `RemoveDirectoryW`
(só vazio); suite 111–120, evil 77–80, unit `nt-file`; cobertura 42/4209;
demanda segue 32. Antes, v0.3 (parte 7): itens 11–14 — `CreateFileW` (UTF-16 estrito, núcleo comum
com a A), `GetFileAttributesW` (DIRECTORY/NORMAL+READONLY, sem ARCHIVE
inventado), `SetFilePointerEx` (`lseek` novo em `host-linux`, `lp` opcional),
`GetFileSizeEx` (END+restore); suite 102–110, evil 73–76; cobertura 38/4209;
demanda segue 32 (nenhum dos 4 no fixture MinGW). Antes, v0.3 (parte 6):
itens 9+10 — ApiSet (`pe::apiset` parseia o namespace v6
real; snapshot 843 rotas; tabela gerada de 175; conformidade travada;
`Sleep` via `api-ms-win-...` carrega com IAT real) + forwarders
(`kernel32!*CS → ntdll!Rtl*`, chase de 1 salto; `kernelbase` perdeu as 4
fns CS e 2 deps; ntdll 2→6 exports); suite/evil verdes sem mudar o guest;
cobertura 34/4209; demanda segue 32 (UCRT é implementação, não roteamento).
Antes, v0.3 (parte 5): família do loader dinâmico — `GetModuleHandleA/W`
(NULL = base da imagem via `ProcessContext.image_base` novo; normalização
Windows; ApiSet → 126), `LoadLibraryA/W` (conjunto carregado; imagem nova
→ 126 honesto), `FreeLibrary` (estático = TRUE); suite 91–101, evil 70–72,
unit `modules`, UTF-16 (`wide_slice`); cobertura 30/4209; **KERNEL32 do
MinGW zerada (14/14)** — demanda 35→32 (só `api-ms-win-crt-*`). Antes,
GUI 1.1: sidecar `rine` no Manager (`bundle.externalBin`,
`pack-sidecar.sh`; `detect_runtime`: `$RINE_BIN` → sidecar → base → `PATH`;
`RUNTIME_NOT_FOUND` instrui). Antes: v0.3 (parte 4): `GetProcAddress` (nome
+ ordinal real do oracle, `==` IAT, NULL+126/127; tokens `HMODULE` opacos
documentados; suite 84–90, evil 68–69, unit `kernel32::modules`).

## Próxima tarefa recomendada

v0.3 (continuação, na ordem da demanda): `FindFirstFileW` (19),
`FindNextFileW` (20), `FindClose` (21) — enumeração de diretórios.
Nessa ordem (cada um com teste).

## Blockers

Sem host Windows (diferencial/oracle real). Git pronto: `main` no
GitHub (`PLNastalli/rine`, árvore limpa); identidade de commit placeholder
`Rine` — `git config user.name/email` p/ personalizar.

## APIs faltantes prioritárias

As 5 KERNEL32 do MinGW ainda em aberto (`tests/windows/hello_mingw.imports.json`).
Cobertura completa em `api-db/coverage.json`.

## Testes falhando / provisório / stubs / dívida

Zero testes falhando (121 passed). Zero stubs (proibidos). Provisório
documentado: singleton de processo (ADR-0003), `access==0`→read-only,
share/flags ignorados, `X:rel`/UNC→NOT_IMPLEMENTED. Dívida: `VirtualQuery`,
`openat2` direto, advapi32, ripgrep-pendente: nenhum.

## Implementação autoritativa (mudanças da sessão)

- Exports implementados: `kernel32::EXPORTS`, `ntdll::EXPORTS` (anti-drift).
- Tradução paths: `nt_file::DriveMap`; proteção seção: `section_protect`.
- Demanda: `runtime::{missing_imports,demand_report}`; oracle: `crates/oracle`.
