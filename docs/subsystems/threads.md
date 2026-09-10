# Threads

Thread inicial única (v0.1–v0.2). `TEB` real instalado via `ARCH_SET_GS`
(`tib.teb_self` = próprio endereço; `peb_ptr`, `image_base`). `LastError`
em TLS Rust (`nt-thread`) espelhando `TEB.LastErrorValue` (unificação via
GS em v0.3).

## TLS (v0.3)

- Valores: `TebMinimal.tls_slots: [u64; 64]` (no fim do TEB; offsets
  anteriores intactos — travado em `winabi/tests/abi.rs`).
- Índices: `nt_thread::TlsBitmap` no `ProcessContext` (process-wide, Mutex).
- Acesso: `nt_thread::teb_tls_slot(teb_ptr, idx)` (`# Safety` documentado);
  `teb_ptr` viaja no contexto como `u64` (mesmo padrão do `cmdline_ptr`).
- API: `TlsAlloc` (exaustão → `TLS_OUT_OF_INDEXES`), `TlsGetValue` (inválido
  → NULL, ambíguo como no Windows), `TlsSetValue`/`TlsFree` (FALSE +
  LastError no inválido); free zera o slot (inobservável, anti-stale).
- Threads extras + TLS de imagem + `__dyn_tls` continuam v0.3
  (`Clone` + TEB/GS por thread). Nenhuma decisão v0.1–v0.2 bloqueia:
  `ExceptionList` já modelado, GS por-thread já funciona.
