# ADR-0003 — Sem servidor central; execução in-process (v0.1)

- Status: aceito (2026-09-10). Revisão prevista: v0.4 (isolamento).
- Context: onde roda o código Windows? Opções: (a) servidor central (estilo
  wineserver) mediando tudo via IPC; (b) in-process direto com primitivas
  Linux eficientes; (c) processo isolado por app.
- Decision: (b) agora, evoluindo para (c) em v0.4. Chamadas Win32/NT executam
  na thread do guest; coordenação global só onde a semântica exigir, via
  protocolo versionado — nunca IPC obrigatório por chamada simples.
- Alternatives: (a) foi rejeitado: latência por chamada e ponto único de
  contenção contradizem "Linux nativo embaixo".
- Why: compatibilidade primeiro, mas sem arquitetura que force IPC; futex/
  mmap são a velocidade do nativo.
- Consequences: singleton `ntdll::ProcessContext` temporário (assinaturas
  `win64` fixas); `ExitProcess` = `exit` do host; documentado como dívida.
- Risks: guest derruba host (aceito em v0.1, ambiente privado; v0.4 isola).
