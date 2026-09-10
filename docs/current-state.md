# Estado atual

_Data: 2026-09-10 (quinta sessão). Versão: **0.2.0-alpha.1**.
Milestone: **v0.2 ~90%** + plataforma de testes diferenciais operacional._

## Milestone atual

v0.2 (processo+filesystem) + transversal: plataforma `difftest`/`rine-test`
pronta e caçando bugs de verdade. Próximo: v0.3 = `hello_mingw.exe` CRT
(demanda travada: 46 imports, 45 em aberto — `tests/windows/`).

## O que funciona

- E2E (filho isolado): hello/file/alloc/args/suite/evil + matrix RINE_PASS.
- `rine --capsule`, `--version`, relatório `RINE-DEMAND` (45 faltantes do MinGW).
- Infra: `rine-api-scan` (5578 DLLs/183k exports em 3s), `api-db/`
  (KERNEL32/NTDLL + coverage 12/4209 BehaviorTested), oracle schema 1
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

- `hello_mingw.exe` (CRT): 45 imports sem implementação (critical sections,
  TLS, LoadLibrary, SEH, api-sets…) = **escopo v0.3**, não bug.
- Diferencial Windows: sem host Windows (formato + probe prontos).
- Ponteiros selvagens do guest derrubam o host (sem SEH até v0.3).
- 32 bits/EFI: scanner pula com motivo (WoW64 futuro).

## Última tarefa concluída

Plataforma de testes (`difftest` + `pe::driver` + `rine-test`): cenários com
seed, normalização explícita, 8 vereditos, shrink, corpus (`files/memory/
handles`), runner paralelo determinístico, fuzz, proptest, concorrência,
matriz com `campaigns[]`, docs (differential/windows-oracle/fuzzing/
behavioral-coverage/non-regression) + ADR-0010. Caçadas: EINVAL access==0,
truncate perdido, trailing-slash, fuga `..` do drive (+ harness: path
relativo). Smoke-apps + baselines + AGENTS r11.

## Próxima tarefa recomendada

v0.3 pela demanda: `TlsGetValue`+TLS slots, critical sections,
`GetLastError` export, `Sleep`, `VirtualQuery`, loader de DLLs
(`LoadLibraryA/GetProcAddress`), api-sets. Nessa ordem (cada um com teste).

## Blockers

Sem host Windows (diferencial/oracle real). Sem repo git (recomendado iniciar;
`target/` e `api-db/` volumoso já têm `.gitignore`).

## APIs faltantes prioritárias

As 13 KERNEL32 do MinGW (`tests/windows/hello_mingw.imports.json`).
Cobertura completa em `api-db/coverage.json`.

## Testes falhando / provisório / stubs / dívida

Zero testes falhando (59 passed). Zero stubs (proibidos). Provisório
documentado: singleton de processo (ADR-0003), `access==0`→read-only,
share/flags ignorados, `X:rel`/UNC→NOT_IMPLEMENTED. Dívida: `VirtualQuery`,
`openat2` direto, advapi32, ripgrep-pendente: nenhum.

## Implementação autoritativa (mudanças da sessão)

- Exports implementados: `kernel32::EXPORTS`, `ntdll::EXPORTS` (anti-drift).
- Tradução paths: `nt_file::DriveMap`; proteção seção: `section_protect`.
- Demanda: `runtime::{missing_imports,demand_report}`; oracle: `crates/oracle`.
