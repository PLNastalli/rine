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

pub mod modules;

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

/// `DWORD TlsAlloc()` — índice ou `TLS_OUT_OF_INDEXES`.
pub extern "win64" fn TlsAlloc_impl() -> u32 {
    match kernelbase::tls_alloc() {
        Ok(idx) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            idx
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            winabi::TLS_OUT_OF_INDEXES
        }
    }
}

/// `BOOL TlsFree(DWORD)` — 1 em sucesso, 0 + LastError caso contrário.
pub extern "win64" fn TlsFree_impl(dw_tls_index: u32) -> i32 {
    match kernelbase::tls_free(dw_tls_index) {
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

/// `LPVOID TlsGetValue(DWORD)` — valor ou NULL (inválido e 0 legítimo
/// são indistinguíveis, como no Windows; sem LastError novo aqui).
pub extern "win64" fn TlsGetValue_impl(dw_tls_index: u32) -> u64 {
    kernelbase::tls_get_value(dw_tls_index)
}

/// `BOOL TlsSetValue(DWORD, LPVOID)` — 1 em sucesso, 0 + LastError caso contrário.
pub extern "win64" fn TlsSetValue_impl(dw_tls_index: u32, lp_tls_value: u64) -> i32 {
    match kernelbase::tls_set_value(dw_tls_index, lp_tls_value) {
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

/// Endereços ntdll para o registro de módulos (`modules.rs`).
/// Vivem aqui porque `modules` é filho deste crate (regra órfã do Rust).
/// Inclui os alvos dos forwarders `kernel32→ntdll` (item 10).
pub(crate) fn ntdll_addr(name: &str) -> Option<u64> {
    match name {
        "RtlExitUserProcess" => Some(ntdll::RtlExitUserProcess_impl as *const () as u64),
        "NtTerminateProcess" => Some(ntdll::NtTerminateProcess_impl as *const () as u64),
        "RtlInitializeCriticalSection" => {
            Some(ntdll::RtlInitializeCriticalSection_impl as *const () as u64)
        }
        "RtlDeleteCriticalSection" => {
            Some(ntdll::RtlDeleteCriticalSection_impl as *const () as u64)
        }
        "RtlEnterCriticalSection" => Some(ntdll::RtlEnterCriticalSection_impl as *const () as u64),
        "RtlLeaveCriticalSection" => Some(ntdll::RtlLeaveCriticalSection_impl as *const () as u64),
        _ => None,
    }
}

/// `FARPROC GetProcAddress(HMODULE hModule, LPCSTR lpProcName)`.
///
/// - `HIWORD(lpProcName)==0` → ordinal (sem dereferência, como no Windows).
/// - Nome → resolução case-sensitive no módulo (pseudo-`HMODULE` opaco de
///   `winabi`; `LoadLibrary` real aposenta os tokens em v0.4).
/// - Falha → NULL + `ERROR_MOD_NOT_FOUND` (126) / `ERROR_PROC_NOT_FOUND`
///   (127); `lpProcName==NULL` → NULL + `INVALID_PARAMETER`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn GetProcAddress_impl(h_module: u64, lp_proc_name: u64) -> u64 {
    if lp_proc_name == 0 {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    }
    let (name, ordinal) = if lp_proc_name <= 0xFFFF {
        (None, Some(lp_proc_name as u16))
    } else {
        // SAFETY: como `WriteFile_impl` (string ANSI do guest, duração da chamada).
        match ansi_slice(lp_proc_name as *const u8) {
            Ok(bytes) => match core::str::from_utf8(bytes) {
                Ok(n) => (Some(n), None),
                Err(_) => {
                    nt_thread::set_last_error(winabi::Win32Error::PROC_NOT_FOUND);
                    return 0;
                }
            },
            Err(e) => {
                nt_thread::set_last_error(e);
                return 0;
            }
        }
    };
    match modules::get_proc_address(h_module, name, ordinal) {
        Ok(addr) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            addr
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `DWORD GetLastError()` — último erro da thread, sem efeitos colaterais.
pub extern "win64" fn GetLastError_impl() -> u32 {
    nt_thread::get_last_error().0
}

/// `VOID Sleep(DWORD)` — dorme `ms` (0 cede; `INFINITE` nunca volta).
pub extern "win64" fn Sleep_impl(dw_milliseconds: u32) {
    kernelbase::sleep_ms(dw_milliseconds);
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

/// Lê string UTF-16 NUL-terminada do guest (cap 16384 u16; sem NUL = inválida).
/// Espelho wide de `ansi_slice`: mesma fronteira, mesmo limite de leitura.
fn wide_slice(ptr: *const u16) -> Result<&'static [u16], winabi::Win32Error> {
    use winabi::Win32Error;
    if ptr.is_null() {
        return Err(Win32Error::INVALID_PARAMETER);
    }
    // SAFETY: memória do próprio processo emulado, válida por contrato Windows;
    // escaneia no máximo 16384 u16 (limite análogo ao ANSI; estouro vira
    // SIGSEGV contido — endurecido com SEH em v0.3, como em `ansi_slice`).
    let raw = unsafe { std::slice::from_raw_parts(ptr, 16384) };
    let len = raw
        .iter()
        .position(|&c| c == 0)
        .ok_or(Win32Error::INVALID_PARAMETER)?;
    Ok(&raw[..len])
}

/// `HMODULE GetModuleHandleA(LPCSTR lpModuleName)`.
///
/// - `NULL` = imagem do processo (base do contexto; nunca falha no Windows).
/// - Nome → normalização Windows (caminho/case/`.dll`); ApiSets ainda não
///   resolvem (item 9) → NULL + `MOD_NOT_FOUND`, nunca chute.
/// - Desconhecido → NULL + `MOD_NOT_FOUND` (126).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn GetModuleHandleA_impl(lp_module_name: *const u8) -> u64 {
    // SAFETY: como `WriteFile_impl` (string ANSI do guest, duração da chamada).
    let name = if lp_module_name.is_null() {
        None
    } else {
        match ansi_slice(lp_module_name) {
            Ok(bytes) => match core::str::from_utf8(bytes) {
                Ok(n) => Some(n),
                Err(_) => {
                    nt_thread::set_last_error(winabi::Win32Error::MOD_NOT_FOUND);
                    return 0;
                }
            },
            Err(e) => {
                nt_thread::set_last_error(e);
                return 0;
            }
        }
    };
    let exe_base = ntdll::require_context().image_base;
    match modules::get_module_handle(name, exe_base) {
        Ok(h) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            h
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `HMODULE GetModuleHandleW(LPCWSTR lpModuleName)` — variante Unicode.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn GetModuleHandleW_impl(lp_module_name: *const u16) -> u64 {
    // SAFETY: como `wide_slice` (string UTF-16 do guest, duração da chamada).
    let owned;
    let name = if lp_module_name.is_null() {
        None
    } else {
        match wide_slice(lp_module_name) {
            Ok(units) => match String::from_utf16(units) {
                Ok(n) => {
                    owned = n;
                    Some(owned.as_str())
                }
                Err(_) => {
                    nt_thread::set_last_error(winabi::Win32Error::MOD_NOT_FOUND);
                    return 0;
                }
            },
            Err(e) => {
                nt_thread::set_last_error(e);
                return 0;
            }
        }
    };
    let exe_base = ntdll::require_context().image_base;
    match modules::get_module_handle(name, exe_base) {
        Ok(h) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            h
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `HMODULE LoadLibraryA(LPCSTR lpLibFileName)` (v0.3: conjunto carregado).
///
/// Módulo conhecido → pseudo-handle (no Windows, carregar o já carregado só
/// incrementa a refcount). Imagem nova do disco = loader dinâmico (v0.4);
/// pedir uma agora → NULL + `MOD_NOT_FOUND`, nunca stub de sucesso.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn LoadLibraryA_impl(lp_lib_file_name: *const u8) -> u64 {
    // SAFETY: como `WriteFile_impl` (string ANSI do guest, duração da chamada).
    let name = match ansi_slice(lp_lib_file_name) {
        Ok(bytes) => match core::str::from_utf8(bytes) {
            Ok(n) => n,
            Err(_) => {
                nt_thread::set_last_error(winabi::Win32Error::MOD_NOT_FOUND);
                return 0;
            }
        },
        Err(e) => {
            nt_thread::set_last_error(e);
            return 0;
        }
    };
    match modules::load_library(name) {
        Ok(h) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            h
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `HMODULE LoadLibraryW(LPCWSTR lpLibFileName)` — variante Unicode.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn LoadLibraryW_impl(lp_lib_file_name: *const u16) -> u64 {
    // SAFETY: como `wide_slice` (string UTF-16 do guest, duração da chamada).
    let name = match wide_slice(lp_lib_file_name) {
        Ok(units) => match String::from_utf16(units) {
            Ok(n) => n,
            Err(_) => {
                nt_thread::set_last_error(winabi::Win32Error::MOD_NOT_FOUND);
                return 0;
            }
        },
        Err(e) => {
            nt_thread::set_last_error(e);
            return 0;
        }
    };
    match modules::load_library(&name) {
        Ok(h) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            h
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `BOOL FreeLibrary(HMODULE hLibModule)`.
///
/// Conjunto estático nunca descarrega (como DLL pinada): TRUE.
/// Desconhecido → FALSE + `MOD_NOT_FOUND`, nunca TRUE falso.
pub extern "win64" fn FreeLibrary_impl(h_lib_module: u64) -> i32 {
    let exe_base = ntdll::require_context().image_base;
    match modules::free_library(h_lib_module, exe_base) {
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

/// `HANDLE CreateFileW(LPCWSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE)`.
/// Idêntica à A, com path UTF-16 (7 args, 3 na stack).
#[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
pub extern "win64" fn CreateFileW_impl(
    lp_file_name: *const u16,
    dw_desired_access: u32,
    _dw_share_mode: u32, // ignorado v0.2 (documentado, como na A)
    _lp_security_attributes: *const u8,
    dw_creation_disposition: u32,
    _dw_flags_and_attributes: u32, // ignorado v0.2 (documentado, como na A)
    _h_template_file: u64,
) -> u64 {
    // SAFETY: como `WriteFile_impl` (string UTF-16 do guest, duração da chamada).
    let path = match wide_slice(lp_file_name) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return WindowsHandle::INVALID.0;
        }
    };
    match kernelbase::create_file_w(path, dw_desired_access, dw_creation_disposition) {
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

/// `DWORD GetFileAttributesW(LPCWSTR)` — attrs ou `INVALID_FILE_ATTRIBUTES`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn GetFileAttributesW_impl(lp_file_name: *const u16) -> u32 {
    // SAFETY: como `WriteFile_impl` (string UTF-16 do guest, duração da chamada).
    let path = match wide_slice(lp_file_name) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return winabi::INVALID_FILE_ATTRIBUTES;
        }
    };
    match kernelbase::get_file_attributes_w(path) {
        Ok(a) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            a
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            winabi::INVALID_FILE_ATTRIBUTES
        }
    }
}

/// `BOOL SetFilePointerEx(HANDLE, LARGE_INTEGER, PLARGE_INTEGER, DWORD)`.
///
/// `distance` (RDX, 64-bit por valor), `lp_new` opcional (NULL = não retorna
/// posição, como no Windows), `method` 0/1/2 = BEGIN/CURRENT/END.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn SetFilePointerEx_impl(
    h_file: u64,
    li_distance_to_move: i64,
    lp_new_file_pointer: *mut i64,
    dw_move_method: u32,
) -> i32 {
    match kernelbase::set_file_pointer_ex(
        WindowsHandle(h_file),
        li_distance_to_move,
        dw_move_method,
    ) {
        Ok(pos) => {
            if !lp_new_file_pointer.is_null() {
                // SAFETY: ponteiro de saída válido por contrato Windows.
                unsafe { *lp_new_file_pointer = pos as i64 };
            }
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `BOOL GetFileSizeEx(HANDLE, PLARGE_INTEGER)` — tamanho sem mover o cursor.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn GetFileSizeEx_impl(h_file: u64, lp_file_size: *mut i64) -> i32 {
    if lp_file_size.is_null() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    }
    match kernelbase::get_file_size_ex(WindowsHandle(h_file)) {
        Ok(size) => {
            // SAFETY: ponteiro de saída válido por contrato Windows.
            unsafe { *lp_file_size = size as i64 };
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// BOOL genérico para as W que devolvem só sucesso/erro sobre um path
/// (`DeleteFileW`, `CreateDirectoryW`, `RemoveDirectoryW`): lê a UTF-16,
/// delega, traduz o erro. Uma função em vez de três corpos idênticos.
fn bool_wide_path(lp_path: *const u16, op: fn(&[u16]) -> Result<(), winabi::Win32Error>) -> i32 {
    // SAFETY: como `WriteFile_impl` (string UTF-16 do guest, duração da chamada).
    let path = match wide_slice(lp_path) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return 0;
        }
    };
    match op(path) {
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

/// `BOOL DeleteFileW(LPCWSTR)`.
pub extern "win64" fn DeleteFileW_impl(lp_file_name: *const u16) -> i32 {
    bool_wide_path(lp_file_name, kernelbase::delete_file_w)
}

/// `BOOL CreateDirectoryW(LPCWSTR, LPSECURITY_ATTRIBUTES)` (attrs ignorados).
pub extern "win64" fn CreateDirectoryW_impl(
    lp_path_name: *const u16,
    _lp_security_attributes: *const u8,
) -> i32 {
    bool_wide_path(lp_path_name, kernelbase::create_directory_w)
}

/// `BOOL RemoveDirectoryW(LPCWSTR)`.
pub extern "win64" fn RemoveDirectoryW_impl(lp_path_name: *const u16) -> i32 {
    bool_wide_path(lp_path_name, kernelbase::remove_directory_w)
}

/// `BOOL MoveFileExW(LPCWSTR, LPCWSTR, DWORD)` — 3 args, sem out-pointer.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn MoveFileExW_impl(
    lp_existing_file_name: *const u16,
    lp_new_file_name: *const u16,
    dw_flags: u32,
) -> i32 {
    // SAFETY: como `WriteFile_impl` (strings UTF-16 do guest, duração da chamada).
    let from = match wide_slice(lp_existing_file_name) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return 0;
        }
    };
    let to = match wide_slice(lp_new_file_name) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return 0;
        }
    };
    match kernelbase::move_file_ex_w(from, to, dw_flags) {
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

/// `LPTOP_LEVEL_EXCEPTION_FILTER SetUnhandledExceptionFilter(filtro)`.
/// Retorna o filtro anterior (NULL na primeira chamada). Sem LastError novo:
/// a função nunca falha (qualquer ponteiro é "válido" até o dispatch).
pub extern "win64" fn SetUnhandledExceptionFilter_impl(lp_top_level_exception_filter: u64) -> u64 {
    kernelbase::set_unhandled_exception_filter(lp_top_level_exception_filter)
}

/// `SIZE_T VirtualQuery(LPCVOID, PMEMORY_BASIC_INFORMATION, SIZE_T)`.
///
/// Retorna bytes escritos (48) ou 0 em falha — nunca parcial: ou a struct
/// inteira cabe ou nada é escrito (o guest confia no retorno, não no buffer).
/// Buffer nulo/pequeno demais ou endereço não mapeado → 0 + LastError.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn VirtualQuery_impl(
    lp_address: u64,
    lp_buffer: *mut winabi::MemoryBasicInformation,
    dw_length: usize,
) -> usize {
    // SAFETY (fronteira ABI): buffer de 48 bytes válido por contrato Windows,
    // mesma address space, só durante a chamada; checado contra nulo e
    // tamanho ANTES de qualquer escrita (ver WriteFile_impl para o padrão).
    if lp_buffer.is_null() || dw_length < core::mem::size_of::<winabi::MemoryBasicInformation>() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    }
    match kernelbase::virtual_query(lp_address) {
        Ok(info) => {
            unsafe { *lp_buffer = info };
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            core::mem::size_of::<winabi::MemoryBasicInformation>()
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `void ExitProcess(UINT)` — nunca retorna.
pub extern "win64" fn ExitProcess_impl(code: u32) -> ! {
    kernelbase::exit_process(code)
}

// NOTA (item 10): `*CriticalSection` NÃO têm código aqui — como no Windows,
// são forwarders para `ntdll!Rtl*` (ver `modules::FORWARDERS`). O guest chama
// o endereço ntdll direto; a lógica única vive em `nt_sync` via ntdll.

/// Tabela autoritativa de exports implementados (fonte única: `resolve()`
/// e o relatório de cobertura `api-db/coverage.json` leem daqui).
/// Teste anti-drift abaixo garante que todo nome resolve.
pub const EXPORTS: &[&str] = &[
    "GetStdHandle",
    "WriteFile",
    "ReadFile",
    "CreateFileA",
    "CreateFileW",
    "GetFileAttributesW",
    "SetFilePointerEx",
    "GetFileSizeEx",
    "DeleteFileW",
    "MoveFileExW",
    "CreateDirectoryW",
    "RemoveDirectoryW",
    "CloseHandle",
    "VirtualAlloc",
    "VirtualFree",
    "VirtualProtect",
    "GetCommandLineW",
    "TlsAlloc",
    "TlsFree",
    "TlsGetValue",
    "TlsSetValue",
    "GetLastError",
    "GetProcAddress",
    "GetModuleHandleA",
    "GetModuleHandleW",
    "LoadLibraryA",
    "LoadLibraryW",
    "FreeLibrary",
    "Sleep",
    "InitializeCriticalSection",
    "DeleteCriticalSection",
    "EnterCriticalSection",
    "LeaveCriticalSection",
    "SetUnhandledExceptionFilter",
    "VirtualQuery",
    "ExitProcess",
];

/// Tabela de exports para o resolvedor de imports (`dll!nome -> endereço`).
/// Aceita o nome direto (`kernel32.dll`) OU qualquer ApiSet roteado para
/// ele (item 9) — fonte única de verdade via `modules::resolve_dll`.
/// MANTER SINCRONIZADO com `EXPORTS` (o teste pega lista→match; a direção
/// contrária é revisão — um braço sem entrada na lista é bug).
pub fn resolve(dll: &str, name: &str) -> Option<u64> {
    if modules::resolve_dll(dll) != Some(modules::KnownModule::Kernel32) {
        return None;
    }
    match name {
        "GetStdHandle" => Some(GetStdHandle_impl as *const () as u64),
        "WriteFile" => Some(WriteFile_impl as *const () as u64),
        "ReadFile" => Some(ReadFile_impl as *const () as u64),
        "CreateFileA" => Some(CreateFileA_impl as *const () as u64),
        "CreateFileW" => Some(CreateFileW_impl as *const () as u64),
        "GetFileAttributesW" => Some(GetFileAttributesW_impl as *const () as u64),
        "SetFilePointerEx" => Some(SetFilePointerEx_impl as *const () as u64),
        "GetFileSizeEx" => Some(GetFileSizeEx_impl as *const () as u64),
        "DeleteFileW" => Some(DeleteFileW_impl as *const () as u64),
        "MoveFileExW" => Some(MoveFileExW_impl as *const () as u64),
        "CreateDirectoryW" => Some(CreateDirectoryW_impl as *const () as u64),
        "RemoveDirectoryW" => Some(RemoveDirectoryW_impl as *const () as u64),
        "CloseHandle" => Some(CloseHandle_impl as *const () as u64),
        "VirtualAlloc" => Some(VirtualAlloc_impl as *const () as u64),
        "VirtualFree" => Some(VirtualFree_impl as *const () as u64),
        "VirtualProtect" => Some(VirtualProtect_impl as *const () as u64),
        "GetCommandLineW" => Some(GetCommandLineW_impl as *const () as u64),
        "TlsAlloc" => Some(TlsAlloc_impl as *const () as u64),
        "TlsFree" => Some(TlsFree_impl as *const () as u64),
        "TlsGetValue" => Some(TlsGetValue_impl as *const () as u64),
        "TlsSetValue" => Some(TlsSetValue_impl as *const () as u64),
        "GetLastError" => Some(GetLastError_impl as *const () as u64),
        "GetProcAddress" => Some(GetProcAddress_impl as *const () as u64),
        "GetModuleHandleA" => Some(GetModuleHandleA_impl as *const () as u64),
        "GetModuleHandleW" => Some(GetModuleHandleW_impl as *const () as u64),
        "LoadLibraryA" => Some(LoadLibraryA_impl as *const () as u64),
        "LoadLibraryW" => Some(LoadLibraryW_impl as *const () as u64),
        "FreeLibrary" => Some(FreeLibrary_impl as *const () as u64),
        "Sleep" => Some(Sleep_impl as *const () as u64),
        // Forwarders (item 10): sem código local — persegue até ntdll.
        // O braço existe para documentar a export; o endereço vem da tabela
        // `modules::FORWARDERS` (ordinais/símbolos do oracle Win11 25H2).
        "InitializeCriticalSection"
        | "DeleteCriticalSection"
        | "EnterCriticalSection"
        | "LeaveCriticalSection" => modules::forward(name),
        "SetUnhandledExceptionFilter" => Some(SetUnhandledExceptionFilter_impl as *const () as u64),
        "VirtualQuery" => Some(VirtualQuery_impl as *const () as u64),
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
