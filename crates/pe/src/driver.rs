//! Driver guest parametrizado para cenários `fileops` (`CreateFileA` et al).
//!
//! Ao contrário dos builders fixos (`file.exe`, `suite.exe`), aqui cada
//! passo carrega sua expectativa: o guest verifica o resultado e sai com
//! `10+índice` no primeiro passo divergente, `0` se todos passarem.
//! Só um handle corrente (`rbx`), como `suite.exe` — suficiente para v1
//! (sequências create→write/read→close; documentado em `docs/fuzzing.md`).
//!
//! Frame `0x68`: shadow 32 + 3 slots de stack + `written@0x38` +
//! `readcount@0x3C` + `readbuf[0x40..0x60)`. Leituras limitadas a 32 bytes
//! (mensagem tem 11) — limite verificado em `build`, não no guest.

use super::builder::{
    assemble, build_rdata_generic, emit_call, emit_fail_unless_equal, emit_fail_unless_not_equal,
    emit_lea_rip, emit_stack_arg, emit_sub_rsp, pad_cc,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DriverError {
    #[error("passos demais (máx 245)")]
    TooManySteps,
    #[error("rdata estourou 0x200 (paths longas/demais?)")]
    RdataOverflow,
    #[error("write len {0} > mensagem ({})", super::builder::file_exe_msg().len())]
    WriteTooLong(u32),
    #[error("read len {0} > buffer (32)")]
    ReadTooLong(u32),
}

/// Um passo com expectativa embutida. `fail_code = 10 + índice`.
#[derive(Debug, Clone)]
pub enum FileOp {
    /// `CreateFileA(path, access, 0, NULL, disp, NORMAL, NULL)` → `rbx`.
    Create {
        path: String,
        access: u32,
        disp: u32,
        expect_valid: bool,
    },
    /// `WriteFile(rbx, msg, len)`; exige `TRUE==expect_ok` e
    /// `written == expect_written` (0 quando `!expect_ok`, como o Windows).
    Write {
        len: u32,
        expect_ok: bool,
        expect_written: u32,
    },
    /// `ReadFile(rbx, buf, len)`; exige `TRUE==expect_ok`,
    /// `count == expect_count` e, se `verify`, `buf == msg`.
    Read {
        len: u32,
        expect_ok: bool,
        expect_count: u32,
        verify: bool,
    },
    /// `CloseHandle(rbx)`; exige `TRUE==expect_ok`.
    Close { expect_ok: bool },
}

const FUNCS: [&str; 5] = [
    "CreateFileA",
    "WriteFile",
    "ReadFile",
    "CloseHandle",
    "ExitProcess",
];
const MSG: &[u8] = b"Hello File\n";
const READBUF_OFF: u8 = 0x40;
const READBUF_LEN: u32 = 32;

/// Monta o guest para `steps`. Erros de FORMA (overflow, lens) viram `Err`;
/// divergências de COMPORTAMENTO viram exit codes no guest (nunca panic).
pub fn build(steps: &[FileOp]) -> Result<Vec<u8>, DriverError> {
    if steps.len() > 245 {
        return Err(DriverError::TooManySteps);
    }
    for op in steps {
        match op {
            FileOp::Write { len, .. } if *len as usize > MSG.len() => {
                return Err(DriverError::WriteTooLong(*len));
            }
            FileOp::Read { len, .. } if *len > READBUF_LEN => {
                return Err(DriverError::ReadTooLong(*len));
            }
            _ => {}
        }
    }
    // Blobs: msg + paths únicos em ordem de aparição.
    let mut blobs: Vec<(String, Vec<u8>)> = vec![("msg".into(), MSG.to_vec())];
    for op in steps {
        if let FileOp::Create { path, .. } = op {
            let name = format!("p{}", blobs.len() - 1);
            if !blobs.iter().any(|(_, b)| b == &path_nul(path)) {
                blobs.push((name, path_nul(path)));
            }
        }
    }
    let blob_refs: Vec<(&str, &[u8])> = blobs
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    // Estimativa conservadora antes do genérico (que daria panic em overflow).
    let est: usize = blob_refs.iter().map(|(_, b)| b.len()).sum::<usize>()
        + FUNCS.iter().map(|f| f.len() + 3).sum::<usize>()
        + "KERNEL32.dll".len()
        + 1
        + (FUNCS.len() + 1) * 16
        + 40
        + 16;
    if est > 0x200 {
        return Err(DriverError::RdataOverflow);
    }
    let r = build_rdata_generic("KERNEL32.dll", &FUNCS, &blob_refs);
    let (i_open, i_write, i_read, i_close, i_exit) =
        (r.iat(0), r.iat(1), r.iat(2), r.iat(3), r.iat(4));
    let msg_rva = r.blob("msg");
    let path_rva = |path: &str| {
        let blob = path_nul(path);
        let (name, _) = blobs
            .iter()
            .find(|(_, b)| *b == blob)
            .expect("path registrado acima");
        r.blob(name)
    };

    let mut c: Vec<u8> = Vec::new();
    emit_sub_rsp(&mut c, 0x68);
    for (i, op) in steps.iter().enumerate() {
        let fail = (10 + i) as u8;
        match op {
            FileOp::Create {
                path,
                access,
                disp,
                expect_valid,
            } => {
                emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], path_rva(path));
                c.extend_from_slice(&[0xBA]);
                c.extend_from_slice(&access.to_le_bytes());
                c.extend_from_slice(&[0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9]);
                emit_stack_arg(&mut c, 5, *disp, false);
                emit_stack_arg(&mut c, 6, 0x80, false);
                emit_stack_arg(&mut c, 7, 0, true);
                emit_call(&mut c, i_open);
                // mov rbx,rax; cmp rbx,-1
                c.extend_from_slice(&[0x48, 0x89, 0xC3, 0x48, 0x83, 0xFB, 0xFF]);
                if *expect_valid {
                    emit_fail_unless_not_equal(&mut c, i_exit, fail);
                } else {
                    emit_fail_unless_equal(&mut c, i_exit, fail);
                }
            }
            FileOp::Write {
                len,
                expect_ok,
                expect_written,
            } => {
                c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
                emit_lea_rip(&mut c, [0x48, 0x8D, 0x15], msg_rva);
                c.extend_from_slice(&[0x41, 0xB8]);
                c.extend_from_slice(&len.to_le_bytes());
                c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x38]); // lea r9,[rsp+0x38]
                emit_stack_arg(&mut c, 5, 0, true);
                emit_call(&mut c, i_write);
                c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax
                if *expect_ok {
                    emit_fail_unless_not_equal(&mut c, i_exit, fail);
                } else {
                    emit_fail_unless_equal(&mut c, i_exit, fail);
                }
                // cmp dword [rsp+0x38],expect_written (0 em falha, como Win32)
                c.extend_from_slice(&[0x83, 0x7C, 0x24, 0x38]);
                c.push(*expect_written as u8);
                emit_fail_unless_equal(&mut c, i_exit, fail);
            }
            FileOp::Read {
                len,
                expect_ok,
                expect_count,
                verify,
            } => {
                c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
                c.extend_from_slice(&[0x48, 0x8D, 0x54, 0x24, READBUF_OFF]); // lea rdx,[buf]
                c.extend_from_slice(&[0x41, 0xB8]);
                c.extend_from_slice(&len.to_le_bytes());
                c.extend_from_slice(&[0x4C, 0x8D, 0x4C, 0x24, 0x3C]); // lea r9,[count]
                emit_stack_arg(&mut c, 5, 0, true);
                emit_call(&mut c, i_read);
                c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax
                if *expect_ok {
                    emit_fail_unless_not_equal(&mut c, i_exit, fail);
                } else {
                    emit_fail_unless_equal(&mut c, i_exit, fail);
                }
                c.extend_from_slice(&[0x83, 0x7C, 0x24, 0x3C]);
                c.push(*expect_count as u8);
                emit_fail_unless_equal(&mut c, i_exit, fail);
                if *verify && *expect_count > 0 {
                    // rsi=buf, rdi=msg, rcx=0..count
                    c.extend_from_slice(&[0x48, 0x8D, 0x74, 0x24, READBUF_OFF]);
                    emit_lea_rip(&mut c, [0x48, 0x8D, 0x3D], msg_rva);
                    c.extend_from_slice(&[0x31, 0xC9]); // xor ecx,ecx
                    let again = c.len();
                    c.extend_from_slice(&[0x8A, 0x04, 0x0E]); // mov al,[rsi+rcx]
                    c.extend_from_slice(&[0x3A, 0x04, 0x0F]); // cmp al,[rdi+rcx]
                    emit_fail_unless_equal(&mut c, i_exit, fail);
                    c.extend_from_slice(&[0xFF, 0xC1]); // inc ecx
                    c.extend_from_slice(&[0x83, 0xF9]);
                    c.push(*expect_count as u8); // cmp ecx,count
                    let jb = c.len();
                    c.extend_from_slice(&[0x72, 0x00]); // jb again
                    super::builder::patch_rel8(&mut c, jb, again);
                }
            }
            FileOp::Close { expect_ok } => {
                c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx
                emit_call(&mut c, i_close);
                c.extend_from_slice(&[0x83, 0xF8, 0x01]); // cmp eax,1
                if *expect_ok {
                    emit_fail_unless_equal(&mut c, i_exit, fail);
                } else {
                    emit_fail_unless_not_equal(&mut c, i_exit, fail);
                }
            }
        }
    }
    c.extend_from_slice(&[0x31, 0xC9]); // xor ecx,ecx
    emit_call(&mut c, i_exit);
    c.extend_from_slice(&[0xEB, 0xFE, 0xC3]);
    let vsize = pad_cc(&mut c) as u32;
    let iat_size = (FUNCS.len() + 1) as u32 * 8;
    Ok(assemble(
        &c,
        &r.bytes,
        vsize,
        r.import_dir,
        40,
        r.iats[0],
        iat_size,
    ))
}

fn path_nul(path: &str) -> Vec<u8> {
    let mut v = path.as_bytes().to_vec();
    v.push(0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(bytes: &[u8]) -> Vec<String> {
        let img = crate::Image::parse(bytes).unwrap();
        let (_d, syms) = img.imports().unwrap();
        let mut names: Vec<String> = syms.iter().filter_map(|s| s.name.clone()).collect();
        names.sort();
        names
    }

    #[test]
    fn roundtrip_parses_and_imports() {
        let bytes = build(&[
            FileOp::Create {
                path: "C:\\d.txt".into(),
                access: 0x4000_0000,
                disp: 2,
                expect_valid: true,
            },
            FileOp::Write {
                len: 11,
                expect_ok: true,
                expect_written: 11,
            },
            FileOp::Close { expect_ok: true },
        ])
        .unwrap();
        assert_eq!(
            parse(&bytes),
            vec![
                "CloseHandle",
                "CreateFileA",
                "ExitProcess",
                "ReadFile",
                "WriteFile"
            ]
        );
    }

    #[test]
    fn shape_errors_not_panics() {
        assert_eq!(
            build(&[FileOp::Write {
                len: 12,
                expect_ok: true,
                expect_written: 12
            }]),
            Err(DriverError::WriteTooLong(12))
        );
        assert_eq!(
            build(&[FileOp::Read {
                len: 33,
                expect_ok: true,
                expect_count: 0,
                verify: false
            }]),
            Err(DriverError::ReadTooLong(33))
        );
        let many: Vec<FileOp> = (0..246)
            .map(|i| FileOp::Close {
                expect_ok: i % 2 == 0,
            })
            .collect();
        assert_eq!(build(&many), Err(DriverError::TooManySteps));
    }
}
