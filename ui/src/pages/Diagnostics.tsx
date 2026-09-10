// Diagnostics: runs recentes com crash view estruturada (§16/§17).
import { useState } from 'react';
import type { RunReport } from '../lib/tauri';
import { EmptyState, PageHeader, SectionCard, StatusBadge } from '../components/ui';

export interface DiagEntry {
  exe: string;
  at: number;
  report: RunReport;
}

function CrashView({ report }: { report: RunReport }) {
  const [open, setOpen] = useState(false);
  const c = report.crash;
  if (!c) return null;
  return (
    <div className="mt-3 rounded-lg border border-[var(--color-rine-err)] p-3">
      <div className="text-sm font-semibold">Application crashed</div>
      <dl className="mt-2 grid grid-cols-2 gap-2 text-sm">
        <div><dt className="text-xs text-[var(--color-rine-muted)]">Exception</dt><dd>{c.reason}</dd></div>
        <div><dt className="text-xs text-[var(--color-rine-muted)]">Phase</dt><dd>{c.phase}</dd></div>
        <div><dt className="text-xs text-[var(--color-rine-muted)]">Location</dt><dd>{c.location}</dd></div>
        <div><dt className="text-xs text-[var(--color-rine-muted)]">Runtime</dt><dd>{c.version}</dd></div>
      </dl>
      <button onClick={() => setOpen((o) => !o)} className="mt-2 text-sm text-[var(--color-rine-accent)]">
        {open ? 'Hide technical details' : 'Show technical details'}
      </button>
      {open && (
        <pre className="mt-2 max-h-48 overflow-auto rounded-lg bg-[var(--color-rine-panel)] p-3 text-xs whitespace-pre-wrap">
          {report.stderr || '(sem stderr)'}
        </pre>
      )}
    </div>
  );
}

export function Diagnostics({ entries }: { entries: DiagEntry[] }) {
  const [sel, setSel] = useState<number | null>(entries.length ? 0 : null);
  const cur = sel !== null ? entries[sel] : null;

  return (
    <div>
      <PageHeader title="Diagnostics" subtitle="Execuções recentes desta sessão." />
      {entries.length === 0 ? (
        <EmptyState title="No runs recorded yet." hint="Execute um EXE na página Run ou Library para ver o diagnóstico aqui." />
      ) : (
        <div className="grid gap-4 lg:grid-cols-[280px_1fr]">
          <SectionCard title="Runs">
            <ul className="flex flex-col gap-1">
              {entries.map((e, i) => (
                <li key={`${e.at}-${i}`}>
                  <button
                    onClick={() => setSel(i)}
                    className={`w-full rounded-lg px-2 py-1.5 text-left text-sm ${i === sel ? 'bg-[var(--color-rine-panel)]' : ''}`}
                  >
                    <div className="truncate">{e.exe.split('/').pop()}</div>
                    <div className="text-xs text-[var(--color-rine-muted)]">
                      {e.report.status} · exit {e.report.exit_code ?? '?'}
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          </SectionCard>
          <div>
            {cur && (
              <SectionCard title={cur.exe}>
                <StatusBadge
                  kind={cur.report.status === 'Exited' && cur.report.exit_code === 0 ? 'ok' : cur.report.status === 'Blocked' ? 'warn' : 'err'}
                  text={`${cur.report.status} · exit ${cur.report.exit_code ?? '?'} · ${cur.report.duration_ms}ms · rine ${cur.report.rine_version}`}
                />
                {cur.report.crash && <CrashView report={cur.report} />}
                {cur.report.missing.length > 0 && (
                  <div className="mt-3">
                    <div className="text-sm font-medium">Missing imports ({cur.report.missing.length})</div>
                    <ul className="mt-1 max-h-40 overflow-auto text-sm text-[var(--color-rine-muted)]">
                      {cur.report.missing.map(([dll, name]) => (
                        <li key={`${dll}!${name}`}>{dll}!{name}</li>
                      ))}
                    </ul>
                  </div>
                )}
                <div className="mt-3 grid gap-3 lg:grid-cols-2">
                  <div>
                    <div className="mb-1 text-xs text-[var(--color-rine-muted)]">stdout</div>
                    <pre className="max-h-48 overflow-auto rounded-lg bg-[var(--color-rine-panel)] p-3 text-xs whitespace-pre-wrap">
                      {cur.report.stdout || '(vazio)'}
                    </pre>
                  </div>
                  <div>
                    <div className="mb-1 text-xs text-[var(--color-rine-muted)]">stderr</div>
                    <pre className="max-h-48 overflow-auto rounded-lg bg-[var(--color-rine-panel)] p-3 text-xs whitespace-pre-wrap">
                      {cur.report.stderr || '(vazio)'}
                    </pre>
                  </div>
                </div>
              </SectionCard>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
