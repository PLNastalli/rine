# kernelbase

Implementação Win32 interna (lógica real; `kernel32` só encaminha).

- Não é: ABI (sem `extern`, sem ponteiros crus).
- API: `get_std_handle`, `write/read_file`, `create_file_a`, `close_handle`,
  `virtual_alloc/free/protect`, `exit_process`. Deps: `ntdll`, `nt-file`.
  Consumers: `kernel32`.
- Erros: `Win32Error` (+ LastError na façade). Unsafe: nenhum.
- Testes: hello + v02 E2E. ADRs: 0004, 0006, 0007.
