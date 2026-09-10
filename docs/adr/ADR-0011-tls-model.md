# ADR-0011 — Modelo TLS: slots no TEB, bitmap no contexto

- Status: aceito (2026-09-10).
- Context: `TlsAlloc/GetValue/SetValue/Free` exigem estado process-wide
  (índices) + per-thread (valores). Onde mora cada parte sem quebrar o
  singleton de processo nem a ownership do TEB?
- Decision:
  1. Valores em `TebMinimal.tls_slots: [u64; 64]` (fim do struct; offsets
     anteriores intactos, travados em `winabi/tests/abi.rs`).
  2. Índices em `nt_thread::TlsBitmap` (Mutex<u64>), dono: `ProcessContext`.
  3. Acesso via `nt_thread::teb_tls_slot(teb_ptr: u64, idx)` (`# Safety`);
     `teb_ptr` viaja no contexto como `u64` — mesmo padrão do `cmdline_ptr`
     (contexto global guarda dados, reinterpretação na façade dona).
  4. Free zera o slot (inobservável; anti-stale). Exaustão → LastError
     documentada como aproximação (Windows não especifica).
- Alternatives: (a) slots no contexto (confundiria thread/processo —
  bloquearia multithread futuro); (b) ler GS via syscall por chamada
  (correto mas syscall extra; o padrão `*_ptr` já resolve); (c) TEB real
  completo agora (offsets verdadeiros exigem ~1800 bytes mapeados; prematuro).
- Why: separa o que é por-thread do que é por-processo desde o dia 1;
  multithread futuro só troca a fonte do `teb_ptr` (GS por thread).
- Consequences: `ntdll`/`kernelbase`/`runtime` dependem de `nt-thread`
  (mapa atualizado); `kernelbase` ganha testes com contexto real instalado.
- Risks: TEB minimal ≠ offsets reais (guest que lê GS:[0x1480] direto
  falha — aceito até TEB completo; nenhuma API pública depende disso hoje).
