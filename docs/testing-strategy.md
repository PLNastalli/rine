# Estratégia de testes

Pirâmide: unitários (por crate) → integração (loader/runtime) → comportamento
(PEs reais) → diferencial (vs Windows real, oracle) → campanhas (milhões de
cenários gerados — ver `docs/differential-testing.md`).

Plataforma (`crates/difftest`, CLI `rine-test`): cenários versionados com
seed, normalização explícita, 8 vereditos, shrink, corpus permanente,
runner paralelo determinístico. Docs: `differential-testing.md`,
`windows-oracle.md`, `fuzzing.md`, `behavioral-coverage.md`,
`non-regression-policy.md` (princípio central: nunca quebrar o que funciona).

## Harness atual (v0.1)

- Unitários: `cargo test --workspace` (parser, handles, memória, relocs…).
- Integração/E2E: `crates/launcher/tests/hello.rs` — gera `hello.exe` via
  `pe::builder`, roda `rine` como **processo filho** (guest chama
  `ExitProcess` = `exit` do filho), verifica `stdout == "Hello World\n"` e
  exit 0. Filho isolado é obrigatório sempre que o guest puder terminar o
  processo.
- Debug: `gdb -batch -ex run -ex bt` (foi assim que achamos o SIGSEGV do
  `written` em `.rdata`); exemplos `dump_hello` e `check_iat` como ferramentas.

## Diferencial (método permanente)

1. Escrever o menor PE que exercita UM comportamento (builder ou, futuro,
   `cl.exe`/`mingw` versionado em `tests/windows/` com hash).
2. Rodar no Windows real, capturar oracle (exit, stdout/stderr, LastError…).
3. Rodar no `rine`, comparar byte-a-byte. Divergência = bug + regression test.
4. Todo fix de compatibilidade começa pelo teste que reproduz.

## Regra standing: `suite.exe` + `evil.exe`

Toda API nova entra no `suite.exe` (`pe::builder::build_suite_exe`, exit 0
só se TODAS passarem, com verificação dentro do guest e marcadores no
stdout). Todo input inválido que deva falhar limpo entra no `evil.exe`
(exit 0 só se todos os 12+ abusos forem recusados sem crash).
`crates/launcher/tests/suite.rs` roda ambos em filho isolado.
Fronteira honesta: ponteiros selvagens NÃO são testados (sem SEH até v0.3 —
derrubariam o host; documentado, não escondido).

## Diferencial e oracle (método permanente, infra pronta)

1. Escrever o menor PE que exercita UM comportamento.
2. Rodar no Windows real via probe (`tests/windows/oracle_probe.c`) →
   `OracleResult` schema 1 em `compatibility/builds/<build>/`.
3. Rodar no `rine` via `oracle::run_case` → mesmo formato.
4. `normalized diff`: compatível só com diff limpo dentro de regras
   documentadas. Status `DifferentiallyVerified` só então.
5. Regressões abertas vivem em `compatibility/regressions/` (uma por caso).

## ABI, fuzz, benches

- ABI: `crates/winabi/tests/abi.rs` (+`tests/abi/README.md`) — contrato binário.
- Fuzz: `fuzz/README.md` + corpus determinístico (`pe`, `nt-file`); libfuzzer
  só se migrarmos para nightly (upgrade path documentado).
- Benches: `bench/` + `rine-bench check` no CI (warn-only em alpha);
  gate real em `docs/adr/ADR-0012-perf-gate.md` (mediana pós-warmup,
  thresholds 10/15, promoção manual auditável).

## Regras

- Nenhum bug importante corrigido sem regression test.
- `tests/differential/` guardará corpus + oracles (v0.2).
- Comportamento observável inclui: return values, NTSTATUS, LastError, I/O,
  side effects em memória/handles, callbacks, timing grosso.
