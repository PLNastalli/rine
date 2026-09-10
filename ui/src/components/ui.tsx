// Design system (§46/§47): componentes reutilizáveis, um visual só.
import type { ReactNode } from 'react';

const shell =
  'rounded-xl border border-[var(--color-rine-border)] bg-[var(--color-rine-card)]';

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="mb-5 flex items-start justify-between gap-4">
      <div>
        <h1 className="text-xl font-semibold text-[var(--color-rine-text)]">{title}</h1>
        {subtitle && <p className="mt-1 text-sm text-[var(--color-rine-muted)]">{subtitle}</p>}
      </div>
      {actions && <div className="flex gap-2">{actions}</div>}
    </div>
  );
}

export function SectionCard({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className={`${shell} p-4`}>
      {title && (
        <h2 className="mb-3 text-sm font-semibold tracking-wide text-[var(--color-rine-muted)] uppercase">
          {title}
        </h2>
      )}
      {children}
    </section>
  );
}

export function MetricCard({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className={`${shell} p-4`}>
      <div className="text-xs text-[var(--color-rine-muted)]">{label}</div>
      <div className="mt-1 text-2xl font-semibold text-[var(--color-rine-text)]">{value}</div>
      {sub && <div className="mt-1 text-xs text-[var(--color-rine-muted)]">{sub}</div>}
    </div>
  );
}

// Badge: sempre ícone + texto + cor (§29 — nunca só cor).
export function StatusBadge({ kind, text }: { kind: 'ok' | 'warn' | 'err' | 'info' | 'muted'; text: string }) {
  const map = {
    ok: ['●', 'text-[var(--color-rine-ok)]'],
    warn: ['▲', 'text-[var(--color-rine-warn)]'],
    err: ['■', 'text-[var(--color-rine-err)]'],
    info: ['◆', 'text-[var(--color-rine-accent)]'],
    muted: ['○', 'text-[var(--color-rine-muted)]'],
  } as const;
  const [glyph, color] = map[kind];
  return (
    <span
      className={`${shell} inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium text-[var(--color-rine-text)]`}
    >
      <span className={color} aria-hidden="true">
        {glyph}
      </span>
      {text}
    </span>
  );
}

export function EmptyState({
  title,
  hint,
  action,
}: {
  title: string;
  hint: string;
  action?: ReactNode;
}) {
  return (
    <div className={`${shell} flex flex-col items-center gap-2 p-10 text-center`}>
      <div className="text-base font-medium text-[var(--color-rine-text)]">{title}</div>
      <div className="max-w-md text-sm text-[var(--color-rine-muted)]">{hint}</div>
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function RunButton({
  onClick,
  disabled,
  children,
}: {
  onClick: () => void;
  disabled?: boolean;
  children?: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="rounded-lg bg-[var(--color-rine-accent)] px-4 py-2 text-sm font-semibold text-white disabled:cursor-not-allowed disabled:opacity-40"
    >
      {children ?? 'Run'}
    </button>
  );
}

export function GhostButton({
  onClick,
  children,
}: {
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`${shell} px-3 py-1.5 text-sm text-[var(--color-rine-text)] hover:border-[var(--color-rine-accent)]`}
    >
      {children}
    </button>
  );
}

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs text-[var(--color-rine-muted)]">{label}</span>
      {children}
    </label>
  );
}

export const inputCls =
  'w-full rounded-lg border border-[var(--color-rine-border)] bg-[var(--color-rine-panel)] px-3 py-2 text-sm text-[var(--color-rine-text)]';

export function ErrorBox({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-[var(--color-rine-err)] p-3 text-sm text-[var(--color-rine-text)]">
      <span className="font-semibold text-[var(--color-rine-err)]">■ Erro. </span>
      {message}
    </div>
  );
}
