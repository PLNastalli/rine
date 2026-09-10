# ADR-0006 — Modelo de erros: `NtStatus` dentro, Win32 na borda

- Status: aceito (2026-09-10).
- Context: dois domínios de erro (NTSTATUS vs `GetLastError`) com mapeamento
  não-trivial.
- Decision: interior retorna `Result<T, NtStatus>` (ou enums que convertem);
  conversão para Win32 SÓ nas façades (`ntstatus_to_win32` + `set_last_error`),
  com tabela gerada de headers em v0.3. `WriteFile` preenche `*written` mesmo
  em falha parcial (semântica observável).
- Alternatives: `anyhow`/strings (perde fidelidade); errno direto (domínio
  errado).
- Why: testes diferenciais comparam códigos exatos; erro tipado guia o
  programador para a semântica certa.
- Consequences: cada nova API documenta seus NTSTATUS possíveis (rustdoc).
- Risks: tabela mínima v0.1 (5 entradas) — expandir por diferencial.
