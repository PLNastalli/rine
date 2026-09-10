// Rine Manager — shell: sidebar + página + status do runtime no topo.
import { useCallback, useEffect, useState } from 'react';
import { Sidebar, type Page } from './components/Sidebar';
import { useRuntime } from './lib/runtime-store';
import type { RunReport, Settings as S } from './lib/tauri';
import { Home } from './pages/Home';
import { Run } from './pages/Run';
import { Library } from './pages/Library';
import { Capsules } from './pages/Capsules';
import { Diagnostics, type DiagEntry } from './pages/Diagnostics';
import { Settings } from './pages/Settings';
import { StatusBadge } from './components/ui';

function TopStatus() {
  const { loading, runtime } = useRuntime();
  return (
    <StatusBadge
      kind={runtime ? 'ok' : loading ? 'muted' : 'err'}
      text={runtime ? `Runtime Ready · ${runtime.version}` : loading ? 'Detectando…' : 'Runtime Error'}
    />
  );
}

function Shell() {
  const [page, setPage] = useState<Page>('home');
  const [diags, setDiags] = useState<DiagEntry[]>([]);
  const [appearance, setAppearance] = useState<'system' | 'dark' | 'light'>('dark');

  useEffect(() => {
    const root = document.documentElement;
    const dark =
      appearance === 'dark' ||
      (appearance === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    root.classList.toggle('dark', dark);
    root.style.colorScheme = dark ? 'dark' : 'light';
  }, [appearance]);

  const onDiagnosed = useCallback((report: RunReport, exe: string) => {
    setDiags((d) => [{ exe, at: Date.now(), report }, ...d].slice(0, 50));
  }, []);

  const applySettings = useCallback((s: S) => setAppearance(s.appearance), []);

  return (
    <div className="flex h-screen bg-[var(--color-rine-bg)] text-[var(--color-rine-text)]">
      <Sidebar page={page} go={setPage} />
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-[var(--color-rine-border)] px-6 py-3">
          <span className="text-sm text-[var(--color-rine-muted)]">Windows runs further.</span>
          <TopStatus />
        </header>
        <main className="flex-1 overflow-auto p-6">
          {page === 'home' && <Home go={setPage} />}
          {page === 'run' && <Run onDiagnosed={onDiagnosed} />}
          {page === 'library' && <Library go={setPage} onDiagnosed={onDiagnosed} />}
          {page === 'capsules' && <Capsules />}
          {page === 'diagnostics' && <Diagnostics entries={diags} />}
          {page === 'settings' && <Settings apply={applySettings} />}
        </main>
      </div>
    </div>
  );
}

export default function App() {
  return <Shell />;
}
