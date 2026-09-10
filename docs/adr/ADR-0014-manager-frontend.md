# ADR-0014 — Rine Manager: Tauri 2 + React + shadcn/ui, core em Rust puro

- Status: aceito (2026-09-10).
- Context: o Rine precisa de um gerenciador utilizável (library, capsules,
  pre-flight, diagnostics) sem contaminar o runtime headless/scriptable
  (`rine app.exe` continua funcionando sem GUI) nem duplicar estado
  (Capsule, API DB, compatibilidade pertencem ao core).
- Decision:
  1. `crates/manager-core`: TODA a lógica (detect/inspect/preflight/run/
     registry/capsules). Rust puro, zero dep de UI, testado sem abrir janela.
     É a `management API` — Tauri commands serão thin adapters sobre ela.
  2. `crates/manager-tauri` + `ui/` (React+TS+Vite+shadcn/ui, Tailwind,
     Lucide): só apresentação + estado por feature. Sem lógica de runtime.
  3. Tauri 2 (não Electron): backend continua Rust (o mesmo processo que
     já orquestra o guest), binário leve, webview do sistema, sem segundo
     backend em Node. IPC = commands tipados + eventos tipados.
  4. Comunicação manager↔runtime hoje = processo (`rine` via argv
     estruturado) + arquivos (TOML/JSON) + `RINE-CRASH schema=1`. IPC
     versionado só se a fronteira virar socket (YAGNI agora); incompatibilidade
     de versão = `ProtocolMismatch` explícito, nunca silencioso.
  5. Dados próprios em JSON versionado XDG (`registry.json`, `settings.json`);
     SQLite só com necessidade provada (runs > 10k ou relacional real).
- Alternatives: (a) Electron (pesado, segundo runtime JS, contra a meta
  "não fazer o Rine parecer pesado"); (b) egui/iced nativo (menos
  ecossistema de design system; shadcn acelera sem acoplar); (c) lógica nos
  Tauri commands (vira `commands.rs` de 300 linhas — proibido pela missão).
- Consequences: workspace ganha `manager-core` (agora) e `manager-tauri` +
  `ui/` (GUI 1); `dependency-map.md` trava que nada do core depende deles;
  CI roda `cargo` gates no core + `typecheck/lint/test/build` no `ui/`.
- Risks: webview ausente em sistema mínimo (mitigado: CLI 100% funcional
  sem GUI); deriva React↔core (mitigado: commands thin + tipos gerados a
  partir do core, nunca lógica duplicada).
