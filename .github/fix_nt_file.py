from pathlib import Path

p = Path("crates/nt-file/src/lib.rs")
text = p.read_text()

text = text.replace(
    "let Some(pos) = win_pattern.rfind(|c| c == '\\\\' || c == '/') else {",
    "let Some(pos) = win_pattern.rfind(['\\\\', '/']) else {",
    1,
)

old_data = '''    let mut data = Win32FindDataW::default();
    data.dw_file_attributes = if md.is_dir() {
        winabi::file_attr::DIRECTORY
    } else {
        winabi::file_attr::NORMAL
    };
'''
new_data = '''    let mut data = Win32FindDataW {
        dw_file_attributes: if md.is_dir() {
            winabi::file_attr::DIRECTORY
        } else {
            winabi::file_attr::NORMAL
        },
        ..Default::default()
    };
'''
if old_data not in text:
    raise SystemExit("find-data initializer anchor not found")
text = text.replace(old_data, new_data, 1)

old_close = '''pub fn close_handle(table: &HandleTable, handle: WindowsHandle) -> Result<(), NtStatus> {
    let obj = table.lookup(handle)?;
    let owned_fd = if obj.typ == ObjectType::File {
        let guard = obj.payload.lock().unwrap();
        let f = match &*guard {
            ObjectPayload::File(f) => f,
            _ => return Err(NtStatus::INVALID_HANDLE),
        };
        if f.owns_fd {
            Some(f.fd)
        } else {
            None
        }
    } else {
        None
    };
    table.close(handle)?;
'''
new_close = '''pub fn close_handle(table: &HandleTable, handle: WindowsHandle) -> Result<(), NtStatus> {
    let obj = table.lookup(handle)?;
    if obj.typ != ObjectType::File {
        return Err(NtStatus::INVALID_HANDLE);
    }
    let owned_fd = {
        let guard = obj.payload.lock().map_err(|_| NtStatus::UNSUCCESSFUL)?;
        let f = match &*guard {
            ObjectPayload::File(f) => f,
            _ => return Err(NtStatus::INVALID_HANDLE),
        };
        f.owns_fd.then_some(f.fd)
    };
    table.close(handle)?;
'''
if old_close not in text:
    raise SystemExit("close_handle anchor not found")
text = text.replace(old_close, new_close, 1)

p.write_text(text)
