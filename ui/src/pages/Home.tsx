// Home: "O Rine está funcionando?" — responde de imediato (§5).
import { useEffect, useState } from 'react';
import { api, type AppEntry } from '../lib/tauri';
import { useRuntime } from '../lib/runtime-store';
import { EmptyState, ErrorBox, MetricCard, PageHeader, SectionCard, StatusBadge } from '../components/ui';
import type { Page } from '../components/Sidebar';

export function Home({ go }: { go: (p: Page) => void }) {
  const { loading, runtime, error, retry } = useRuntime();
  const [apps, setApps] = useState<AppEntry[] | null>(null);

  useEffect(() => {
    api
      .listApps()
      .then(setApps)
      .catch(() => setApps([]));
  }, []);

  return (
    <div>
      <PageHeader
        title="Good evening"
        subtitle="Estado do runtime e atividade recente."
        actions={
          <StatusBadge
            kind={runtime ? 'ok' : loading ? 'muted' : 'err'}
            text={runtime ? `Runtime Ready · ${runtime.version}` : loading ? 'Detectando…' : 'Runtime Error'}
          />
        }
      />
      {error && (
        <div className="mb-4">
          <ErrorBox message={error} />
          <button onClick={retry} className="mt-2 text-sm text-[var(--color-rine-accent)]">
            Tentar de novo
          </button>
        </div>
      )}
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <MetricCard label="Runtime" value={runtime ? runtime.version : '—'} sub="Windows x86_64 → Linux x86_64" />
        <MetricCard label="Apps cadastrados" value={apps ? String(apps.length) : '…'} sub="Library local" />
        <MetricCard
          label="Último run"
          value={apps?.find((a) => a.last_run)?.last_run?.exit_code === 0 ? 'exit 0' : '—'}
          sub={apps?.find((a) => a.last_run)?.name ?? 'nenhum ainda'}
        />
        <MetricCard label="Capsules" value="—" sub="Abra a página Capsules" />
      </div>

      <div className="mt-5">
        <SectionCard title="Recent applications">
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
                  Run an EXE
                </button>
              }
            />
          ) : (
            <ul className="flex flex-col gap-2">
              {apps.slice(0, 5).map((a) => (
                <li
                  key={a.id}
                  className="flex items-center justify-between rounded-lg border border-[var(--color-rine-border)] px-3 py-2"
                >
                  <div>
                    <div className="text-sm font-medium">{a.name}</div>
                    <div className="text-xs text-[var(--color-rine-muted)]">{a.exe_path}</div>
                  </div>
                  <StatusBadge
                    kind={a.last_run?.exit_code === 0 ? 'ok' : a.last_run ? 'warn' : 'muted'}
                    text={a.last_run ? `exit ${a.last_run.exit_code ?? '?'}` : 'Unknown'}
                  />
                </li>
              ))}
            </ul>
          )}
        </SectionCard>
      </div>
    </div>
  );
}
