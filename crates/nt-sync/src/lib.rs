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

/// Inicializa uma critical section (livre, sem dono, semáforo nulo).
/// Espelha `RtlInitializeCriticalSection` sem a alocação de debug info
/// (nenhum consumidor interno lê `DebugInfo`; documentado, não esquecido).
pub fn initialize_cs(cs: &mut winabi::CriticalSection) {
    *cs = winabi::CriticalSection::unowned();
}

/// Adquire a CS para `tid`. Livre → toma posse; mesmo dono → recursão+1.
/// Dono diferente = contenção real: impossível single-threaded (v0.2);
/// retorna `UNSUCCESSFUL` em vez de travar para sempre — futex/wait chega
/// com threads (v0.3+), e este `Err` vira o ponto de bloqueio.
pub fn enter_cs(cs: &mut winabi::CriticalSection, tid: u32) -> NtStatus {
    if cs.owning_thread == 0 {
        cs.owning_thread = tid as u64;
        cs.recursion_count = 1;
        cs.lock_count = 0;
        NtStatus::SUCCESS
    } else if cs.owning_thread == tid as u64 {
        cs.recursion_count += 1;
        cs.lock_count += 1;
        NtStatus::SUCCESS
    } else {
        NtStatus::UNSUCCESSFUL
    }
}

/// Libera uma aquisição. Dono errado ou já livre = `INVALID_PARAMETER`
/// (Windows real é indefinido aqui — checked builds quebram; escolhemos
/// erro explícito em vez de corrupção silenciosa, documentado).
pub fn leave_cs(cs: &mut winabi::CriticalSection, tid: u32) -> NtStatus {
    if cs.owning_thread != tid as u64 || cs.recursion_count <= 0 {
        return NtStatus::INVALID_PARAMETER;
    }
    cs.recursion_count -= 1;
    cs.lock_count -= 1;
    if cs.recursion_count == 0 {
        cs.owning_thread = 0;
        cs.lock_count = -1;
    }
    NtStatus::SUCCESS
}

/// Deleta a CS. Nada alocado internamente em v0.2 (semáforo só existe sob
/// contenção; debug info nunca alocada) → zera o estado para que uso
/// pós-delete seja visível em vez de stale silencioso.
pub fn delete_cs(cs: &mut winabi::CriticalSection) {
    *cs = winabi::CriticalSection {
        debug_info: 0,
        lock_count: 0,
        _pad0: 0,
        recursion_count: 0,
        _pad1: 0,
        owning_thread: 0,
        lock_semaphore: 0,
        spin_count: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_lifecycle_recursion_and_release() {
        let mut cs = winabi::CriticalSection::unowned();
        assert_eq!(cs.lock_count, -1);
        assert_eq!(enter_cs(&mut cs, 4), NtStatus::SUCCESS);
        assert_eq!(
            (cs.owning_thread, cs.recursion_count, cs.lock_count),
            (4, 1, 0)
        );
        assert_eq!(enter_cs(&mut cs, 4), NtStatus::SUCCESS); // recursão
        assert_eq!((cs.recursion_count, cs.lock_count), (2, 1));
        assert_eq!(leave_cs(&mut cs, 4), NtStatus::SUCCESS);
        assert_eq!((cs.recursion_count, cs.lock_count), (1, 0));
        assert_eq!(leave_cs(&mut cs, 4), NtStatus::SUCCESS);
        assert_eq!(
            (cs.owning_thread, cs.recursion_count, cs.lock_count),
            (0, 0, -1)
        );
    }

    #[test]
    fn cs_wrong_owner_and_double_free_rejected() {
        let mut cs = winabi::CriticalSection::unowned();
        assert_eq!(leave_cs(&mut cs, 4), NtStatus::INVALID_PARAMETER); // livre
        assert_eq!(enter_cs(&mut cs, 4), NtStatus::SUCCESS);
        assert_eq!(leave_cs(&mut cs, 8), NtStatus::INVALID_PARAMETER); // outro dono
        assert_eq!(enter_cs(&mut cs, 8), NtStatus::UNSUCCESSFUL); // contenção
        delete_cs(&mut cs);
        assert_eq!(cs.owning_thread, 0); // sem stale
    }
}
