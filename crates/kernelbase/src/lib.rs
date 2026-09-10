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

/// `CreateFileA`: ANSI bytes → núcleo comum. Retorna handle ou erro.
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
    create_file_str(path, access, disposition)
}

/// `CreateFileW`: UTF-16 → núcleo comum. Surrogate solitário/inválido =
/// `INVALID_PARAMETER` (o `nt-file` opera sobre `str`; UTF-16 não-canônico
/// do Windows real é futuro documentado, nunca `lossy` silencioso).
pub fn create_file_w(
    path_wide: &[u16],
    access: u32,
    disposition: u32,
) -> Result<WindowsHandle, Win32Error> {
    let path = wide_to_string(cut_nul_w(path_wide)?)?;
    create_file_str(&path, access, disposition)
}
/// Núcleo comum A/W (opções + `NtCreateFile`; a diferença é só o encoding).
fn create_file_str(path: &str, access: u32, disposition: u32) -> Result<WindowsHandle, Win32Error> {
    let opts = nt_file::CreateOptions::from_win32(access, disposition).map_err(nt_to_win32)?;
    ntdll::nt_create_file(path, &opts).map_err(nt_to_win32)
}

/// `GetFileAttributesW(path) -> attrs` (`INVALID_FILE_ATTRIBUTES` na façade).
pub fn get_file_attributes_w(path_wide: &[u16]) -> Result<u32, Win32Error> {
    let path = wide_to_string(cut_nul_w(path_wide)?)?;
    ntdll::nt_file_attributes(&path).map_err(nt_to_win32)
}
/// `SetFilePointerEx(h, dist, method) -> nova posição`.
pub fn set_file_pointer_ex(
    handle: WindowsHandle,
    distance: i64,
    method: u32,
) -> Result<u64, Win32Error> {
    ntdll::nt_set_file_pointer(handle, distance, method).map_err(nt_to_win32)
}

/// `GetFileSizeEx(h) -> tamanho`.
pub fn get_file_size_ex(handle: WindowsHandle) -> Result<u64, Win32Error> {
    ntdll::nt_file_size(handle).map_err(nt_to_win32)
}

/// Converte UTF-16 do guest em `String` (NUL-terminado já cortado pelo
/// caller via `wide_slice`; aqui só a validação estrita).
fn wide_to_string(units: &[u16]) -> Result<String, Win32Error> {
    String::from_utf16(units).map_err(|_| Win32Error::INVALID_PARAMETER)
}

/// `DeleteFileW(path)`.
pub fn delete_file_w(path_wide: &[u16]) -> Result<(), Win32Error> {
    let path = wide_to_string(cut_nul_w(path_wide)?)?;
    let ctx = ntdll::require_context();
    nt_file::delete_file(&ctx.fsys, &path).map_err(nt_to_win32)
}

/// `MoveFileExW(from, to, flags)`: só `MOVEFILE_REPLACE_EXISTING`(1)
/// altera comportamento; demais flags documentadas como ignoradas v0.3
/// (WRITE_THROUGH sem efeito observável em rename local).
pub fn move_file_ex_w(from_wide: &[u16], to_wide: &[u16], flags: u32) -> Result<(), Win32Error> {
    let from = wide_to_string(cut_nul_w(from_wide)?)?;
    let to = wide_to_string(cut_nul_w(to_wide)?)?;
    let ctx = ntdll::require_context();
    nt_file::move_file(&ctx.fsys, &from, &to, flags & 1 != 0).map_err(nt_to_win32)
}

/// `CreateDirectoryW(path)`.
pub fn create_directory_w(path_wide: &[u16]) -> Result<(), Win32Error> {
    let path = wide_to_string(cut_nul_w(path_wide)?)?;
    let ctx = ntdll::require_context();
    nt_file::create_directory(&ctx.fsys, &path).map_err(nt_to_win32)
}

/// `RemoveDirectoryW(path)`.
pub fn remove_directory_w(path_wide: &[u16]) -> Result<(), Win32Error> {
    let path = wide_to_string(cut_nul_w(path_wide)?)?;
    let ctx = ntdll::require_context();
    nt_file::remove_directory(&ctx.fsys, &path).map_err(nt_to_win32)
}

/// Corta no primeiro NUL (limite 32768 já garantido por `wide_slice`).
fn cut_nul_w(path_wide: &[u16]) -> Result<&[u16], Win32Error> {
    if path_wide.len() > 32768 {
        return Err(Win32Error::INVALID_PARAMETER);
    }
    let nul = path_wide
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(path_wide.len());
    Ok(&path_wide[..nul])
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

/// `VirtualQuery(addr) -> MEMORY_BASIC_INFORMATION`.
///
/// Mapeamento de cada campo (travado por testes em `nt-memory` + E2E):
/// - `BaseAddress`/`RegionSize`: região que contém `addr` (0 = não mapeado);
/// - `AllocationBase`: base da reserva (= `BaseAddress`; sem subdivisão);
/// - `AllocationProtect`: proteção da reserva inicial;
/// - `State`: `MEM_COMMIT`/`MEM_RESERVE` (`MEM_FREE` nunca sai daqui —
///   não-mapeado retorna `Err`, e o guest vê 0);
/// - `Protect`: atual (`PAGE_NOACCESS` se só reservada — igual ao host);
/// - `Type`: `MEM_PRIVATE` (VirtualAlloc) / `MEM_IMAGE` (PE rastreada).
pub fn virtual_query(addr: u64) -> Result<winabi::MemoryBasicInformation, Win32Error> {
    use nt_memory::{RegionKind, RegionState};
    let info = ntdll::nt_query_virtual_memory(addr as usize).map_err(nt_to_win32)?;
    let state = match info.state {
        RegionState::Committed => winabi::mem_state::COMMIT,
        RegionState::Reserved => winabi::mem_state::RESERVE,
    };
    let protect = if info.state == RegionState::Committed {
        info.protect.bits()
    } else {
        // Reservada = PROT_NONE no host: NOACCESS observável.
        winabi::PageProtect::NOACCESS.bits()
    };
    let mem_type = match info.kind {
        RegionKind::Private => winabi::mem_type::PRIVATE,
        RegionKind::Image => winabi::mem_type::IMAGE,
    };
    Ok(winabi::MemoryBasicInformation {
        base_address: info.base as u64,
        allocation_base: info.base as u64,
        allocation_protect: info.alloc_protect.bits(),
        partition_id: 0,
        _pad0: 0,
        region_size: info.len as u64,
        state,
        protect,
        mem_type,
        _pad1: 0,
    })
}

/// `SetUnhandledExceptionFilter(filtro) -> filtro anterior` (0 = nenhum).
/// Troca atômica, sem falha possível: qualquer `u64` é aceito porque o valor
/// é opaco até o dispatch SEH (v0.3+) — validá-lo agora seria adivinhação.
pub fn set_unhandled_exception_filter(filter: u64) -> u64 {
    let ctx = ntdll::require_context();
    ctx.unhandled_filter
        .swap(filter, std::sync::atomic::Ordering::SeqCst)
}

/// `ExitProcess(code)`: nunca retorna.
pub fn exit_process(code: u32) -> ! {
    // Delega ao NT (ponto único de saída).
    ntdll::rtl_exit_user_process(code)
}

/// `TlsAlloc() -> índice`: aloca um slot TLS do processo.
/// Esgotados os 64, `Err(NOT_ENOUGH_MEMORY)` (Windows não documenta
/// LastError aqui; documentamos a aproximação em vez de inventar valor).
pub fn tls_alloc() -> Result<u32, Win32Error> {
    let ctx = ntdll::require_context();
    ctx.tls_bitmap.alloc().ok_or(Win32Error::NOT_ENOUGH_MEMORY)
}

/// `TlsGetValue(index) -> valor`: 0 para índice inválido/não-alocado
/// (ambíguo com valor 0 legítimo — semântica Windows exata, não nossa).
pub fn tls_get_value(index: u32) -> u64 {
    let Some(ctx) = ntdll::process_context() else {
        return 0;
    };
    if !ctx.tls_bitmap.is_allocated(index) {
        return 0;
    }
    // SAFETY: `teb_ptr` aponta para o TEB instalado e vivo (garantido pelo
    // `Emulator`, que o mantém no `Box` durante todo o guest); índice
    // validado acima (< 64); leitura u64 alinhada, só esta thread.
    unsafe { *nt_thread::teb_tls_slot(ctx.teb_ptr, index) }
}

/// `TlsSetValue(index, valor)`: erro se fora de alcance/não-alocado.
pub fn tls_set_value(index: u32, value: u64) -> Result<(), Win32Error> {
    let Some(ctx) = ntdll::process_context() else {
        return Err(Win32Error::INVALID_PARAMETER);
    };
    if !ctx.tls_bitmap.is_allocated(index) {
        return Err(Win32Error::INVALID_PARAMETER);
    }
    // SAFETY: como em `tls_get_value` (escrita em vez de leitura).
    unsafe {
        *nt_thread::teb_tls_slot(ctx.teb_ptr, index) = value;
    }
    Ok(())
}

/// `TlsFree(index)`: libera o índice e zera o slot (valor pós-free é
/// inobservável — o índice volta a falhar em Get/Set; zerar evita vazar
/// dado stale para o próximo dono do índice).
pub fn tls_free(index: u32) -> Result<(), Win32Error> {
    let Some(ctx) = ntdll::process_context() else {
        return Err(Win32Error::INVALID_PARAMETER);
    };
    if !ctx.tls_bitmap.free(index) {
        return Err(Win32Error::INVALID_PARAMETER);
    }
    // SAFETY: como em `tls_get_value`; índice era válido há uma instrução.
    unsafe {
        *nt_thread::teb_tls_slot(ctx.teb_ptr, index) = 0;
    }
    Ok(())
}

/// `Sleep(ms)`: 0 cede a vez (`yield`); `INFINITE` bloqueia sem limite
/// (fiel ao Windows — testes NUNCA usam INFINITE, travaria a suíte).
pub fn sleep_ms(ms: u32) {
    if ms == 0 {
        std::thread::yield_now();
    } else if ms == winabi::INFINITE {
        loop {
            std::thread::park();
        }
    } else {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Instala um contexto mínimo uma vez (global por binário de teste;
    /// idempotente e inofensivo aos demais testes, que não leem contexto).
    fn test_context() {
        use nt_object::HandleTable;
        use std::sync::Arc;
        use winabi::{PebMinimal, TebMinimal, Tib, WindowsHandle};
        // TEB no heap, vivo durante todo o teste (`Box` segurado aqui por
        // `Box::leak` proposital: o contexto global o referencia além do
        // escopo; vazamento de 592 bytes em teste, documentado).
        let teb: Box<TebMinimal> = Box::new(TebMinimal {
            tib: Tib {
                exception_list: 0,
                stack_base: 0,
                stack_limit: 0,
                subsystem_tib: 0,
                fiber_data: 0,
                arbitrary_stack: 0,
                teb_self: 0,
            },
            last_error_value: 0,
            _pad: 0,
            peb_ptr: 0,
            image_base: 0,
            tls_slots: [0; winabi::TLS_SLOTS],
        });
        let teb_ptr = Box::leak(teb) as *mut TebMinimal as u64;
        let _peb = Box::new(PebMinimal {
            image_base: 0,
            process_parameters: 0,
            loader_data: 0,
        });
        let table = Arc::new(HandleTable::new());
        ntdll::install_context(Arc::new(ntdll::ProcessContext {
            table,
            stdout_handle: WindowsHandle::NULL,
            stderr_handle: WindowsHandle::NULL,
            cmdline_ptr: 0,
            teb_ptr,
            mem: nt_memory::MemoryManager::new(),
            fsys: nt_file::DriveMap::new(std::collections::HashMap::new(), None),
            image_base: 0, // testes sem load: GetModuleHandle(NULL) não se aplica
            tls_bitmap: nt_thread::TlsBitmap::new(),
            unhandled_filter: std::sync::atomic::AtomicU64::new(0),
        }));
    }

    #[test]
    fn tls_lifecycle_through_real_path() {
        test_context();
        let idx = tls_alloc().expect("slot livre");
        assert_eq!(tls_get_value(idx), 0);
        tls_set_value(idx, 0x1122_3344_5566_7788).expect("set válido");
        assert_eq!(tls_get_value(idx), 0x1122_3344_5566_7788);
        // Inválidos: Get retorna 0, Set/Free recusam.
        assert_eq!(tls_get_value(999), 0);
        assert!(tls_set_value(999, 1).is_err());
        assert!(tls_free(999).is_err());
        tls_free(idx).expect("free válido");
        assert_eq!(tls_get_value(idx), 0); // desalocado → NULL de novo
        assert!(tls_set_value(idx, 1).is_err()); // set pós-free recusa
    }

    #[test]
    fn sleep_zero_returns() {
        sleep_ms(0);
        sleep_ms(1);
    }

    #[test]
    fn unhandled_filter_swap_roundtrip() {
        test_context();
        // Estado inicial: nenhum filtro (como no Windows).
        assert_eq!(set_unhandled_exception_filter(0x1234), 0);
        // Segunda troca devolve a anterior e instala a nova.
        assert_eq!(set_unhandled_exception_filter(0), 0x1234);
        // Voltou a nenhum.
        assert_eq!(set_unhandled_exception_filter(0), 0);
    }
}
