# Rine Manager — arquitetura (GUI 0)

> Frontend opcional. O runtime nunca depende do Manager; deps do Manager
> nunca vazam para os crates do core (`dependency-map.md`).

## Fronteira (a GUI só toca o que já existe)

```text
React (ui/) ──Tauri commands (thin)──► manager-core ──┬──► processo `rine`
                                                      ├──► crates (pe/runtime/...)
                                                      └──► arquivos (api-db/coverage.json,
                                                           bench/baselines, capsule.toml)
```

Proibido: React → `kernel32`/`nt-*` direto; lógica de runtime em commands;
`shell=true` / concatenação de comando (sempre `std::process::Command`
estruturado em `manager-core::run`).

## Pontos de conexão reais (código atual, não aspiração)

| Capacidade GUI | Fonte de verdade | Estado |
|---|---|---|
| Runtime detection | `rine --version` → `rine {CARGO_PKG_VERSION}` (`launcher/src/main.rs`) | ✅ |
| PE inspection | `pe::Image::parse` + `imports()` + metadados (machine/subsystem/entry) | ✅ |
| Pre-flight (import surface) | `runtime::missing_imports(bytes)` + `api-db/*/coverage.json` (status) | ✅ |
| Run + captura | spawn `rine [--capsule F] exe args…`, stdout/stderr/exit | ✅ |
| Crash tipado | `RINE-CRASH schema=1` em stderr (`docs/crash-format.md`) | ✅ (fase load/enter; RIP/sinais = v0.3+) |
| Demand (blockers) | `runtime::demand_report` em falha de load | ✅ |
| Capsules | `runtime::Capsule::load_toml` (schema v0.2: `[app]` + `[drives]`) | ✅ leitura; criação = escrever o mesmo schema |
| Cobertura de APIs | `coverage.json` + `kernel32::EXPORTS`/`ntdll::EXPORTS` (somente leitura) | ✅ |
| Performance | `bench/baselines/*.json` via `perf` (somente leitura; promoção continua CLI com `--reason`) | ✅ leitura |
| Registry editor | `nt-registry` existe mas sem API de gestão estável | ❌ ausente em GUI 0 (read-only futuro) |
| Oracle Windows | `crates/oracle` formato pronto, sem host Windows | ❌ status "Disconnected" fixo |
| Módulos dinâmicos | `kernel32::modules` estático (tokens opacos `winabi`) | ⚠️ exibir como opaco; `LoadLibrary` (v0.4) aposenta |

## Terminologia obrigatória (anti-fake)

- Imports resolvidos = `Static import surface satisfied` / `Import surface satisfied`.
- NUNCA `Compatible` por imports (compatibilidade real exige execução/workflow).
- Estados de execução: `Blocked` (falha load + demanda) | `Crashed` (`RINE-CRASH`) |
  `Exited(code)` | `Running` (transiente). Estados de API: os 6 de `coverage.json`.
- Capacidades inexistentes: `Coming soon` / `Experimental` /
  `Unavailable in current runtime` — nunca botão funcional falso.

## Versionamento manager ↔ runtime

Protocolo hoje = argv + arquivos + `RINE-CRASH schema=1` + versão do binário.
`manager-core::detect_runtime` recusa versão incompatível com
`ManagerError::ProtocolMismatch` (nunca segue silencioso). Versão esperada:
`0.2.0-alpha.3` (sobe junto com o workspace; IPC versionado só se a
fronteira virar socket — hoje é processo + arquivo, YAGNI).

## Layout de código

```text
crates/manager-core/   # lógica (testável sem GUI): detect/inspect/preflight/
                       # run/registry/capsules/settings + ManagerError. Sem Tauri.
crates/manager-tauri/  # shell Tauri 2: 13 thin commands + janela (sem lógica).
ui/                    # React+TS+Vite+Tailwind+Lucide: 6 páginas GUI 0
                       # (Home/Run/Library/Capsules/Diagnostics/Settings) +
                       # Compatibility/Performance como "soon" desabilitado.
docs/manager/          # esta pasta: architecture/data-model/runtime-interface/
                       # security/design-system.
```

## GUI 1 entregue (2026-09-10) + nota de assets

- `tauri` com feature `custom-protocol` (OBRIGATÓRIA: sem ela `dev=true`
  sempre e o `dist` nunca é embutido — janela cai no `devUrl`).
- Debug (`cargo run -p manager-tauri`): com `custom-protocol`, serve a UI
  embutida standalone (provado: processo vivo, zero conexões à 1420, sem
  Node). `tauri dev` continua usando o dev server (`npm run dev`, :1420).
- Release: `dist/` embutido (brotli, verificado byte a byte no binário).
- Janela provada: webview carrega a UI (2 conexões persistentes com dev
  server no modo dev; standalone no modo normal). E2E funcional via
  `manager-core` (`vertical_slice`: hello → Hello World, exit 0).

## Decisões adiadas (com condição)

- SQLite vs JSON: JSON versionado basta (registry pequeno, escrita pontual);
  reavaliar se runs > 10k ou queries relacionais reais.
- `manager-tauri`/`ui/`: após `manager-core` verde + E2E `hello.exe` via core.
- Registry editor, Oracle UI, histórico de bench: GUI 2+, quando o core
  correspondente existir (nunca antes).
