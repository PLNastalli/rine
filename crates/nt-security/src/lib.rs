//! `nt-security`: tokens, SIDs, access checks.
//!
//! v0.1: stub tipado. Todo acesso é permitido (single-user local);
//! enforcement real no milestone v0.5. A API já retorna `NtStatus`
//! para que callers não precisem mudar.

use winabi::NtStatus;

#[derive(Debug, Clone)]
pub struct Token {
    pub user: String,
    pub elevated: bool,
}

impl Token {
    pub fn current() -> Self {
        Self {
            user: "rine-user".into(),
            elevated: false,
        }
    }
}

pub fn check_access(_token: &Token, _desired: u32) -> NtStatus {
    NtStatus::SUCCESS
}
