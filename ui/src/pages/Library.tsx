// Library: apps cadastrados, estados de dados reais (§8). Remove = só cadastro.
import { useEffect, useState } from 'react';
import { api, errMsg, type AppEntry } from '../lib/tauri';
import {
  EmptyState,
  ErrorBox,
  GhostButton,
  PageHeader,
  RunButton,
  SectionCard,
  StatusBadge,
} from '../components/ui';
import type { Page } from '../components/Sidebar';

export function Library({
  go,
  onDiagnosed,
}: {
  go: (p: Page) => void;
  onDiagnosed: (r: import('../lib/tauri').RunReport, exe: string) => void;
}) {
  const [apps, setApps] = useState<AppEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const reload = () => {
    api
      .listApps()
      .then(setApps)
      .catch((e: unknown) => setError(errMsg(e)));
  };
  useEffect(reload, []);

  const run = async (a: AppEntry) => {
    setBusy(a.id);
    setError(null);
    try {
      const r = await api.runRegisteredApp(a.id, [], 30_000);
      onDiagnosed(r, a.exe_path);
      reload();
    } catch (e: unknown) {
      setError(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  const remove = async (a: AppEntry) => {
    // Remove só o cadastro — o .exe permanece no disco (regra §8).
    if (!window.confirm(`Remover "${a.name}" da Library? (o arquivo é mantido)`)) return;
    try {
      await api.removeApp(a.id);
      reload();
    } catch (e: unknown) {
      setError(errMsg(e));
    }
  };

  return (
    <div>
      <PageHeader title="Library" subtitle="Aplicativos Windows cadastrados." />
      {error && (
        <div className="mb-4">
          <ErrorBox message={error} />
        </div>
      )}
      {apps === null ? (
        <p className="text-sm text-[var(--color-rine-muted)]">Carregando…</p>
      ) : apps.length === 0 ? (
        <EmptyState
          title="No Windows applications yet."
          hint="Add an executable to get started."
          action={
            <button
              onClick={() => go('run')}
              className="rounded-lg bg-[var(--color-rine-accent)] px-4 py-2 text-sm font-semibold text-white"
            >
              Open Run
            </button>
          }
        />
      ) : (
        <SectionCard>
          <ul className="flex flex-col gap-2">
            {apps.map((a) => (
              <li
                key={a.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-[var(--color-rine-border)] px-3 py-2"
              >
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{a.name}</div>
                  <div className="truncate text-xs text-[var(--color-rine-muted)]">{a.exe_path}</div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <StatusBadge
                    kind={a.last_run?.exit_code === 0 ? 'ok' : a.last_run ? 'warn' : 'muted'}
                    text={a.last_run ? `${a.last_run.status} · exit ${a.last_run.exit_code ?? '?'}` : 'Unknown'}
                  />
                  <RunButton onClick={() => run(a)} disabled={busy === a.id}>
                    {busy === a.id ? '…' : 'Run'}
                  </RunButton>
                  <GhostButton onClick={() => remove(a)}>Remove</GhostButton>
                </div>
              </li>
            ))}
          </ul>
        </SectionCard>
      )}
    </div>
  );
}
