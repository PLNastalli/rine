//! Registro de módulos conhecidos (`GetProcAddress`, futuro `GetModuleHandle`).
//!
//! Vive em `kernel32` (e não em `kernelbase`/`nt-loader`) por um motivo
//! concreto: os endereços resolvidos AQUI — `X_impl as *const () as u64` —
//! nascem neste crate, e `kernelbase` não pode depender de `kernel32`
//! (ciclo). Quando `LoadLibrary` chegar (v0.4), este registro estático
//! vira a semente do carregador dinâmico em `nt-loader`/`runtime`
//! (TODO rastreável: condição = `LoadLibraryA` implementado; então
//! `modules` delega à lista de módulos mapeados e os pseudo-`HMODULE`s
//! de `winabi` se aposentam).
//!
//! Lógica pura e tipada (testável sem guest); a façade `extern "win64"`
//! em `lib.rs` só converte ABI.

use winabi::Win32Error;

/// Tabela ApiSet gerada (arquivo próprio por tamanho + regeneração limpa).
#[path = "apiset_table.rs"]
mod apiset_table;

/// Módulo conhecido pelo runtime (hoje: só os implementados internamente).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownModule {
    Kernel32,
    Ntdll,
}

/// `HMODULE` opaco → módulo. `0`/desconhecido = `None` (o caller vira
/// `NULL` + `ERROR_MOD_NOT_FOUND`, como no Windows).
pub fn lookup(hmodule: u64) -> Option<KnownModule> {
    match hmodule {
        winabi::KERNEL32_PSEUDO_BASE => Some(KnownModule::Kernel32),
        winabi::NTDLL_PSEUDO_BASE => Some(KnownModule::Ntdll),
        _ => None,
    }
}

/// `HMODULE` do módulo (`GetModuleHandle`/`LoadLibrary` o devolvem,
/// `GetProcAddress`/`FreeLibrary` o aceitam de volta).
pub fn pseudo_of(module: KnownModule) -> u64 {
    match module {
        KnownModule::Kernel32 => winabi::KERNEL32_PSEUDO_BASE,
        KnownModule::Ntdll => winabi::NTDLL_PSEUDO_BASE,
    }
}

/// Normaliza nome de módulo como o loader Windows: ignora caminho
/// (`C:\x\kernel32.dll`, `...\kernel32`), case-insensitive, com ou sem
/// extensão `.dll`. ApiSets (`api-ms-win-*`) NÃO resolvem aqui (item 9
/// da demanda — `None` honesto até lá, nunca chute).
pub fn normalize(name: &str) -> Option<KnownModule> {
    let file = name
        .rsplit(['\\', '/', ':'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase();
    let base = file.strip_suffix(".dll").unwrap_or(&file);
    match base {
        "kernel32" => Some(KnownModule::Kernel32),
        "ntdll" => Some(KnownModule::Ntdll),
        _ => None,
    }
}

/// Resolução de DLL (item 9 da demanda): nome direto OU namespace ApiSet.
/// `api-ms-win-core-synch-l1-1-0.dll` → `KnownModule::Kernel32`, e o símbolo
/// resolve no de sempre. Fora do namespace implementado → `None` (o caller
/// vira `MOD_NOT_FOUND` honesto — UCRT/USER32/etc. seguem em demanda real).
pub fn resolve_dll(dll: &str) -> Option<KnownModule> {
    normalize(dll).or_else(|| apiset_table::apiset_host(dll))
}

/// `GetModuleHandle(nome?) -> HMODULE`.
/// Usa `resolve_dll` (nomes diretos + ApiSet; ver item 9).
///
/// - `None` (NULL) = imagem do processo (`exe_base` do contexto — nunca
///   falha no Windows; aqui devolve o valor do contexto como está).
/// - Nome conhecido → pseudo-`HMODULE` opaco (tokens `winabi`, nunca
///   dereferenciáveis — ver `docs/compatibility.md`).
/// - Desconhecido → `MOD_NOT_FOUND` (126).
pub fn get_module_handle(name: Option<&str>, exe_base: u64) -> Result<u64, Win32Error> {
    match name {
        None => Ok(exe_base),
        Some(n) => resolve_dll(n)
            .map(pseudo_of)
            .ok_or(Win32Error::MOD_NOT_FOUND),
    }
}

/// `LoadLibrary(nome) -> HMODULE` (v0.3: só o conjunto já carregado).
/// Usa `resolve_dll` (nomes diretos + ApiSet).
///
/// Módulo conhecido → pseudo-handle (no Windows, carregar o já carregado
/// só incrementa a refcount e devolve o handle — observavelmente igual).
/// Carregar imagens NOVAS do disco é o loader dinâmico (v0.4); pedir uma
/// agora retorna `MOD_NOT_FOUND` honesto em vez de stub de sucesso.
pub fn load_library(name: &str) -> Result<u64, Win32Error> {
    resolve_dll(name)
        .map(pseudo_of)
        .ok_or(Win32Error::MOD_NOT_FOUND)
}

/// `FreeLibrary(h)`.
///
/// Conjunto estático nunca descarrega (como `kernel32.dll` pinada no
/// Windows): pseudo-handles e a base do EXE devolvem `Ok` (TRUE).
/// Handle desconhecido → `MOD_NOT_FOUND` (nunca TRUE falso).
pub fn free_library(hmodule: u64, exe_base: u64) -> Result<(), Win32Error> {
    if lookup(hmodule).is_some() || (exe_base != 0 && hmodule == exe_base) {
        Ok(())
    } else {
        Err(Win32Error::MOD_NOT_FOUND)
    }
}

/// Ordinais reais (Windows 11 25H2, via `rine-api-scan` em
/// `api-db/windows-11-25h2-x64/dlls/*.json` — metadado ABI, fato
/// observável, não código proprietário). Só os exports que o Rine
/// implementa; ordinal sem entrada = `PROC_NOT_FOUND` honesto.
const KERNEL32_ORDINALS: &[(u16, &str)] = &[
    (772, "GetStdHandle"),
    (1633, "WriteFile"),
    (1194, "ReadFile"),
    (223, "CreateFileA"),
    (158, "CloseHandle"),
    (1557, "VirtualAlloc"),
    (1560, "VirtualFree"),
    (1563, "VirtualProtect"),
    (513, "GetCommandLineW"),
    (1515, "TlsAlloc"),
    (1516, "TlsFree"),
    (1517, "TlsGetValue"),
    (1519, "TlsSetValue"),
    (654, "GetLastError"),
    (1481, "Sleep"),
    (918, "InitializeCriticalSection"),
    (304, "DeleteCriticalSection"),
    (344, "EnterCriticalSection"),
    (1009, "LeaveCriticalSection"),
    (1465, "SetUnhandledExceptionFilter"),
    (1565, "VirtualQuery"),
    (734, "GetProcAddress"),
    (391, "ExitProcess"),
];

/// Resolve `nome` no módulo (case-sensitive, como o Windows em
/// `GetProcAddress`; imports do linker já são exatos).
pub fn resolve_by_name(module: KnownModule, name: &str) -> Option<u64> {
    match module {
        KnownModule::Kernel32 => super::resolve("KERNEL32.dll", name),
        KnownModule::Ntdll => super::ntdll_addr(name),
    }
}

/// Resolve ordinal no módulo (`HIWORD(lpProcName)==0` no Windows).
pub fn resolve_by_ordinal(module: KnownModule, ordinal: u16) -> Option<u64> {
    let table: &[(u16, &str)] = match module {
        KnownModule::Kernel32 => KERNEL32_ORDINALS,
        KnownModule::Ntdll => NTDLL_ORDINALS,
    };
    let name = table.iter().find(|(o, _)| *o == ordinal)?.1;
    resolve_by_name(module, name)
}

const NTDLL_ORDINALS: &[(u16, &str)] = &[
    (1000, "RtlExitUserProcess"),
    (658, "NtTerminateProcess"),
    (1190, "RtlInitializeCriticalSection"),
    (915, "RtlDeleteCriticalSection"),
    (977, "RtlEnterCriticalSection"),
    (1282, "RtlLeaveCriticalSection"),
];

/// Forwarders `kernel32→ntdll` (item 10; ordinais/símbolos do oracle Win11
/// 25H2 — `api-db/.../KERNEL32.json`, campo `forwarder`). Como no Windows,
/// estes nomes NÃO têm código em kernel32: `resolve()` persegue a cadeia
/// até o endereço ntdll (limite 1 salto em v0 — cadeia mais longa = futuro
/// documentado, nunca recursão aberta).
const FORWARDERS: &[(&str, &str)] = &[
    ("InitializeCriticalSection", "RtlInitializeCriticalSection"),
    ("DeleteCriticalSection", "RtlDeleteCriticalSection"),
    ("EnterCriticalSection", "RtlEnterCriticalSection"),
    ("LeaveCriticalSection", "RtlLeaveCriticalSection"),
];

/// Persegue um forwarder `kernel32!nome → ntdll!alvo`.
/// `None` = nome local (código próprio) ou forwarder sem alvo implementado.
pub fn forward(name: &str) -> Option<u64> {
    let target = FORWARDERS.iter().find(|(n, _)| *n == name)?.1;
    super::ntdll_addr(target)
}

/// `GetProcAddress(h, nome|ordinal) -> endereço` ou erro Win32.
///
/// - Módulo desconhecido → `MOD_NOT_FOUND` (126).
/// - Símbolo/ordinal ausente → `PROC_NOT_FOUND` (127).
/// - Sucesso nunca falha (endereço host válido, chamável pelo guest).
pub fn get_proc_address(
    hmodule: u64,
    proc_name: Option<&str>,
    ordinal: Option<u16>,
) -> Result<u64, Win32Error> {
    let module = lookup(hmodule).ok_or(Win32Error::MOD_NOT_FOUND)?;
    let addr = match (proc_name, ordinal) {
        (Some(n), _) => resolve_by_name(module, n),
        (None, Some(o)) => resolve_by_ordinal(module, o),
        (None, None) => None,
    };
    addr.ok_or(Win32Error::PROC_NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names_resolve_to_real_addresses() {
        // `WriteFile` por nome == endereço da IAT (o guest compara igual).
        let a = get_proc_address(winabi::KERNEL32_PSEUDO_BASE, Some("WriteFile"), None).unwrap();
        assert_eq!(a, super::super::WriteFile_impl as *const () as u64);
        assert_ne!(a, 0);
        // Nomes são case-sensitive como no Windows.
        assert!(get_proc_address(winabi::KERNEL32_PSEUDO_BASE, Some("writefile"), None).is_err());
    }

    #[test]
    fn real_ordinals_resolve() {
        // 1633 = WriteFile no Windows 11 25H2 (oracle).
        let by_ord = get_proc_address(winabi::KERNEL32_PSEUDO_BASE, None, Some(1633)).unwrap();
        let by_name =
            get_proc_address(winabi::KERNEL32_PSEUDO_BASE, Some("WriteFile"), None).unwrap();
        assert_eq!(by_ord, by_name);
        // NTDLL também: 1000 = RtlExitUserProcess.
        assert!(get_proc_address(winabi::NTDLL_PSEUDO_BASE, None, Some(1000)).is_ok());
    }

    #[test]
    fn unknown_module_symbol_ordinal_fail_clean() {
        assert_eq!(
            get_proc_address(0xDEAD_BEEF, Some("WriteFile"), None),
            Err(Win32Error::MOD_NOT_FOUND)
        );
        assert_eq!(
            get_proc_address(0, Some("WriteFile"), None),
            Err(Win32Error::MOD_NOT_FOUND)
        );
        assert_eq!(
            get_proc_address(winabi::KERNEL32_PSEUDO_BASE, Some("NoSuchExportZZZ"), None),
            Err(Win32Error::PROC_NOT_FOUND)
        );
        assert_eq!(
            get_proc_address(winabi::KERNEL32_PSEUDO_BASE, None, Some(1)),
            Err(Win32Error::PROC_NOT_FOUND)
        ); // ordinal de outro export
        assert_eq!(
            get_proc_address(winabi::KERNEL32_PSEUDO_BASE, None, None),
            Err(Win32Error::PROC_NOT_FOUND)
        );
    }

    #[test]
    fn module_names_normalize_like_windows() {
        let k32 = winabi::KERNEL32_PSEUDO_BASE;
        let ntdll = winabi::NTDLL_PSEUDO_BASE;
        // Case-insensitive, com/sem extensão, com caminho.
        for n in [
            "kernel32.dll",
            "KERNEL32.DLL",
            "kernel32",
            "C:\\Windows\\System32\\kernel32.dll",
            "C:/Windows/System32/KERNEL32",
        ] {
            assert_eq!(normalize(n), Some(KnownModule::Kernel32), "{n}");
            assert_eq!(get_module_handle(Some(n), 0), Ok(k32), "{n}");
            assert_eq!(load_library(n), Ok(k32), "{n}");
        }
        assert_eq!(normalize("ntdll.dll"), Some(KnownModule::Ntdll));
        assert_eq!(get_module_handle(Some("NTDLL"), 0), Ok(ntdll));
        // `normalize` é só nomes diretos (ApiSet vive em `resolve_dll`).
        assert_eq!(normalize("api-ms-win-crt-heap-l1-1-0.dll"), None);
        // ApiSet→UCRT: namespace resolve, host não implementado → 126 honesto.
        assert_eq!(
            get_module_handle(Some("api-ms-win-crt-heap-l1-1-0.dll"), 0),
            Err(Win32Error::MOD_NOT_FOUND)
        );
        assert_eq!(load_library("user32.dll"), Err(Win32Error::MOD_NOT_FOUND));
    }

    #[test]
    fn apiset_routes_sample() {
        // Núcleo do item 9: sets reais roteiam para o host implementado
        // (namespaces copiados do snapshot — nunca inventar versões).
        let k32 = winabi::KERNEL32_PSEUDO_BASE;
        assert_eq!(
            get_module_handle(Some("api-ms-win-core-synch-ansi-l1-1-0.dll"), 0),
            Ok(k32)
        );
        assert_eq!(
            get_module_handle(Some("API-MS-WIN-core-SYNCH-ansi-L1-1-0"), 0),
            Ok(k32)
        ); // case-insensitive, sem .dll
        assert_eq!(load_library("api-ms-win-core-memory-l1-1-9.dll"), Ok(k32)); // host kernelbase → superfície kernel32
        assert_eq!(
            get_module_handle(Some("api-ms-win-core-rtlsupport-l1-1-1.dll"), 0),
            Ok(winabi::NTDLL_PSEUDO_BASE)
        );
        assert_eq!(
            get_module_handle(Some("ext-ms-win-ntuser-synch-l1-1-0.dll"), 0),
            Err(Win32Error::MOD_NOT_FOUND)
        ); // ext-ms-* fora do mapa v0
    }

    /// Conformidade: a tabela gerada == fixture `apiset-map.json` para os
    /// hosts implementados (kernel32/kernelbase→Kernel32, ntdll→Ntdll), e
    /// NENHUM namespace de outro host entra na tabela. Roda o snapshot?
    /// Regenere a tabela (cabeçalho de `apiset_table.rs`).
    #[test]
    fn apiset_table_matches_snapshot() {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../api-db/windows-11-25h2-x64/apiset-map.json");
        let text = std::fs::read_to_string(&p).expect("fixture apiset-map.json");
        let v: serde_json::Value = serde_json::from_str(&text).expect("JSON válido");
        let mut rows = 0;
        for r in v["routes"].as_array().expect("routes") {
            let ns = r["namespace"].as_str().expect("namespace");
            let host = r["host"].as_str().expect("host").to_ascii_lowercase();
            let want = match host.as_str() {
                "kernel32.dll" | "kernelbase.dll" => Some(KnownModule::Kernel32),
                "ntdll.dll" => Some(KnownModule::Ntdll),
                _ => None,
            };
            assert_eq!(
                super::apiset_table::apiset_host(&format!("{ns}.dll")),
                want,
                "drift em {ns} (host {host})"
            );
            if want.is_some() {
                rows += 1;
            }
        }
        // A tabela deve cobrir exatamente as rotas implementáveis (nem a
        // mais — UCRT/USER32 fingindo — nem a menos).
        assert_eq!(
            rows,
            super::apiset_table::APISET_TABLE.len(),
            "tabela incompleta"
        );
    }

    #[test]
    fn null_module_is_the_exe_base() {
        // `GetModuleHandle(NULL)` = base da imagem (nunca falha no Windows).
        assert_eq!(
            get_module_handle(None, 0x0014_0000_0000),
            Ok(0x0014_0000_0000)
        );
        // `FreeLibrary` no conjunto estático = TRUE; fora = erro, nunca TRUE falso.
        assert!(free_library(winabi::KERNEL32_PSEUDO_BASE, 0).is_ok());
        assert!(free_library(0x0014_0000_0000, 0x0014_0000_0000).is_ok());
        assert_eq!(
            free_library(0xDEAD_BEEF, 0x0014_0000_0000),
            Err(Win32Error::MOD_NOT_FOUND)
        );
    }

    #[test]
    fn critical_sections_forward_to_ntdll() {
        // Item 10: como no Windows, `kernel32!EnterCriticalSection` resolve
        // para o endereço de `ntdll!RtlEnterCriticalSection` (sem código
        // local — a lógica única vive em `nt_sync` via ntdll).
        for (k32, rtl) in [
            ("InitializeCriticalSection", "RtlInitializeCriticalSection"),
            ("DeleteCriticalSection", "RtlDeleteCriticalSection"),
            ("EnterCriticalSection", "RtlEnterCriticalSection"),
            ("LeaveCriticalSection", "RtlLeaveCriticalSection"),
        ] {
            let via_k32 = super::super::resolve("KERNEL32.dll", k32).unwrap();
            let via_ntdll = super::super::ntdll_addr(rtl).unwrap();
            assert_eq!(via_k32, via_ntdll, "{k32}");
            assert_ne!(via_k32, 0);
            // GetProcAddress pelo pseudo-HMODULE enxerga o mesmo endereço.
            assert_eq!(
                get_proc_address(winabi::KERNEL32_PSEUDO_BASE, Some(k32), None).unwrap(),
                via_ntdll,
                "{k32}"
            );
        }
        // Ordinal do forwarder também persegue (344 = EnterCriticalSection
        // na KERNEL32; 977 é o ordinal do ALVO na NTDLL — domínios distintos).
        assert_eq!(
            get_proc_address(winabi::KERNEL32_PSEUDO_BASE, None, Some(344)).unwrap(),
            super::super::ntdll_addr("RtlEnterCriticalSection").unwrap()
        );
    }
}
