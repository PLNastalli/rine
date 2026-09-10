# kernel32

Façade Win32 pública: marshalling ABI → `kernelbase`. NENHUMA lógica aqui.

- Não é: implementação (qualquer `if` de comportamento é bug de camada).
- API: `GetStdHandle/WriteFile/ReadFile/CreateFileA/CloseHandle/`
  `VirtualAlloc/VirtualFree/VirtualProtect/GetCommandLineW`
  `TlsAlloc/TlsFree/TlsGetValue/TlsSetValue/GetLastError/Sleep`
  `Initialize/Delete/Enter/LeaveCriticalSection/SetUnhandledExceptionFilter/ExitProcess_impl`
  (`extern "win64"`), `resolve`, `EXPORTS` (tabela autoritativa, anti-drift).
  Deps: `winabi`, `kernelbase`, `ntdll`, `nt-thread`.
  Consumers: `runtime` (resolvedor de imports).
- Invariantes: aridade/convenção/nomes == Windows; `*written` + LastError
  sempre consistentes. Unsafe: `from_raw_parts`/`*lp_written` com `SAFETY`.
- Testes: `launcher/tests/hello.rs`. ADRs: 0004, 0006. Futuro: novas APIs
  seguem o mesmo padrão (thin + teste diferencial primeiro).
