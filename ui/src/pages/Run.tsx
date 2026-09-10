// Run: drop/select → inspect → pre-flight → Run (§6/§7). Nunca executa sozinho.
import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, errMsg, type PeReport, type PreflightReport, type RunReport } from '../lib/tauri';
import {
  EmptyState,
  ErrorBox,
  Field,
  GhostButton,
  PageHeader,
  RunButton,
  SectionCard,
  StatusBadge,
  inputCls,
} from '../components/ui';

export function Run({ onDiagnosed }: { onDiagnosed: (r: RunReport, exe: string) => void }) {
  const [exe, setExe] = useState('');
  const [rep, setRep] = useState<PeReport | null>(null);
  const [pf, setPf] = useState<PreflightReport | null>(null);
  const [args, setArgs] = useState('');
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<RunReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pick = async () => {
    const sel = await open({ filters: [{ name: 'Windows executable', extensions: ['exe'] }] });
    if (typeof sel === 'string') void analyze(sel);
  };

  const analyze = async (path: string) => {
    setError(null);
    setRep(null);
    setPf(null);
    setReport(null);
    setExe(path);
    try {
      // Inspeção primeiro (parse, sem executar); pre-flight em seguida.
      const [r, p] = await Promise.all([api.inspectPe(path), api.preflight(path)]);
      setRep(r);
      setPf(p);
    } catch (e: unknown) {
      setError(errMsg(e));
    }
  };

  const run = async () => {
    if (!exe) return;
    setRunning(true);
    setError(null);
    try {
      // O usuário clicou Run: intenção explícita (§38). Args viram cmdline Win32.
      const r = await api.runExe({
        exe,
        capsule: null,
        args: args.trim() ? args.trim().split(/\s+/) : [],
        workdir: null,
        env_extra: [],
        timeout_ms: 30_000,
      });
      setReport(r);
      onDiagnosed(r, exe);
    } catch (e: unknown) {
      setError(errMsg(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div>
      <PageHeader title="Run" subtitle="Selecione, analise e execute um Windows EXE." />
      {error && (
        <div className="mb-4">
          <ErrorBox message={error} />
        </div>
      )}
      {!exe ? (
        <button
          onClick={pick}
          className="flex w-full flex-col items-center gap-2 rounded-xl border-2 border-dashed border-[var(--color-rine-border)] p-14 text-center hover:border-[var(--color-rine-accent)]"
        >
          <span className="text-lg font-medium">Drop a Windows executable here</span>
          <span className="text-sm text-[var(--color-rine-muted)]">ou clique para Select .exe (só inspeciona — nunca executa sozinho)</span>
        </button>
      ) : (
        <div className="flex flex-col gap-4">
          <SectionCard title="Executable">
            <div className="flex items-center justify-between gap-3">
              <div className="truncate text-sm">{exe}</div>
              <GhostButton onClick={pick}>Trocar</GhostButton>
            </div>
            {rep && (
              <dl className="mt-3 grid grid-cols-2 gap-2 text-sm lg:grid-cols-3">
                <div><dt className="text-xs text-[var(--color-rine-muted)]">Architecture</dt><dd>{rep.machine === 0x8664 ? 'x86_64' : `0x${rep.machine.toString(16)}`}</dd></div>
                <div><dt className="text-xs text-[var(--color-rine-muted)]">Subsystem</dt><dd>{rep.subsystem === 3 ? 'Windows CUI' : rep.subsystem === 2 ? 'Windows GUI' : rep.subsystem}</dd></div>
                <div><dt className="text-xs text-[var(--color-rine-muted)]">Entry point</dt><dd>0x{rep.entry_point_rva.toString(16)}</dd></div>
                <div><dt className="text-xs text-[var(--color-rine-muted)]">Imports</dt><dd>{rep.imports.length}</dd></div>
                <div className="col-span-2 lg:col-span-3"><dt className="text-xs text-[var(--color-rine-muted)]">DLL dependencies</dt><dd>{rep.dll_dependencies.join(', ') || '—'}</dd></div>
              </dl>
            )}
          </SectionCard>

          {pf && (
            <SectionCard title="Compatibility pre-check">
              <div className="flex items-center gap-3">
                <StatusBadge
                  kind={pf.verdict === 'ImportSurfaceSatisfied' ? 'ok' : 'warn'}
                  text={
                    pf.verdict === 'ImportSurfaceSatisfied'
                      ? 'Static import surface satisfied'
                      : `Likely blocked · ${pf.supported}/${pf.total_imports} supported`
                  }
                />
              </div>
              {pf.missing.length > 0 && (
                <ul className="mt-3 flex max-h-40 flex-col gap-1 overflow-auto text-sm">
                  {pf.missing.map(([dll, name]) => (
                    <li key={`${dll}!${name}`} className="text-[var(--color-rine-muted)]">
                      {dll}!{name} <span className="text-[var(--color-rine-err)]">Missing</span>
                    </li>
                  ))}
                </ul>
              )}
            </SectionCard>
          )}

          <SectionCard title="Launch">
            <div className="flex flex-col gap-3">
              <Field label="Arguments (viram a cmdline Win32)">
                <input value={args} onChange={(e) => setArgs(e.target.value)} className={inputCls} placeholder="arg1 arg2" />
              </Field>
              <div>
                <RunButton onClick={run} disabled={running || !rep}>
                  {running ? 'Running…' : 'Run'}
                </RunButton>
              </div>
            </div>
          </SectionCard>

          {report && (
            <SectionCard title="Result">
              <div className="flex items-center gap-3">
                <StatusBadge
                  kind={report.status === 'Exited' && report.exit_code === 0 ? 'ok' : report.status === 'Blocked' ? 'warn' : 'err'}
                  text={`${report.status}${report.exit_code !== null ? ` · exit ${report.exit_code}` : ''} · ${report.duration_ms}ms`}
                />
              </div>
              <pre className="mt-3 max-h-48 overflow-auto rounded-lg bg-[var(--color-rine-panel)] p-3 text-xs whitespace-pre-wrap">
                {report.stdout || '(sem stdout)'}
              </pre>
              {report.missing.length > 0 && (
                <p className="mt-2 text-sm text-[var(--color-rine-muted)]">
                  Bloqueado por {report.missing.length} imports — ver Diagnostics.
                </p>
              )}
            </SectionCard>
          )}
        </div>
      )}
      {!exe && (
        <div className="mt-4">
          <EmptyState title="Nenhum executável" hint="O arquivo é só inspecionado aqui. A execução exige o botão Run." />
        </div>
      )}
    </div>
  );
}
