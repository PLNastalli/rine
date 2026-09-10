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
            },
        }
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
    }

    #[test]
    fn last_error_roundtrip() {
        set_last_error(Win32Error::INVALID_HANDLE);
        assert_eq!(get_last_error(), Win32Error::INVALID_HANDLE);
        set_last_error(Win32Error::SUCCESS);
    }
}
