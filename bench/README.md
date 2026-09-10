# bench/ — regression gate de performance (medir antes de otimizar)

Filosofia:

```text
Performance philosophy:
Correctness first.
Compatibility second only to correctness.
Performance regressions must be visible.
Optimization must preserve Windows behavior.
```

Regra: **regressão de performance não é automaticamente bug se compra
compatibilidade necessária — mas nunca pode acontecer invisível.**
Nunca sacrifique compatibilidade para vencer benchmark.

## Método (warmup + mediana, nunca execução única)

Cada benchmark: 1 batch de warmup (descartado: cold cache, alocação
inicial) + 10 batches medidos; amostra = ns/op por batch; decisão pela
**mediana** (robusta a outliers). Rode da raiz do repo, build `dev`
(padrão; `release` muda os números — registrado no ambiente).

```bash
cargo run -q -p perf --bin rine-bench -- run [--out results.json] [--batches N]
cargo run -q -p perf --bin rine-bench -- compare <results.json> <baseline>
cargo run -q -p perf --bin rine-bench -- check [--baseline PATH] [--strict]
```

`check` = run + compare num passo (o que o CI executa).

## Política (alpha: alerta, sem bloqueio)

```text
<= 10% mais lento   PASS
> 10% e <= 15%      WARNING
> 15%               PERFORMANCE REGRESSION WARNING (não bloqueia em alpha)
```

Futura (`--strict` / `Policy::stable`): `>5%` injustificado = FAIL.
Trocar a política é trocar `compare::Policy` + documentar.

## Baselines

`baselines/v0.2.0-alpha.1.json`: mediana + amostras + ambiente completo
(commit, CPU, kernel, toolchain, profile, flags, timestamp). Metodologia
anterior (média de batch único, sem warmup) media 1981/3394ns + 14us —
registrado como história, sem comparação direta: metodologia mudou
(ver `method_version`; método diferente nunca compara).

## Promoção (manual e auditável, nunca automática)

```bash
rine-bench promote results.json --baseline bench/baselines/X.json \
  --reason "..." [--by ...] [--accept-regression]
```

Sem `--reason` recusa. Se a nova baseline piora algo, exige também
`--accept-regression` (a piora fica registrada com motivo).

## Ruído assumido

Frequência dinâmica, scheduler, filesystem cache e logging são mitigados
(warmup, batches, mediana, sem I/O no caminho medido), não eliminados.
Runners compartilhados (GitHub-hosted) têm variação alta: o CI publica e
avisa, sem hard-fail (ver `.github/workflows/ci.yml`); self-hosted dedicado
no futuro decide pelo `env` gravado (comparar máquinas diferentes como
iguais é bug — o relatório grita `env difere`).

## Benchmarks (IDs estáveis; mudou o escopo → mude o ID)

- `pe.parse.hello` — parse+imports+exports de hello.exe
- `pe.parse.suite` — parse+imports+exports de suite.exe
- `loader.full.suite` — map+relocs+imports+protect de suite.exe (**sem enter**;
  enter terminaria o processo; pular etapas para melhorar número é proibido)
