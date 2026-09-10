//! `nt-sync`: primitivas de sincronização NT.
//!
//! v0.1: `Event` com `Mutex<bool>` (suficiente para single-thread).
//! Futex/eventfd/epoll quando a semântica exigir (ver ADR-0005).

use std::sync::Mutex;
use winabi::NtStatus;

#[derive(Debug)]
pub struct Event {
    pub manual_reset: bool,
    pub signaled: Mutex<bool>,
}

impl Event {
    pub fn new(manual_reset: bool, initial: bool) -> Self {
        Self {
            manual_reset,
            signaled: Mutex::new(initial),
        }
    }

    pub fn set(&self) {
        *self.signaled.lock().unwrap() = true;
    }

    pub fn reset(&self) {
        *self.signaled.lock().unwrap() = false;
    }

    pub fn is_signaled(&self) -> bool {
        *self.signaled.lock().unwrap()
    }
}

pub fn nt_set_event(e: &Event) -> NtStatus {
    e.set();
    NtStatus::SUCCESS
}
