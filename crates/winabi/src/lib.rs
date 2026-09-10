//! `winabi`: Windows ABI primitives.
//!
//! Responsabilidade: definir exatamente os tipos, layouts, constantes e
//! convenções observáveis do Windows x86_64. Nada aqui executa lógica;
//! é o vocabulário que todas as outras crates compartilham.
//!
//! Ver também: `docs/subsystems/pe-loader.md`, `docs/adr/ADR-0004-windows-abi-boundary.md`.

use bitflags::bitflags;

/// NTSTATUS: código de status de 32 bits.
///
/// Layout: `severity(2) | customer(1) | reserved(1) | facility(12) | code(16)`.
/// `0x00000000` == `STATUS_SUCCESS`. Valores com bit 31 set são falha.
/// Preservar o valor exato é requisito de compatibilidade (oracle = Windows real).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NtStatus(pub u32);

impl NtStatus {
    pub const SUCCESS: Self = Self(0x00000000);
    pub const WAIT_0: Self = Self(0x00000000);
    pub const UNSUCCESSFUL: Self = Self(0xC0000001);
    pub const INVALID_HANDLE: Self = Self(0xC0000008);
    pub const INVALID_PARAMETER: Self = Self(0xC000000D);
    pub const NO_MEMORY: Self = Self(0xC0000017);
    pub const ILLEGAL_INSTRUCTION: Self = Self(0xC000001D);
    pub const NOT_IMPLEMENTED: Self = Self(0xC0000002);
    pub const ACCESS_DENIED: Self = Self(0xC0000022);
    pub const OBJECT_NAME_NOT_FOUND: Self = Self(0xC0000034);
    pub const OBJECT_NAME_COLLISION: Self = Self(0xC0000035);
    pub const OBJECT_PATH_INVALID: Self = Self(0xC0000039);
    pub const END_OF_FILE: Self = Self(0xC0000011);
    pub const IMAGE_NOT_AT_BASE: Self = Self(0x40000003);

    /// `true` se `severity >= ERROR` (bit 31).
    #[inline]
    pub fn is_error(self) -> bool {
        (self.0 & 0x8000_0000) != 0
    }

    #[inline]
    pub fn is_success(self) -> bool {
        self.0 == 0
    }
}

impl From<NtStatus> for u32 {
    fn from(s: NtStatus) -> u32 {
        s.0
    }
}

impl core::fmt::Display for NtStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NTSTATUS(0x{:08X})", self.0)
    }
}

/// Resultado interno tipado. `Err` carrega sempre um `NtStatus` real,
/// nunca uma string genérica. Conversão para Win32 é explícita.
pub type NtResult<T> = Result<T, NtStatus>;

/// Erro Win32 (`GetLastError`). Domínio separado de `NtStatus`.
/// Conversão NTSTATUS -> Win32 via tabela (ver `ntstatus_to_win32`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Win32Error(pub u32);

impl Win32Error {
    pub const SUCCESS: Self = Self(0);
    pub const INVALID_HANDLE: Self = Self(6);
    pub const NOT_ENOUGH_MEMORY: Self = Self(8);
    pub const INVALID_PARAMETER: Self = Self(87);
    pub const ACCESS_DENIED: Self = Self(5);
    pub const NOT_SUPPORTED: Self = Self(50);
    pub const FILE_NOT_FOUND: Self = Self(2);
}

/// Mapeamento mínimo NTSTATUS -> Win32 para v0.1.
/// Tabela completa será gerada de `winerror.h` (ver milestone v0.3).
pub fn ntstatus_to_win32(status: NtStatus) -> Win32Error {
    match status {
        NtStatus::SUCCESS => Win32Error::SUCCESS,
        NtStatus::INVALID_HANDLE => Win32Error::INVALID_HANDLE,
        NtStatus::NO_MEMORY => Win32Error::NOT_ENOUGH_MEMORY,
        NtStatus::INVALID_PARAMETER => Win32Error::INVALID_PARAMETER,
        NtStatus::ACCESS_DENIED => Win32Error::ACCESS_DENIED,
        _ => Win32Error::NOT_SUPPORTED,
    }
}

/// HANDLE Windows opaco (64 bits no kernel moderno; valores user-mode).
///
/// NÃO é um fd Linux. É um índice geracional na `HandleTable`
/// (ver `nt-object` e `docs/subsystems/handles.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WindowsHandle(pub u64);

impl WindowsHandle {
    pub const NULL: Self = Self(0);
    pub const INVALID: Self = Self(u64::MAX);
    /// Pseudo-handles de console, resolvidos via `GetStdHandle`.
    pub const STD_INPUT: i32 = -10;
    pub const STD_OUTPUT: i32 = -11;
    pub const STD_ERROR: i32 = -12;
}

/// Endereço virtual absoluto (u64) e RVA (u32, relativo à ImageBase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtualAddress(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Rva(pub u32);

impl Rva {
    #[inline]
    pub fn to_va(self, image_base: u64) -> VirtualAddress {
        VirtualAddress(image_base.wrapping_add(self.0 as u64))
    }
}

/// `RTL_CRITICAL_SECTION` x86_64 (48 bytes).
///
/// Layout exato do Windows: DebugInfo, LockCount, RecursionCount,
/// OwningThread, LockSemaphore, SpinCount. Guest aloca (stack/.data) e
/// passa o ponteiro; o runtime lê/escreve estes campos — qualquer divergência
/// de offset corrompe memória do guest silenciosamente (travado em abi.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct CriticalSection {
    pub debug_info: u64,
    pub lock_count: i32,
    pub _pad0: u32,
    pub recursion_count: i32,
    pub _pad1: u32,
    pub owning_thread: u64,
    pub lock_semaphore: u64,
    pub spin_count: u64,
}

impl CriticalSection {
    /// Estado pós-`InitializeCriticalSection` (livre, sem dono).
    pub const fn unowned() -> Self {
        Self {
            debug_info: 0,
            lock_count: -1,
            _pad0: 0,
            recursion_count: 0,
            _pad1: 0,
            owning_thread: 0,
            lock_semaphore: 0,
            spin_count: 0,
        }
    }
}

bitflags! {
    /// Flags `AllocationType` de `VirtualAlloc` / `NtAllocateVirtualMemory`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AllocType: u32 {
        const COMMIT  = 0x00001000;
        const RESERVE = 0x00002000;
        const DECOMMIT = 0x00004000;
        const RELEASE = 0x00008000;
        const RESET   = 0x00080000;
        const TOP_DOWN = 0x00100000;
        const LARGE_PAGES = 0x20000000;
    }

    /// Flags `Protect` (`PAGE_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageProtect: u32 {
        const NOACCESS = 0x01;
        const READONLY = 0x02;
        const READWRITE = 0x04;
        const WRITECOPY = 0x08;
        const EXECUTE = 0x10;
        const EXECUTE_READ = 0x20;
        const EXECUTE_READWRITE = 0x40;
        const EXECUTE_WRITECOPY = 0x80;
        const GUARD = 0x100;
        const NOCACHE = 0x200;
        const WRITECOMBINE = 0x400;
    }

    /// Características de seção PE (`IMAGE_SCN_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SectionCharacteristics: u32 {
        const CNT_CODE = 0x00000020;
        const CNT_INITIALIZED_DATA = 0x00000040;
        const CNT_UNINITIALIZED_DATA = 0x00000080;
        const MEM_EXECUTE = 0x20000000;
        const MEM_READ = 0x40000000;
        const MEM_WRITE = 0x80000000;
    }
}

/// Machine types (`IMAGE_FILE_MACHINE_*`).
pub mod machine {
    pub const UNKNOWN: u16 = 0x0;
    pub const AMD64: u16 = 0x8664;
    pub const I386: u16 = 0x14c;
    pub const ARM64: u16 = 0xaa64;
}

/// Subsystem (`IMAGE_SUBSYSTEM_*`).
pub mod subsystem {
    pub const UNKNOWN: u16 = 0;
    pub const NATIVE: u16 = 1;
    pub const WINDOWS_GUI: u16 = 2;
    pub const WINDOWS_CUI: u16 = 3;
}

/// DLL characteristics (`IMAGE_DLLCHARACTERISTICS_*`).
pub mod dllchar {
    pub const DYNAMIC_BASE: u16 = 0x0040;
    pub const NX_COMPAT: u16 = 0x0100;
}

/// Relocation types AMD64 (`IMAGE_REL_AMD64_*`).
pub mod reloc {
    pub const ABSOLUTE: u16 = 0;
    pub const ADDR64: u16 = 1;
    pub const ADDR32: u16 = 2;
    pub const ADDR32NB: u16 = 3;
    pub const REL32: u16 = 4;
}

/// Convenção de chamada Windows x64 (para referência e asserts):
///
/// ```text
/// args: RCX, RDX, R8, R9, depois [RSP+0x20], [RSP+0x28], ...
/// retorno: RAX
/// caller aloca 32 bytes de shadow space; RSP 16-alinhado antes de CALL.
/// callee preserva RBX, RBP, RDI, RSI, RSP, R12-R15.
/// ```
pub mod x64call {
    /// Tamanho do shadow space que o caller deve reservar.
    pub const SHADOW_SPACE: usize = 32;
    /// Nomes dos registradores de argumentos, em ordem.
    pub const ARG_REGS: [&str; 4] = ["rcx", "rdx", "r8", "r9"];
}

/// TEB mínimo observável (layout compatível com Windows).
///
/// O Windows real tem ~0x1800+ bytes; aqui modelamos o prefixo que
/// programas v0.1 podem inspecionar: TIB + LastError + PEB ptr + ImageBase.
/// O TEB real é instalado via `arch_prctl(ARCH_SET_GS, teb)`.
///
/// ```text
/// GS:[0x00] = TIB.ExceptionList ... GS:[0x30] = TEB self ptr
/// GS:[0x34] = LastErrorValue      GS:[0x60] = PEB ptr
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Tib {
    pub exception_list: u64,
    pub stack_base: u64,
    pub stack_limit: u64,
    pub subsystem_tib: u64,
    pub fiber_data: u64,
    pub arbitrary_stack: u64,
    pub teb_self: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TebMinimal {
    pub tib: Tib,
    /// Offset 0x34 no TEB real (u32). Mantemos u64 aqui por alinhamento
    /// simplificado; o launcher escreve no offset byte-exato 0x34.
    pub last_error_value: u32,
    pub _pad: u32,
    pub peb_ptr: u64,
    pub image_base: u64,
    /// Slots TLS da thread (`TlsGetValue/SetValue`), 64 valores.
    /// Ficam no fim para não deslocar nenhum offset documentado acima.
    /// TEB real tem 64 slots + expansão; expansão chega com multithread (v0.3).
    pub tls_slots: [u64; 64],
}

/// Número de slots TLS por thread (Windows: 64 + 1024 de expansão).
pub const TLS_SLOTS: usize = 64;

/// Retorno de `TlsAlloc` em falha (sem slot livre).
pub const TLS_OUT_OF_INDEXES: u32 = 0xFFFF_FFFF;

/// `Sleep(INFINITE)`: dorme sem limite (nunca acorda sozinho).
pub const INFINITE: u32 = 0xFFFF_FFFF;

/// PEB mínimo observável.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PebMinimal {
    pub image_base: u64,
    pub process_parameters: u64,
    pub loader_data: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntstatus_severity() {
        assert!(NtStatus::SUCCESS.is_success());
        assert!(NtStatus::INVALID_HANDLE.is_error());
    }

    #[test]
    fn teb_offsets_documented() {
        // Documenta os offsets que o assembly/GS depende.
        // TIB tem 7*u64 = 56 bytes; LastError fica logo depois.
        assert_eq!(core::mem::size_of::<Tib>(), 56);
    }
}
