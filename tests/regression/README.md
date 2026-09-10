# tests/regression — corpus permanente (um bug corrigido nunca volta)

Cada `*.json` = `{scenario, expect, history}`: caso minimizado que JÁ
falhou um dia + expectativa travada + história do bug. Replay:
- `files/*`: guest real (`launcher/tests/corpus_files.rs`).
- `memory/*`, `handles/*`: modelo×real (`difftest/tests/regression.rs`).
- CLI: `rine-test corpus [--dir]` roda tudo; `minimize --promote` adiciona
  (sempre de falha real; `history` obrigatório).

Casos atuais vêm de bugs reais desta sessão (ver campo `history` em cada
arquivo). `scenario_id` é display (replay usa target/seed/params).
