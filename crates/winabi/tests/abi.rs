//! Suíte ABI: layouts, offsets, valores e convenção observáveis do Windows.
//!
//! Qualquer divergência aqui QUEBRA compatibilidade binária — estes testes
//! são o contrato. Referência: headers Windows + `windows-reference/` (oracle).

use std::mem::{align_of, offset_of, size_of};
use winabi::*;

#[test]
fn scalar_widths() {
    assert_eq!(size_of::<NtStatus>(), 4);
    assert_eq!(size_of::<Win32Error>(), 4);
    assert_eq!(size_of::<WindowsHandle>(), 8);
    assert_eq!(size_of::<VirtualAddress>(), 8);
    assert_eq!(size_of::<Rva>(), 4);
    assert_eq!(WindowsHandle::NULL.0, 0);
    assert_eq!(WindowsHandle::INVALID.0, u64::MAX);
}

#[test]
fn ntstatus_bits() {
    assert_eq!(NtStatus::SUCCESS.0, 0x00000000);
    assert_eq!(NtStatus::INVALID_HANDLE.0, 0xC0000008);
    assert!(NtStatus::INVALID_HANDLE.is_error());
    assert!(!NtStatus::SUCCESS.is_error());
    // Mapeamento mínimo honesto.
    assert_eq!(
        ntstatus_to_win32(NtStatus::INVALID_HANDLE),
        Win32Error::INVALID_HANDLE
    );
}

#[test]
fn memory_flags_match_windows() {
    assert_eq!(AllocType::COMMIT.bits(), 0x1000);
    assert_eq!(AllocType::RESERVE.bits(), 0x2000);
    assert_eq!(AllocType::RELEASE.bits(), 0x8000);
    assert_eq!(PageProtect::READONLY.bits(), 0x02);
    assert_eq!(PageProtect::READWRITE.bits(), 0x04);
    assert_eq!(PageProtect::EXECUTE_READ.bits(), 0x20);
    assert_eq!(PageProtect::EXECUTE_READWRITE.bits(), 0x40);
    assert_eq!(machine::AMD64, 0x8664);
    assert_eq!(subsystem::WINDOWS_CUI, 3);
}

#[test]
fn tib_teb_peb_layout() {
    // TIB: 7 ponteiros; offsets que o modelo GS assume.
    assert_eq!(size_of::<Tib>(), 56);
    assert_eq!(offset_of!(Tib, exception_list), 0);
    assert_eq!(offset_of!(Tib, teb_self), 48);
    // TEB mínimo: TIB + LastError + PEB + ImageBase + TLS (v0.3: slots no fim,
    // sem deslocar nenhum offset acima — invariante travado aqui).
    assert_eq!(offset_of!(TebMinimal, tib), 0);
    assert_eq!(offset_of!(TebMinimal, last_error_value), 56);
    assert_eq!(offset_of!(TebMinimal, peb_ptr), 64);
    assert_eq!(offset_of!(TebMinimal, image_base), 72);
    assert_eq!(offset_of!(TebMinimal, tls_slots), 80);
    assert_eq!(size_of::<TebMinimal>(), 80 + 64 * 8);
    assert_eq!(align_of::<TebMinimal>(), 8);
    assert_eq!(TLS_SLOTS, 64);
    assert_eq!(TLS_OUT_OF_INDEXES, 0xFFFF_FFFF);
    assert_eq!(INFINITE, 0xFFFF_FFFF);
    assert_eq!(offset_of!(PebMinimal, image_base), 0);
    // Convenção documentada da chamada x64.
    assert_eq!(x64call::SHADOW_SPACE, 32);
    assert_eq!(x64call::ARG_REGS, ["rcx", "rdx", "r8", "r9"]);
}

/// Chamada Rust→win64 com 6 args (2 na stack): prova que o host honra a
/// convenção nos dois sentidos (guest→host é provado pelos E2E).
extern "win64" fn probe6(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) -> u64 {
    a.wrapping_add(b)
        .wrapping_add(c)
        .wrapping_add(d)
        .wrapping_add(e)
        .wrapping_add(f)
}

#[test]
fn win64_call_both_directions() {
    assert_eq!(probe6(1, 2, 3, 4, 5, 6), 21);
    assert_eq!(probe6(u64::MAX, 1, 0, 0, 0, 0), 0); // wrap como no Windows
}
