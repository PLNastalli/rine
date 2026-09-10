# ntdll

Façade NT: `Nt*/Rtl*` finos sobre `nt-*` + contexto do processo emulado.

- Não é: lógica Win32 (isso é `kernelbase`).
- API: `install/require_process_context`, `nt_write/read/create_file`,
  `nt_close`, `nt_allocate/free/protect_virtual_memory`,
  `RtlExitUserProcess_impl`, `NtTerminateProcess_impl`
  (`extern "win64"`, `non_snake_case` intencional).
  Deps: `winabi`, `nt-object`, `nt-memory`, `nt-file`. Consumers:
  `kernelbase`, `runtime`.
- Invariantes: ponto único de saída (`rtl_exit_user_process`); contexto
  instalado antes de qualquer export. Erros: `NtStatus`. Unsafe: nenhum
  (só tipos de função `extern`).
- Testes: via hello E2E. ADRs: 0003, 0004, 0006. Futuro: remover global
  (contexto por-TEB/GS).
