# Formato de crash do Rine (`RINE-CRASH`, schema 1)

Todo crash (panic do runtime; sinais do guest viram SEH em v0.3) imprime em
stderr um bloco `RINE-CRASH` — nunca só `Segmentation fault`.

```text
RINE-CRASH schema=1
version: 0.2.0-alpha.1
executable: /caminho/foo.exe
phase: load|enter
reason: <mensagem do panic ou sinal>
location: <arquivo:linha se disponível>
```

Campos e regras:

- `timestamp`: segundos Unix (facilita correlação; sem dependências).
- `version`: `rine --version` (qual binário crashou importa).
- `executable`: argv do guest (qual PE estava rodando).
- `phase`: `load` (parse/map/imports) ou `enter` (código guest na CPU).
- `reason`/`location`: do panic hook; para sinais (v0.3+): RIP, signal,
  módulo, thread (lista completa na missão — evoluir este doc então).
- Saída normal (stdout do guest) nunca é poluída: crash vai para stderr.

Implementação: hook em `launcher::install_crash_hook(exe, phase)` +
`format_crash_report(...)` pura e testada. RIP/register snapshot e
stack unwind do guest: v0.3 (com SEH/signals).
