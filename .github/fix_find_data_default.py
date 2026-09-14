from pathlib import Path

p = Path("crates/winabi/src/lib.rs")
text = p.read_text()
text = text.replace(
    "#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]\n#[repr(C)]\npub struct Win32FindDataW",
    "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(C)]\npub struct Win32FindDataW",
    1,
)
anchor = "    pub c_alternate_file_name: [u16; 14],\n}\n\n/// `MEMORY_BASIC_INFORMATION`"
impl_block = '''    pub c_alternate_file_name: [u16; 14],\n}\n\nimpl Default for Win32FindDataW {\n    fn default() -> Self {\n        Self {\n            dw_file_attributes: 0,\n            ft_creation_time: FileTime::default(),\n            ft_last_access_time: FileTime::default(),\n            ft_last_write_time: FileTime::default(),\n            n_file_size_high: 0,\n            n_file_size_low: 0,\n            dw_reserved0: 0,\n            dw_reserved1: 0,\n            c_file_name: [0; 260],\n            c_alternate_file_name: [0; 14],\n        }\n    }\n}\n\n/// `MEMORY_BASIC_INFORMATION`'''
if anchor not in text:
    raise SystemExit("WIN32_FIND_DATAW anchor not found")
text = text.replace(anchor, impl_block, 1)
p.write_text(text)
