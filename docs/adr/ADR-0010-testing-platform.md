# ADR-0010 — Plataforma de testes: modelos executáveis + proptest, sem libfuzzer

- Status: aceito (2026-09-10).
- Context: validar contra milhões de cenários sem escrever milhões de testes;
  toolchain stable por reprodutibilidade; sem host Windows no ambiente.
- Decision:
  1. Nova crate `difftest` (cenário versionado + seed, normalização explícita,
     comparador com 8 vereditos, shrink ddmin, corpus, runner paralelo
     determinístico, CLI `rine-test`). Reutiliza `oracle` (formato+runner,
     estendido com timeout) e `pe::builder` (via novo `pe::driver`
     parametrizado) — nada duplicado.
  2. Diferencial em 2 velocidades: guest real (fileops, ~3k casos/s,
     fidelidade total) e in-process contra modelos (handles/memory/parser,
     90k–1M casos/s). Modelo = especificação executável mínima, não segunda
     implementação; divergência investiga-se dos dois lados.
  3. `proptest` (dev-dep) em vez de `quickcheck`: seeds determinísticas,
     shrinking integrado, manutenção ativa. Fuzz sem libfuzzer (exige
     nightly): mutação determinística própria + seeds fixas.
  4. `pe::driver` com expectativas embutidas no guest (exit 10+i no passo
     divergente); `MemoryManager`/`HandleTable` testados via API tipada.
- Alternatives: (a) quickcheck (shrinking mais fraco); (b) cargo-fuzz
  (quebra stable); (c) tudo via guest (lento demais p/ milhões);
  (d) tudo in-process (perde fidelidade ABI).
- Why: escala sem perder determinismo; cada camada testada no nível certo.
- Consequences: `proptest` só em dev-deps (MSRV de release intacto);
  `tests/regression/{files,memory,handles}/` como contrato permanente.
- Risks: modelo errado gera falso-positivo (mitigado: modelo mínimo +
  revisão + campanhas que já acharam 4 bugs reais nesta sessão).
- Supersedes: n/a (complementa ADR-0009 no eixo comportamental).
