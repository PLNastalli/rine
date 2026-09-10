# Rine Manager — modelo de dados (só metadata própria)

> Capsule config, API DB e status de compatibilidade pertencem ao Rine.
> O Manager guarda SÓ: apps cadastrados, runs recentes, preferências.
> Formato versionado (`schema`), migração explícita, nunca duplicar o core.

## `registry.json` (XDG: `~/.local/share/rine-manager/registry.json`)

```json
{
  "schema": 1,
  "apps": [
    {
      "id": "01J… (ulid)",
      "name": "Suitetest",
      "exe_path": "/home/u/apps/suite.exe",
      "capsule_path": "/home/u/apps/capsule.toml",
      "added_at_unix": 1757420000,
      "last_run": {
        "at_unix": 1757420100,
        "status": "Exited",
        "exit_code": 0
      }
    }
  ]
}
```

- `remove` apaga só o cadastro (nunca o `.exe`; deletar programa = ação
  explícita separada, com confirmação).
- `status` do app (`Unknown|ImportBlocked|Starts|WorkflowPartial|Working|
  Regression|Crashed`) vem de dados reais: último pre-flight + último run.
  `Working` exige run `Exited(0)` + import surface satisfied; nada é
  inferido automaticamente além dessa regra.

## `RunReport` (por execução; retido por `diagnostics.retain_runs`, default 50)

```text
{ app_id, started_at, duration_ms, status, exit_code?,
  stdout_trunc(64KiB), stderr_trunc(64KiB),
  crash?: CrashInfo, missing?: [(dll, name)], capsule_path?, rine_version }
```

Logs gigantes: truncar + `log_path` para o arquivo completo; viewer
virtualizado na UI (nunca carrega tudo no DOM).

## `settings.json` (`schema: 1`)

`appearance(system|dark|light) · default_capsule_dir · confirm_destructive
· log_level(info default) · retain_runs · developer_mode(false default)`.

## Eventos tipados (UI reage a eventos, não a parse de log espalhado)

`AppStarting | AppStarted{pid} | ImportBlocked{missing} |
GuestException{…} (v0.3+) | AppExited{code} | RuntimeError{…} |
DiagnosticGenerated{report_path}`.
Hoje o runtime só produz texto: a adaptação mora isolada em
`manager-core::run::classify` (único lugar que lê stderr).
