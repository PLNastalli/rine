# ADR-0002 — Handles geracionais, nunca fds

- Status: aceito (2026-09-10).
- Context: como representar `HANDLE`? Opções: (a) passar fd Linux direto;
  (b) tabela própria com IDs geracionais.
- Decision: (b). `HandleTable → (gen<<32)|idx`; `KernelObject` por `Arc`;
  stale → `STATUS_INVALID_HANDLE`.
- Alternatives: (a) é simples mas vaza abstração (close/dup semânticas
  diferentes, pseudo-handles, herança, `INVALID_HANDLE_VALUE` vs -1) e permite
  confusão fd/handle — classe inteira de bugs de segurança.
- Why: semântica Windows exata (lifetime, duplicação futura, tipos) e
  contenção de erro (stale detectado, não reutilizado silenciosamente).
- Consequences: toda I/O resolve handle→objeto→fd; overhead de HashMap/lock
  (irrelevante perto de syscall; granularidade fina em v0.4).
- Risks: lock global da tabela (contenção futura — sharding por milestone).
