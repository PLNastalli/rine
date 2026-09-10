# Handles

HANDLE Windows ≠ fd Linux. Modelo próprio, documentado e testado.

## Encoding

`handle = (generation << 32) | index`, `index` múltiplo de 4, `index ≥ 4`
(como user-mode real). Geração 0 = inválido. `NULL=0`, `INVALID=u64::MAX`.

## Ciclo de vida

`insert` (reusa slot livre com geração+1) → `lookup` (checa geração; stale
→ `STATUS_INVALID_HANDLE`) → `close` (libera slot, objeto morre com o último
`Arc`). `Arc<KernelObject>` = refcount; fechar handle não destrói objeto com
referências abertas (semântica NT).

## Objetos (v0.2)

`File { fd, path, writable, owns_fd }` real (console: fd 1/2 com
`owns_fd=false` — nunca fechados; arquivos criados: `owns_fd=true`).
Payload tem UMA variante (`File`); `Event` real chega em v0.3.
`close_handle` fecha o fd possuído (antes vazava).

Invariante: handle reutilizado NUNCA ressuscita acesso antigo (teste
`stale_generation_rejected`). Auditoria: `nt-object` unit.
Namespaces (`\Device\` etc.): planejados v0.3 — struct especulativa removida
na auditoria 2026-09-10; voltar com testes quando houver consumidor.
