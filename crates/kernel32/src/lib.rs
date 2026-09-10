//! `kernel32`: fachada fina Win32.
//!
//! Regra: NENHUMA lógica aqui além de marshalling ABI -> `kernelbase`.
//! Cada export preserva nome, aridade e convenção Windows x64; o corpo
//! converte ponteiros brutos em slices tipados e delega.
//!
//! Nomes de export seguem a grafia Windows (`GetStdHandle`) —
//! `non_snake_case` permitido por compatibilidade de símbolo.
#![allow(non_snake_case)]

use winabi::WindowsHandle;

/// `HANDLE GetStdHandle(DWORD nStdHandle)` — `ECX -> RAX`.
/// Retorna `INVALID_HANDLE_VALUE` (-1) em erro (semântica Win32 exata).
pub extern "win64" fn GetStdHandle_impl(n_std_handle: i32) -> u64 {
    match kernelbase::get_std_handle(n_std_handle) {
        Ok(h) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            h.0
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            WindowsHandle::INVALID.0
        }
    }
}

/// `BOOL WriteFile(HANDLE, LPCVOID, DWORD, LPDWORD, LPOVERLAPPED)`.
/// Retorna 1 (TRUE) com `*written` preenchido, ou 0 (FALSE) + LastError.
///
/// A assinatura é congelada pela ABI Windows (não pode ser `unsafe fn` sem
/// quebrar o tipo do endereço na IAT); a dereferência é justificada em SAFETY.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn WriteFile_impl(
    h_file: u64,
    lp_buffer: *const u8,
    n_bytes: u32,
    lp_written: *mut u32,
    _overlapped: *const u8,
) -> i32 {
    // SAFETY (fronteira ABI):
    // - `lp_buffer`..`+n_bytes` e `lp_written` são válidos por contrato Windows:
    //   o caller (código x86_64 do PE, mesma address space) garante.
    // - quem garante: o PE compilado pelo Windows; violação (ponteiro selvagem)
    //   lê/escreve memória do próprio processo emulado — falha contida como
    //   STATUS_ACCESS_VIOLATION futuro (v0.3 instalará guard via SEH/signals).
    // - tempo de validade: duração da chamada.
    let buf = if lp_buffer.is_null() && n_bytes == 0 {
        &[][..]
    } else if lp_buffer.is_null() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    } else {
        unsafe { std::slice::from_raw_parts(lp_buffer, n_bytes as usize) }
    };
    match kernelbase::write_file(WindowsHandle(h_file), buf) {
        Ok(n) => {
            if !lp_written.is_null() {
                unsafe { *lp_written = n };
            }
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            if !lp_written.is_null() {
                unsafe { *lp_written = 0 };
            }
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// Lê string ANSI NUL-terminada do guest (cap 32768; sem NUL = inválida).
fn ansi_slice(ptr: *const u8) -> Result<&'static [u8], winabi::Win32Error> {
    use winabi::Win32Error;
    if ptr.is_null() {
        return Err(Win32Error::INVALID_PARAMETER);
    }
    // SAFETY: memória do próprio processo emulado, válida por contrato Windows;
    // escaneia no máximo 32768 bytes (limite do objeto mapeado mais próximo
    // estoura em SIGSEGV contido — endurecido com SEH em v0.3).
    let raw = unsafe { std::slice::from_raw_parts(ptr, 32768) };
    let len = raw
        .iter()
        .position(|&c| c == 0)
        .ok_or(Win32Error::INVALID_PARAMETER)?;
    Ok(&raw[..len])
}

/// `HANDLE CreateFileA(LPCSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE)`.
/// 7 args (3 na stack). Retorna handle ou `INVALID_HANDLE_VALUE`.
#[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
pub extern "win64" fn CreateFileA_impl(
    lp_file_name: *const u8,
    dw_desired_access: u32,
    _dw_share_mode: u32, // ignorado v0.2 (documentado)
    _lp_security_attributes: *const u8,
    dw_creation_disposition: u32,
    _dw_flags_and_attributes: u32, // ignorado v0.2 (documentado)
    _h_template_file: u64,
) -> u64 {
    // SAFETY: como `WriteFile_impl` (ponteiros do guest, duração da chamada).
    let path = match ansi_slice(lp_file_name) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return WindowsHandle::INVALID.0;
        }
    };
    match kernelbase::create_file_a(path, dw_desired_access, dw_creation_disposition) {
        Ok(h) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            h.0
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            WindowsHandle::INVALID.0
        }
    }
}

/// `BOOL ReadFile(HANDLE, LPVOID, DWORD, LPDWORD, LPOVERLAPPED)`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn ReadFile_impl(
    h_file: u64,
    lp_buffer: *mut u8,
    n_bytes: u32,
    lp_read: *mut u32,
    _overlapped: *const u8,
) -> i32 {
    // SAFETY: como `WriteFile_impl`; buffer é empréstimo exclusivo do guest.
    let buf = if lp_buffer.is_null() && n_bytes == 0 {
        &mut [][..]
    } else if lp_buffer.is_null() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    } else {
        unsafe { std::slice::from_raw_parts_mut(lp_buffer, n_bytes as usize) }
    };
    match kernelbase::read_file(WindowsHandle(h_file), buf) {
        Ok(n) => {
            if !lp_read.is_null() {
                unsafe { *lp_read = n };
            }
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            if !lp_read.is_null() {
                unsafe { *lp_read = 0 };
            }
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `BOOL CloseHandle(HANDLE)`.
pub extern "win64" fn CloseHandle_impl(h_object: u64) -> i32 {
    match kernelbase::close_handle(WindowsHandle(h_object)) {
        Ok(()) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `LPVOID VirtualAlloc(LPVOID, SIZE_T, DWORD, DWORD)` — NULL (0) em falha.
pub extern "win64" fn VirtualAlloc_impl(
    lp_address: u64,
    dw_size: usize,
    fl_allocation_type: u32,
    fl_protect: u32,
) -> u64 {
    use winabi::{AllocType, PageProtect};
    let alloc = match AllocType::from_bits(fl_allocation_type) {
        Some(a) => a,
        None => {
            nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
            return 0;
        }
    };
    let prot = match PageProtect::from_bits(fl_protect) {
        Some(p) => p,
        None => {
            nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
            return 0;
        }
    };
    match kernelbase::virtual_alloc(lp_address, dw_size, alloc, prot) {
        Ok(base) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            base
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `BOOL VirtualFree(LPVOID, SIZE_T, DWORD)`.
pub extern "win64" fn VirtualFree_impl(lp_address: u64, _dw_size: usize, dw_free_type: u32) -> i32 {
    use winabi::AllocType;
    let ft = match AllocType::from_bits(dw_free_type) {
        Some(a) => a,
        None => {
            nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
            return 0;
        }
    };
    match kernelbase::virtual_free(lp_address, ft) {
        Ok(()) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `BOOL VirtualProtect(LPVOID, SIZE_T, DWORD, PDWORD)`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn VirtualProtect_impl(
    lp_address: u64,
    _dw_size: usize,
    fl_new_protect: u32,
    lp_old_protect: *mut u32,
) -> i32 {
    use winabi::PageProtect;
    if lp_old_protect.is_null() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    }
    let prot = match PageProtect::from_bits(fl_new_protect) {
        Some(p) => p,
        None => {
            nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
            return 0;
        }
    };
    match kernelbase::virtual_protect(lp_address, prot) {
        Ok(old) => {
            // SAFETY: ponteiro de saída válido por contrato Windows.
            unsafe { *lp_old_protect = old.bits() };
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `LPWSTR GetCommandLineW(void)` — ponteiro para o UTF-16 do contexto.
/// Nunca falha em loads via `Emulator` (sempre preenchido).
pub extern "win64" fn GetCommandLineW_impl() -> *const u16 {
    let ctx = ntdll::require_context();
    // SAFETY: `u64`→ponteiro; o endereço é o buffer `_cmdline_w` do Emulator,
    // vivo durante todo o guest, somente-leitura, NUL-terminado.
    ctx.cmdline_ptr as *const u16
}

/// `void ExitProcess(UINT)` — nunca retorna.
pub extern "win64" fn ExitProcess_impl(code: u32) -> ! {
    kernelbase::exit_process(code)
}

/// Tabela autoritativa de exports implementados (fonte única: `resolve()`
/// e o relatório de cobertura `api-db/coverage.json` leem daqui).
/// Teste anti-drift abaixo garante que todo nome resolve.
pub const EXPORTS: &[&str] = &[
    "GetStdHandle",
    "WriteFile",
    "ReadFile",
    "CreateFileA",
    "CloseHandle",
    "VirtualAlloc",
    "VirtualFree",
    "VirtualProtect",
    "GetCommandLineW",
    "ExitProcess",
];

/// Tabela de exports para o resolvedor de imports (`dll!nome -> endereço`).
/// Nomes case-insensitive (Windows); o resolvedor normaliza.
/// MANTER SINCRONIZADO com `EXPORTS` (o teste pega lista→match; a direção
/// contrária é revisão — um braço sem entrada na lista é bug).
pub fn resolve(dll: &str, name: &str) -> Option<u64> {
    if !dll.eq_ignore_ascii_case("kernel32.dll") && !dll.eq_ignore_ascii_case("kernel32") {
        return None;
    }
    match name {
        "GetStdHandle" => Some(GetStdHandle_impl as *const () as u64),
        "WriteFile" => Some(WriteFile_impl as *const () as u64),
        "ReadFile" => Some(ReadFile_impl as *const () as u64),
        "CreateFileA" => Some(CreateFileA_impl as *const () as u64),
        "CloseHandle" => Some(CloseHandle_impl as *const () as u64),
        "VirtualAlloc" => Some(VirtualAlloc_impl as *const () as u64),
        "VirtualFree" => Some(VirtualFree_impl as *const () as u64),
        "VirtualProtect" => Some(VirtualProtect_impl as *const () as u64),
        "GetCommandLineW" => Some(GetCommandLineW_impl as *const () as u64),
        "ExitProcess" => Some(ExitProcess_impl as *const () as u64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anti-drift: todo nome em EXPORTS resolve para um endereço real.
    #[test]
    fn exports_all_resolve() {
        assert!(!EXPORTS.is_empty());
        for name in EXPORTS {
            assert!(
                resolve("KERNEL32.dll", name).is_some(),
                "{name} em EXPORTS mas sem braço em resolve()"
            );
            assert!(resolve("kernel32", name).is_some());
        }
        assert!(resolve("KERNEL32.dll", "Nope").is_none());
        assert!(resolve("NTDLL.dll", "GetStdHandle").is_none());
    }
}
