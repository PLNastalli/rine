//! `runtime`: orquestração do processo emulado + execução do entry point.
//!
//! Camadas:
//! ```text
//! PE bytes → loader::map_image → nt-loader relocs/imports → TEB/PEB + GS → call entry
//! ```
//! `unsafe` só na chamada do entry e na instalação do GS (fronteiras).

use loader::MappedImage;
use nt_object::HandleTable;
use std::sync::Arc;
use thiserror::Error;
use winabi::{PebMinimal, TebMinimal, Tib};

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("map: {0}")]
    Map(#[from] loader::MapError),
    #[error("loader: {0}")]
    Loader(#[from] nt_loader::LoaderError),
    #[error("host: {0}")]
    Host(#[from] host_linux::HostError),
    #[error("pe: {0}")]
    Pe(#[from] pe::PeError),
    #[error("capsule: {0}")]
    Capsule(String),
}

/// Capsule da aplicação (config declarativa por-app).
///
/// Formato de arquivo (`capsule.toml`, v0.2):
/// ```toml
/// [app]
/// name = "filetest"
/// windows_version = [10, 0]
/// arch = "x86_64"
/// current_dir = "C:\\work"      # opcional; default: CWD do host
/// [drives]
/// C = "/tmp/capsule-c"           # letra maiúscula -> dir do host
/// ```
#[derive(Debug, Clone)]
pub struct Capsule {
    pub app_name: String,
    pub windows_version: (u32, u32),
    pub arch: &'static str,
    /// `A`..=`Z` -> diretório host. Chave sempre maiúscula.
    pub drives: std::collections::HashMap<char, String>,
    /// Diretório corrente Win32 (`C:\work` ou relativo); `None` = CWD host.
    pub current_dir: Option<String>,
}

impl Default for Capsule {
    fn default() -> Self {
        Self {
            app_name: "hello".into(),
            windows_version: (10, 0),
            arch: "x86_64",
            drives: std::collections::HashMap::new(),
            current_dir: None,
        }
    }
}

impl Capsule {
    /// Carrega `capsule.toml` do disco. Erros de formato são explícitos
    /// (capsule inválida = falha no load, nunca default silencioso).
    pub fn load_toml(path: &str) -> Result<Self, RuntimeError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| RuntimeError::Capsule(format!("{path}: {e}")))?;
        let v: toml::Value =
            toml::from_str(&text).map_err(|e| RuntimeError::Capsule(format!("{path}: {e}")))?;
        let app = &v["app"];
        let mut cap = Capsule {
            app_name: app
                .get("name")
                .and_then(|s| s.as_str())
                .unwrap_or("app")
                .to_string(),
            windows_version: match app.get("windows_version") {
                Some(toml::Value::Array(a)) if a.len() == 2 => (
                    a[0].as_integer().unwrap_or(10) as u32,
                    a[1].as_integer().unwrap_or(0) as u32,
                ),
                _ => (10, 0),
            },
            arch: "x86_64",
            drives: std::collections::HashMap::new(),
            current_dir: app
                .get("current_dir")
                .and_then(|s| s.as_str())
                .map(str::to_string),
        };
        if let Some(toml::Value::Table(drives)) = v.get("drives") {
            for (k, val) in drives {
                let letter = k.to_ascii_uppercase().chars().next().unwrap_or('?');
                if let Some(dir) = val.as_str() {
                    cap.drives.insert(letter, dir.to_string());
                }
            }
        }
        Ok(cap)
    }
}

/// Processo emulado carregado e pronto para executar.
pub struct Emulator {
    capsule: Capsule,
    mapped: Option<MappedImage>,
    teb: Box<TebMinimal>,
    peb: Box<PebMinimal>,
    image_base: u64,
    /// Linha de comando UTF-16 NUL-terminada (viva durante o guest;
    /// `GetCommandLineW` retorna ponteiro para cá).
    _cmdline_w: Vec<u16>,
    /// Bloco opaco de parâmetros com `UNICODE_STRING CommandLine` no offset
    /// 0x70 (igual ao `RTL_USER_PROCESS_PARAMETERS` x64 real).
    _params_block: Box<[u8; 0x80]>,
}

/// Offset de `CommandLine` em `RTL_USER_PROCESS_PARAMETERS` x64.
pub const PARAMS_CMDLINE_OFF: usize = 0x70;

impl Emulator {
    /// Carrega `pe_bytes` na memória (map + relocs + imports + TEB/PEB).
    /// `guest_cmdline` é a linha de comando Win32 completa (ex.:
    /// `"file.exe foo bar"`); vira UTF-16 observável via `GetCommandLineW`.
    pub fn load(
        capsule: Capsule,
        pe_bytes: &[u8],
        guest_cmdline: &str,
    ) -> Result<Self, RuntimeError> {
        // 1. HandleTable + console + linha de comando UTF-16 + estado do processo.
        let table = Arc::new(HandleTable::new());
        let (stdout, stderr) = table.init_console();
        let mut cmdline_w: Vec<u16> = guest_cmdline.encode_utf16().collect();
        cmdline_w.push(0);
        // Bloco de parâmetros: UNICODE_STRING { Length, MaximumLength, Buffer }.
        let mut params = Box::new([0u8; 0x80]);
        let chars = (cmdline_w.len() - 1).min(0x7FFF);
        params[PARAMS_CMDLINE_OFF..PARAMS_CMDLINE_OFF + 2]
            .copy_from_slice(&((chars * 2) as u16).to_le_bytes());
        params[PARAMS_CMDLINE_OFF + 2..PARAMS_CMDLINE_OFF + 4]
            .copy_from_slice(&((cmdline_w.len() * 2) as u16).to_le_bytes());
        // +4..+8 padding; Buffer @+8.
        let cmdline_ptr = cmdline_w.as_ptr() as u64;
        params[PARAMS_CMDLINE_OFF + 8..PARAMS_CMDLINE_OFF + 16]
            .copy_from_slice(&cmdline_ptr.to_le_bytes());
        // Contexto global só no fim (passo 7): `teb_ptr` exige o TEB criado.
        // Até lá, `mem`/`fsys` vivem como locais (passo 5b usa direto).
        let mem = nt_memory::MemoryManager::new();
        let fsys = nt_file::DriveMap::new(capsule.drives.clone(), capsule.current_dir.clone());

        // 2. Mapeia a imagem.
        let (mapped, img) = loader::map_image(pe_bytes)?;
        let base = mapped.base;
        let size = mapped.size;

        // 3. Relocs (exige RW — o mapeamento nasce RW).
        // SAFETY: `base`..`+size` mapeado RW pelo passo 2; blocos do parse.
        let relocs = img.relocs()?;
        unsafe { nt_loader::apply_relocs(base, size, mapped.image_base_preferred, &relocs)? };

        // 4. Imports → IAT.
        let (_descs, symbols) = img.imports()?;
        // SAFETY: IAT dentro da imagem RW; resolvedor retorna endereços
        // `extern "win64"` válidos (funções do próprio processo host).
        unsafe {
            nt_loader::resolve_imports(base, size, &symbols, |dll, name, ord| {
                resolve_import(dll, name, ord)
            })?;
        }

        // 5. Proteção final por seção.
        loader::protect_sections(&mapped, &img)?;

        // 5b. Registra a imagem no gerente de memória para que
        // `VirtualAlloc` nunca colida com ela (fonte única de regiões).
        {
            let base = mapped.base as usize;
            mem.track_external(
                base,
                img.size_of_headers as usize,
                winabi::PageProtect::READONLY,
            );
            for s in &img.sections {
                let len = s.virtual_size.max(s.raw_size) as usize;
                if len == 0 {
                    continue;
                }
                mem.track_external(
                    base + s.virtual_address as usize,
                    len,
                    nt_memory::section_protect(s.characteristics),
                );
            }
        }

        // 6. PEB/TEB observáveis.
        let image_base = base as u64;
        let params_ptr = &*params as *const [u8; 0x80] as u64;
        let mut peb = Box::new(PebMinimal {
            image_base,
            process_parameters: params_ptr,
            loader_data: 0,
        });
        let peb_ptr = &*peb as *const PebMinimal as u64;
        let mut teb = Box::new(TebMinimal {
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
            peb_ptr,
            image_base,
            tls_slots: [0; winabi::TLS_SLOTS],
        });
        teb.tib.teb_self = &*teb as *const TebMinimal as u64;
        // `peb` deve ficar pinned: movemos para o struct (endereço estável
        // após o Box; `peb_ptr` acima já capturou o endereço do Box).
        let _ = &mut peb;

        // 7. Contexto global das façades (TEB já existe: `teb_ptr` estável).
        ntdll::install_context(Arc::new(ntdll::ProcessContext {
            table: table.clone(),
            stdout_handle: stdout,
            stderr_handle: stderr,
            cmdline_ptr,
            teb_ptr: &*teb as *const TebMinimal as u64,
            mem,
            fsys,
            image_base,
            tls_bitmap: nt_thread::TlsBitmap::new(),
            unhandled_filter: std::sync::atomic::AtomicU64::new(0),
        }));

        Ok(Self {
            capsule,
            mapped: Some(mapped),
            teb,
            peb,
            image_base,
            _cmdline_w: cmdline_w,
            _params_block: params,
        })
    }

    /// Capsule que originou este processo (drives, versão, quirks).
    pub fn capsule(&self) -> &Capsule {
        &self.capsule
    }

    pub fn image_base(&self) -> u64 {
        self.image_base
    }

    /// Ponteiro do PEB observável (mantido vivo pelo `Box` em `self.peb`).
    pub fn peb_address(&self) -> u64 {
        &*self.peb as *const PebMinimal as u64
    }

    pub fn entry_point(&self) -> *mut u8 {
        self.mapped
            .as_ref()
            .expect("emulator sem imagem")
            .entry_point()
    }

    /// Executa o entry point no CPU real. Não retorna se o guest chamar
    /// `ExitProcess` (o host termina com o mesmo código — modelo v0.1).
    ///
    /// SAFETY (fronteira ABI+GS):
    /// - `teb` vive no `Box` do `self` durante toda a chamada; GS aponta
    ///   para ele e é restaurado no retorno normal.
    /// - o entry é código x86_64 do PE com convenção Windows, sem args;
    ///   a CALL preserva o alinhamento RSP%16==8 esperado.
    /// - violação (guest corrompe stack/GS): falha contida no processo host
    ///   (único processo em v0.1); v0.4 isolará via fork/namespaces.
    pub fn enter(&mut self) -> Result<u32, RuntimeError> {
        let teb_ptr = &*self.teb as *const TebMinimal as *const u8;
        let prev_gs = host_linux::set_gs_base(teb_ptr)?;
        let entry = self.entry_point();

        // Chama como `extern "win64" fn()`.
        type WinEntry = extern "win64" fn();
        let f: WinEntry = unsafe { std::mem::transmute(entry) };
        // Se o guest retornar (incomum para EXE), restauramos GS e damos 0.
        f();
        let _ = host_linux::restore_gs_base(prev_gs);
        Ok(0)
    }
}

/// Loop de demanda: lista TODOS os imports sem implementação no Rine
/// (sem duplicatas, em ordem). Usado pelo launcher para imprimir o relatório
/// `RINE-DEMAND` quando o load falha — a demanda real que prioriza o
/// próximo trabalho (ver `docs/roadmap.md`). Nunca falha por import ausente.
pub fn missing_imports(pe_bytes: &[u8]) -> Result<Vec<(String, String)>, RuntimeError> {
    let img = pe::Image::parse(pe_bytes)?;
    let (_descs, symbols) = img.imports()?;
    let mut missing = Vec::new();
    for s in &symbols {
        if resolve_import(&s.dll, s.name.as_deref(), s.ordinal).is_none() {
            let name = s
                .name
                .clone()
                .unwrap_or_else(|| format!("#{}", s.ordinal.unwrap_or(0)));
            let entry = (s.dll.clone(), name);
            if !missing.contains(&entry) {
                missing.push(entry);
            }
        }
    }
    Ok(missing)
}

/// Relatório `RINE-DEMAND` pronto para stderr (uma linha por import).
pub fn demand_report(pe_bytes: &[u8]) -> String {
    match missing_imports(pe_bytes) {
        Ok(list) if list.is_empty() => "rine: todos os imports resolvem?! (bug no loader)".into(),
        Ok(list) => {
            let mut s = format!(
                "rine: demanda total ({} imports sem implementação):\n",
                list.len()
            );
            for (dll, name) in list {
                s.push_str(&format!("  {dll}!{name}\n"));
            }
            s
        }
        Err(e) => format!("rine: nem listar imports foi possível: {e}"),
    }
}
/// Resolvedor de imports: kernel32 + ntdll implementados (+ ApiSet → host).
/// Retorna endereço host da implementação `extern "win64"`.
fn resolve_import(dll: &str, name: Option<&str>, ordinal: Option<u16>) -> Option<u64> {
    // Item 9: namespace ApiSet roteia para o host (`api-ms-win-core-synch…
    // → kernel32); o símbolo resolve (ou falta honestamente) no destino.
    let is_kernel32 = matches!(
        kernel32::modules::resolve_dll(dll),
        Some(kernel32::modules::KnownModule::Kernel32)
    );
    let dll_l = dll.to_ascii_lowercase();
    let is_ntdll = dll_l == "ntdll.dll"
        || dll_l == "ntdll"
        || matches!(
            kernel32::modules::resolve_dll(dll),
            Some(kernel32::modules::KnownModule::Ntdll)
        );
    if is_kernel32 {
        if let Some(n) = name {
            // kernel32 exporta também por nome exato (case-sensitive no Windows
            // para GetProcAddress; imports por nome do linker são exatos).
            if let Some(a) = kernel32::resolve(dll, n) {
                tracing::debug!("import resolvido: {dll}!{n}");
                return Some(a);
            }
        }
        tracing::debug!("import AUSENTE: {dll}!{}", name.unwrap_or("?"));
        return None;
    }
    if is_ntdll {
        match (name, ordinal) {
            (Some("RtlExitUserProcess"), _) => {
                Some(ntdll::RtlExitUserProcess_impl as *const () as u64)
            }
            (Some("NtTerminateProcess"), _) => {
                Some(ntdll::NtTerminateProcess_impl as *const () as u64)
            }
            _ => None,
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demand_lists_only_unresolved() {
        // PE sintético: 1 import inexistente + 1 existente.
        // (`CreateFileW` já foi esse exemplo — virou implementado em v0.3;
        // `FindFirstFileW` segue em demanda. A troca é intencional.)
        let r =
            pe::builder::build_rdata_generic("KERNEL32.dll", &["FindFirstFileW", "WriteFile"], &[]);
        let mut code = vec![0xC3u8];
        while code.len() < 0x200 {
            code.push(0xCC);
        }
        let vsize = code.len() as u32;
        let bytes = pe::builder::assemble(&code, &r.bytes, vsize, r.import_dir, 40, r.iats[0], 24);
        let missing = missing_imports(&bytes).unwrap();
        assert_eq!(
            missing,
            vec![("KERNEL32.dll".to_string(), "FindFirstFileW".to_string())]
        );
        // WriteFile resolve → fora da lista. Suite real: lista vazia.
        let suite = pe::builder::build_suite_exe();
        assert!(missing_imports(&suite).unwrap().is_empty());
    }

    #[test]
    fn load_resolves_all_imports() {
        let bytes = pe::builder::build_minimal_hello();
        let emu = Emulator::load(Capsule::default(), &bytes, "hello.exe").expect("load");
        assert!(emu.image_base() != 0);
        // IAT[0] deve apontar para o GetStdHandle real (não mais Hint RVA).
        let iat0 = unsafe { *((emu.image_base() + 0x2090) as *const u64) };
        assert_ne!(iat0, 0x0014_0000_0000 + 0x2040);
        assert_eq!(iat0, kernel32::GetStdHandle_impl as *const () as u64);
    }

    /// Item 9 (ApiSet): PE que importa `Sleep` via namespace
    /// (`api-ms-win-core-synch-ansi-l1-1-0`, host kernel32 no mapa real)
    /// carrega e a IAT aponta para o `Sleep` de verdade — mesma prova do
    /// teste acima, exercendo a rota ApiSet→host de ponta a ponta.
    #[test]
    fn load_routes_apiset_to_host() {
        let r = pe::builder::build_rdata_generic(
            "API-MS-WIN-CORE-SYNCH-ANSI-L1-1-0.DLL",
            &["Sleep"],
            &[],
        );
        let mut code = vec![0xC3u8];
        while code.len() < 0x200 {
            code.push(0xCC);
        }
        let vsize = code.len() as u32;
        let iat0_rva = r.iats[0];
        let bytes = pe::builder::assemble(&code, &r.bytes, vsize, r.import_dir, 40, r.iats[0], 24);
        // Sem demanda: o namespace roteou e o símbolo existe no host.
        assert!(missing_imports(&bytes).unwrap().is_empty());
        let emu = Emulator::load(Capsule::default(), &bytes, "apiset.exe").expect("load");
        let iat0 = unsafe { *((emu.image_base() + iat0_rva as u64) as *const u64) };
        assert_eq!(iat0, kernel32::Sleep_impl as *const () as u64);
        // ApiSet→UCRT continua honesto: namespace resolve, símbolo falta.
        let r2 =
            pe::builder::build_rdata_generic("api-ms-win-crt-heap-l1-1-0.dll", &["malloc"], &[]);
        let bytes2 =
            pe::builder::assemble(&code, &r2.bytes, vsize, r2.import_dir, 40, r2.iats[0], 24);
        assert_eq!(
            missing_imports(&bytes2).unwrap(),
            vec![(
                "api-ms-win-crt-heap-l1-1-0.dll".to_string(),
                "malloc".to_string()
            )]
        );
    }
}
