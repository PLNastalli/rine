# Threads

Thread inicial única em v0.1. `TEB` real instalado via `ARCH_SET_GS`
(`tib.teb_self` = próprio endereço; `peb_ptr`, `image_base`). `LastError`
em TLS Rust (`nt-thread`) espelhando `TEB.LastErrorValue` (unificação via
GS em v0.3). Threads extras + TLS de imagem + `__dyn_tls` em v0.3
(`Clone` + TEB por thread). Nenhuma decisão v0.1 bloqueia: `ExceptionList`
já modelado, GS por-thread já funciona.
