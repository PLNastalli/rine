# Rine Manager — interface com o runtime (contratos exatos)

## Detecção

- Candidatos, nesta ordem: `$RINE_BIN`, **sidecar** (ao lado do executável
  atual — layout oficial do instalador e de `target/debug/` em dev),
  `<base>/rine`, `<base>/../target/debug/rine`, `PATH`.
- Validação: `<bin> --version` stdout == `rine <semver>`; qualquer outra
  saída/exit != 0 = binário inválido (não tenta adivinhar).
- Compatível: mesma versão do workspace (`0.2.0-alpha.3`); diferente =
  `ProtocolMismatch` (GUI mostra `Runtime version mismatch`, não executa).
- Ausente: `RUNTIME_NOT_FOUND` com instrução acionável (`cargo build
  -p launcher`, sidecar ou `$RINE_BIN`) — nunca "arquivo inexistente" seco.

## Distribuição (sidecar)

O instalador entrega `rine` + `rine-manager` lado a lado; o Tauri declara
`bundle.externalBin: ["binaries/rine"]` e
`crates/manager-tauri/pack-sidecar.sh` gera
`binaries/rine-<triple>` (ignorado pelo git; só o README é versionado).
Em dev: `cargo build -p launcher` antes de `cargo run -p manager-tauri`.

## Inspeção PE (sem executar)

`pe::Image::parse(bytes)` → `imports()` → `[{dll, name|#ordinal}]`.
Metadados: `machine`, `subsystem`, `entry_rva`, `image_base`.
Falha de parse = `UnsupportedPe` (nunca panic; bytes arbitrários do usuário).

## Pre-flight (antes do Run)

1. `missing = runtime::missing_imports(bytes)` → `(dll, nome)` sem duplicatas.
2. `total` = nº de símbolos importados; `supported = total − missing.len()`.
3. Veredito: vazio → `Import surface satisfied`; senão → `Likely blocked`
   + lista ordenada (KERNEL32 primeiro, depois alfabético).
4. Terminologia: ver `architecture.md`. `GetProcAddress`/`LoadLibrary`
   dinâmicos NÃO aparecem aqui (são runtime, não imports estáticos).

## Execução

```
rine [--capsule <toml>] <exe> [guest-args...]   (argv estruturado, sem shell)
cwd = workdir pedido (default: dir do exe) · env herdado + extras pedidos
```

Captura: stdout (bytes do guest, intocados), stderr (logs + relatórios),
exit code. Classificação:

| Sinal | Estado |
|---|---|
| stderr contém `RINE-CRASH schema=1` | `Crashed` (+ `CrashInfo{version,executable,phase,reason,location,timestamp}`) |
| exit != 0 + `RINE-DEMAND` em stderr | `Blocked` (+ lista de imports) |
| demais | `Exited(code)` |

Timeout: pedido cancela o filho (`kill`), estado `Crashed{reason: timeout}` —
nunca espera infinita na thread da UI (operações pesadas fora da UI thread).

## Capsules (schema v0.2 — a GUI não inventa chaves)

```toml
[app]
name = "x"                 # obrigatório (default "app" se ausente no load)
windows_version = [10, 0]  # opcional
arch = "x86_64"            # fixo (lido, nunca editável)
current_dir = "C:\\work"   # opcional
[drives]
C = "/host/path"           # letra maiúscula; GUI avisa antes de / ou $HOME
```

Criar = escrever este schema; campos futuros do runtime aparecem
`disabled/coming soon` até o runtime os ler de verdade.

## Leitura (nunca escrita pelo Manager)

- `api-db/windows-11-25h2-x64/coverage.json` (status por API).
- `bench/baselines/*.json` (cards current-vs-baseline via `perf`).
- `compatibility/matrix.json` (estados PASS/FAIL/KNOWN_DIFFERENCE/…).
