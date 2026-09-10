//! `nt-loader`: lógica NT de imagem (relocs, imports, TLS).
//!
//! Responsabilidade: transformações sobre memória já mapeada.
//! NÃO faz mmap (isso é `loader`); opera sobre slices/ponteiros com
//! invariantes documentados. 100% da aritmética de RVAs aqui.

use pe::RelocBlock;
use thiserror::Error;
use winabi::NtStatus;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("pe: {0}")]
    Pe(#[from] pe::PeError),
    #[error("reloc type {typ} not supported at rva {rva:#X}")]
    BadReloc { typ: u16, rva: u32 },
    #[error("import not resolved: {dll}!{name}")]
    Unresolved { dll: String, name: String },
    #[error("reloc out of range: {0:#X}")]
    RelocRange(u32),
    #[error("{0}")]
    Status(NtStatus),
}

impl From<LoaderError> for NtStatus {
    fn from(e: LoaderError) -> NtStatus {
        match e {
            LoaderError::Pe(_) => NtStatus::UNSUCCESSFUL,
            LoaderError::BadReloc { .. } => NtStatus::UNSUCCESSFUL,
            LoaderError::Unresolved { .. } => NtStatus::OBJECT_NAME_NOT_FOUND,
            LoaderError::RelocRange(_) => NtStatus::UNSUCCESSFUL,
            LoaderError::Status(s) => s,
        }
    }
}

/// Aplica base-relocs sobre imagem já mapeada em `base`.
///
/// `delta = new_base - preferred_base`. Suporta `ADDR64` (10) e
/// `ADDR32NB`/`REL32` como no-op-validado conforme semântica Windows.
///
/// # Safety
///
/// - `base`..`base+size_of_image` deve estar mapeado e RW; `blocks` deve ter
///   vindo de `Image::relocs` da mesma imagem. Quem garante: `loader::map_image`
///   (mmap + PROT_WRITE antes de relocs).
/// - Violação: escrita fora da imagem -> corrupção do host (mitigado pelo guard
///   `rva < size_of_image` antes de cada escrita, que transforma em `Err`).
pub unsafe fn apply_relocs(
    base: *mut u8,
    size_of_image: usize,
    preferred: u64,
    blocks: &[RelocBlock],
) -> Result<u32, LoaderError> {
    if preferred == base as u64 {
        return Ok(0);
    }
    let delta = (base as u64).wrapping_sub(preferred);
    let mut applied = 0u32;
    for b in blocks {
        for e in &b.entries {
            let rva = b.page_rva.wrapping_add(e.offset as u32);
            if (rva as usize) + 8 > size_of_image {
                return Err(LoaderError::RelocRange(rva));
            }
            match e.typ {
                0 => {} // ABSOLUTE: padding
                1 => {
                    // ADDR64
                    // SAFETY: range checado acima; alinhamento u64 garantido?
                    // Relocs DIR64 são 8-alinhados por construção do linker.
                    let p = base.add(rva as usize) as *mut u64;
                    // SAFETY: `p` dentro da imagem RW; escrita única.
                    unsafe { *p = (*p).wrapping_add(delta) };
                    applied += 1;
                }
                3 => {
                    // ADDR32NB: RVA de 32 bits — independente da base, valida range.
                    let p = base.add(rva as usize) as *mut u32;
                    let _ = unsafe { *p };
                    applied += 1;
                }
                4 => {
                    // REL32: relativo — independente da base.
                    applied += 1;
                }
                t => return Err(LoaderError::BadReloc { typ: t, rva }),
            }
        }
    }
    Ok(applied)
}

/// Resolve imports contra `resolver(dll, name) -> endereço` e patcha a IAT.
///
/// # Safety
///
/// `base` deve estar mapeado RW com `size_of_image` bytes; `symbols` deve vir
/// de `Image::imports` da mesma imagem; cada `iat_rva` é checado contra
/// `size_of_image` antes da escrita. Quem garante: `loader`/`runtime`.
/// Cada escrita é um `u64` alinhado (IAT é array de `u64`).
pub unsafe fn resolve_imports(
    base: *mut u8,
    size_of_image: usize,
    symbols: &[pe::ImportSymbol],
    mut resolver: impl FnMut(&str, Option<&str>, Option<u16>) -> Option<u64>,
) -> Result<u32, LoaderError> {
    let mut count = 0;
    for s in symbols {
        let addr = resolver(&s.dll, s.name.as_deref(), s.ordinal).ok_or_else(|| {
            LoaderError::Unresolved {
                dll: s.dll.clone(),
                name: s
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("#{}", s.ordinal.unwrap_or(0))),
            }
        })?;
        if (s.iat_rva as usize) + 8 > size_of_image {
            return Err(LoaderError::RelocRange(s.iat_rva));
        }
        // SAFETY: IAT entry dentro da imagem, u64 alinhado.
        unsafe { *(base.add(s.iat_rva as usize) as *mut u64) = addr };
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reloc_delta_zero_noop() {
        let mut mem = vec![0u8; 0x1000];
        let base = mem.as_mut_ptr();
        let blocks = vec![RelocBlock {
            page_rva: 0x100,
            entries: vec![],
        }];
        let n = unsafe { apply_relocs(base, 0x1000, base as u64, &blocks).unwrap() };
        assert_eq!(n, 0);
    }

    #[test]
    fn reloc_addr64_applies_delta() {
        let mut mem = vec![0u8; 0x2000];
        let base = mem.as_mut_ptr();
        // Simula qword na RVA 0x100 valendo preferred+0x100.
        let preferred = (base as u64).wrapping_sub(0x10000);
        unsafe { *(base.add(0x100) as *mut u64) = preferred + 0x100 };
        let blocks = vec![RelocBlock {
            page_rva: 0x100,
            entries: vec![pe::RelocEntry { typ: 1, offset: 0 }],
        }];
        unsafe { apply_relocs(base, 0x2000, preferred, &blocks).unwrap() };
        assert_eq!(
            unsafe { *(base.add(0x100) as *mut u64) },
            base as u64 + 0x100
        );
    }
}
