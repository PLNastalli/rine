//! Construtor de PEs mínimos para testes.
//!
//! Gera um `hello.exe` PE32+ x86_64 válido, sem toolchain Windows,
//! com imports `KERNEL32.dll!{GetStdHandle,WriteFile,ExitProcess}`
//! e entry point em hand-assembled Windows-x64 que imprime `Hello World\n`.
//!
//! Layout:
//! ```text
//! headers (0x200 file-aligned)
//! .text VA 0x1000 (código RIP-relative, sem relocs obrigatórios)
//! .rdata VA 0x2000 (msg, dll name, Hint/Name, ILT/IAT, ImportDir)
//! ImageBase preferencial 0x140000000
//! ```

pub(crate) const IMAGE_BASE: u64 = 0x0014_0000_0000;
const SECT_ALIGN: u32 = 0x1000;
const FILE_ALIGN: u32 = 0x200;
pub(crate) const TEXT_RVA: u32 = 0x1000;
pub(crate) const RDATA_RVA: u32 = 0x2000;

fn w16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn w32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn w64(v: &mut Vec<u8>, x: u64) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// Monta o `.text` com RIP-relatives calculados.
///
/// Convenção de um compilador real: o `written` (`LPDWORD`) é um local de
/// stack (`[rsp+0x28]`, acima do shadow space), NUNCA um global em `.rdata`
/// — `.rdata` é read-only após `protect_sections`, como no Windows real.
/// Frame: `sub rsp,0x38` (56 bytes: 32 shadow + 8 quinto arg + 8 written +
/// 8 pad), mantendo RSP 16-alinhado antes de cada CALL.
fn build_text() -> Vec<u8> {
    let mut c: Vec<u8> = Vec::new();
    // sub rsp,0x38
    c.extend_from_slice(&[0x48, 0x83, 0xEC, 0x38]);
    // mov ecx, -11 (0xFFFFFFF5)
    c.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]);

    let iat0 = RDATA_RVA + 0x90; // GetStdHandle
    let iat1 = RDATA_RVA + 0x98; // WriteFile
    let iat2 = RDATA_RVA + 0xA0; // ExitProcess
    let msg = RDATA_RVA;

    // helper para rel32: alvo_rva - (text_rva + next_off)
    let rel = |target: u32, next_off: usize| -> i32 {
        (target as i64 - (TEXT_RVA as i64 + next_off as i64)) as i32
    };

    // call qword [rel iat0]; next = 0x0F
    c.extend_from_slice(&[0xFF, 0x15]);
    c.extend_from_slice(&rel(iat0, 0x0F).to_le_bytes());
    // mov rcx, rax
    c.extend_from_slice(&[0x48, 0x89, 0xC1]);
    // lea rdx,[rel msg]; next = 0x19
    c.extend_from_slice(&[0x48, 0x8D, 0x15]);
    c.extend_from_slice(&rel(msg, 0x19).to_le_bytes());
    // mov r8d, 12
    c.extend_from_slice(&[0x41, 0xB8, 0x0C, 0x00, 0x00, 0x00]);
    // lea r9,[rsp+0x28]  (written local; 5 bytes: 4C 8D 4C 24 28)
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x28]);
    // mov qword [rsp+0x20], 0  (lpOverlapped = NULL)
    c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]);
    // call qword [rel iat1]; next = 0x33
    c.extend_from_slice(&[0xFF, 0x15]);
    c.extend_from_slice(&rel(iat1, 0x33).to_le_bytes());
    // xor ecx, ecx
    c.extend_from_slice(&[0x31, 0xC9]);
    // call qword [rel iat2]; next = 0x3B
    c.extend_from_slice(&[0xFF, 0x15]);
    c.extend_from_slice(&rel(iat2, 0x3B).to_le_bytes());
    // jmp $ (fallback) ; ret
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);

    assert_eq!(c.len(), 0x3E, "tamanho do código inesperado");
    while c.len() < 0x200 {
        c.push(0xCC); // int3 pad
    }
    c
}

/// Monta o `.rdata` (0x200 bytes).
fn build_rdata() -> Vec<u8> {
    let mut r = vec![0u8; 0x200];
    // msg @0x00
    r[0x00..0x0C].copy_from_slice(b"Hello World\n");
    // dll name @0x20
    r[0x20..0x2D].copy_from_slice(b"KERNEL32.dll\0");
    // Hint/Name @0x40 GetStdHandle
    r[0x40] = 0;
    r[0x41] = 0;
    r[0x42..0x42 + 13].copy_from_slice(b"GetStdHandle\0");
    // Hint/Name @0x50 WriteFile
    r[0x50] = 0;
    r[0x51] = 0;
    r[0x52..0x52 + 10].copy_from_slice(b"WriteFile\0");
    // Hint/Name @0x60 ExitProcess
    r[0x60] = 0;
    r[0x61] = 0;
    r[0x62..0x62 + 12].copy_from_slice(b"ExitProcess\0");

    let put64 = |r: &mut Vec<u8>, off: usize, v: u64| {
        r[off..off + 8].copy_from_slice(&v.to_le_bytes());
    };
    let hn0 = (RDATA_RVA + 0x40) as u64;
    let hn1 = (RDATA_RVA + 0x50) as u64;
    let hn2 = (RDATA_RVA + 0x60) as u64;
    // ILT @0x70
    put64(&mut r, 0x70, hn0);
    put64(&mut r, 0x78, hn1);
    put64(&mut r, 0x80, hn2);
    put64(&mut r, 0x88, 0);
    // IAT @0x90 (inicialmente igual; loader patcha)
    put64(&mut r, 0x90, hn0);
    put64(&mut r, 0x98, hn1);
    put64(&mut r, 0xA0, hn2);
    put64(&mut r, 0xA8, 0);
    // Import Directory @0xB0
    let put32 = |r: &mut Vec<u8>, off: usize, v: u32| {
        r[off..off + 4].copy_from_slice(&v.to_le_bytes());
    };
    put32(&mut r, 0xB0, RDATA_RVA + 0x70); // OriginalFirstThunk
    put32(&mut r, 0xB4, 0);
    put32(&mut r, 0xB8, 0);
    put32(&mut r, 0xBC, RDATA_RVA + 0x20); // Name
    put32(&mut r, 0xC0, RDATA_RVA + 0x90); // FirstThunk
                                           // desc nulo @0xC4..0xD7 já zero
                                           // written dword @0xD8 já zero
    r
}

/// Constrói o PE `hello.exe` completo como bytes.
/// Congelado como âncora de regressão: não alterar layout sem atualizar
/// o teste E2E + esta documentação.
pub fn build_minimal_hello() -> Vec<u8> {
    let text = build_text();
    let rdata = build_rdata();
    assemble(
        &text,
        &rdata,
        0x40,
        RDATA_RVA + 0xB0,
        40,
        RDATA_RVA + 0x90,
        32,
    )
}

/// Monta headers + seções ao redor de `.text`/`.rdata` prontos.
/// Parâmetros: tamanho virtual real do código + RVA/tamanho do Import
/// Directory e da IAT. Tamanhos raw arredondados para FILE_ALIGN.
/// Único emissor de headers — todo PE de teste passa por aqui.
pub fn assemble(
    text: &[u8],
    rdata: &[u8],
    text_vsize: u32,
    import_rva: u32,
    import_size: u32,
    iat_rva: u32,
    iat_size: u32,
) -> Vec<u8> {
    let text_raw = align_up(text.len(), FILE_ALIGN as usize);
    let rdata_raw = align_up(rdata.len(), FILE_ALIGN as usize);
    assert!(
        TEXT_RVA as usize + text_raw <= RDATA_RVA as usize,
        "text invade rdata"
    );
    assert!(
        RDATA_RVA as usize + rdata_raw <= 0x3000,
        "rdata além da imagem"
    );

    let mut out: Vec<u8> = Vec::new();
    // --- DOS header (64 bytes) ---
    out.extend_from_slice(&[0x4D, 0x5A]); // MZ
    out.resize(0x3C, 0);
    w32(&mut out, 0x80); // e_lfanew
    out.resize(0x80, 0);
    // --- PE sig ---
    out.extend_from_slice(b"PE\0\0");
    // --- COFF (20) ---
    w16(&mut out, 0x8664); // Machine AMD64
    w16(&mut out, 2); // NumberOfSections
    w32(&mut out, 0); // TimeDateStamp
    w32(&mut out, 0); // PointerToSymbolTable
    w32(&mut out, 0); // NumberOfSymbols
    w16(&mut out, 240); // SizeOfOptionalHeader (112 + 16*8)
    w16(&mut out, 0x0022); // Characteristics EXECUTABLE|LINE_NUMS_STRIPPED? usa 0x22 típico
                           // --- Optional header PE32+ (112 + dirs) ---
    w16(&mut out, 0x20B); // Magic PE32+
    out.push(14);
    out.push(0); // linker ver
    w32(&mut out, text_raw as u32); // SizeOfCode
    w32(&mut out, rdata_raw as u32); // SizeOfInitializedData
    w32(&mut out, 0); // SizeOfUninitializedData
    w32(&mut out, TEXT_RVA); // AddressOfEntryPoint
    w32(&mut out, TEXT_RVA); // BaseOfCode
    w64(&mut out, IMAGE_BASE); // ImageBase
    w32(&mut out, SECT_ALIGN); // SectionAlignment
    w32(&mut out, FILE_ALIGN); // FileAlignment
    w16(&mut out, 6);
    w16(&mut out, 0); // OS ver
    w16(&mut out, 0);
    w16(&mut out, 0); // image ver
    w16(&mut out, 6);
    w16(&mut out, 0); // subsystem ver
    w32(&mut out, 0); // Win32VersionValue
    w32(&mut out, 0x3000); // SizeOfImage
    w32(&mut out, 0x200); // SizeOfHeaders
    w32(&mut out, 0); // CheckSum
    w16(&mut out, 3); // Subsystem CUI
    w16(&mut out, 0x0140); // DllCharacteristics DYNAMIC_BASE|NX_COMPAT
    w64(&mut out, 0x100000); // SizeOfStackReserve
    w64(&mut out, 0x1000); // SizeOfStackCommit
    w64(&mut out, 0x100000); // SizeOfHeapReserve
    w64(&mut out, 0x1000); // SizeOfHeapCommit
    w32(&mut out, 0); // LoaderFlags
    w32(&mut out, 16); // NumberOfRvaAndSizes
                       // DataDirectories 16x8. IMPORT=idx1, IAT=idx12
    for i in 0..16 {
        match i {
            1 => {
                w32(&mut out, import_rva);
                w32(&mut out, import_size);
            }
            12 => {
                w32(&mut out, iat_rva);
                w32(&mut out, iat_size);
            }
            _ => {
                w32(&mut out, 0);
                w32(&mut out, 0);
            }
        }
    }
    // --- Section headers 2x40 ---
    // .text
    {
        let mut name = [0u8; 8];
        name[..5].copy_from_slice(b".text");
        out.extend_from_slice(&name);
        w32(&mut out, text_vsize); // VirtualSize (código real)
        w32(&mut out, TEXT_RVA); // VirtualAddress
        w32(&mut out, text_raw as u32); // SizeOfRawData
        w32(&mut out, 0x200); // PointerToRawData
        w32(&mut out, 0);
        w32(&mut out, 0);
        w16(&mut out, 0);
        w16(&mut out, 0);
        w32(&mut out, 0x60000020); // CNT_CODE|EXECUTE|READ
    }
    // .rdata
    {
        let mut name = [0u8; 8];
        name[..6].copy_from_slice(b".rdata");
        out.extend_from_slice(&name);
        w32(&mut out, rdata.len() as u32); // VirtualSize (dado real)
        w32(&mut out, RDATA_RVA); // VirtualAddress
        w32(&mut out, rdata_raw as u32); // SizeOfRawData
        w32(&mut out, 0x200 + text_raw as u32); // PointerToRawData
        w32(&mut out, 0);
        w32(&mut out, 0);
        w16(&mut out, 0);
        w16(&mut out, 0);
        w32(&mut out, 0x40000040); // CNT_INIT_DATA|READ
    }
    // pad até SizeOfHeaders (0x200)
    assert!(
        out.len() <= 0x200,
        "headers excederam 0x200: {:#X}",
        out.len()
    );
    out.resize(0x200, 0);
    // seções (completadas até o múltiplo de FILE_ALIGN)
    out.extend_from_slice(text);
    out.resize(0x200 + text_raw, 0);
    out.extend_from_slice(rdata);
    out.resize(0x200 + text_raw + rdata_raw, 0);
    out
}

/// Layout do `.rdata` gerado para um PE de teste.
pub struct RdataLayout {
    /// Bytes finais (tamanho real usado; `assemble` alinha o raw).
    pub bytes: Vec<u8>,
    /// blob nomeado -> RVA.
    pub blobs: std::collections::HashMap<String, u32>,
    /// IAT[i] (RVA) na ordem de `funcs`.
    pub iats: Vec<u32>,
    /// RVA do Import Directory.
    pub import_dir: u32,
}

impl RdataLayout {
    pub fn blob(&self, name: &str) -> u32 {
        self.blobs[name]
    }
    pub fn iat(&self, idx: usize) -> u32 {
        self.iats[idx]
    }
}

/// Monta `.rdata` genérico: blobs + Hint/Names + ILT + IAT + ImportDir.
/// `dll` ex. `"KERNEL32.dll"`; `funcs` na ordem da IAT.
pub fn build_rdata_generic(dll: &str, funcs: &[&str], blobs: &[(&str, &[u8])]) -> RdataLayout {
    // 0x400 comporta ~16 imports + blobs típicos; `assemble` alinha o raw.
    // (0x200 estourou no suite v0.3 com 16 imports — ver assert abaixo.)
    let mut buf = vec![0u8; 0x400];
    let mut blobs_map = std::collections::HashMap::new();
    let mut off = 0usize;
    for (name, data) in blobs {
        buf[off..off + data.len()].copy_from_slice(data);
        blobs_map.insert(name.to_string(), RDATA_RVA + off as u32);
        off += data.len();
    }
    // Hint/Name empacotados.
    let mut hint_rvas = Vec::new();
    for f in funcs {
        buf[off] = 0;
        buf[off + 1] = 0;
        buf[off + 2..off + 2 + f.len()].copy_from_slice(f.as_bytes());
        buf[off + 2 + f.len()] = 0;
        hint_rvas.push((RDATA_RVA + off as u32) as u64);
        off += 2 + f.len() + 1;
    }
    // Nome da DLL.
    off = align_up(off, 2);
    let dll_rva = RDATA_RVA + off as u32;
    buf[off..off + dll.len()].copy_from_slice(dll.as_bytes());
    buf[off + dll.len()] = 0;
    off += dll.len() + 1;
    // ILT e IAT (u64, alinhados).
    off = align_up(off, 8);
    let ilt = RDATA_RVA + off as u32;
    for (i, h) in hint_rvas.iter().enumerate() {
        buf[off + i * 8..off + i * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    off += (funcs.len() + 1) * 8;
    let iat = RDATA_RVA + off as u32;
    let mut iats = Vec::new();
    for (i, h) in hint_rvas.iter().enumerate() {
        buf[off + i * 8..off + i * 8 + 8].copy_from_slice(&h.to_le_bytes());
        iats.push(iat + i as u32 * 8);
    }
    off += (funcs.len() + 1) * 8;
    // Import Directory (1 desc + nulo).
    let dir = RDATA_RVA + off as u32;
    let put32 = |buf: &mut Vec<u8>, o: usize, v: u32| {
        buf[o..o + 4].copy_from_slice(&v.to_le_bytes());
    };
    put32(&mut buf, off, ilt);
    put32(&mut buf, off + 12, dll_rva);
    put32(&mut buf, off + 16, iat);
    off += 40;
    assert!(off <= 0x400, "rdata estourou: {off:#X}");
    buf.truncate(off);
    RdataLayout {
        bytes: buf,
        blobs: blobs_map,
        iats,
        import_dir: dir,
    }
}

pub(crate) fn align_up(v: usize, a: usize) -> usize {
    (v + a - 1) & !(a - 1)
}

/// Emite `sub rsp,imm` com o encoding correto: imm8 só vale para 0..=127
/// (é COM SINAL — 0x88 seria −120!). Acima disso, imm32.
/// Regressão documentada: `file.exe` usava `48 83 EC 88` e o RSP subia.
pub(crate) fn emit_sub_rsp(c: &mut Vec<u8>, n: u32) {
    assert!(n % 8 == 0 && (0x28..=0x1000).contains(&n), "frame inválido");
    if n <= 127 {
        c.extend_from_slice(&[0x48, 0x83, 0xEC, n as u8]);
    } else {
        c.extend_from_slice(&[0x48, 0x81, 0xEC]);
        c.extend_from_slice(&n.to_le_bytes());
    }
}
/// rel32 para `call [rip]`/`lea [rip]`: alvo_rva - (TEXT_RVA + next_off).
pub(crate) fn rel32(target_rva: u32, next_off: usize) -> [u8; 4] {
    ((target_rva as i64 - (TEXT_RVA as i64 + next_off as i64)) as i32).to_le_bytes()
}

/// Emite `call qword [rip+rel(iat_rva)]` no fim de `c`.
pub(crate) fn emit_call(c: &mut Vec<u8>, iat_rva: u32) {
    c.extend_from_slice(&[0xFF, 0x15]);
    let next = c.len() + 4;
    c.extend_from_slice(&rel32(iat_rva, next));
}

/// Emite `lea reg,[rip+rel(rva)]`. `op` inclui REX+opcode+modrm (3 bytes).
pub(crate) fn emit_lea_rip(c: &mut Vec<u8>, op: [u8; 3], target_rva: u32) {
    c.extend_from_slice(&op);
    let next = c.len() + 4;
    c.extend_from_slice(&rel32(target_rva, next));
}

/// Corrige um `jcc rel8` emitido como `[0x7X, 0x00]` em `at`.
pub(crate) fn patch_rel8(c: &mut [u8], at: usize, target: usize) {
    let rel = target as i64 - (at as i64 + 2);
    assert!((-128..=127).contains(&rel), "salto rel8 fora de alcance");
    c[at + 1] = rel as u8;
}

/// Corrige um `jcc rel8` para mirar o fim atual de `c`.
pub(crate) fn patch_here(c: &mut [u8], at: usize) {
    let target = c.len();
    patch_rel8(c, at, target);
}

/// REGRA DE OURO 2 (skip-exit): após `cmp`/`test`, o salto que PULA o
/// `exit(code)` deve mirar o caso BOM. "Falhe a menos que IGUAL" = JE (0x74);
/// "falhe a menos que DIFERENTE" = JNE (0x75). Trocar os dois aprova o
/// fracasso e reprova o sucesso (regressão real no `suite.exe`: count==11
/// caía no exit 43). Estes helpers emitem a sequência inteira.
pub(crate) fn emit_fail_unless_equal(c: &mut Vec<u8>, i_exit: u32, code: u8) {
    let j = c.len();
    c.extend_from_slice(&[0x74, 0x00]); // je ok
    c.push(0xB9);
    c.extend_from_slice(&(code as u32).to_le_bytes());
    emit_call(c, i_exit);
    patch_here(c, j);
}

/// "Falhe a menos que DIFERENTE" (bom = ZF=0): JNE pula o exit.
pub(crate) fn emit_fail_unless_not_equal(c: &mut Vec<u8>, i_exit: u32, code: u8) {
    let j = c.len();
    c.extend_from_slice(&[0x75, 0x00]); // jne ok
    c.push(0xB9);
    c.extend_from_slice(&(code as u32).to_le_bytes());
    emit_call(c, i_exit);
    patch_here(c, j);
}
/// REGRA DE OURO (convenção Windows x64): args 1–4 em RCX/RDX/R8/R9,
/// do 5º em diante na stack em `[rsp+0x20 + (n-5)*8]` (caller).
/// Escrever o 4º argumento na stack desloca TODOS os seguintes —
/// regressão real ocorrida em `file.exe` (R9 nunca zerado; exit 99).
/// Este helper emite o store do n-ésimo argumento (>= 5) com o offset certo.
pub(crate) fn emit_stack_arg(c: &mut Vec<u8>, argn: u32, imm: u32, is_qword: bool) {
    assert!(argn >= 5, "args 1-4 vão em registradores, nunca na stack");
    let off = 0x20 + (argn - 5) * 8;
    assert!(off < 0x80, "frame além do suportado");
    if is_qword {
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, off as u8]);
    } else {
        c.extend_from_slice(&[0xC7, 0x44, 0x24, off as u8]);
    }
    c.extend_from_slice(&imm.to_le_bytes());
}

pub(crate) fn pad_cc(c: &mut Vec<u8>) -> usize {
    let real = c.len();
    while c.len() < 0x200 {
        c.push(0xCC);
    }
    real
}

/// Nome do arquivo manipulado por `file.exe` (drive da Capsule no teste).
pub fn file_exe_name() -> &'static str {
    "C:\\v02test.txt"
}

/// Conteúdo escrito/lido por `file.exe` (exit code = tamanho = 11).
pub fn file_exe_msg() -> &'static [u8] {
    b"Hello File\n"
}

/// Nome do arquivo manipulado por `suite.exe`.
pub fn suite_exe_name() -> &'static str {
    "C:\\suitetest.txt"
}

/// `suite.exe`: TODAS as APIs v0.x em cadeia, com verificação no guest.
/// Console→arquivo(write/read+compare)→memória(alloc/protect/free)→cmdline.
/// Exit 0 = tudo passou; 41–48 = estágio que falhou (ver corpo).
/// REGRA STANDING (testing-strategy): toda API nova entra neste EXE.
pub fn build_suite_exe() -> Vec<u8> {
    let funcs = [
        "GetStdHandle",
        "WriteFile",
        "CreateFileA",
        "ReadFile",
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
        "Sleep",
        "ExitProcess",
    ];
    let r = build_rdata_generic(
        "KERNEL32.dll",
        &funcs,
        &[
            ("start", b"RINE-SUITE-START\n"),
            ("ok", b"RINE-SUITE-OK\n"),
            ("fmsg", file_exe_msg()),
            ("fname", b"C:\\suitetest.txt\0"),
        ],
    );
    let (i_std, i_write, i_open, i_read, i_close) =
        (r.iat(0), r.iat(1), r.iat(2), r.iat(3), r.iat(4));
    let (i_alloc, i_free, i_prot, i_cmd, i_tls_alloc, i_tls_free) =
        (r.iat(5), r.iat(6), r.iat(7), r.iat(8), r.iat(9), r.iat(10));
    let (i_tls_get, i_tls_set, i_err, i_sleep, i_exit) =
        (r.iat(11), r.iat(12), r.iat(13), r.iat(14), r.iat(15));
    let (m_start, m_ok, m_fmsg, m_fname) = (
        r.blob("start"),
        r.blob("ok"),
        r.blob("fmsg"),
        r.blob("fname"),
    );
    let mut c: Vec<u8> = Vec::new();
    emit_sub_rsp(&mut c, 0xA8);
    // Helper local: exit(code) — 5+6 bytes.
    // (expandido inline abaixo via fail_to.)
    // --- 1. console ---
    c.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]); // mov ecx,-11
    emit_call(&mut c, i_std);
    c.extend_from_slice(&[0x48, 0x89, 0xC3]); // mov rbx,rax
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], m_start);
    c.extend_from_slice(&[0x41, 0xB8, 0x11, 0x00, 0x00, 0x00]); // mov r8d,17
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x38]); // lea r9,[rsp+0x38]
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_write);
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)
    emit_fail_unless_not_equal(&mut c, i_exit, 41);
    // --- 2a. create+write ---
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_fname);
    c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x40]);
    c.extend_from_slice(&[0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9]); // xor r8d; xor r9d
    emit_stack_arg(&mut c, 5, 2, false);
    emit_stack_arg(&mut c, 6, 0x80, false);
    emit_stack_arg(&mut c, 7, 0, true);
    emit_call(&mut c, i_open);
    c.extend_from_slice(&[0x48, 0x89, 0xC3, 0x48, 0x83, 0xFB, 0xFF]); // mov rbx,rax; cmp -1 (válido)
    emit_fail_unless_not_equal(&mut c, i_exit, 42);
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], m_fmsg);
    c.extend_from_slice(&[0x41, 0xB8, 0x0B, 0x00, 0x00, 0x00]); // mov r8d,11
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x38]);
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_write);
    c.extend_from_slice(&[0x48, 0x89, 0xD9]);
    emit_call(&mut c, i_close);
    // --- 2b. reopen+read ---
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_fname);
    c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x80]);
    c.extend_from_slice(&[0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9]);
    emit_stack_arg(&mut c, 5, 3, false);
    emit_stack_arg(&mut c, 6, 0x80, false);
    emit_stack_arg(&mut c, 7, 0, true);
    emit_call(&mut c, i_open);
    c.extend_from_slice(&[0x48, 0x89, 0xC3]); // mov rbx,rax (reopen sempre existe aqui)
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    c.extend_from_slice(&[0x48, 0x8D, 0x54, 0x24, 0x40]); // lea rdx,[rsp+0x40]
    c.extend_from_slice(&[0x41, 0xB8, 0x40, 0x00, 0x00, 0x00]); // mov r8d,64
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x3C]); // lea r9,[rsp+0x3C]
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_read);
    c.extend_from_slice(&[0x48, 0x89, 0xD9]);
    emit_call(&mut c, i_close);
    // count == 11?
    c.extend_from_slice(&[0x83, 0x7C, 0x24, 0x3C, 0x0B]); // cmp dword [rsp+0x3C],11
    emit_fail_unless_equal(&mut c, i_exit, 43);
    // compare loop 11 bytes: rsi=readbuf, rdi=msg
    c.extend_from_slice(&[0x48, 0x8D, 0x74, 0x24, 0x40]); // lea rsi,[rsp+0x40]
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x3D], m_fmsg); // lea rdi,[msg]
    c.extend_from_slice(&[0x31, 0xC9]); // xor ecx,ecx
    let again = c.len();
    c.extend_from_slice(&[0x8A, 0x04, 0x0E]); // mov al,[rsi+rcx]
    c.extend_from_slice(&[0x3A, 0x04, 0x0F]); // cmp al,[rdi+rcx]
    emit_fail_unless_equal(&mut c, i_exit, 43);
    c.extend_from_slice(&[0xFF, 0xC1]); // inc ecx
    c.extend_from_slice(&[0x83, 0xF9, 0x0B]); // cmp ecx,11
    let jb = c.len();
    c.extend_from_slice(&[0x72, 0x00]); // jb again
    patch_rel8(&mut c, jb, again);
    // --- 3. memória ---
    c.extend_from_slice(&[0x31, 0xC9]);
    c.extend_from_slice(&[0xBA, 0x00, 0x10, 0x00, 0x00]);
    c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]);
    c.extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]);
    emit_call(&mut c, i_alloc);
    c.extend_from_slice(&[0x48, 0x89, 0xC3, 0x48, 0x85, 0xDB]); // mov rbx,rax; test (não-NULL)
    emit_fail_unless_not_equal(&mut c, i_exit, 44);
    c.extend_from_slice(&[0xC6, 0x03, 0x7E]); // mov byte [rbx],0x7E
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    c.extend_from_slice(&[0xBA, 0x00, 0x10, 0x00, 0x00]);
    c.extend_from_slice(&[0x41, 0xB8, 0x02, 0x00, 0x00, 0x00]);
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x38]);
    emit_call(&mut c, i_prot);
    c.extend_from_slice(&[0x44, 0x0F, 0xB6, 0x13]); // movzx r10d,byte [rbx]
    c.extend_from_slice(&[0x41, 0x83, 0xFA, 0x7E]); // cmp r10d,0x7E
    emit_fail_unless_equal(&mut c, i_exit, 45);
    c.extend_from_slice(&[0x48, 0x89, 0xD9, 0x31, 0xD2]); // mov rcx,rbx; xor edx,edx
    c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]);
    emit_call(&mut c, i_free);
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)
    emit_fail_unless_not_equal(&mut c, i_exit, 46);
    // --- 4. cmdline não-vazia ---
    emit_call(&mut c, i_cmd);
    c.extend_from_slice(&[0x66, 0x83, 0x38, 0x00]); // cmp word [rax],0 (não-vazia)
    emit_fail_unless_not_equal(&mut c, i_exit, 47);
    // --- 5. TLS roundtrip: alloc → set → get → free → get(falha) ---
    emit_call(&mut c, i_tls_alloc); // eax = índice
    c.extend_from_slice(&[0x83, 0xF8, 0xFF]); // cmp eax,-1 (TLS_OUT_OF_INDEXES?)
    emit_fail_unless_not_equal(&mut c, i_exit, 63);
    c.extend_from_slice(&[0x89, 0xC3]); // mov ebx,eax (guarda o índice)
    c.extend_from_slice(&[0x89, 0xD9]); // mov ecx,ebx
                                        // movabs rdx,0x1122334455667788 (B8+rdx=BA; B8 seria rax!)
    c.extend_from_slice(&[0x48, 0xBA, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
    emit_call(&mut c, i_tls_set);
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)
    emit_fail_unless_not_equal(&mut c, i_exit, 64);
    c.extend_from_slice(&[0x89, 0xD9]); // mov ecx,ebx
    emit_call(&mut c, i_tls_get);
    // cmp rax,r10 via r10 (imm64 não cabe em cmp direto).
    // REX 0x4C = W+R (reg=r10 via R, r/m=rax); 0x49 seria REX.W+B (errado!).
    c.extend_from_slice(&[0x49, 0xBA, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
    c.extend_from_slice(&[0x4C, 0x39, 0xD0]); // cmp rax,r10
    emit_fail_unless_equal(&mut c, i_exit, 65);
    c.extend_from_slice(&[0x89, 0xD9]); // mov ecx,ebx
    emit_call(&mut c, i_tls_free);
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)
    emit_fail_unless_not_equal(&mut c, i_exit, 66);
    c.extend_from_slice(&[0x89, 0xD9]); // mov ecx,ebx
    emit_call(&mut c, i_tls_get); // pós-free → NULL
    c.extend_from_slice(&[0x48, 0x85, 0xC0]); // test rax,rax
    emit_fail_unless_equal(&mut c, i_exit, 67);
    // --- 6. GetLastError: força erro (handle 999) e confere 87 ---
    c.extend_from_slice(&[0xB9, 0xE7, 0x03, 0x00, 0x00]); // mov ecx,999
    emit_call(&mut c, i_std); // INVALID + LastError=87 (ignora rax)
    emit_call(&mut c, i_err);
    c.extend_from_slice(&[0x83, 0xF8, 0x57]); // cmp eax,87 (INVALID_PARAMETER)
    emit_fail_unless_equal(&mut c, i_exit, 68);
    // --- 7. Sleep(5) retorna (sem assert observável; prova não-trava) ---
    c.extend_from_slice(&[0xB9, 0x05, 0x00, 0x00, 0x00]); // mov ecx,5
    emit_call(&mut c, i_sleep);
    // --- 8. OK final ---
    c.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]);
    emit_call(&mut c, i_std);
    c.extend_from_slice(&[0x48, 0x89, 0xC1]); // mov rcx,rax
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], m_ok);
    c.extend_from_slice(&[0x41, 0xB8, 0x0E, 0x00, 0x00, 0x00]); // mov r8d,14
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x38]);
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_write);
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)
    emit_fail_unless_not_equal(&mut c, i_exit, 48);
    c.extend_from_slice(&[0x31, 0xC9]); // xor ecx,ecx
    emit_call(&mut c, i_exit);
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);
    let vsize = pad_cc(&mut c) as u32;
    let iat_size = (funcs.len() + 1) as u32 * 8;
    assemble(&c, &r.bytes, vsize, r.import_dir, 40, r.iats[0], iat_size)
}

/// `file.exe`: CreateFileA→WriteFile→Close→reopen→ReadFile→exit(bytes lidos).
/// Exit 11 no sucesso; 99/98 em falha de abertura.
pub fn build_file_exe() -> Vec<u8> {
    let funcs = [
        "CreateFileA",
        "WriteFile",
        "ReadFile",
        "CloseHandle",
        "ExitProcess",
    ];
    let r = build_rdata_generic(
        "KERNEL32.dll",
        &funcs,
        &[("msg", file_exe_msg()), ("fname", b"C:\\v02test.txt\0")],
    );
    let (iat_open, iat_write, iat_read, iat_close, iat_exit) =
        (r.iat(0), r.iat(1), r.iat(2), r.iat(3), r.iat(4));
    let (msg, fname) = (r.blob("msg"), r.blob("fname"));
    let mut c: Vec<u8> = Vec::new();
    emit_sub_rsp(&mut c, 0x88); // frame: shadow + 3 stack args + locais
                                // open(fname, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, NORMAL, NULL)
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], fname); // lea rcx,[fname] (arg1)
    c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x40]); // mov edx,0x40000000 (arg2)
    c.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d,r8d (arg3)
    c.extend_from_slice(&[0x45, 0x31, 0xC9]); // xor r9d,r9d (arg4 = NULL)
    emit_stack_arg(&mut c, 5, 2, false); // CREATE_ALWAYS
    emit_stack_arg(&mut c, 6, 0x80, false); // FILE_ATTRIBUTE_NORMAL
    emit_stack_arg(&mut c, 7, 0, true); // hTemplate = NULL
    emit_call(&mut c, iat_open);
    c.extend_from_slice(&[0x48, 0x89, 0xC3]); // mov rbx,rax
    c.extend_from_slice(&[0x48, 0x83, 0xFB, 0xFF]); // cmp rbx,-1
    let j1 = c.len();
    c.extend_from_slice(&[0x75, 0x00]); // jnz ok1
    c.extend_from_slice(&[0xB9, 0x63, 0x00, 0x00, 0x00]); // mov ecx,99
    emit_call(&mut c, iat_exit);
    let ok1 = c.len();
    patch_rel8(&mut c, j1, ok1);
    // WriteFile(rbx, msg, 11, &written=[rsp+0x40], NULL)
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], msg); // lea rdx,[msg]
    c.extend_from_slice(&[0x41, 0xB8, 0x0B, 0x00, 0x00, 0x00]); // mov r8d,11
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x40]); // lea r9,[rsp+0x40]
    c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]); // overlapped=NULL
    emit_call(&mut c, iat_write);
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    emit_call(&mut c, iat_close);
    // reopen(fname, GENERIC_READ, 0, NULL, OPEN_EXISTING, NORMAL, NULL)
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], fname);
    c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x80]); // mov edx,0x80000000
    c.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d,r8d (arg3)
    c.extend_from_slice(&[0x45, 0x31, 0xC9]); // xor r9d,r9d (arg4 = NULL)
    emit_stack_arg(&mut c, 5, 3, false); // OPEN_EXISTING
    emit_stack_arg(&mut c, 6, 0x80, false);
    emit_stack_arg(&mut c, 7, 0, true);
    emit_call(&mut c, iat_open);
    c.extend_from_slice(&[0x48, 0x89, 0xC3]);
    c.extend_from_slice(&[0x48, 0x83, 0xFB, 0xFF]);
    let j2 = c.len();
    c.extend_from_slice(&[0x75, 0x00]); // jnz ok2
    c.extend_from_slice(&[0xB9, 0x62, 0x00, 0x00, 0x00]); // mov ecx,98
    emit_call(&mut c, iat_exit);
    let ok2 = c.len();
    patch_rel8(&mut c, j2, ok2);
    // ReadFile(rbx, buf=[rsp+0x48], 64, &read=[rsp+0x44], NULL)
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    c.extend_from_slice(&[0x48, 0x8D, 0x54, 0x24, 0x48]); // lea rdx,[rsp+0x48]
    c.extend_from_slice(&[0x41, 0xB8, 0x40, 0x00, 0x00, 0x00]); // mov r8d,64
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x44]); // lea r9,[rsp+0x44]
    c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]);
    emit_call(&mut c, iat_read);
    c.extend_from_slice(&[0x48, 0x89, 0xD9]);
    emit_call(&mut c, iat_close);
    c.extend_from_slice(&[0x8B, 0x4C, 0x24, 0x44]); // mov ecx,[rsp+0x44]
    emit_call(&mut c, iat_exit);
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);
    let vsize = pad_cc(&mut c) as u32;
    let iat_size = (funcs.len() + 1) as u32 * 8;
    assemble(&c, &r.bytes, vsize, r.import_dir, 40, r.iats[0], iat_size)
}

/// `alloc.exe`: VirtualAlloc→write→VirtualProtect→read→VirtualFree→exit(byte).
/// Exit 66 (`'B'`) no sucesso; 101/102 em falha.
pub fn build_alloc_exe() -> Vec<u8> {
    let funcs = [
        "VirtualAlloc",
        "VirtualFree",
        "VirtualProtect",
        "ExitProcess",
    ];
    let r = build_rdata_generic("KERNEL32.dll", &funcs, &[]);
    let (ia_alloc, ia_free, ia_prot, ia_exit) = (r.iat(0), r.iat(1), r.iat(2), r.iat(3));
    let mut c: Vec<u8> = Vec::new();
    emit_sub_rsp(&mut c, 0x28);
    c.extend_from_slice(&[0x31, 0xC9]); // xor ecx,ecx
    c.extend_from_slice(&[0xBA, 0x00, 0x10, 0x00, 0x00]); // mov edx,0x1000
    c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]); // mov r8d,0x3000
    c.extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]); // mov r9d,0x04
    emit_call(&mut c, ia_alloc);
    c.extend_from_slice(&[0x48, 0x89, 0xC3]); // mov rbx,rax
    c.extend_from_slice(&[0x48, 0x85, 0xDB]); // test rbx,rbx
    let j1 = c.len();
    c.extend_from_slice(&[0x75, 0x00]);
    c.extend_from_slice(&[0xB9, 0x65, 0x00, 0x00, 0x00]); // mov ecx,101
    emit_call(&mut c, ia_exit);
    let ok1 = c.len();
    patch_rel8(&mut c, j1, ok1);
    c.extend_from_slice(&[0xC6, 0x03, 0x42]); // mov byte [rbx],66
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    c.extend_from_slice(&[0xBA, 0x00, 0x10, 0x00, 0x00]); // mov edx,0x1000
    c.extend_from_slice(&[0x41, 0xB8, 0x02, 0x00, 0x00, 0x00]); // mov r8d,0x02
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x20]); // lea r9,[rsp+0x20]
    emit_call(&mut c, ia_prot);
    c.extend_from_slice(&[0x44, 0x0F, 0xB6, 0x13]); // movzx r10d,byte [rbx]
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    c.extend_from_slice(&[0x31, 0xD2]); // xor edx,edx
    c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]); // mov r8d,0x8000
    emit_call(&mut c, ia_free);
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax
    let j2 = c.len();
    c.extend_from_slice(&[0x75, 0x00]);
    c.extend_from_slice(&[0xB9, 0x66, 0x00, 0x00, 0x00]); // mov ecx,102
    emit_call(&mut c, ia_exit);
    let ok2 = c.len();
    patch_rel8(&mut c, j2, ok2);
    c.extend_from_slice(&[0x44, 0x89, 0xD1]); // mov ecx,r10d
    emit_call(&mut c, ia_exit);
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);
    let vsize = pad_cc(&mut c) as u32;
    let iat_size = (funcs.len() + 1) as u32 * 8;
    assemble(&c, &r.bytes, vsize, r.import_dir, 40, r.iats[0], iat_size)
}

/// `args.exe`: GetCommandLineW→strlen16→exit(len). Prova o caminho
/// launcher→UTF-16→PEB→guest.
pub fn build_args_exe() -> Vec<u8> {
    let funcs = ["GetCommandLineW", "ExitProcess"];
    let r = build_rdata_generic("KERNEL32.dll", &funcs, &[]);
    let (ia_cmd, ia_exit) = (r.iat(0), r.iat(1));
    let mut c: Vec<u8> = Vec::new();
    emit_sub_rsp(&mut c, 0x28);
    emit_call(&mut c, ia_cmd); // rax = LPWSTR
    c.extend_from_slice(&[0x31, 0xC9]); // xor ecx,ecx
    let again = c.len();
    c.extend_from_slice(&[0x0F, 0xB7, 0x14, 0x48]); // movzx edx,word [rax+rcx*2]
    c.extend_from_slice(&[0x66, 0x85, 0xD2]); // test dx,dx
    let jz = c.len();
    c.extend_from_slice(&[0x74, 0x00]); // jz done
    c.extend_from_slice(&[0xFF, 0xC1]); // inc ecx
    let jmp = c.len();
    c.extend_from_slice(&[0xEB, 0x00]); // jmp again
    let done = c.len();
    patch_rel8(&mut c, jz, done);
    patch_rel8(&mut c, jmp, again);
    emit_call(&mut c, ia_exit); // ExitProcess(len)
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);
    let vsize = pad_cc(&mut c) as u32;
    let iat_size = (funcs.len() + 1) as u32 * 8;
    assemble(&c, &r.bytes, vsize, r.import_dir, 40, r.iats[0], iat_size)
}

/// `evil.exe`: robustez — 15 abusos que DEVEM falhar limpo.
/// Cada estágio espera recusa (FALSE/NULL/INVALID); aceitar = bug do runtime.
/// Exit 0 = tudo contido; 51–65 = estágio que se comportou mal.
/// FRONTEIRA DOCUMENTADA: ponteiros selvagens NÃO estão aqui (sem SEH até
/// v0.3, falhariam o host — ver testing-strategy); só valores/handles/flags.
pub fn build_evil_exe() -> Vec<u8> {
    let funcs = [
        "GetStdHandle",
        "WriteFile",
        "CreateFileA",
        "ReadFile",
        "CloseHandle",
        "VirtualAlloc",
        "VirtualFree",
        "TlsGetValue",
        "TlsSetValue",
        "TlsFree",
        "ExitProcess",
    ];
    let r = build_rdata_generic(
        "KERNEL32.dll",
        &funcs,
        &[
            ("msg", b"evil\n"),
            ("zpath", b"Z:\\nope.txt\0"),
            ("fname", b"C:\\evil_disp.txt\0"),
        ],
    );
    let (i_std, i_write, i_open, i_read, i_close, i_alloc, i_free) = (
        r.iat(0),
        r.iat(1),
        r.iat(2),
        r.iat(3),
        r.iat(4),
        r.iat(5),
        r.iat(6),
    );
    let (i_tls_get, i_tls_set, i_tls_free, i_exit) = (r.iat(7), r.iat(8), r.iat(9), r.iat(10));
    let (m_msg, m_zpath, m_fname) = (r.blob("msg"), r.blob("zpath"), r.blob("fname"));
    let mut c: Vec<u8> = Vec::new();
    emit_sub_rsp(&mut c, 0x48);
    // 1. WriteFile(handle selvagem) deve dar FALSE (51 se TRUE).
    c.extend_from_slice(&[0x48, 0xB9, 0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x00, 0x00]); // mov rcx,0xDEADBEEF
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], m_msg);
    c.extend_from_slice(&[0x41, 0xB8, 0x05, 0x00, 0x00, 0x00]);
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x30]);
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_write);
    evil_expect_zero(&mut c, i_exit, 51);
    // 2. ReadFile(handle selvagem) → FALSE (52).
    c.extend_from_slice(&[0x48, 0xB9, 0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x00, 0x00]);
    c.extend_from_slice(&[0x48, 0x8D, 0x54, 0x24, 0x38]); // lea rdx,[rsp+0x38]
    c.extend_from_slice(&[0x41, 0xB8, 0x08, 0x00, 0x00, 0x00]);
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x30]);
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_read);
    evil_expect_zero(&mut c, i_exit, 52);
    // 3. CloseHandle(selvagem) → FALSE (53).
    c.extend_from_slice(&[0x48, 0xB9, 0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x00, 0x00]);
    emit_call(&mut c, i_close);
    evil_expect_zero(&mut c, i_exit, 53);
    // 4. GetStdHandle(-11) deve FUNCIONAR → rbx (54 se INVALID).
    c.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]);
    emit_call(&mut c, i_std);
    c.extend_from_slice(&[0x48, 0x89, 0xC3, 0x48, 0x83, 0xFB, 0xFF]); // mov rbx,rax; cmp -1
    evil_expect_not_equal(&mut c, i_exit, 54);
    // 5. CloseHandle(stdout) → TRUE (owns_fd=false; 55 se não).
    c.extend_from_slice(&[0x48, 0x89, 0xD9]);
    emit_call(&mut c, i_close);
    c.extend_from_slice(&[0x83, 0xF8, 0x01]); // cmp eax,1
    evil_expect_equal(&mut c, i_exit, 55);
    // 6. WriteFile(handle fechado=stale) → FALSE (56).
    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], m_msg);
    c.extend_from_slice(&[0x41, 0xB8, 0x05, 0x00, 0x00, 0x00]);
    c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x30]);
    emit_stack_arg(&mut c, 5, 0, true);
    emit_call(&mut c, i_write);
    evil_expect_zero(&mut c, i_exit, 56);
    // 7. CreateFileA(drive inexistente) → INVALID (57).
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_zpath);
    c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x80]);
    c.extend_from_slice(&[0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9]);
    emit_stack_arg(&mut c, 5, 3, false);
    emit_stack_arg(&mut c, 6, 0x80, false);
    emit_stack_arg(&mut c, 7, 0, true);
    emit_call(&mut c, i_open);
    c.extend_from_slice(&[0x48, 0x83, 0xF8, 0xFF]); // cmp rax,-1; recusa esperada
    evil_expect_equal(&mut c, i_exit, 57);
    // 8. CreateFileA(disposition=9) → INVALID (58).
    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_fname);
    c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x80]);
    c.extend_from_slice(&[0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9]);
    emit_stack_arg(&mut c, 5, 9, false);
    emit_stack_arg(&mut c, 6, 0x80, false);
    emit_stack_arg(&mut c, 7, 0, true);
    emit_call(&mut c, i_open);
    c.extend_from_slice(&[0x48, 0x83, 0xF8, 0xFF]); // cmp rax,-1; recusa esperada
    evil_expect_equal(&mut c, i_exit, 58);
    // 9. VirtualAlloc(size=0) → NULL (59).
    c.extend_from_slice(&[0x31, 0xC9, 0x31, 0xD2]); // xor ecx,ecx; xor edx,edx
    c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]);
    c.extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]);
    emit_call(&mut c, i_alloc);
    c.extend_from_slice(&[0x48, 0x85, 0xC0]); // test rax,rax; espera 0
    evil_expect_jz_ok(&mut c, i_exit, 59);
    // 10. VirtualAlloc(type=0) → NULL (60).
    c.extend_from_slice(&[0x31, 0xC9]);
    c.extend_from_slice(&[0xBA, 0x00, 0x10, 0x00, 0x00]);
    c.extend_from_slice(&[0x45, 0x31, 0xC0]); // xor r8d (type=0)
    c.extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]);
    emit_call(&mut c, i_alloc);
    c.extend_from_slice(&[0x48, 0x85, 0xC0]);
    evil_expect_jz_ok(&mut c, i_exit, 60);
    // 11. VirtualFree(endereço solto) → FALSE (61).
    c.extend_from_slice(&[0xB9, 0x00, 0x50, 0x34, 0x12]); // mov ecx,0x12345000
    c.extend_from_slice(&[0x31, 0xD2]);
    c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]);
    emit_call(&mut c, i_free);
    evil_expect_zero(&mut c, i_exit, 61);
    // 12. GetStdHandle(999) → INVALID (62).
    c.extend_from_slice(&[0xB9, 0xE7, 0x03, 0x00, 0x00]); // mov ecx,999
    emit_call(&mut c, i_std);
    c.extend_from_slice(&[0x48, 0x83, 0xF8, 0xFF]); // cmp rax,-1; recusa esperada
    evil_expect_equal(&mut c, i_exit, 62);
    // 13. TlsGetValue(0xFFFFFFFF) → NULL (63).
    c.extend_from_slice(&[0xB9, 0xFF, 0xFF, 0xFF, 0xFF]); // mov ecx,-1
    emit_call(&mut c, i_tls_get);
    c.extend_from_slice(&[0x48, 0x85, 0xC0]); // test rax,rax; espera 0
    evil_expect_jz_ok(&mut c, i_exit, 63);
    // 14. TlsSetValue(0xFFFFFFFF, 1) → FALSE (64).
    c.extend_from_slice(&[0xB9, 0xFF, 0xFF, 0xFF, 0xFF]); // mov ecx,-1
    c.extend_from_slice(&[0xBA, 0x01, 0x00, 0x00, 0x00]); // mov edx,1
    emit_call(&mut c, i_tls_set);
    evil_expect_zero(&mut c, i_exit, 64);
    // 15. TlsFree(0xFFFFFFFF) → FALSE (65).
    c.extend_from_slice(&[0xB9, 0xFF, 0xFF, 0xFF, 0xFF]); // mov ecx,-1
    emit_call(&mut c, i_tls_free);
    evil_expect_zero(&mut c, i_exit, 65);
    // Tudo contido:
    c.extend_from_slice(&[0x31, 0xC9]);
    emit_call(&mut c, i_exit);
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);
    let vsize = pad_cc(&mut c) as u32;
    let iat_size = (funcs.len() + 1) as u32 * 8;
    assemble(&c, &r.bytes, vsize, r.import_dir, 40, r.iats[0], iat_size)
}

/// Se EAX==0 segue; senão `ExitProcess(code)`. (BOOL FALSE esperado.)
fn evil_expect_zero(c: &mut Vec<u8>, i_exit: u32, code: u8) {
    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax
    let j = c.len();
    c.extend_from_slice(&[0x75, 0x00]); // jnz fail
    let jmp = c.len();
    c.extend_from_slice(&[0xEB, 0x00]); // jmp ok
    let fail = c.len();
    patch_rel8(c, j, fail);
    c.push(0xB9);
    c.extend_from_slice(&(code as u32).to_le_bytes());
    emit_call(c, i_exit);
    patch_here(c, jmp);
}

/// Após `cmp`, segue se IGUAL (recusa/valor esperado confirmado);
/// senão `ExitProcess(code)`. Padrão: `jz ok` pula o exit.
fn evil_expect_equal(c: &mut Vec<u8>, i_exit: u32, code: u8) {
    let j = c.len();
    c.extend_from_slice(&[0x74, 0x00]); // jz ok
    c.push(0xB9);
    c.extend_from_slice(&(code as u32).to_le_bytes());
    emit_call(c, i_exit);
    patch_here(c, j);
}

/// Após `cmp`, segue se DIFERENTE (handle válido, p.ex.); senão exit.
/// Padrão: `jnz ok` pula o exit.
fn evil_expect_not_equal(c: &mut Vec<u8>, i_exit: u32, code: u8) {
    let j = c.len();
    c.extend_from_slice(&[0x75, 0x00]); // jnz ok
    c.push(0xB9);
    c.extend_from_slice(&(code as u32).to_le_bytes());
    emit_call(c, i_exit);
    patch_here(c, j);
}
/// Se RAX==0 segue (NULL esperado); senão `ExitProcess(code)`.
/// Caller emite `test rax,rax` antes.
fn evil_expect_jz_ok(c: &mut Vec<u8>, i_exit: u32, code: u8) {
    let j = c.len();
    c.extend_from_slice(&[0x75, 0x00]); // jnz fail
    let jmp = c.len();
    c.extend_from_slice(&[0xEB, 0x00]); // jmp ok
    let fail = c.len();
    patch_rel8(c, j, fail);
    c.push(0xB9);
    c.extend_from_slice(&(code as u32).to_le_bytes());
    emit_call(c, i_exit);
    patch_here(c, jmp);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_size_stable() {
        let b = build_minimal_hello();
        assert_eq!(b.len(), 0x600);
        assert_eq!(&b[0..2], b"MZ");
    }

    #[test]
    fn sub_rsp_encoding_covers_signed_imm8_trap() {
        let mut c = Vec::new();
        emit_sub_rsp(&mut c, 0x28);
        assert_eq!(c, vec![0x48, 0x83, 0xEC, 0x28]);
        c.clear();
        emit_sub_rsp(&mut c, 0x88);
        assert_eq!(c, vec![0x48, 0x81, 0xEC, 0x88, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn new_exes_parse_with_expected_imports() {
        for (bytes, names) in [
            (
                build_file_exe(),
                vec![
                    "CloseHandle",
                    "CreateFileA",
                    "ExitProcess",
                    "ReadFile",
                    "WriteFile",
                ],
            ),
            (
                build_alloc_exe(),
                vec![
                    "ExitProcess",
                    "VirtualAlloc",
                    "VirtualFree",
                    "VirtualProtect",
                ],
            ),
            (build_args_exe(), vec!["ExitProcess", "GetCommandLineW"]),
            (
                build_suite_exe(),
                vec![
                    "CloseHandle",
                    "CreateFileA",
                    "ExitProcess",
                    "GetCommandLineW",
                    "GetLastError",
                    "GetStdHandle",
                    "ReadFile",
                    "Sleep",
                    "TlsAlloc",
                    "TlsFree",
                    "TlsGetValue",
                    "TlsSetValue",
                    "VirtualAlloc",
                    "VirtualFree",
                    "VirtualProtect",
                    "WriteFile",
                ],
            ),
            (
                build_evil_exe(),
                vec![
                    "CloseHandle",
                    "CreateFileA",
                    "ExitProcess",
                    "GetStdHandle",
                    "ReadFile",
                    "TlsFree",
                    "TlsGetValue",
                    "TlsSetValue",
                    "VirtualAlloc",
                    "VirtualFree",
                    "WriteFile",
                ],
            ),
        ] {
            let img = crate::Image::parse(&bytes).expect("parse");
            let (_d, syms) = img.imports().expect("imports");
            let mut got: Vec<String> = syms.iter().filter_map(|s| s.name.clone()).collect();
            got.sort();
            assert_eq!(got, names);
        }
    }
}
