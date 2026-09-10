//! `loader`: mapeador de imagem PE (reserve → copy → protect).
//!
//! Responsabilidade: adquirir memória do host e materializar a imagem.
//! Relocs/imports são `nt-loader` (lógica pura); aqui só orquestra mmap.
//!
//! ```text
//! disk → parse → reserve address space → map sections → relocs → imports → protect
//! ```

use nt_loader::LoaderError;
use pe::Image;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MapError {
    #[error("pe: {0}")]
    Pe(#[from] pe::PeError),
    #[error("loader: {0}")]
    Loader(#[from] LoaderError),
    #[error("host: {0}")]
    Host(#[from] host_linux::HostError),
    #[error("size of image inválido: {0:#X}")]
    BadImageSize(u32),
    #[error("seção fora da imagem: {0}")]
    SectionRange(String),
}

/// Imagem mapeada viva. `Drop` libera via `munmap`.
pub struct MappedImage {
    pub base: *mut u8,
    pub size: usize,
    pub entry_rva: u32,
    pub image_base_preferred: u64,
}

// SAFETY: `MappedImage` possui exclusivamente a região; `Send` é seguro
// porque o runtime a transfere entre estágios single-threaded.
unsafe impl Send for MappedImage {}

impl MappedImage {
    pub fn entry_point(&self) -> *mut u8 {
        // SAFETY: entry_rva validado < size no `map_image`.
        unsafe { self.base.add(self.entry_rva as usize) }
    }

    pub fn base_addr(&self) -> u64 {
        self.base as u64
    }
}

impl Drop for MappedImage {
    fn drop(&mut self) {
        if self.size > 0 && !self.base.is_null() {
            let _ = host_linux::release(self.base, self.size);
        }
    }
}

/// Mapeia `data` (bytes do PE) em memória executável (RW inicial;
/// o caller aplica proteção final via `protect_sections`).
pub fn map_image(data: &[u8]) -> Result<(MappedImage, Image<'_>), MapError> {
    let img = Image::parse(data)?;
    if img.size_of_image == 0 || img.size_of_image > 0x4000_0000 {
        return Err(MapError::BadImageSize(img.size_of_image));
    }
    let size = img.size_of_image as usize;

    // Tenta a ImageBase preferencial (comportamento Windows com ASLR
    // desabilitado para EXEs não-DYNAMICBASE; nosso hello tem DYNAMICBASE
    // mas tenta mesmo assim — fallback limpo se ocupado).
    let preferred = img.image_base;
    let page = host_linux::page_size();
    let aligned_pref = (preferred as usize) & !(page - 1);
    let base = match host_linux::reserve_fixed(aligned_pref as *mut u8, size) {
        Ok(p) => p,
        Err(_) => host_linux::reserve_anonymous(size)?,
    };

    // SAFETY: `base`..`base+size` recém-mapeado e RW; cópias limitadas por
    // `size` (checado por seção abaixo).
    unsafe {
        // Headers: copia min(len, SizeOfHeaders).
        let hlen = (data.len().min(img.size_of_headers as usize)).min(size);
        std::ptr::copy_nonoverlapping(data.as_ptr(), base, hlen);
        // Seções.
        for s in &img.sections {
            let vend = (s.virtual_address as usize).saturating_add(s.raw_size as usize);
            if vend > size {
                host_linux::release(base, size)?;
                return Err(MapError::SectionRange(s.name.clone()));
            }
            if s.raw_size == 0 {
                continue;
            }
            let src_off = s.raw_offset as usize;
            let src_end = src_off.saturating_add(s.raw_size as usize);
            if src_end > data.len() {
                host_linux::release(base, size)?;
                return Err(MapError::SectionRange(s.name.clone()));
            }
            std::ptr::copy_nonoverlapping(
                data.as_ptr().add(src_off),
                base.add(s.virtual_address as usize),
                s.raw_size as usize,
            );
            // Zero-fill até VirtualSize quando maior que RawSize (.bss).
            if s.virtual_size > s.raw_size {
                let extra = (s.virtual_size - s.raw_size) as usize;
                if (s.virtual_address as usize) + (s.virtual_size as usize) <= size {
                    std::ptr::write_bytes(
                        base.add(s.virtual_address as usize + s.raw_size as usize),
                        0,
                        extra,
                    );
                }
            }
        }
    }

    let entry_rva = img.entry_point_rva;
    if (entry_rva as usize) >= size {
        host_linux::release(base, size)?;
        return Err(MapError::BadImageSize(entry_rva));
    }

    tracing::debug!(
        base = format_args!("{base:p}"),
        size,
        sections = img.sections.len(),
        entry_rva = format_args!("{entry_rva:#X}"),
        "imagem mapeada"
    );

    Ok((
        MappedImage {
            base,
            size,
            entry_rva,
            image_base_preferred: img.image_base,
        },
        img,
    ))
}

/// Aplica proteção final por seção (mapeamento em `nt_memory::section_protect`).
/// Deve rodar DEPOIS de relocs+imports (que exigem RW).
pub fn protect_sections(mapped: &MappedImage, img: &Image) -> Result<(), MapError> {
    let page = host_linux::page_size();
    for s in &img.sections {
        let prot = nt_memory::host_protect(nt_memory::section_protect(s.characteristics));
        // Arredonda o intervalo para páginas.
        let start = (s.virtual_address as usize) & !(page - 1);
        let end = align_up(
            s.virtual_address as usize + s.virtual_size.max(s.raw_size) as usize,
            page,
        )
        .min(mapped.size);
        if end > start {
            // SAFETY: intervalo dentro da imagem mapeada.
            unsafe { host_linux::protect(mapped.base.add(start), end - start, prot)? };
        }
    }
    Ok(())
}

fn align_up(v: usize, a: usize) -> usize {
    (v + a - 1) & !(a - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_hello_and_protect() {
        let bytes = pe::builder::build_minimal_hello();
        let (mapped, img) = map_image(&bytes).expect("map");
        assert!(!mapped.base.is_null());
        // Entry contém `sub rsp,0x38` (48 83 EC 38).
        let ep = mapped.entry_point();
        let first: [u8; 4] = unsafe { std::slice::from_raw_parts(ep, 4).try_into().unwrap() };
        assert_eq!(first, [0x48, 0x83, 0xEC, 0x38]);
        protect_sections(&mapped, &img).expect("protect");
        // Não libera manualmente: Drop faz munmap.
    }
}
