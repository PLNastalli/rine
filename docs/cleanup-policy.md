# Política de limpeza arquitetural (standing policy)

> Substitua, migre, valide e delete. Não empilhe gerações de arquitetura.
> Git guarda o passado. O source tree representa o presente.

O Rine não acumula camadas mortas, implementações antigas, fallbacks
esquecidos, duplicações ou hacks permanentes.

## Princípio de modificação

```text
implementação antiga → entender → criar/confirmar testes → implementar
substituta → validar equivalência → trocar consumidores → REMOVER a antiga
```

Nunca: old + new + fallback + fallback-do-fallback + hack + "TODO remover depois".

## Regras

1. **Uma fonte de verdade** por conceito (`HandleTable`, parser PE,
   `MemoryManager`, `NtStatus`, tradução de paths…). Duas implementações do
   mesmo conceito exigem justificativa escrita em `current-state.md`
   (por que duas, qual é a final, quem usa a antiga, que teste protege,
   condição de remoção).
2. **Sem medo de deletar** (git tem histórico), **sem deletar cego**
   (compatibilidade estranha pode ser necessária: pesquisar consumidores,
   testes, ADRs, quirks antes).
3. **Núcleo limpo**: `if exe == "foo"` no core é proibido; exceções vão para
   `compat/` (quirks documentados). Abstrações erradas são corrigidas
   (testar→migrar→deletar), não escondidas sob nova camada.
4. **Nomes temporários não sobrevivem** (`_old`, `_v2`, `_new`…): ao concluir,
   a nova vira o nome canônico e a antiga some.
5. **Sem código comentado** como arquivo-morto; sem `#[allow(dead_code)]`
   para silenciar arquitetura mal resolvida (investigar: futuro documentado,
   API deliberada testada, ou DELETE).
6. **TODOs rastreáveis** (razão + condição de resolução) ou resolvidos na hora.
7. **Remoção leva junto**: imports, deps Cargo, flags, módulos/testes/docs
   do comportamento morto.
8. **Ordem de prioridade**: compatibilidade correta > segurança > arquitetura
   > simplicidade > performance. Nunca deletar comportamento necessário
   só para diminuir LOC.
9. **Teste antes de remover** (capturar comportamento → teste → novo →
   comparar → migrar → remover). Para APIs Windows, o oracle é o Windows real.
10. **Limpeza incremental**: só a região tocada pela tarefa
    (boy-scout no caminho, sem reescrita global).
11. **ADR novo** (nunca editar o antigo) quando uma decisão arquitetural é
    substituída, com `Supersedes`.
12. Auditoria final em cada tarefa: mortos, duplicações,
    comentários de comportamento antigo, TODOs sem sentido, deps órfãs, hacks
    no lugar errado.

## Checklist de finalização

`nova em uso, antiga removida, sem código comentado, sem duplicação,
imports/deps mortos removidos, warnings corrigidos (não silenciados),
testes passam, fmt+clippy+test verdes, docs + current-state atualizados.`
