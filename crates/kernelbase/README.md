# kernelbase

Implementação Win32 interna (lógica real; `kernel32` só encaminha).

- Não é: ABI (sem `extern`, sem ponteiros crus).
- API: `get_std_handle`, `write/read_file`, `create_file_a`, `close_handle`,
  `virtual_alloc/free/protect`, `tls_alloc/get/set/free`, `sleep_ms`,
  `initialize/delete/enter/leave_critical_section`,
  `set_unhandled_exception_filter`, `exit_process`.
  Deps: `ntdll`, `nt-file`, `nt-thread` (slots TLS), `nt-sync` (CS), `tracing`.
  Consumers: `kernel32`.
- Erros: `Win32Error` (+ LastError na façade). Unsafe: nenhum.
- Testes: hello + v02/v03 E2E; ciclo TLS com contexto real (unit). ADRs:
  0004, 0006, 0007, 0011.
