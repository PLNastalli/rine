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

/// `InitializeCriticalSection(cs)`: estado livre canônico.
/// `cs` já validado (não-nulo) na façade.
pub fn initialize_critical_section(cs: &mut winabi::CriticalSection) {
    nt_sync::initialize_cs(cs);
}

/// `DeleteCriticalSection(cs)`: libera recursos internos (nenhum em v0.2).
pub fn delete_critical_section(cs: &mut winabi::CriticalSection) {
    nt_sync::delete_cs(cs);
}

/// `EnterCriticalSection(cs)`: adquire para a thread corrente.
/// Contenção real é impossível single-threaded; se um dia ocorrer, o `Err`
/// interno vira futex/wait (v0.3+) — hoje seria bug interno, então registra.
pub fn enter_critical_section(cs: &mut winabi::CriticalSection) {
    let st = nt_sync::enter_cs(cs, nt_thread::current_tid());
    debug_assert!(
        st.is_success(),
        "contenção impossível single-threaded: {st}"
    );
}

/// `LeaveCriticalSection(cs)`: libera uma aquisição. Dono errado = bug do
/// caller (indefinido no Windows); aqui: ignora após registrar, em vez de
/// corromper estado silenciosamente. SEH futuro pode elevar a exceção.
pub fn leave_critical_section(cs: &mut winabi::CriticalSection) {
    if nt_sync::leave_cs(cs, nt_thread::current_tid()).is_error() {
        tracing::warn!("LeaveCriticalSection sem posse (bug do guest)");
    }
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
            tls_bitmap: nt_thread::TlsBitmap::new(),
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
}
