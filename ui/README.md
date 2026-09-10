# Rine Manager — frontend (`ui/`)

React + TypeScript + Vite + Tailwind + Lucide. Apresentação pura: toda a
lógica mora em `crates/manager-core`, consumida via 13 thin Tauri commands
(`crates/manager-tauri`). Componentes nunca importam `@tauri-apps/api`
direto — tudo passa por `src/lib/tauri.ts`.

```bash
npm install
cargo build -p launcher   # o sidecar `rine` (a Home exige; sem ele: RUNTIME_NOT_FOUND)
npm run dev        # dev server :1420 (janela debug via `tauri dev`/CLI)
npm run typecheck  # tsc
npm run lint       # oxlint (zero warnings)
npm test           # vitest (6 testes, invoke mockado, sem janela)
npm run build      # dist/ (embutido no binário Tauri em release)
```

Páginas GUI 0: Home, Run, Library, Capsules, Diagnostics, Settings.
Compatibility/Performance aparecem desabilitadas ("soon") até o core
correspondente existir. `dist/` e `node_modules/` não vão ao git.
