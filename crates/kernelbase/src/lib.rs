//! `kernelbase`: implementação Win32 interna (camada real).
//!
//! `kernel32` apenas encaminha; a lógica vive aqui, sobre `ntdll`.
//! Funções safe e tipadas; as exports `extern "win64"` estão em `kernel32`.

use winabi::{AllocType, NtStatus, PageProtect, Win32Error, WindowsHandle};

/// Erros Win32 extras usados aqui (`winerror.h`; tabela gerada em v0.3).
const ERROR_FILE_NOT_FOUND: Win32Error = Win32Error(2);
const ERROR_FILE_EXISTS: Win32Error = Win32Error(80);

fn nt_to_win32(s: NtStatus) -> Win32Error {
    match s {
        NtStatus::SUCCESS => Win32Error::SUCCESS,
        NtStatus::INVALID_HANDLE => Win32Error::INVALID_HANDLE,
        NtStatus::NO_MEMORY => Win32Error::NOT_ENOUGH_MEMORY,
        NtStatus::INVALID_PARAMETER => Win32Error::INVALID_PARAMETER,
        NtStatus::ACCESS_DENIED => Win32Error::ACCESS_DENIED,
        NtStatus::OBJECT_NAME_NOT_FOUND => ERROR_FILE_NOT_FOUND,
        NtStatus::OBJECT_NAME_COLLISION => ERROR_FILE_EXISTS,
        _ => Win32Error::NOT_SUPPORTED,
    }
}

/// `GetStdHandle(nStdHandle)`: resolve -10/-11/-12 via contexto ntdll.
pub fn get_std_handle(n_std_handle: i32) -> Result<WindowsHandle, Win32Error> {
    let ctx = ntdll::require_context();
    match n_std_handle {
        -10 => Err(Win32Error::FILE_NOT_FOUND), // stdin não modelado em v0.1
        -11 => Ok(ctx.stdout_handle),
        -12 => Ok(ctx.stderr_handle),
        _ => Err(Win32Error::INVALID_PARAMETER),
    }
}

/// `WriteFile(h, buf) -> bytes escritos`. Preenche `*written` no caller.
pub fn write_file(handle: WindowsHandle, buf: &[u8]) -> Result<u32, Win32Error> {
    ntdll::nt_write_file(handle, buf).map_err(nt_to_win32)
}

/// `ReadFile(h, buf) -> bytes lidos` (0 em EOF, como Win32).
pub fn read_file(handle: WindowsHandle, buf: &mut [u8]) -> Result<u32, Win32Error> {
    ntdll::nt_read_file(handle, buf).map_err(nt_to_win32)
}

/// `CreateFileA`: ANSI bytes → `NtCreateFile`. Retorna handle ou erro.
pub fn create_file_a(
    path_ansi: &[u8],
    access: u32,
    disposition: u32,
) -> Result<WindowsHandle, Win32Error> {
    if path_ansi.len() > 32767 {
        return Err(Win32Error::INVALID_PARAMETER);
    }
    let nul = path_ansi
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(path_ansi.len());
    let path = std::str::from_utf8(&path_ansi[..nul]).map_err(|_| Win32Error::INVALID_PARAMETER)?;
    let opts = nt_file::CreateOptions::from_win32(access, disposition).map_err(nt_to_win32)?;
    ntdll::nt_create_file(path, &opts).map_err(nt_to_win32)
}

/// `CloseHandle(h) -> TRUE/FALSE` (LastError é da façade).
pub fn close_handle(handle: WindowsHandle) -> Result<(), Win32Error> {
    match ntdll::nt_close(handle) {
        NtStatus::SUCCESS => Ok(()),
        s => Err(nt_to_win32(s)),
    }
}

/// `VirtualAlloc(addr, size, type, protect) -> base ou 0 (NULL)`.
pub fn virtual_alloc(
    hint: u64,
    size: usize,
    alloc: AllocType,
    protect: PageProtect,
) -> Result<u64, Win32Error> {
    ntdll::nt_allocate_virtual_memory(hint as usize, size, alloc, protect)
        .map(|b| b as u64)
        .map_err(nt_to_win32)
}

/// `VirtualFree(addr, size, type)`.
pub fn virtual_free(base: u64, free_type: AllocType) -> Result<(), Win32Error> {
    ntdll::nt_free_virtual_memory(base as usize, free_type).map_err(nt_to_win32)
}

/// `VirtualProtect(addr, size, new) -> proteção anterior`.
/// `size` v0.2: região integral (parcial em v0.3 com `VirtualQuery`).
pub fn virtual_protect(base: u64, protect: PageProtect) -> Result<PageProtect, Win32Error> {
    ntdll::nt_protect_virtual_memory(base as usize, protect).map_err(nt_to_win32)
}

/// `ExitProcess(code)`: nunca retorna.
pub fn exit_process(code: u32) -> ! {
    // Delega ao NT (ponto único de saída).
    ntdll::rtl_exit_user_process(code)
}
