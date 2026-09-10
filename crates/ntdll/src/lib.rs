//! `ntdll`: fachada NT sobre os subsistemas internos.
//!
//! Camada: `Nt*` / `Rtl*` com semântica NT exata por fora,
//! chamadas tipadas (`NtResult`, `Arc<KernelObject>`) por dentro.
//! Engelobam `nt-object`, `nt-memory`, `nt-file`, etc. — sem lógica de
//! negócio Win32 (isso é `kernelbase`).
//!
//! Nomes de export seguem a grafia NT (`RtlExitUserProcess`).
#![allow(non_snake_case)]

use nt_object::HandleTable;
use std::sync::{Arc, Mutex, OnceLock};
use winabi::{NtStatus, WindowsHandle};

/// Contexto do processo emulado (singleton v0.1).
///
/// Limitação documentada (ADR-0003): hoje um processo emulado por host.
/// Futuro: contexto por-TEB via GS, sem global. O global existe porque as
/// exports `extern "win64"` têm assinatura fixa do Windows e não podem
/// receber `&self`; o Windows resolve o equivalente via GS:[0x60] (PEB).
pub struct ProcessContext {
    pub table: Arc<HandleTable>,
    pub stdout_handle: WindowsHandle,
    pub stderr_handle: WindowsHandle,
    /// Linha de comando UTF-16 NUL-terminada (`GetCommandLineW`), como
    /// endereço `u64` (não ponteiro: o contexto é global/`Sync`, então guarda
    /// dados; a reinterpretação unsafe vive só na façade que o consome).
    /// Aponta para `Emulator::_cmdline_w`, vivo durante todo o guest.
    /// Leitura somente; endereço estável (Vec nunca realocado após o load).
    /// 0 = ausente (nunca em loads via `Emulator`, que sempre preenchem).
    pub cmdline_ptr: u64,
    /// Base do TEB da thread inicial, como endereço `u64` (mesmo padrão do
    /// `cmdline_ptr`: o contexto global guarda dados; a reinterpretação
    /// unsafe vive em `nt_thread::teb_tls_slot`, dona do conceito TEB).
    /// Aponta para o `Box<TebMinimal>` do `Emulator`, vivo durante o guest.
    pub teb_ptr: u64,
    /// Bitmap de índices TLS do processo (valores ficam no TEB de cada thread).
    pub tls_bitmap: nt_thread::TlsBitmap,
    /// Filtro de exceção não-tratada do processo (`SetUnhandledExceptionFilter`).
    /// Ponteiro opaco do guest (nunca dereferenciado sem SEH/dispatch, v0.3+);
    /// `AtomicU64` porque é um único valor trocado por swap (sem lock).
    /// 0 = nenhum (estado inicial do Windows).
    pub unhandled_filter: std::sync::atomic::AtomicU64,
    /// Gerente de memória virtual do processo (`VirtualAlloc` e v0.2+).
    pub mem: nt_memory::MemoryManager,
    /// Filesystem resolvido da Capsule (tradução de paths).
    pub fsys: nt_file::DriveMap,
}

static CONTEXT: OnceLock<Mutex<Option<Arc<ProcessContext>>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<Arc<ProcessContext>>> {
    CONTEXT.get_or_init(|| Mutex::new(None))
}

/// Instala o contexto antes de executar código Windows. Chamado uma vez
/// por `runtime::Emulator::load`.
pub fn install_context(ctx: Arc<ProcessContext>) {
    *slot().lock().unwrap() = Some(ctx);
}

pub fn process_context() -> Option<Arc<ProcessContext>> {
    slot().lock().unwrap().clone()
}

pub fn require_context() -> Arc<ProcessContext> {
    process_context().expect("ntdll: process context not installed (bug no runtime)")
}

/// `NtWriteFile` interno: escreve no objeto `File` do handle.
pub fn nt_write_file(handle: WindowsHandle, buf: &[u8]) -> Result<u32, NtStatus> {
    let ctx = require_context();
    nt_file::write_file(&ctx.table, handle, buf)
}

/// `NtReadFile` interno: lê até `buf.len()` bytes.
pub fn nt_read_file(handle: WindowsHandle, buf: &mut [u8]) -> Result<u32, NtStatus> {
    let ctx = require_context();
    nt_file::read_file(&ctx.table, handle, buf)
}

/// `NtCreateFile` interno (simplificado v0.2): cria/abre pelo path Win32.
pub fn nt_create_file(
    win_path: &str,
    opts: &nt_file::CreateOptions,
) -> Result<WindowsHandle, NtStatus> {
    let ctx = require_context();
    nt_file::create_file(&ctx.table, &ctx.fsys, win_path, opts)
}

/// `NtClose` interno (fecha handle + fd possuído).
pub fn nt_close(handle: WindowsHandle) -> NtStatus {
    let ctx = match process_context() {
        Some(c) => c,
        None => return NtStatus::INVALID_HANDLE,
    };
    match nt_file::close_handle(&ctx.table, handle) {
        Ok(()) => NtStatus::SUCCESS,
        Err(s) => s,
    }
}

/// `NtAllocateVirtualMemory` interno (região do processo emulado).
pub fn nt_allocate_virtual_memory(
    hint: usize,
    size: usize,
    alloc: winabi::AllocType,
    protect: winabi::PageProtect,
) -> Result<usize, NtStatus> {
    let ctx = require_context();
    ctx.mem
        .allocate(hint, size, alloc, protect)
        .map_err(NtStatus::from)
}

/// `NtFreeVirtualMemory` interno.
pub fn nt_free_virtual_memory(base: usize, free_type: winabi::AllocType) -> Result<(), NtStatus> {
    let ctx = require_context();
    ctx.mem.free(base, free_type).map_err(NtStatus::from)
}

/// `NtProtectVirtualMemory` interno. Retorna a proteção anterior.
pub fn nt_protect_virtual_memory(
    base: usize,
    protect: winabi::PageProtect,
) -> Result<winabi::PageProtect, NtStatus> {
    let ctx = require_context();
    ctx.mem
        .protect_region(base, protect)
        .map_err(NtStatus::from)
}

/// `RtlExitUserProcess(code)`: registra o exit code e encerra o host.
/// Ponto único de saída — ver `runtime` (restaura GS antes, em `enter`).
pub fn rtl_exit_user_process(code: u32) -> ! {
    // Registra para diagnóstico; o processo host termina com o mesmo código
    // (modelo single-process v0.1, documentado em `docs/architecture.md`).
    tracing::info!(code, "RtlExitUserProcess");
    std::process::exit(code as i32)
}

/// Export com ABI Windows x64 para a IAT (`ntdll!RtlExitUserProcess`).
/// Assinatura real: `void RtlExitUserProcess(UINT code)`.
pub extern "win64" fn RtlExitUserProcess_impl(code: u32) -> ! {
    rtl_exit_user_process(code)
}

/// Tabela autoritativa de exports implementados (fonte única: resolvedor
/// em `runtime` + cobertura `api-db/coverage.json` leem daqui).
pub const EXPORTS: &[&str] = &["RtlExitUserProcess", "NtTerminateProcess"];

/// `NtTerminateProcess(handle, code)`: v0.1 só suporta processo atual.
pub fn nt_terminate_process(handle: WindowsHandle, code: u32) -> NtStatus {
    let _ = handle;
    rtl_exit_user_process(code)
}

/// Export `ntdll!NtTerminateProcess` (HANDLE, UINT) -> NTSTATUS.
/// Na prática não retorna para o processo atual.
pub extern "win64" fn NtTerminateProcess_impl(_handle: u64, code: u32) -> u32 {
    nt_terminate_process(WindowsHandle(_handle), code).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_nonempty() {
        assert_eq!(EXPORTS, &["RtlExitUserProcess", "NtTerminateProcess"]);
    }
}
