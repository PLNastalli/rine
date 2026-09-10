# nt-file

Semântica `NtCreateFile`/Win32 sobre I/O Linux. v0.1: console (`write_file`).

- Não é: parser de paths NT (v0.2), syscalls diretos (via `host-linux`).
- API: `DriveMap::translate`, `create/write/read/close_file`,
  `CreateOptions::from_win32`. Deps: `winabi`, `nt-object`, `host-linux`.
  Consumers: `ntdll`.
- Invariantes: `*written` sempre preenchido pelo caller; tipo checado
  (`File` vs outros); `owns_fd` (console nunca fecha). Erros: `NtStatus`.
  Unsafe: nenhum.
- Testes: tradução + roundtrip create/write/read + E2E `file.exe`.
  ADRs: 0005, 0006, 0007.
- Testes: via hello E2E (v0.2: `file.exe`). ADRs: 0005, 0006.
