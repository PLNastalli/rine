# Windows Oracle

O Windows real é o oracle comportamental principal. Wine NUNCA é oracle final.

## Formato (`crates/oracle`, schema 1)

`OracleResult`: teste, lado (`rine`/`windows`), versões, exit code, crash,
stdout/stderr (bytes mandam), observations (return value, LastError,
NTSTATUS), arquivos criados. Campo aditivo `timed_out` (default false —
JSON antigo continua válido). Leitores rejeitam `schema` desconhecido.

## Produtores

- **Rine**: `oracle::run_case` (filho isolado, timeout com kill, drenagem de
  pipes — sem deadlock) e `difftest::exec::run_fileops_guest`.
- **Windows**: `tests/windows/oracle_probe.c` (MinGW; compila aqui, RODA SÓ
  no Windows real; imprime um `OracleResult` por linha). Resultados medidos
  vão para `compatibility/builds/<build>/`. **Nunca commitar fictício.**

## Cache e hashes

Resultados Windows são caros: reutilizar permitido SOMENTE para a mesma
tupla `(Windows build, test version, scenario)` — ver `docs/adr/ADR-0009-*`.
Cenários carregam `seed` + `scenario_id` (FNV-1a estável); resultados citam
`rine_version`, build do Windows e schema. Sem build registrado, sem mistura.

## Sem host Windows (estado atual)

Diferencial Rine×Windows está infra-pronto e bloqueado em ambiente. O que
roda hoje: Rine×modelo (difftest), Rine×baseline gravada (matriz), Rine×Rine
(determinismo). `DifferentiallyVerified` continua zerado por construção.
