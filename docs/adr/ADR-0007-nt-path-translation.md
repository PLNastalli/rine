# ADR-0007 — Tradução de paths NT via `DriveMap` (snapshot no contexto)

- Status: aceito (2026-09-10). Milestone: v0.2.
- Context: `CreateFileA("C:\\x")` precisa virar path do host. Onde mora o
  mapa drive→dir e quando traduzir?
- Decision: `nt_file::DriveMap` é a fonte única da tradução
  (`C:\`→drive, `\??\`→strip, relativo→CWD; UNC/`\\.\`/`X:rel`→
  `NOT_IMPLEMENTED` testado). `Capsule` (em `runtime`) é a fonte de
  configuração; no load, um snapshot `DriveMap` é instalado no
  `ProcessContext.fsys`. Conversão unidirecional, lógica num só lugar.
- Alternatives: (a) traduzir no launcher e passar fds (quebra semântica:
  o guest deve poder abrir paths arbitrários em runtime); (b) `Capsule`
  compartilhada via `Arc` no contexto (ciclo `runtime→ntdll→runtime`;
  exigiria mover `Capsule` de crate — reavaliar se o snapshot doer).
- Why: guest resolve paths em tempo de chamada (como no Windows);
  snapshot evita acoplamento `nt-file→runtime`.
- Consequences: drives fixos por load (recarregar Capsule = novo load);
  documentado em `architecture.md`.
- Risks: divergência Capsule↔snapshot se algo mutar drives (nada muta em v0.2).
