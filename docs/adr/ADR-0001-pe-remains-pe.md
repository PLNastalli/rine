# ADR-0001 — PE continua PE; código original na CPU

- Status: aceito (2026-09-10). Milestone: v0.1.
- Context: como executar `.exe` x86_64 no Linux? Opções: (a) converter para
  ELF; (b) emular CPU (QEMU); (c) mapear o PE e saltar para o entry original.
- Decision: (c). Parseamos, reservamos, copiamos seções, aplicamos relocs,
  patchamos IAT, protegemos, chamamos o entry. Conversão para ELF é proibida;
  emulação de CPU é proibida (princípio do projeto).
- Alternatives: (a) quebraria relocs/*.pdata* e auto-modificação; (b) 10–100x
  mais lento e desnecessário (mesmo ISA).
- Why: fidelidade máxima (o próprio código do vendor roda) + performance
  nativa; loader vira o componente crítico — exatamente onde o projeto investe.
- Consequences: precisamos de ABI `win64` no host, GS/TEB, IAT com endereços
  reais; bugs de loader são SIGSEGV (debug com gdb).
- Risks: código guest selvagem roda com privilégio do host (mitigação: v0.4
  isola por processo; v0.3 SEH contém falhas).
