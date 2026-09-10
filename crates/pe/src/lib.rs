//! `pe`: parser PE32+ (x86_64).
//!
//! Responsabilidade: parsear bytes de disco em estruturas tipadas.
//! NÃO mapeia memória, NÃO aplica relocs, NÃO resolve imports
//! (isso é `nt-loader`/`loader`). 100% safe Rust.
//!
//! Suporta: DOS header, NT headers, sections, data dirs,
//! imports, exports (nomes), base relocs, TLS dir (leitura).

pub mod apiset;
pub mod builder;
pub mod driver;

use thiserror::Error;
use tracing::debug;
use winabi::Rva;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeError {
    #[error("too small: need {need} bytes, have {have}")]
    TooSmall { need: usize, have: usize },
    #[error("bad MZ magic: {0:#06X}")]
    BadMz(u16),
    #[error("bad PE signature: {0:#010X}")]
    BadPe(u32),
    #[error("not PE32+ (magic {0:#06X}); only x86_64 supported in v0.1")]
    NotPe32Plus(u16),
    #[error("not AMD64 (machine {0:#06X})")]
    NotAmd64(u16),
    #[error("bad e_lfanew: {0:#X}")]
    BadLfanew(u32),
    #[error("rva {rva:#X} not in any section")]
    RvaUnmapped { rva: u32 },
    #[error("section {0} name not UTF-8")]
    BadSectionName(usize),
    #[error("import table malformed: {0}")]
    BadImports(&'static str),
    #[error("export table malformed: {0}")]
    BadExports(&'static str),
    #[error("reloc table malformed: {0}")]
    BadRelocs(&'static str),
    #[error("apiset namespace malformed: {0}")]
    BadApiSet(&'static str),
}

/// Índices de DataDirectory.
pub mod dir {
    pub const EXPORT: usize = 0;
    pub const IMPORT: usize = 1;
    pub const RESOURCE: usize = 2;
    pub const EXCEPT: usize = 3;
    pub const CERT: usize = 4;
    pub const BASERELOC: usize = 5;
    pub const DEBUG: usize = 6;
    pub const TLS: usize = 9;
    pub const IAT: usize = 12;
}

#[derive(Debug, Clone, Copy)]
pub struct DataDir {
    pub rva: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub raw_size: u32,
    pub raw_offset: u32,
    pub characteristics: u32,
}

#[derive(Debug, Clone)]
pub struct ImportDescriptor {
    /// RVA da Import Lookup Table (OriginalFirstThunk; pode ser 0 -> usa FirstThunk).
    pub lookup_rva: u32,
    /// RVA da Import Address Table (FirstThunk).
    pub iat_rva: u32,
    pub dll_name_rva: u32,
    pub dll_name: String,
}

#[derive(Debug, Clone)]
pub struct ImportSymbol {
    pub dll: String,
    /// None = import por ordinal.
    pub name: Option<String>,
    pub ordinal: Option<u16>,
    /// RVA da entrada IAT a ser patchada.
    pub iat_rva: u32,
    /// true se a entrada usa IMAGE_SNAP_BY_ORDINAL.
    pub by_ordinal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSymbol {
    pub name: Option<String>,
    /// Ordinal REAL (OrdinalBase + índice). Era índice na v0.1 (bug de
    /// semântica corrigido para o scanner; ver teste `exports_real_ordinals`).
    pub ordinal: u32,
    pub rva: u32,
    pub forwarder: Option<Forwarder>,
}

/// Export encaminhado (`"KERNELBASE.CreateFileW"` ou `"NTDLL.#12"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forwarder {
    pub dll: String,
    pub symbol: ForwardedSymbol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardedSymbol {
    Name(String),
    Ordinal(u16),
}

impl Forwarder {
    /// Parseia `"Dll.Name"` / `"Dll.#123"`. Ponto final separa (nomes de
    /// função não contêm `.`; API Sets usam `-`/`+`, preservados).
    pub fn parse(s: &str) -> Option<Self> {
        let (dll, sym) = s.rsplit_once('.')?;
        if dll.is_empty() || sym.is_empty() {
            return None;
        }
        let symbol = match sym.strip_prefix('#') {
            Some(n) => ForwardedSymbol::Ordinal(n.parse().ok()?),
            None => ForwardedSymbol::Name(sym.to_string()),
        };
        Some(Self {
            dll: dll.to_string(),
            symbol,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RelocBlock {
    pub page_rva: u32,
    pub entries: Vec<RelocEntry>,
}

#[derive(Debug, Clone, Copy)]
pub struct RelocEntry {
    pub typ: u16,
    pub offset: u16,
}

/// Imagem PE parseada (view de disco; borrows `data`).
#[derive(Debug)]
pub struct Image<'a> {
    data: &'a [u8],
    /// `IMAGE_FILE_MACHINE_*` (sempre AMD64 após parse v0.2+; outros
    /// rejeitados — WoW64 futuro relaxa).
    pub machine: u16,
    pub image_base: u64,
    pub entry_point_rva: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub sections: Vec<Section>,
    pub dirs: [DataDir; 16],
    pub number_of_sections: u16,
}

fn u16le(b: &[u8], off: usize) -> Result<u16, PeError> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(PeError::TooSmall {
            need: off + 2,
            have: b.len(),
        })
}

fn u32le(b: &[u8], off: usize) -> Result<u32, PeError> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(PeError::TooSmall {
            need: off + 4,
            have: b.len(),
        })
}

fn u64le(b: &[u8], off: usize) -> Result<u64, PeError> {
    b.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
        .ok_or(PeError::TooSmall {
            need: off + 8,
            have: b.len(),
        })
}

fn cstr(b: &[u8], off: usize, max: usize) -> Result<String, PeError> {
    let end = b.len().min(off.saturating_add(max));
    if off >= b.len() {
        return Err(PeError::TooSmall {
            need: off + 1,
            have: b.len(),
        });
    }
    let slice = &b[off..end];
    let nul = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf8(slice[..nul].to_vec()).map_err(|_| PeError::BadSectionName(off))
}

impl<'a> Image<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, PeError> {
        if data.len() < 64 {
            return Err(PeError::TooSmall {
                need: 64,
                have: data.len(),
            });
        }
        let mz = u16le(data, 0)?;
        if mz != 0x5A4D {
            return Err(PeError::BadMz(mz));
        }
        let lfanew = u32le(data, 0x3C)? as usize;
        if lfanew.saturating_add(6) > data.len() || lfanew > 0x10000 {
            return Err(PeError::BadLfanew(lfanew as u32));
        }
        let pe = u32le(data, lfanew)?;
        if pe != 0x00004550 {
            return Err(PeError::BadPe(pe));
        }
        // COFF header @ lfanew+4
        let coff = lfanew + 4;
        let machine = u16le(data, coff)?;
        if machine != winabi::machine::AMD64 {
            return Err(PeError::NotAmd64(machine));
        }
        let n_sections = u16le(data, coff + 2)?;
        let opt_size = u16le(data, coff + 16)? as usize;
        let opt = coff + 20;
        let magic = u16le(data, opt)?;
        if magic != 0x20B {
            return Err(PeError::NotPe32Plus(magic));
        }
        let entry_rva = u32le(data, opt + 16)?;
        let image_base = u64le(data, opt + 24)?;
        let section_align = u32le(data, opt + 32)?;
        let file_align = u32le(data, opt + 36)?;
        let subsystem = u16le(data, opt + 68)?;
        let dll_chars = u16le(data, opt + 70)?;
        let size_of_image = u32le(data, opt + 56)?;
        let size_of_headers = u32le(data, opt + 60)?;
        let n_rva_sizes = u32le(data, opt + 108)? as usize;

        let mut dirs = [DataDir { rva: 0, size: 0 }; 16];
        let dir_base = opt + 112;
        for (i, slot) in dirs.iter_mut().enumerate().take(16.min(n_rva_sizes)) {
            let rva = u32le(data, dir_base + i * 8).unwrap_or(0);
            let size = u32le(data, dir_base + i * 8 + 4).unwrap_or(0);
            *slot = DataDir { rva, size };
        }

        let sec_base = opt + opt_size;
        let mut sections = Vec::with_capacity(n_sections as usize);
        for i in 0..n_sections as usize {
            let off = sec_base + i * 40;
            if off + 40 > data.len() {
                return Err(PeError::TooSmall {
                    need: off + 40,
                    have: data.len(),
                });
            }
            let raw_name = &data[off..off + 8];
            let nul = raw_name.iter().position(|&c| c == 0).unwrap_or(8);
            let name = String::from_utf8_lossy(&raw_name[..nul]).into_owned();
            sections.push(Section {
                name,
                virtual_size: u32le(data, off + 8)?,
                virtual_address: u32le(data, off + 12)?,
                raw_size: u32le(data, off + 16)?,
                raw_offset: u32le(data, off + 20)?,
                characteristics: u32le(data, off + 36)?,
            });
        }

        debug!(
            sections = n_sections,
            entry = format_args!("{entry_rva:#X}"),
            "pe parsed"
        );

        Ok(Self {
            data,
            machine,
            image_base,
            entry_point_rva: entry_rva,
            size_of_image,
            size_of_headers,
            section_alignment: section_align,
            file_alignment: file_align,
            subsystem,
            dll_characteristics: dll_chars,
            sections,
            dirs,
            number_of_sections: n_sections,
        })
    }

    /// Converte RVA -> offset no arquivo (dados de disco).
    pub fn rva_to_offset(&self, rva: u32) -> Result<usize, PeError> {
        if (rva as usize) < self.size_of_headers as usize && (rva as usize) < self.data.len() {
            // Headers estão mapeados no início. Aproximação válida porque
            // SizeOfHeaders é file-aligned e headers ocupam o prefixo.
            return Ok(rva as usize);
        }
        for s in &self.sections {
            let vsize = s.virtual_size.max(1);
            if rva >= s.virtual_address && rva < s.virtual_address.saturating_add(vsize) {
                let delta = rva - s.virtual_address;
                return Ok(s.raw_offset as usize + delta as usize);
            }
            // Dados além de VirtualSize mas dentro de RawSize (ex.: imports):
            if rva >= s.virtual_address
                && rva < s.virtual_address.saturating_add(s.raw_size)
                && delta_fits(s, rva)
            {
                let delta = rva - s.virtual_address;
                return Ok(s.raw_offset as usize + delta as usize);
            }
        }
        Err(PeError::RvaUnmapped { rva })
    }

    pub fn slice_at_rva(&self, rva: u32, len: usize) -> Result<&'a [u8], PeError> {
        let off = self.rva_to_offset(rva)?;
        self.data.get(off..off + len).ok_or(PeError::TooSmall {
            need: off + len,
            have: self.data.len(),
        })
    }

    pub fn rva(&self, r: u32) -> Rva {
        Rva(r)
    }

    /// Lê a tabela de imports. Retorna descritores + símbolos com IAT RVAs.
    pub fn imports(&self) -> Result<(Vec<ImportDescriptor>, Vec<ImportSymbol>), PeError> {
        let dir = self.dirs[dir::IMPORT];
        if dir.rva == 0 {
            return Ok((Vec::new(), Vec::new()));
        }
        let mut descs = Vec::new();
        let mut syms = Vec::new();
        let mut idx = 0u32;
        loop {
            let off = self.rva_to_offset(dir.rva + idx * 20)?;
            let lookup = u32le(self.data, off).map_err(|_| PeError::BadImports("lookup"))?;
            let _ts = u32le(self.data, off + 4).map_err(|_| PeError::BadImports("ts"))?;
            let _fwd = u32le(self.data, off + 8).map_err(|_| PeError::BadImports("fwd"))?;
            let name_rva = u32le(self.data, off + 12).map_err(|_| PeError::BadImports("name"))?;
            let iat = u32le(self.data, off + 16).map_err(|_| PeError::BadImports("iat"))?;
            if lookup == 0 && name_rva == 0 && iat == 0 {
                break;
            }
            let name_off = self
                .rva_to_offset(name_rva)
                .map_err(|_| PeError::BadImports("dllname"))?;
            let dll_name =
                cstr(self.data, name_off, 256).map_err(|_| PeError::BadImports("dllname-utf8"))?;
            descs.push(ImportDescriptor {
                lookup_rva: lookup,
                iat_rva: iat,
                dll_name_rva: name_rva,
                dll_name: dll_name.clone(),
            });

            // Caminha pela ILT (ou IAT se OriginalFirstThunk == 0).
            let table = if lookup != 0 { lookup } else { iat };
            let mut j = 0u32;
            loop {
                let ent_off = self
                    .rva_to_offset(table + j * 8)
                    .map_err(|_| PeError::BadImports("ilt-range"))?;
                let ent = u64le(self.data, ent_off).map_err(|_| PeError::BadImports("ilt-read"))?;
                if ent == 0 {
                    break;
                }
                if ent & 0x8000_0000_0000_0000 != 0 {
                    syms.push(ImportSymbol {
                        dll: dll_name.clone(),
                        name: None,
                        ordinal: Some((ent & 0xFFFF) as u16),
                        iat_rva: iat + j * 8,
                        by_ordinal: true,
                    });
                } else {
                    let hint_name_rva = (ent & 0xFFFF_FFFF) as u32;
                    let hn_off = self
                        .rva_to_offset(hint_name_rva)
                        .map_err(|_| PeError::BadImports("hintname"))?;
                    // +2 hint
                    let sym_name = cstr(self.data, hn_off + 2, 512)
                        .map_err(|_| PeError::BadImports("symname"))?;
                    syms.push(ImportSymbol {
                        dll: dll_name.clone(),
                        name: Some(sym_name),
                        ordinal: None,
                        iat_rva: iat + j * 8,
                        by_ordinal: false,
                    });
                }
                j += 1;
                if j > 4096 {
                    return Err(PeError::BadImports("ilt-too-long"));
                }
            }

            idx += 1;
            if idx > 256 {
                return Err(PeError::BadImports("too-many-dlls"));
            }
        }
        Ok((descs, syms))
    }

    /// Lê a tabela de exports: TODAS as funções (com e sem nome),
    /// com ordinais REAIS e forwarders estruturados.
    pub fn exports(&self) -> Result<Vec<ExportSymbol>, PeError> {
        let dir = self.dirs[dir::EXPORT];
        if dir.rva == 0 {
            return Ok(Vec::new());
        }
        let base = self.rva_to_offset(dir.rva)?;
        let get = |o: usize| u32le(self.data, base + o).map_err(|_| PeError::BadExports("hdr"));
        let _flags = get(0)?;
        let _ts = get(4)?;
        let _maj = u16le(self.data, base + 8).map_err(|_| PeError::BadExports("ver"))?;
        let _minor = u16le(self.data, base + 10).map_err(|_| PeError::BadExports("ver"))?;
        let _name_rva = get(12)?;
        let ord_base = get(16)?;
        let n_funcs = get(20)? as usize;
        let n_names = get(24)? as usize;
        let addr_funcs = get(28)?;
        let addr_names = get(32)?;
        let addr_ordinals = get(36)?;
        if n_names > 65536 || n_funcs > 65536 {
            return Err(PeError::BadExports("too-many"));
        }
        // Mapa índice->nome (ordinais sem nome ficam com None).
        let mut names_by_idx: std::collections::HashMap<u32, String> =
            std::collections::HashMap::new();
        for i in 0..n_names {
            let name_rva = u32le(self.data, self.rva_to_offset(addr_names + i as u32 * 4)?)
                .map_err(|_| PeError::BadExports("nameptr"))?;
            let ord_idx = u16le(self.data, self.rva_to_offset(addr_ordinals + i as u32 * 2)?)
                .map_err(|_| PeError::BadExports("ord"))? as u32;
            let name = cstr(self.data, self.rva_to_offset(name_rva)?, 512)
                .map_err(|_| PeError::BadExports("name"))?;
            names_by_idx.insert(ord_idx, name);
        }
        let mut out = Vec::new();
        for idx in 0..n_funcs as u32 {
            let func_rva = u32le(self.data, self.rva_to_offset(addr_funcs + idx * 4)?)
                .map_err(|_| PeError::BadExports("func"))?;
            if func_rva == 0 {
                continue; // slot vazio (gap de ordinal)
            }
            // Forwarder se func_rva cai dentro do diretório de exports.
            let forwarder = if func_rva >= dir.rva && func_rva < dir.rva + dir.size {
                let s = cstr(self.data, self.rva_to_offset(func_rva)?, 512)
                    .map_err(|_| PeError::BadExports("fwd"))?;
                Some(Forwarder::parse(&s).ok_or(PeError::BadExports("fwd-parse"))?)
            } else {
                None
            };
            out.push(ExportSymbol {
                name: names_by_idx.remove(&idx),
                ordinal: ord_base + idx,
                rva: func_rva,
                forwarder,
            });
        }
        Ok(out)
    }

    /// Lê blocos de base-relocation (`IMAGE_BASE_RELOCATION`).
    pub fn relocs(&self) -> Result<Vec<RelocBlock>, PeError> {
        let dir = self.dirs[dir::BASERELOC];
        if dir.rva == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let mut off = self.rva_to_offset(dir.rva)?;
        let end = off + dir.size as usize;
        while off + 8 <= end {
            let page = u32le(self.data, off).map_err(|_| PeError::BadRelocs("page"))?;
            let size = u32le(self.data, off + 4).map_err(|_| PeError::BadRelocs("size"))? as usize;
            if size == 0 {
                break;
            }
            if size < 8 || off + size > end {
                return Err(PeError::BadRelocs("size-range"));
            }
            let count = (size - 8) / 2;
            let mut entries = Vec::with_capacity(count);
            for i in 0..count {
                let e =
                    u16le(self.data, off + 8 + i * 2).map_err(|_| PeError::BadRelocs("entry"))?;
                entries.push(RelocEntry {
                    typ: e >> 12,
                    offset: e & 0x0FFF,
                });
            }
            out.push(RelocBlock {
                page_rva: page,
                entries,
            });
            off += size;
            if out.len() > 4096 {
                return Err(PeError::BadRelocs("too-many-blocks"));
            }
        }
        Ok(out)
    }
}

fn delta_fits(s: &Section, rva: u32) -> bool {
    // Helper: garante que o delta cabe no raw (evita wrap).
    rva.wrapping_sub(s.virtual_address) < s.raw_size
}

#[cfg(test)]
mod tests {
    use super::builder::build_minimal_hello;
    use super::*;

    #[test]
    fn parse_minimal_hello() {
        let bytes = build_minimal_hello();
        let img = Image::parse(&bytes).expect("parse");
        assert_eq!(img.entry_point_rva, 0x1000);
        assert_eq!(img.sections.len(), 2);
        let (_d, syms) = img.imports().expect("imports");
        let names: Vec<_> = syms.iter().filter_map(|s| s.name.clone()).collect();
        assert!(names.contains(&"GetStdHandle".to_string()));
        assert!(names.contains(&"WriteFile".to_string()));
        assert!(names.contains(&"ExitProcess".to_string()));
    }

    #[test]
    fn rejects_bad_magic() {
        let b = vec![0u8; 128];
        assert!(Image::parse(&b).is_err());
    }

    #[test]
    fn rva_unmapped_errors() {
        let bytes = build_minimal_hello();
        let img = Image::parse(&bytes).unwrap();
        assert!(img.rva_to_offset(0xFFFF_F000).is_err());
    }

    #[test]
    fn forwarder_parse_shapes() {
        let f = Forwarder::parse("KERNELBASE.CreateFileW").unwrap();
        assert_eq!(f.dll, "KERNELBASE");
        assert_eq!(f.symbol, ForwardedSymbol::Name("CreateFileW".into()));
        let g = Forwarder::parse("NTDLL.#12").unwrap();
        assert_eq!(g.symbol, ForwardedSymbol::Ordinal(12));
        assert!(Forwarder::parse("sem-ponto").is_none());
        assert!(Forwarder::parse("A.").is_none());
    }

    /// Valida exports contra DLLs REAIS da referência (oracle estrutural).
    /// Pula graciosamente se `windows-reference/` não estiver presente
    /// (clone sem os 4.4G ainda compila e testa todo o resto).
    #[test]
    fn exports_against_real_kernel32() {
        let path = std::env::var("RINE_WINDOWS_REF").unwrap_or_else(|_| {
            let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.push("../../windows-reference/win11-25h2/System32/kernel32.dll");
            p.to_string_lossy().into_owned()
        });
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("SKIP exports_against_real_kernel32: sem {path}");
                return;
            }
        };
        let img = Image::parse(&bytes).expect("kernel32 real deve parsear");
        assert_eq!(img.machine, winabi::machine::AMD64);
        let exps = img.exports().expect("exports reais");
        assert!(exps.len() > 1000, "kernel32 tem milhares de exports");
        // Ordinais reais: únicos e > 0.
        let mut ords: Vec<u32> = exps.iter().map(|e| e.ordinal).collect();
        ords.sort_unstable();
        ords.dedup();
        assert_eq!(ords.len(), exps.len(), "ordinais duplicados?");
        assert!(ords.iter().all(|&o| o > 0));
        // Âncora confirmada via objdump: AcquireSRWLockExclusive é
        // forwarder para NTDLL.RtlAcquireSRWLockExclusive.
        let fwd = exps
            .iter()
            .find(|e| e.name.as_deref() == Some("AcquireSRWLockExclusive"))
            .expect("AcquireSRWLockExclusive");
        match &fwd.forwarder {
            Some(f) => {
                assert_eq!(f.dll.to_ascii_uppercase(), "NTDLL");
                assert_eq!(
                    f.symbol,
                    ForwardedSymbol::Name("RtlAcquireSRWLockExclusive".into())
                );
            }
            None => panic!("RtlAcquireSRWLockExclusive deveria ser forwarder"),
        }
        // Todo forwarder parseado tem alvo não-vazio.
        for e in &exps {
            if let Some(f) = &e.forwarder {
                assert!(!f.dll.is_empty());
            }
        }
    }
}
