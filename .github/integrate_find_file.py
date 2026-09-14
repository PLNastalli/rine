from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


builder = Path("crates/pe/src/builder.rs")
text = builder.read_text()

text = text.replace(
    "/// Exit 0 = tudo passou; 41–48/63–120 = estágio que falhou (ver corpo).",
    "/// Exit 0 = tudo passou; 41–48/63–124 = estágio que falhou (ver corpo).",
    1,
)

# Suite imports: append find-file APIs immediately before ExitProcess.
suite_anchor = '''        "MoveFileExW",\n        "RemoveDirectoryW",\n        "ExitProcess",\n'''
suite_repl = '''        "MoveFileExW",\n        "RemoveDirectoryW",\n        "FindFirstFileW",\n        "FindNextFileW",\n        "FindClose",\n        "ExitProcess",\n'''
if suite_anchor not in text:
    raise SystemExit("suite import anchor not found")
text = text.replace(suite_anchor, suite_repl, 1)

# Search pattern C:\\suitedir\\*.txt.
blob_anchor = '''            (\n                "nt_wide",\n'''
blob_repl = '''            (\n                "wfind",\n                b"C\\x00:\\x00\\\\\\x00s\\x00u\\x00i\\x00t\\x00e\\x00d\\x00i\\x00r\\x00\\\\\\x00*\\x00.\\x00t\\x00x\\x00t\\x00\\x00\\x00",\n            ),\n            (\n                "nt_wide",\n'''
if blob_anchor not in text:
    raise SystemExit("suite blob anchor not found")
text = text.replace(blob_anchor, blob_repl, 1)

text = text.replace("        r.iat(35),\n    );", "        r.iat(38),\n    );", 1)
iat_anchor = '''    let (i_mkdir_w, i_del_w, i_move_w, i_rmdir_w) = (r.iat(31), r.iat(32), r.iat(33), r.iat(34));\n'''
iat_repl = iat_anchor + '''    let (i_find_first, i_find_next, i_find_close) = (r.iat(35), r.iat(36), r.iat(37));\n'''
if iat_anchor not in text:
    raise SystemExit("suite iat anchor not found")
text = text.replace(iat_anchor, iat_repl, 1)

blob_vars = '''    let (m_wdir, m_wfile1, m_wfile2) = (r.blob("wdir"), r.blob("wfile1"), r.blob("wfile2"));\n'''
if blob_vars not in text:
    raise SystemExit("suite blob-vars anchor not found")
text = text.replace(blob_vars, blob_vars + '    let m_wfind = r.blob("wfind");\n', 1)

text = text.replace(
    "    emit_sub_rsp(&mut c, 0x108); // frame: shadow + args + locais + CS[0xA8..0xD8] + MBI[0xD8..0x108]",
    "    emit_sub_rsp(&mut c, 0x378); // + WIN32_FIND_DATAW[0x108..0x358]; mantém alinhamento Win64",
    1,
)

insert_anchor = '''    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx\n    emit_call(&mut c, i_close);\n    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)\n    emit_fail_unless_not_equal(&mut c, i_exit, 114);\n    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_wfile1); // move f1 → f2\n'''
insert_repl = '''    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx\n    emit_call(&mut c, i_close);\n    c.extend_from_slice(&[0x85, 0xC0]); // test eax,eax (TRUE esperado)\n    emit_fail_unless_not_equal(&mut c, i_exit, 114);\n    // --- 16. Enumeração W: FindFirst(*.txt) → único f1 → NO_MORE_FILES → FindClose.\n    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_wfind);\n    c.extend_from_slice(&[0x48, 0x8D, 0x94, 0x24, 0x08, 0x01, 0x00, 0x00]); // lea rdx,[rsp+0x108]\n    emit_call(&mut c, i_find_first);\n    c.extend_from_slice(&[0x48, 0x89, 0xC3]); // mov rbx,rax\n    c.extend_from_slice(&[0x48, 0x83, 0xFB, 0xFF]); // cmp rbx,-1\n    emit_fail_unless_not_equal(&mut c, i_exit, 121);\n    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx\n    c.extend_from_slice(&[0x48, 0x8D, 0x94, 0x24, 0x08, 0x01, 0x00, 0x00]); // out buffer\n    emit_call(&mut c, i_find_next);\n    c.extend_from_slice(&[0x85, 0xC0]); // FALSE esperado: só existe f1.txt\n    emit_fail_unless_equal(&mut c, i_exit, 122);\n    emit_call(&mut c, i_err);\n    c.extend_from_slice(&[0x83, 0xF8, 0x12]); // ERROR_NO_MORE_FILES = 18\n    emit_fail_unless_equal(&mut c, i_exit, 123);\n    c.extend_from_slice(&[0x48, 0x89, 0xD9]); // mov rcx,rbx\n    emit_call(&mut c, i_find_close);\n    c.extend_from_slice(&[0x85, 0xC0]); // TRUE esperado\n    emit_fail_unless_not_equal(&mut c, i_exit, 124);\n    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_wfile1); // move f1 → f2\n'''
if insert_anchor not in text:
    raise SystemExit("suite insertion anchor not found")
text = text.replace(insert_anchor, insert_repl, 1)
text = text.replace("    // --- 16. OK final ---", "    // --- 17. OK final ---", 1)

# Evil: three API-specific contained abuses.
text = text.replace(
    "/// `evil.exe`: robustez — 17 abusos que DEVEM falhar limpo.\n/// Cada estágio espera recusa (FALSE/NULL/INVALID); aceitar = bug do runtime.\n/// Exit 0 = tudo contido; 51–67 = estágio que se comportou mal.",
    "/// `evil.exe`: robustez — 33 abusos que DEVEM falhar limpo.\n/// Cada estágio espera recusa (FALSE/NULL/INVALID); aceitar = bug do runtime.\n/// Exit 0 = tudo contido; 51–83 = estágio que se comportou mal.",
    1,
)
if suite_anchor not in text:
    raise SystemExit("evil import anchor not found")
text = text.replace(suite_anchor, suite_repl, 1)
text = text.replace(
    '''        r.iat(11),\n        r.iat(23),\n    );\n''',
    '''        r.iat(11),\n        r.iat(26),\n    );\n''',
    1,
)
evil_iat_anchor = '''    let (i_mkdir_w, i_del_w, i_move_w, i_rmdir_w) = (r.iat(19), r.iat(20), r.iat(21), r.iat(22));\n'''
if evil_iat_anchor not in text:
    raise SystemExit("evil iat anchor not found")
text = text.replace(
    evil_iat_anchor,
    evil_iat_anchor + '    let (i_find_first, i_find_next, i_find_close) = (r.iat(23), r.iat(24), r.iat(25));\n',
    1,
)
evil_end = '''    // 30. RemoveDirectoryW(Z:\\nope.txt) → FALSE (80).\n    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_wzpath);\n    emit_call(&mut c, i_rmdir_w);\n    evil_expect_zero(&mut c, i_exit, 80);\n    // Tudo contido:\n'''
evil_end_repl = '''    // 30. RemoveDirectoryW(Z:\\nope.txt) → FALSE (80).\n    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_wzpath);\n    emit_call(&mut c, i_rmdir_w);\n    evil_expect_zero(&mut c, i_exit, 80);\n    // 31. FindNextFileW(handle selvagem, buffer válido) → FALSE (81).\n    c.extend_from_slice(&[0x48, 0xB9, 0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x00, 0x00]);\n    c.extend_from_slice(&[0x48, 0x8D, 0x54, 0x24, 0x38]);\n    emit_call(&mut c, i_find_next);\n    evil_expect_zero(&mut c, i_exit, 81);\n    // 32. FindClose(handle selvagem) → FALSE (82).\n    c.extend_from_slice(&[0x48, 0xB9, 0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x00, 0x00]);\n    emit_call(&mut c, i_find_close);\n    evil_expect_zero(&mut c, i_exit, 82);\n    // 33. FindFirstFileW(path, NULL output) → INVALID_HANDLE_VALUE (83).\n    emit_lea_rip(&mut c, [0x48, 0x8D, 0x0D], m_wzpath);\n    c.extend_from_slice(&[0x31, 0xD2]); // xor edx,edx\n    emit_call(&mut c, i_find_first);\n    c.extend_from_slice(&[0x48, 0x83, 0xF8, 0xFF]);\n    evil_expect_equal(&mut c, i_exit, 83);\n    // Tudo contido:\n'''
if evil_end not in text:
    raise SystemExit("evil ending anchor not found")
text = text.replace(evil_end, evil_end_repl, 1)

# Parser-level expected import lists, both sorted.
text = text.replace(
    '''                    "ExitProcess",\n                    "FreeLibrary",\n''',
    '''                    "ExitProcess",\n                    "FindClose",\n                    "FindFirstFileW",\n                    "FindNextFileW",\n                    "FreeLibrary",\n''',
    1,
)
# Evil vector has ExitProcess followed by FreeLibrary too; replace second occurrence.
text = text.replace(
    '''                    "ExitProcess",\n                    "FreeLibrary",\n''',
    '''                    "ExitProcess",\n                    "FindClose",\n                    "FindFirstFileW",\n                    "FindNextFileW",\n                    "FreeLibrary",\n''',
    1,
)

builder.write_text(text)

# Old tests deliberately used FindFirstFileW as a missing API. Keep their intent
# by moving to the still-unimplemented ANSI variant.
for path in ["crates/runtime/src/lib.rs", "crates/manager-core/src/preflight.rs"]:
    p = Path(path)
    t = p.read_text()
    if "FindFirstFileW" not in t:
        raise SystemExit(f"missing-import anchor not found in {path}")
    t = t.replace("`FindFirstFileW` segue em demanda. A troca é intencional.", "`FindFirstFileA` segue em demanda. A troca é intencional.", 1)
    t = t.replace('"FindFirstFileW"', '"FindFirstFileA"')
    p.write_text(t)

print("suite/evil find-file integration applied")
