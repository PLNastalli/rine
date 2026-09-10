# AGENTS.md — Constituição do Rine (para IAs e humanos)

> Uma sessão nova deve conseguir continuar só lendo este arquivo + os docs
> indicados. Nunca confie em memória externa. O source tree representa o
> presente; o git (quando existir — **hoje NÃO há repo git**) guardará o passado.

## O que é

Runtime de compatibilidade Windows-para-Linux em Rust. PE x86_64 roda o
**código original direto na CPU** (sem VM, sem emulação, sem Wine).
Princípio: *Windows por fora, Rust seguro por dentro, Linux nativo embaixo.*

Versão: `0.2.0-alpha.1` (`rine --version`). Semver real: alpha por milestone
em curso, `0.MINOR.0` ao fechar, `PATCH` só para fixes (`docs/roadmap.md`).

## Ordem de leitura obrigatória (toda sessão)

1. Este `AGENTS.md`
2. `README.md`
3. `docs/current-state.md` (verdade atual: funciona / falta / próximo passo)
4. `docs/architecture.md` (arquitetura REAL, não aspiracional)
5. `docs/roadmap.md` + `docs/dependency-map.md`
6. Doc do subsistema + ADRs + testes da área a alterar

## O que funciona (com evidência, não afirmação)

- `hello.exe` hand-built → `Hello World`, exit 0 (`launcher/tests/hello.rs`).
- v0.2: `file/alloc/args.exe` + **`suite.exe`** (todas as APIs em cadeia,
  exit 0) + **`evil.exe`** (12 abusos contidos, exit 0) — `tests/suite.rs`,
  `tests/v02.rs`. `hello.exe` é âncora: hash `c9ba94…` verificado.
- 21 exports kernel32 + 2 ntdll. Cobertura honesta em `docs/compatibility.md`
  e `api-db/coverage.json` (estados: Missing/Stub/Partial/Implemented/
  BehaviorTested/DifferentiallyVerified — nada é "implementado" sem teste).
- Referência Windows real: `windows-reference/win11-25h2/System32`
  (3410 DLLs; **oracle, nunca dependência**). MinGW x86_64 disponível no host.

## Regras inegociáveis

1. **ABI Windows é sagrada** (convenção, nomes, ordinais, layouts, NTSTATUS,
   LastError, PEB/TEB). Melhorias só INTERNAS.
2. **Sem stubs mentirosos**: stub temporário = marcado `Stub`, nunca contado
   como compatibilidade. Load falha limpo em import não resolvido.
3. **Demanda real define prioridade**: app → import faltante → api-db →
   teste → implementar → validar. Scanner gera conhecimento, não stubs.
4. **Limpeza** (`docs/cleanup-policy.md`): substituir→migrar→validar→deletar.
   Sem `_old/_v2`, sem código comentado, sem deps mortas, sem warnings
   silenciados.
5. **`unsafe` só nas fronteiras** com `SAFETY:`/`# Safety` (`docs/safety-model.md`).
6. **Sem `unwrap/expect` no runtime**; erros tipados (`NtStatus` dentro,
   Win32 na borda).
7. **Toda API nova entra no `suite.exe`; todo abuso contido entra no `evil.exe`.**
8. **Nunca declarar compatibilidade sem prova** (teste que demonstra).
9. Proprietário Microsoft: analisar/oracle OK; copiar implementação, NÃO.
10. FS da referência é case-insensitivo na prática (`KernelBase.dll` existe,
    `kernelbase.dll` "não" — o scanner normaliza).
11. **Nunca quebre o que funciona** (`docs/non-regression-policy.md`):
    baseline antes, ordem unit→smoke depois, `REGRESSION` é grave, todo bug
    ganha teste em `tests/regression/`, matriz sem regressão silenciosa.

## Onde mora cada coisa

- Parse PE: `crates/pe` (único parser — reutilizar, não duplicar).
- Scan/oracle/diff infra: `crates/api-scan`, `crates/oracle`, `api-db/`,
  `compatibility/`, `tests/{abi,behavior,integration,differential,windows}/`.
- Façades finas: `kernel32→kernelbase→ntdll`; lógica em `nt-*`; syscalls
  SÓ em `host-linux`. Quirks em `compat/` (nunca `if exe=="x"` no core).
- Docs são memória: `docs/` + ADRs + `current-state.md` atualizado SEMPRE.

## Gates (tudo verde ou a tarefa não acabou)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Checklist integral em `docs/cleanup-policy.md` + Definition of Done na missão.
