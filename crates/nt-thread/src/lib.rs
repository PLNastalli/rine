//! `nt-thread`: modelo de thread NT (TEB, TLS, LastError).
//!
//! v0.1: thread inicial única. TEB real instalável via GS (ver `runtime`).

use std::cell::Cell;
use winabi::{TebMinimal, Tib, Win32Error};

#[derive(Debug)]
pub struct Thread {
    pub tid_win: u32,
    pub teb: TebMinimal,
}

impl Thread {
    pub fn new(tid_win: u32, image_base: u64, peb_ptr: u64) -> Self {
        Self {
            tid_win,
            teb: TebMinimal {
                tib: Tib {
                    exception_list: 0,
                    stack_base: 0,
                    stack_limit: 0,
                    subsystem_tib: 0,
                    fiber_data: 0,
                    arbitrary_stack: 0,
                    teb_self: 0, // preenchido quando o TEB é instalado
                },
                last_error_value: 0,
                _pad: 0,
                peb_ptr,
                image_base,
                tls_slots: [0; winabi::TLS_SLOTS],
            },
        }
    }
}

/// Bitmap de índices TLS do processo (64 bits, um por slot).
/// Índices são process-wide; VALORES são por-thread (no TEB).
/// Dono: `ntdll::ProcessContext`; multithread usa o Mutex interno (v0.3+).
#[derive(Debug, Default)]
pub struct TlsBitmap {
    inner: std::sync::Mutex<u64>,
}

impl TlsBitmap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Aloca um índice livre, ou `None` se os 64 esgotados.
    pub fn alloc(&self) -> Option<u32> {
        let mut bits = self.inner.lock().unwrap();
        let free = (!*bits).trailing_zeros();
        if free >= 64 {
            return None;
        }
        *bits |= 1 << free;
        Some(free)
    }

    /// Libera um índice. `false` se fora de alcance ou já livre.
    pub fn free(&self, index: u32) -> bool {
        if index >= 64 {
            return false;
        }
        let mut bits = self.inner.lock().unwrap();
        let mask = 1u64 << index;
        if *bits & mask == 0 {
            return false;
        }
        *bits &= !mask;
        true
    }

    /// `true` se o índice está alocado (e no alcance).
    pub fn is_allocated(&self, index: u32) -> bool {
        if index >= 64 {
            return false;
        }
        *self.inner.lock().unwrap() & (1u64 << index) != 0
    }
}

/// Ponteiro para `teb.tls_slots[index]` a partir do endereço base do TEB.
///
/// # Safety
/// - `teb_base` deve apontar para um `TebMinimal` vivo e válido (o instalado
///   via GS pelo `runtime`, ou o `Box` do `Emulator` durante o load);
/// - `index < TLS_SLOTS` (checagem é do chamador — ver `kernelbase::tls_*`);
/// - válido só enquanto o TEB viver; acesso exclusivo à thread dona
///   (single-thread v0.2; GS por thread em v0.3+).
/// - Violação: leitura/escrita em memória arbitrária do host.
pub unsafe fn teb_tls_slot(teb_base: u64, index: u32) -> *mut u64 {
    debug_assert!((index as usize) < winabi::TLS_SLOTS);
    // SAFETY: garantido pelo chamador (ver acima); aritmética em BYTES a
    // partir da base (`*mut u8`), `tls_slots` é `[u64; 64]` alinhado dentro
    // de struct `#[repr(C)]`, então o endereço final é válido e alinhado.
    unsafe {
        ((teb_base as *mut u8).add(core::mem::offset_of!(TebMinimal, tls_slots)) as *mut u64)
            .add(index as usize)
    }
}

thread_local! {
    static LAST_ERROR: Cell<u32> = const { Cell::new(0) };
}

pub fn set_last_error(e: Win32Error) {
    LAST_ERROR.with(|c| c.set(e.0));
}

pub fn get_last_error() -> Win32Error {
    Win32Error(LAST_ERROR.with(|c| c.get()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_carries_teb_identity() {
        let t = Thread::new(0x2000, 0x0014_0000_0000, 0x5000);
        assert_eq!(t.tid_win, 0x2000);
        assert_eq!(t.teb.image_base, 0x0014_0000_0000);
        assert_eq!(t.teb.peb_ptr, 0x5000);
        assert_eq!(t.teb.last_error_value, 0);
        assert_eq!(t.teb.tls_slots, [0; winabi::TLS_SLOTS]);
    }

    #[test]
    fn teb_tls_slot_roundtrip() {
        let mut t = Thread::new(1, 2, 3);
        let base = &mut t.teb as *mut TebMinimal as u64;
        // SAFETY: `base` aponta para TEB vivo no escopo do teste; índices válidos.
        unsafe {
            *teb_tls_slot(base, 0) = 0xDEAD;
            *teb_tls_slot(base, 63) = 0xBEEF;
        }
        assert_eq!(t.teb.tls_slots[0], 0xDEAD);
        assert_eq!(t.teb.tls_slots[63], 0xBEEF);
        assert_eq!(t.teb.tls_slots[1], 0);
    }

    #[test]
    fn tls_bitmap_alloc_free_cycle() {
        let b = TlsBitmap::new();
        let i0 = b.alloc().unwrap();
        let i1 = b.alloc().unwrap();
        assert_ne!(i0, i1);
        assert!(b.is_allocated(i0));
        assert!(!b.free(99)); // fora de alcance
        assert!(b.free(i0));
        assert!(!b.is_allocated(i0));
        assert!(!b.free(i0)); // duplo free recusa
                              // Reuso do menor livre.
        assert_eq!(b.alloc().unwrap(), i0.min(i1));
    }

    #[test]
    fn tls_bitmap_exhausts_at_64() {
        let b = TlsBitmap::new();
        let mut got = Vec::new();
        for _ in 0..winabi::TLS_SLOTS {
            got.push(b.alloc().unwrap());
        }
        got.sort_unstable();
        got.dedup();
        assert_eq!(got.len(), winabi::TLS_SLOTS);
        assert!(b.alloc().is_none()); // 65º falha
        assert!(!b.is_allocated(64));
    }

    #[test]
    fn last_error_roundtrip() {
        set_last_error(Win32Error::INVALID_HANDLE);
        assert_eq!(get_last_error(), Win32Error::INVALID_HANDLE);
        set_last_error(Win32Error::SUCCESS);
    }
}
