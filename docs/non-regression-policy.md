# Política de não-regressão (princípio central)

> **Nunca quebre algo que já funcionava.** Compatibilidade acumulada é
> patrimônio. Se funcionava ontem, deve continuar funcionando amanhã —
> salvo decisão explícita e documentada de mudança de comportamento.

## Antes de alterar um subsistema

1. Descubra quais comportamentos dependem dele (testes + `smoke-apps/`).
2. Registre o baseline atual (campanha curta + `bench` se tocar hot path).
3. Só então mude. Primeira hipótese diante de teste antigo falhando:
   **"introduzi uma regressão?"** — nunca "o teste está errado".

## Depois da alteração (ordem de validação)

`unit → subsystem → regression → ABI → differential → integration →
smoke-apps`. Compare matriz antes/depois. App que funcionava e parou =
mudança NÃO pronta.

## Estados da matriz (`compatibility/matrix.json`)

`PASS` | `FAIL` | `KNOWN_DIFFERENCE` | `NEW_PASS` | `REGRESSION`.
`REGRESSION` é grave: exige causa, issue, apps afetados, plano de
recuperação e marca explícita na matriz. Proibido: apagar teste que falha,
alterar expectation para passar no CI, marcar `KNOWN_DIFFERENCE` sem
investigação, silenciar crash, remover comportamento suportado por
inconveniência.

## Regression obrigatório

`bug → reproducer mínimo → teste falha → correção → teste passa →
tests/regression/`. `rine-test minimize --promote` faz o caminho;
fuzz segue o mesmo fluxo (seed salva). Concorrência: seed + threads +
barreiras registradas.

## Refatoração e deleção

Refatorar = mesmo comportamento externo + arquitetura melhor (baseline
antes/depois). Deletar código velho só depois de provar cobertura pela
nova implementação (testes da antiga passam na nova primeiro).

## Exceção: segurança > compatibilidade

Memory unsafety, UB e vulnerabilidade se corrigem mesmo quebrando
comportamento dependente: preserve o legítimo, documente o inevitável.

## Release gate

Nenhum merge/release com regressão conhecida sem justificativa explícita
(documentada na matriz + current-state). Métrica principal: quantas APIs
funcionavam antes, quantas agora, **quantas regrediram** (alvo: zero).
