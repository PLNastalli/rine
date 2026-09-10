// Camada única de acesso ao backend (§35/§36 da missão).
// Componentes NUNCA importam @tauri-apps/api diretamente: tudo passa daqui,
// com tipos espelhando o serde de manager-core/manager-tauri.
import { invoke } from '@tauri-apps/api/core';

export interface CmdError {
  code: string;
  message: string;
}

export interface RuntimeDto {
  version: string;
  binary: string;
  compatible: boolean;
}

export interface ImportRef {
  dll: string;
  name: string | null;
  ordinal: number | null;
}

export interface PeReport {
  filename: string;
  size_bytes: number;
  machine: number;
  subsystem: number;
  entry_point_rva: number;
  image_base: number;
  imports: ImportRef[];
  dll_dependencies: string[];
}

export type PreflightVerdict = 'ImportSurfaceSatisfied' | 'LikelyBlocked';

export interface PreflightReport {
  total_imports: number;
  supported: number;
  missing: [string, string][];
  verdict: PreflightVerdict;
}

export type RunStatus = 'Blocked' | 'Crashed' | 'Exited';

export interface CrashInfo {
  version: string;
  executable: string;
  phase: string;
  reason: string;
  location: string;
  timestamp: number;
}

export interface RunReport {
  status: RunStatus;
  exit_code: number | null;
  stdout: string;
  stderr: string;
  crash: CrashInfo | null;
  missing: [string, string][];
  duration_ms: number;
  rine_version: string;
}

export interface LastRun {
  at_unix: number;
  status: RunStatus;
  exit_code: number | null;
}

export interface AppEntry {
  id: string;
  name: string;
  exe_path: string;
  capsule_path: string | null;
  added_at_unix: number;
  last_run: LastRun | null;
}

export interface CapsuleSummary {
  path: string;
  name: string;
  windows_version: [number, number];
  drives: string[];
  valid: boolean;
  error: string | null;
}

export type Appearance = 'system' | 'dark' | 'light';
export type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';

export interface Settings {
  schema: number;
  appearance: Appearance;
  default_capsule_dir: string;
  confirm_destructive: boolean;
  log_level: LogLevel;
  retain_runs: number;
  developer_mode: boolean;
}

export interface RunPayload {
  exe: string;
  capsule: string | null;
  args: string[];
  workdir: string | null;
  env_extra: [string, string][];
  timeout_ms: number;
}

export interface CreateCapsulePayload {
  name: string;
  parent_dir: string;
  drives: [string, string][];
  current_dir: string | null;
}

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T> =>
  invoke<T>(cmd, args);

export const api = {
  detectRuntime: () => call<RuntimeDto>('detect_runtime'),
  inspectPe: (path: string) => call<PeReport>('inspect_pe', { path }),
  preflight: (path: string) => call<PreflightReport>('preflight', { path }),
  runExe: (payload: RunPayload) => call<RunReport>('run_exe', { payload }),
  runRegisteredApp: (id: string, args: string[], timeout_ms: number) =>
    call<RunReport>('run_registered_app', { id, args, timeoutMs: timeout_ms }),
  listApps: () => call<AppEntry[]>('list_apps'),
  addApp: (exe_path: string, name?: string | null) =>
    call<AppEntry>('add_app', { exePath: exe_path, name: name ?? null }),
  removeApp: (id: string) => call<void>('remove_app', { id }),
  setAppCapsule: (id: string, capsule: string | null) =>
    call<void>('set_app_capsule', { id, capsule }),
  listCapsules: (dir: string) => call<CapsuleSummary[]>('list_capsules', { dir }),
  createCapsule: (payload: CreateCapsulePayload) =>
    call<string>('create_capsule', { payload }),
  getSettings: () => call<Settings>('get_settings'),
  saveSettings: (value: Settings) => call<void>('save_settings', { value }),
};

export const isCmdError = (e: unknown): e is CmdError =>
  typeof e === 'object' && e !== null && 'code' in e && 'message' in e;

export const errMsg = (e: unknown): string =>
  isCmdError(e) ? `[${e.code}] ${e.message}` : String(e);

// Detecção sem throw (para o provider: estado carrega {runtime, error}).
export const detectRuntimeSafe = async (): Promise<{
  runtime: RuntimeDto | null;
  error: string | null;
}> => {
  try {
    return { runtime: await api.detectRuntime(), error: null };
  } catch (e: unknown) {
    return { runtime: null, error: errMsg(e) };
  }
};
