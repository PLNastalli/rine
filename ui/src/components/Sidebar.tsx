// Sidebar esquerda (§4): recolhível; futuro = disabled/coming soon, nunca falso.
import { useState } from 'react';
import {
  BookOpen,
  Cpu,
  FlaskConical,
  Gauge,
  Home,
  Library,
  Package,
  Play,
  Settings,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';

export type Page = 'home' | 'library' | 'run' | 'capsules' | 'diagnostics' | 'settings';

const main: { id: Page; label: string; icon: typeof Home }[] = [
  { id: 'home', label: 'Home', icon: Home },
  { id: 'library', label: 'Library', icon: Library },
  { id: 'run', label: 'Run', icon: Play },
  { id: 'capsules', label: 'Capsules', icon: Package },
];

const second: { id: Page; label: string; icon: typeof Home }[] = [
  { id: 'diagnostics', label: 'Diagnostics', icon: FlaskConical },
  { id: 'settings', label: 'Settings', icon: Settings },
];

// Páginas futuras: visíveis mas desabilitadas (honestidade, §1).
const soon = [
  { label: 'Compatibility', icon: BookOpen },
  { label: 'Performance', icon: Gauge },
];

export function Sidebar({ page, go }: { page: Page; go: (p: Page) => void }) {
  const [collapsed, setCollapsed] = useState(false);
  const w = collapsed ? 'w-14' : 'w-52';

  const item = (id: Page, label: string, Icon: typeof Home) => (
    <button
      key={id}
      onClick={() => go(id)}
      aria-current={page === id ? 'page' : undefined}
      className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm ${
        page === id
          ? 'bg-[var(--color-rine-panel)] text-[var(--color-rine-text)]'
          : 'text-[var(--color-rine-muted)] hover:text-[var(--color-rine-text)]'
      }`}
    >
      <Icon size={18} aria-hidden="true" />
      {!collapsed && label}
    </button>
  );

  return (
    <nav
      aria-label="Navegação principal"
      className={`${w} flex shrink-0 flex-col gap-1 border-r border-[var(--color-rine-border)] bg-[var(--color-rine-panel)] p-3 transition-[width]`}
    >
      <div className="mb-2 px-1">
        {!collapsed && (
          <>
            <div className="flex items-center gap-2 text-base font-bold text-[var(--color-rine-text)]">
              <Cpu size={20} className="text-[var(--color-rine-accent)]" aria-hidden="true" />
              Rine
            </div>
            <div className="text-xs text-[var(--color-rine-muted)]">Windows runs further.</div>
          </>
        )}
      </div>
      {main.map((m) => item(m.id, m.label, m.icon))}
      <div className="my-2 border-t border-[var(--color-rine-border)]" />
      {second.map((m) => item(m.id, m.label, m.icon))}
      <div className="my-2 border-t border-[var(--color-rine-border)]" />
      {soon.map((s) => (
        <div
          key={s.label}
          title="Coming soon — indisponível no runtime atual"
          className="flex w-full cursor-not-allowed items-center gap-3 rounded-lg px-3 py-2 text-sm text-[var(--color-rine-muted)] opacity-50"
        >
          <s.icon size={18} aria-hidden="true" />
          {!collapsed && (
            <span>
              {s.label} <span className="text-xs">(soon)</span>
            </span>
          )}
        </div>
      ))}
      <div className="mt-auto">
        <button
          onClick={() => setCollapsed((c) => !c)}
          aria-label={collapsed ? 'Expandir sidebar' : 'Recolher sidebar'}
          className="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm text-[var(--color-rine-muted)] hover:text-[var(--color-rine-text)]"
        >
          {collapsed ? <ChevronRight size={18} /> : <ChevronLeft size={18} />}
          {!collapsed && 'Recolher'}
        </button>
      </div>
    </nav>
  );
}
