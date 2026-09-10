// Settings: preferências da GUI (só UI; nada do runtime, §23).
import { useEffect, useState } from 'react';
import { api, errMsg, type Appearance, type LogLevel, type Settings as S } from '../lib/tauri';
import { ErrorBox, Field, PageHeader, SectionCard, inputCls } from '../components/ui';

export function Settings({ apply }: { apply: (s: S) => void }) {
  const [s, setS] = useState<S | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    api
      .getSettings()
      .then(setS)
      .catch((e: unknown) => setError(errMsg(e)));
  }, []);

  const save = async () => {
    if (!s) return;
    setError(null);
    setSaved(false);
    try {
      await api.saveSettings(s);
      apply(s);
      setSaved(true);
    } catch (e: unknown) {
      setError(errMsg(e));
    }
  };

  if (!s) return <p className="text-sm text-[var(--color-rine-muted)]">Carregando…</p>;

  return (
    <div>
      <PageHeader title="Settings" subtitle="Preferências do Manager (nunca do runtime)." />
      {error && (
        <div className="mb-4">
          <ErrorBox message={error} />
        </div>
      )}
      <div className="flex max-w-xl flex-col gap-4">
        <SectionCard title="Appearance">
          <Field label="Tema">
            <select
              value={s.appearance}
              onChange={(e) => setS({ ...s, appearance: e.target.value as Appearance })}
              className={inputCls}
            >
              <option value="system">System</option>
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
          </Field>
        </SectionCard>
        <SectionCard title="General">
          <div className="flex flex-col gap-3">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={s.confirm_destructive}
                onChange={(e) => setS({ ...s, confirm_destructive: e.target.checked })}
              />
              Confirmar ações destrutivas
            </label>
            <Field label="Diretório default de capsules">
              <input
                value={s.default_capsule_dir}
                onChange={(e) => setS({ ...s, default_capsule_dir: e.target.value })}
                className={inputCls}
              />
            </Field>
          </div>
        </SectionCard>
        <SectionCard title="Diagnostics">
          <div className="flex flex-col gap-3">
            <Field label="Nível de log">
              <select
                value={s.log_level}
                onChange={(e) => setS({ ...s, log_level: e.target.value as LogLevel })}
                className={inputCls}
              >
                <option value="error">Error</option>
                <option value="warn">Warn</option>
                <option value="info">Info</option>
                <option value="debug">Debug</option>
                <option value="trace">Trace</option>
              </select>
            </Field>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={s.developer_mode}
                onChange={(e) => setS({ ...s, developer_mode: e.target.checked })}
              />
              Developer mode (difftest, bench, oracle — quando existirem na UI)
            </label>
          </div>
        </SectionCard>
        <div className="flex items-center gap-3">
          <button
            onClick={save}
            className="rounded-lg bg-[var(--color-rine-accent)] px-4 py-2 text-sm font-semibold text-white"
          >
            Save settings
          </button>
          {saved && <span className="text-sm text-[var(--color-rine-ok)]">● Settings saved</span>}
        </div>
      </div>
    </div>
  );
}
