// Fonte única do runtime: store externa + useSyncExternalStore.
// Sem useEffect (sem set-state-in-effect), sem provider (sem only-export),
// fetch único compartilhado, retry explícito. O primitivo certo do React
// para "sincronizar com sistema externo" (o binário rine).
import { useSyncExternalStore } from 'react';
import { detectRuntimeSafe, type RuntimeDto } from './tauri';

export interface RuntimeState {
  loading: boolean;
  runtime: RuntimeDto | null;
  error: string | null;
}

let state: RuntimeState = { loading: true, runtime: null, error: null };
const listeners = new Set<() => void>();
let inflight = false;

function emit() {
  listeners.forEach((l) => l());
}

function fetch() {
  if (inflight) return;
  inflight = true;
  state = { ...state, loading: true, error: null };
  emit();
  void detectRuntimeSafe().then(({ runtime, error }) => {
    inflight = false;
    state = { loading: false, runtime, error };
    emit();
  });
}

function subscribe(notify: () => void): () => void {
  listeners.add(notify);
  if (listeners.size === 1) fetch(); // primeiro assinante dispara a detecção
  return () => {
    listeners.delete(notify);
  };
}

function snapshot(): RuntimeState {
  return state;
}

export function useRuntime() {
  const s = useSyncExternalStore(subscribe, snapshot);
  return { ...s, retry: fetch };
}
