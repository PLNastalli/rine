# Rine Manager — design system (contrato UI, GUI 1+)

Dark default (+ light/system). Rine blue como destaque. Sem glow/
glassmorphism/animações: ferramenta de sistema séria.

## Badges (domínios separados, sempre ícone + texto + cor)

- API (`coverage.json`): `Missing|Stub|Partial|Implemented|BehaviorTested|
  DifferentiallyVerified`.
- Execução (`run::RunStatus` + app): `Blocked|Running|Exited|Crashed`
  (+ `Unknown|ImportBlocked|Starts|WorkflowPartial|Working|Regression` na Library).
- Performance: `Pass|Warning|Regression` (só leitura do `rine-bench check`).

## Componentes (reutilizar, não variar)

`PageHeader · StatusBadge · MetricCard · EmptyState · DataTable ·
SearchInput · SectionCard · ConfirmDialog · RunButton · RuntimeStatus ·
CompatibilityBadge · PathPicker · LogViewer · Sidebar`.

## Regras

- Sidebar esquerda recolhível (Home/Library/Run/Capsules ·
  Compatibility/Diagnostics/Performance · Settings/About).
- Empty states explicam o próximo passo (nunca tela vazia muda).
- Toasts só para eventos rápidos; crash/erro importante fica visível na página.
- A11y: navegação teclado, foco visível, ARIA/labels, texto escalável.
- Performance: sem polling agressivo; PE scan/run/logs fora da thread da UI.
- Atalhos: `Ctrl+O` abrir EXE, `Ctrl+K` busca (se existir), `Ctrl+,`
  settings, `Ctrl+R` rodar selecionado.
