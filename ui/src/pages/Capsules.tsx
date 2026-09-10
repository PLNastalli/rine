// Capsules: lista + wizard mínimo (só chaves do schema v0.2, §10).
import { useState } from 'react';
import { api, errMsg, type CapsuleSummary } from '../lib/tauri';
import {
  EmptyState,
  ErrorBox,
  Field,
  GhostButton,
  PageHeader,
  SectionCard,
  StatusBadge,
  inputCls,
} from '../components/ui';

export function Capsules() {
  const [dir, setDir] = useState('');
  const [list, setList] = useState<CapsuleSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('');
  const [driveC, setDriveC] = useState('');

  const load = async () => {
    if (!dir) return;
    setError(null);
    try {
      setList(await api.listCapsules(dir));
    } catch (e: unknown) {
      setError(errMsg(e));
    }
  };

  const create = async () => {
    if (!name.trim() || !dir) return;
    setError(null);
    try {
      await api.createCapsule({
        name: name.trim(),
        parent_dir: dir,
        drives: driveC.trim() ? [['C', driveC.trim()]] : [],
        current_dir: null,
      });
      setName('');
      setCreating(false);
      await load();
    } catch (e: unknown) {
      setError(errMsg(e));
    }
  };

  return (
    <div>
      <PageHeader
        title="Capsules"
        subtitle="Ambientes isolados por aplicação (drives, diretório, perfil Windows)."
        actions={<GhostButton onClick={() => setCreating((c) => !c)}>Create Capsule</GhostButton>}
      />
      {error && (
        <div className="mb-4">
          <ErrorBox message={error} />
        </div>
      )}
      {creating && (
        <div className="mb-4">
          <SectionCard title="Create Capsule">
            <div className="flex flex-col gap-3">
              <Field label="Name">
                <input value={name} onChange={(e) => setName(e.target.value)} className={inputCls} placeholder="game" />
              </Field>
              <Field label="C: → diretório host (isolado; evite / ou $HOME)">
                <input value={driveC} onChange={(e) => setDriveC(e.target.value)} className={inputCls} placeholder="~/Rine/capsules/game/drive_c" />
              </Field>
              <div>
                <button
                  onClick={create}
                  disabled={!name.trim() || !dir}
                  className="rounded-lg bg-[var(--color-rine-accent)] px-4 py-2 text-sm font-semibold text-white disabled:opacity-40"
                >
                  Create
                </button>
              </div>
            </div>
          </SectionCard>
        </div>
      )}
      <SectionCard title="Location">
        <div className="flex gap-2">
          <input
            value={dir}
            onChange={(e) => setDir(e.target.value)}
            className={inputCls}
            placeholder="~/Rine/capsules"
          />
          <GhostButton onClick={load}>List</GhostButton>
        </div>
      </SectionCard>
      <div className="mt-4">
        {list === null ? (
          <EmptyState title="No Capsules yet." hint="Create one to isolate an application's environment." />
        ) : list.length === 0 ? (
          <EmptyState title="Nenhuma capsule aqui." hint="O diretório não contém capsule.toml." />
        ) : (
          <div className="grid gap-3 lg:grid-cols-2">
            {list.map((c) => (
              <SectionCard key={c.path} title={c.name}>
                <div className="flex items-center gap-2 text-sm">
                  <StatusBadge kind={c.valid ? 'ok' : 'err'} text={c.valid ? 'Válida' : 'Inválida'} />
                  <span className="text-[var(--color-rine-muted)]">
                    Windows {c.windows_version[0]}.{c.windows_version[1]} · drives {c.drives.join(', ') || '—'}
                  </span>
                </div>
                <div className="mt-1 truncate text-xs text-[var(--color-rine-muted)]">{c.path}</div>
                {c.error && <div className="mt-2 text-sm text-[var(--color-rine-err)]">{c.error}</div>}
              </SectionCard>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
