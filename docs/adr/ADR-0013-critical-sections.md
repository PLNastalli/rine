# ADR-0013 — Critical sections single-threaded (semáforo futuro)

- Status: aceito (2026-09-10).
- Context: MinGW CRT exige as 4 APIs, mas o runtime é single-threaded:
  contenção real é impossível hoje e futex/wait só faz sentido com threads.
- Decision:
  1. Layout `winabi::CriticalSection` exato (48 bytes, travado em abi.rs);
     lógica tipada em `nt-sync` (`initialize/enter/leave/delete_cs`),
     ponteiros crus só nas façades `kernel32` (padrão `WriteFile_impl`).
  2. `enter` em CS de outro dono retorna `UNSUCCESSFUL` em vez de travar
     (este `Err` é o futuro ponto de bloqueio futex — v0.3+).
  3. `leave` sem posse é ignorado com `tracing::warn` (Windows é indefinido;
     corromper silenciosamente seria pior; SEH futuro pode elevar exceção).
  4. `delete` zera o struct (nada alocado internamente ainda; uso pós-delete
     fica visível em vez de stale). `DebugInfo` nunca alocada (documentado).
  5. `OwningThread` = `nt_thread::current_tid()` (constante documentada
     até multithread real).
- Alternatives: (a) bloquear com spin infinito (travaria diagnóstico);
  (b) semáforo eventfd desde já (mecanismo sem consumidor nem threads —
  YAGNI); (c) stubs que retornam sucesso sem estado (mentirosos — proibidos).
- Why: semântica observável correta no mundo single-threaded, com os
  ganchos exatos onde threads/futex entrarão.
- Consequences: guest verifica campos da struct no `suite.exe` (saídas
  69–73); `kernelbase` ganha dep `nt-sync` + `tracing`.
- Risks: app que depende de bloqueio real entre threads ainda não roda
  (fora de escopo até threads; load não trava por isso).
