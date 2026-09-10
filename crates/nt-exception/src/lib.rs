//! `nt-exception`: modelo de exceções NT / SEH.
//!
//! v0.1: tipos + registro de handler chain. Tradução signal->SEH
//! (SIGSEGV->ACCESS_VIOLATION etc.) no milestone v0.3; nenhuma decisão
//! v0.1 a impossibilita (TEB.ExceptionList já modelado, GS instalável).

use winabi::NtStatus;

pub mod code {
    pub const ACCESS_VIOLATION: u32 = 0xC0000005;
    pub const ILLEGAL_INSTRUCTION: u32 = 0xC000001D;
    pub const INT_DIVIDE_BY_ZERO: u32 = 0xC0000094;
    pub const STACK_OVERFLOW: u32 = 0xC00000FD;
}

#[derive(Debug, Clone)]
pub struct ExceptionRecord {
    pub code: u32,
    pub flags: u32,
    pub address: u64,
    pub parameters: Vec<u64>,
}

impl ExceptionRecord {
    pub fn access_violation(address: u64) -> Self {
        Self {
            code: code::ACCESS_VIOLATION,
            flags: 0,
            address,
            parameters: vec![0, address],
        }
    }

    pub fn status(&self) -> NtStatus {
        NtStatus(self.code)
    }
}
