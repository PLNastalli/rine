# ADR-0008 — Formato `capsule.toml` (congelado v0.2)

- Status: aceito (2026-09-10). Milestone: v0.2.
- Context: cada app precisa de config declarativa (drives, versão, cwd).
- Decision: TOML com `[app]` (name, windows_version, arch, current_dir
  opcional) + `[drives]` (LETRA = dir host). Parse estrito: erro aborta o
  load, nunca default silencioso. Ver `compat/profiles/README.md`.
- Alternatives: JSON (menos comentários, pior para humanos); YAML
  (especificação gigante para um arquivo simples).
- Why: legível, comentável, parser `toml` pequeno e auditável.
- Consequences: `runtime` depende de `toml`; formato versionado por
  milestone (campos novos sempre opcionais).
- Risks: nenhum relevante.
