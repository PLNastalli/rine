//! `host-linux`: única fronteira com o kernel Linux.
//!
//! Todo acesso a syscall vai por aqui. Nenhuma outra crate pode chamar
//! `libc`/`rustix` diretamente (convenção auditável via `grep`).
//! Safe wrappers com invariantes documentados; `unsafe` só aqui e no
//! trampolim de entry point (`runtime`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostError {
    #[error("mmap failed: {0}")]
    Mmap(i32),
    #[error("mprotect failed: {0}")]
    Mprotect(i32),
    #[error("munmap failed: {0}")]
    Munmap(i32),
    #[error("arch_prctl failed: {0}")]
    ArchPrctl(i32),
    #[error("io failed: {0}")]
    Io(i32),
}

/// Proteção para mmap/mprotect (espelha PAGE_* sem expor valores Linux).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prot {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl Prot {
    pub const NONE: Self = Self {
        read: false,
        write: false,
        exec: false,
    };
    pub const R: Self = Self {
        read: true,
        write: false,
        exec: false,
    };
    pub const RW: Self = Self {
        read: true,
        write: true,
        exec: false,
    };
    pub const RX: Self = Self {
        read: true,
        write: false,
        exec: true,
    };
    pub const RWX: Self = Self {
        read: true,
        write: true,
        exec: true,
    };
}

/// Reserva `len` bytes legíveis/escritas (anon, private).
/// Retorna o endereço base alinhado a página.
///
/// SAFETY (fronteira):
/// - invariante: `len > 0`; chamador deve `release` exatamente uma vez.
/// - quem garante: `nt-memory` (regiões rastreadas) — ver `safety-model.md`.
/// - violação: vazamento ou double-munmap (abort do kernel, não UB Rust).
pub fn reserve_anonymous(len: usize) -> Result<*mut u8, HostError> {
    if len == 0 {
        return Err(HostError::Mmap(libc::EINVAL));
    }
    // SAFETY: parâmetros validados; MAP_ANONYMOUS|PRIVATE, fd=-1, offset=0.
    // Retorno MAP_FAILED checado. O mapeamento é válido até `release`.
    let ptr = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        Err(HostError::Mmap(last_errno()))
    } else {
        Ok(ptr as *mut u8)
    }
}

/// Reserva em endereço fixo sugerido (para tentar ImageBase preferencial).
/// Falha de forma limpa (`Err`) se ocupado — o loader faz fallback.
pub fn reserve_fixed(hint: *mut u8, len: usize) -> Result<*mut u8, HostError> {
    // SAFETY: como acima + `hint` page-aligned (garantido por `nt-loader`).
    // MAP_FIXED_NOREPLACE nunca destrói mapeamento existente: ou sucede
    // em `hint` ou falha com EEXIST — sem corrupção.
    let ptr = unsafe {
        libc::mmap(
            hint as *mut libc::c_void,
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED_NOREPLACE,
            -1,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        Err(HostError::Mmap(last_errno()))
    } else {
        Ok(ptr as *mut u8)
    }
}

/// Ajusta proteção de `[addr, addr+len)`.
///
/// SAFETY: `addr` page-aligned, `len` múltiplo de página, região viva.
/// Garantido por `nt-memory::Region`. Violação => SIGSEGV (não UB silencioso).
pub fn protect(addr: *mut u8, len: usize, prot: Prot) -> Result<(), HostError> {
    let flags = (if prot.read { libc::PROT_READ } else { 0 })
        | (if prot.write { libc::PROT_WRITE } else { 0 })
        | (if prot.exec { libc::PROT_EXEC } else { 0 });
    // SAFETY: endereço/tamanho validados pelo chamador rastreado.
    let r = unsafe { libc::mprotect(addr as *mut libc::c_void, len, flags) };
    if r != 0 {
        Err(HostError::Mprotect(last_errno()))
    } else {
        Ok(())
    }
}

/// Lê até `buf.len()` bytes do fd (UMA syscall, como `ReadFile`:
/// short count em EOF é sucesso, não erro). Retorna bytes lidos.
/// Repete só em EINTR.
pub fn read_upto(fd: i32, buf: &mut [u8]) -> Result<usize, HostError> {
    if buf.is_empty() {
        return Ok(0);
    }
    loop {
        // SAFETY: `buf` é empréstimo exclusivo vivo; `read` não retém o ponteiro.
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            let e = last_errno();
            if e == libc::EINTR {
                continue;
            }
            return Err(HostError::Io(e));
        }
        return Ok(n as usize);
    }
}

/// Origem do seek (espelha FILE_BEGIN/CURRENT/END sem expor `SEEK_*`).#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// Início (`SEEK_SET`).
    Start,
    /// Posição atual (`SEEK_CUR`).
    Current,
    /// Fim (`SEEK_END`).
    End,
}

/// `lseek`: reposiciona o offset do fd; retorna o novo offset.
/// Repete só em EINTR. Erro típico: ESPIPE (fd não-seekable, ex. console) —
/// o caller (`nt-file`) mapeia para `NtStatus`.
pub fn seek(fd: i32, offset: i64, from: SeekFrom) -> Result<u64, HostError> {
    let whence = match from {
        SeekFrom::Start => libc::SEEK_SET,
        SeekFrom::Current => libc::SEEK_CUR,
        SeekFrom::End => libc::SEEK_END,
    };
    loop {
        // SAFETY: `lseek` não toca memória, só o offset do fd.
        let n = unsafe { libc::lseek(fd, offset as libc::off_t, whence) };
        if n < 0 {
            let e = last_errno();
            if e == libc::EINTR {
                continue;
            }
            return Err(HostError::Io(e));
        }
        return Ok(n as u64);
    }
}

/// Erro cross-device? (`rename` entre filesystems; o caller faz copy+remove).
/// Centralizado aqui porque só `host-linux` lê `libc::EXDEV`
/// (`ErrorKind::CrossesDevices` exigiria Rust 1.83+; MSRV é 1.80).
pub fn is_cross_device(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(libc::EXDEV)
}

/// Fecha um fd possuído pelo runtime.
///
/// SAFETY: `fd` deve ser possuído (nunca 0/1/2 do host, nunca duplicado).
/// Quem garante: `nt-file` (`owns_fd`, ver `FileObject`). Double-close
/// atingiria um fd reutilizado — por isso a ownership é rastreada.
pub fn close_fd(fd: i32) -> Result<(), HostError> {
    // SAFETY: fd possuído; `close` não retém nada.
    let r = unsafe { libc::close(fd) };
    if r != 0 {
        Err(HostError::Io(last_errno()))
    } else {
        Ok(())
    }
}
/// Libera `[addr, addr+len)`.
///
/// SAFETY: exatamente a região retornada por `reserve_*`; uso após free
/// é proibido pelo borrow de `nt-memory::Mapping` (ver safety-model).
pub fn release(addr: *mut u8, len: usize) -> Result<(), HostError> {
    // SAFETY: região viva e com tamanho exato.
    let r = unsafe { libc::munmap(addr as *mut libc::c_void, len) };
    if r != 0 {
        Err(HostError::Munmap(last_errno()))
    } else {
        Ok(())
    }
}

pub fn page_size() -> usize {
    // SAFETY: sysconf(_SC_PAGESIZE) é sempre válido; fallback 4096.
    let v = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if v > 0 {
        v as usize
    } else {
        4096
    }
}

/// Escreve exatamente `buf` no fd (repetindo em writes parciais/EINTR).
pub fn write_all(fd: i32, mut buf: &[u8]) -> Result<usize, HostError> {
    let mut total = 0;
    while !buf.is_empty() {
        // SAFETY: `buf` é slice vivo; `write` não retém o ponteiro.
        let n = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) };
        if n < 0 {
            let e = last_errno();
            if e == libc::EINTR {
                continue;
            }
            return Err(HostError::Io(e));
        }
        let n = n as usize;
        total += n;
        buf = &buf[n..];
    }
    Ok(total)
}

/// Códigos `arch_prctl(2)` — definidos localmente para não depender de
/// constantes instáveis do crate `libc` (ver `asm/prctl.h` do kernel).
const ARCH_SET_GS: libc::c_int = 0x1001;
const ARCH_GET_GS: libc::c_int = 0x1004;

/// Define a base GS (TEB) da thread atual. Retorna a base anterior.
///
/// SAFETY: `base` deve apontar para um TEB válido e vivo durante toda a
/// execução do código Windows (garantido por `runtime::ThreadContext`,
/// que mantém o `Box<TebMinimal>` + `release` ordenado). GS é restaurado
/// no retorno (ver `runtime::enter`). Efeito é por-thread.
pub fn set_gs_base(base: *const u8) -> Result<usize, HostError> {
    let mut prev: usize = 0;
    // SAFETY: arch_prctl GET/SET_GS com ponteiros válidos; sem efeitos além de GS.
    let r = unsafe {
        let g = libc::syscall(libc::SYS_arch_prctl, ARCH_GET_GS, &mut prev as *mut usize);
        if g != 0 {
            return Err(HostError::ArchPrctl(last_errno()));
        }
        libc::syscall(libc::SYS_arch_prctl, ARCH_SET_GS, base as usize)
    };
    if r != 0 {
        Err(HostError::ArchPrctl(last_errno()))
    } else {
        Ok(prev)
    }
}

/// Restaura a base GS anterior.
pub fn restore_gs_base(prev: usize) -> Result<(), HostError> {
    // SAFETY: `prev` veio de `set_gs_base` nesta mesma thread.
    let r = unsafe { libc::syscall(libc::SYS_arch_prctl, ARCH_SET_GS, prev) };
    if r != 0 {
        Err(HostError::ArchPrctl(last_errno()))
    } else {
        Ok(())
    }
}

fn last_errno() -> i32 {
    // SAFETY: errno por-thread, leitura imediata após syscall que falhou.
    unsafe { *libc::__errno_location() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_protect_release_roundtrip() {
        let ps = page_size();
        let p = reserve_anonymous(ps * 2).unwrap();
        protect(p, ps * 2, Prot::R).unwrap();
        protect(p, ps * 2, Prot::RW).unwrap();
        release(p, ps * 2).unwrap();
    }

    #[test]
    fn zero_len_fails_cleanly() {
        assert!(reserve_anonymous(0).is_err());
    }
}
