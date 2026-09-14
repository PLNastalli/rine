from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


# winabi: status/error constants + exact WIN32_FIND_DATAW layout.
replace_once(
    "crates/winabi/src/lib.rs",
    "    pub const END_OF_FILE: Self = Self(0xC0000011);\n    pub const IMAGE_NOT_AT_BASE: Self = Self(0x40000003);",
    "    pub const END_OF_FILE: Self = Self(0xC0000011);\n    pub const NO_MORE_FILES: Self = Self(0x80000006);\n    pub const OBJECT_PATH_NOT_FOUND: Self = Self(0xC000003A);\n    pub const IMAGE_NOT_AT_BASE: Self = Self(0x40000003);",
)
replace_once(
    "crates/winabi/src/lib.rs",
    "    pub const FILE_NOT_FOUND: Self = Self(2);\n    /// `ERROR_MOD_NOT_FOUND`",
    "    pub const FILE_NOT_FOUND: Self = Self(2);\n    pub const PATH_NOT_FOUND: Self = Self(3);\n    pub const NO_MORE_FILES: Self = Self(18);\n    /// `ERROR_MOD_NOT_FOUND`",
)
replace_once(
    "crates/winabi/src/lib.rs",
    "/// `MEMORY_BASIC_INFORMATION` x86_64 (48 bytes, layout exato `winnt.h`).",
    '''/// `FILETIME` (100-ns ticks since 1601-01-01 UTC).\n#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]\n#[repr(C)]\npub struct FileTime {\n    pub low_date_time: u32,\n    pub high_date_time: u32,\n}\n\n/// `WIN32_FIND_DATAW` x86_64. The struct is 592 bytes with 4-byte alignment.\n#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]\n#[repr(C)]\npub struct Win32FindDataW {\n    pub dw_file_attributes: u32,\n    pub ft_creation_time: FileTime,\n    pub ft_last_access_time: FileTime,\n    pub ft_last_write_time: FileTime,\n    pub n_file_size_high: u32,\n    pub n_file_size_low: u32,\n    pub dw_reserved0: u32,\n    pub dw_reserved1: u32,\n    pub c_file_name: [u16; 260],\n    pub c_alternate_file_name: [u16; 14],\n}\n\n/// `MEMORY_BASIC_INFORMATION` x86_64 (48 bytes, layout exato `winnt.h`).''',
)

# nt-object: search handles are real generational handles with mutable enumeration state.
replace_once(
    "crates/nt-object/src/lib.rs",
    "pub enum ObjectType {\n    File,",
    "pub enum ObjectType {\n    File,\n    FileSearch,",
)
replace_once(
    "crates/nt-object/src/lib.rs",
    '            Self::File => "File",',
    '            Self::File => "File",\n            Self::FileSearch => "FileSearch",',
)
replace_once(
    "crates/nt-object/src/lib.rs",
    "pub enum ObjectPayload {\n    File(FileObject),\n}",
    "pub enum ObjectPayload {\n    File(FileObject),\n    FileSearch(FileSearchObject),\n}",
)
replace_once(
    "crates/nt-object/src/lib.rs",
    "#[derive(Debug)]\npub struct FileObject {",
    '''#[derive(Debug)]\npub struct FileSearchObject {\n    pub entries: Vec<winabi::Win32FindDataW>,\n    pub next_index: usize,\n}\n\n#[derive(Debug)]\npub struct FileObject {''',
)

# nt-file: directory enumeration, DOS-style wildcard subset, and search-handle lifecycle.
replace_once(
    "crates/nt-file/src/lib.rs",
    "use nt_object::{FileObject, HandleTable, KernelObject, ObjectPayload, ObjectType};",
    "use nt_object::{FileObject, FileSearchObject, HandleTable, KernelObject, ObjectPayload, ObjectType};",
)
replace_once(
    "crates/nt-file/src/lib.rs",
    "use winabi::{NtStatus, WindowsHandle};",
    "use winabi::{NtStatus, Win32FindDataW, WindowsHandle};",
)
# ObjectPayload stopped being single-variant; make all File accesses explicit.
p = Path("crates/nt-file/src/lib.rs")
text = p.read_text()
old = "    let ObjectPayload::File(f) = &*guard;"
new = '''    let f = match &*guard {\n        ObjectPayload::File(f) => f,\n        _ => return Err(NtStatus::INVALID_HANDLE),\n    };'''
if text.count(old) < 3:
    raise SystemExit("expected File payload bindings in nt-file")
text = text.replace(old, new)
p.write_text(text)

find_impl = r'''
/// Starts a `FindFirstFileW` enumeration and returns the first matching entry.
/// The returned handle is a real generational object handle and must be closed
/// with `find_close`/`FindClose`, not generic `CloseHandle`.
pub fn find_first_file(
    table: &HandleTable,
    fsys: &DriveMap,
    win_pattern: &str,
) -> Result<(WindowsHandle, Win32FindDataW), NtStatus> {
    let (dir_win, pattern) = split_find_pattern(win_pattern)?;
    let dir = fsys.translate(dir_win)?;
    let read_dir = std::fs::read_dir(&dir).map_err(dir_io_to_status)?;
    let mut matches = Vec::new();

    for item in read_dir {
        let entry = item.map_err(dir_io_to_status)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| NtStatus::NOT_IMPLEMENTED)?;
        if wildcard_match(pattern, &name) {
            matches.push((name, entry));
        }
    }
    matches.sort_by(|(a, _), (b, _)| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });

    let mut entries = Vec::with_capacity(matches.len());
    for (name, entry) in matches {
        entries.push(find_data_from_entry(&name, &entry)?);
    }
    let first = entries
        .first()
        .copied()
        .ok_or(NtStatus::OBJECT_NAME_NOT_FOUND)?;

    let obj = Arc::new(KernelObject {
        typ: ObjectType::FileSearch,
        name: None,
        rights: 0,
        payload: std::sync::Mutex::new(ObjectPayload::FileSearch(FileSearchObject {
            entries,
            next_index: 1,
        })),
    });
    Ok((table.insert(obj), first))
}

/// Advances a directory enumeration. Exhaustion is `STATUS_NO_MORE_FILES`.
pub fn find_next_file(
    table: &HandleTable,
    handle: WindowsHandle,
) -> Result<Win32FindDataW, NtStatus> {
    let obj = table.lookup(handle)?;
    if obj.typ != ObjectType::FileSearch {
        return Err(NtStatus::INVALID_HANDLE);
    }
    let mut guard = obj.payload.lock().map_err(|_| NtStatus::UNSUCCESSFUL)?;
    let search = match &mut *guard {
        ObjectPayload::FileSearch(search) => search,
        _ => return Err(NtStatus::INVALID_HANDLE),
    };
    let data = search
        .entries
        .get(search.next_index)
        .copied()
        .ok_or(NtStatus::NO_MORE_FILES)?;
    search.next_index += 1;
    Ok(data)
}

/// Closes only a search handle. A stale/double-close is `INVALID_HANDLE`.
pub fn find_close(table: &HandleTable, handle: WindowsHandle) -> Result<(), NtStatus> {
    let obj = table.lookup(handle)?;
    if obj.typ != ObjectType::FileSearch {
        return Err(NtStatus::INVALID_HANDLE);
    }
    table.close(handle)
}

fn split_find_pattern(win_pattern: &str) -> Result<(&str, &str), NtStatus> {
    if win_pattern.is_empty() {
        return Err(NtStatus::INVALID_PARAMETER);
    }
    let Some(pos) = win_pattern.rfind(|c| c == '\\' || c == '/') else {
        return Ok((".", win_pattern));
    };
    let pattern = &win_pattern[pos + 1..];
    if pattern.is_empty() {
        return Err(NtStatus::INVALID_PARAMETER);
    }
    let bytes = win_pattern.as_bytes();
    let dir = if pos == 2 && bytes.get(1) == Some(&b':') {
        &win_pattern[..=pos]
    } else if pos == 0 {
        return Err(NtStatus::NOT_IMPLEMENTED);
    } else {
        &win_pattern[..pos]
    };
    Ok((dir, pattern))
}

fn wildcard_match(pattern: &str, name: &str) -> bool {
    let normalized = if pattern.eq_ignore_ascii_case("*.*") {
        "*"
    } else {
        pattern
    };
    let p: Vec<char> = normalized.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let mut prev = vec![false; n.len() + 1];
    prev[0] = true;
    for pc in p {
        let mut cur = vec![false; n.len() + 1];
        if pc == '*' {
            cur[0] = prev[0];
            for j in 1..=n.len() {
                cur[j] = cur[j - 1] || prev[j];
            }
        } else {
            for j in 1..=n.len() {
                cur[j] = prev[j - 1] && (pc == '?' || pc == n[j - 1]);
            }
        }
        prev = cur;
    }
    prev[n.len()]
}

fn find_data_from_entry(name: &str, entry: &std::fs::DirEntry) -> Result<Win32FindDataW, NtStatus> {
    let md = entry.metadata().map_err(|e| io_to_status(e, 3))?;
    let encoded: Vec<u16> = name.encode_utf16().collect();
    if encoded.len() >= 260 {
        return Err(NtStatus::NOT_IMPLEMENTED);
    }
    let mut data = Win32FindDataW::default();
    data.dw_file_attributes = if md.is_dir() {
        winabi::file_attr::DIRECTORY
    } else {
        winabi::file_attr::NORMAL
    };
    if md.permissions().readonly() {
        data.dw_file_attributes |= winabi::file_attr::READONLY;
    }
    data.ft_creation_time = host_time_to_filetime(md.created());
    data.ft_last_access_time = host_time_to_filetime(md.accessed());
    data.ft_last_write_time = host_time_to_filetime(md.modified());
    let size = if md.is_file() { md.len() } else { 0 };
    data.n_file_size_high = (size >> 32) as u32;
    data.n_file_size_low = size as u32;
    data.c_file_name[..encoded.len()].copy_from_slice(&encoded);
    Ok(data)
}

fn host_time_to_filetime(time: std::io::Result<std::time::SystemTime>) -> winabi::FileTime {
    const WINDOWS_TO_UNIX_SECONDS: u128 = 11_644_473_600;
    let Ok(time) = time else {
        return winabi::FileTime::default();
    };
    let Ok(since_unix) = time.duration_since(std::time::UNIX_EPOCH) else {
        return winabi::FileTime::default();
    };
    let ticks = (WINDOWS_TO_UNIX_SECONDS + since_unix.as_secs() as u128)
        .saturating_mul(10_000_000)
        .saturating_add((since_unix.subsec_nanos() as u128) / 100)
        .min(u64::MAX as u128) as u64;
    winabi::FileTime {
        low_date_time: ticks as u32,
        high_date_time: (ticks >> 32) as u32,
    }
}

fn dir_io_to_status(e: std::io::Error) -> NtStatus {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => NtStatus::OBJECT_PATH_NOT_FOUND,
        ErrorKind::PermissionDenied => NtStatus::ACCESS_DENIED,
        ErrorKind::InvalidInput => NtStatus::INVALID_PARAMETER,
        _ => NtStatus::UNSUCCESSFUL,
    }
}

'''
replace_once(
    "crates/nt-file/src/lib.rs",
    "/// Fecha handle; se o objeto for `File` com fd possuído, fecha o fd.",
    find_impl + "/// Fecha handle; se o objeto for `File` com fd possuído, fecha o fd.",
)
# CloseHandle must not consume a find handle: Windows requires FindClose.
replace_once(
    "crates/nt-file/src/lib.rs",
    '''pub fn close_handle(table: &HandleTable, handle: WindowsHandle) -> Result<(), NtStatus> {\n    let obj = table.lookup(handle)?;\n    let owned_fd = if obj.typ == ObjectType::File {\n        let guard = obj.payload.lock().unwrap();\n        let f = match &*guard {\n            ObjectPayload::File(f) => f,\n            _ => return Err(NtStatus::INVALID_HANDLE),\n        };\n        if f.owns_fd {\n            Some(f.fd)\n        } else {\n            None\n        }\n    } else {\n        None\n    };''',
    '''pub fn close_handle(table: &HandleTable, handle: WindowsHandle) -> Result<(), NtStatus> {\n    let obj = table.lookup(handle)?;\n    if obj.typ != ObjectType::File {\n        return Err(NtStatus::INVALID_HANDLE);\n    }\n    let owned_fd = {\n        let guard = obj.payload.lock().map_err(|_| NtStatus::UNSUCCESSFUL)?;\n        let f = match &*guard {\n            ObjectPayload::File(f) => f,\n            _ => return Err(NtStatus::INVALID_HANDLE),\n        };\n        f.owns_fd.then_some(f.fd)\n    };''',
)

# ntdll typed bridge.
replace_once(
    "crates/ntdll/src/lib.rs",
    '''pub fn nt_file_attributes(win_path: &str) -> Result<u32, NtStatus> {\n    let ctx = require_context();\n    nt_file::file_attributes(&ctx.fsys, win_path)\n}\n''',
    '''pub fn nt_file_attributes(win_path: &str) -> Result<u32, NtStatus> {\n    let ctx = require_context();\n    nt_file::file_attributes(&ctx.fsys, win_path)\n}\n\n/// Starts a Win32 directory enumeration.\npub fn nt_find_first_file(\n    win_pattern: &str,\n) -> Result<(WindowsHandle, winabi::Win32FindDataW), NtStatus> {\n    let ctx = require_context();\n    nt_file::find_first_file(&ctx.table, &ctx.fsys, win_pattern)\n}\n\n/// Advances a directory enumeration.\npub fn nt_find_next_file(handle: WindowsHandle) -> Result<winabi::Win32FindDataW, NtStatus> {\n    let ctx = require_context();\n    nt_file::find_next_file(&ctx.table, handle)\n}\n\n/// Closes a directory enumeration handle.\npub fn nt_find_close(handle: WindowsHandle) -> Result<(), NtStatus> {\n    let ctx = require_context();\n    nt_file::find_close(&ctx.table, handle)\n}\n''',
)

# kernelbase: NTSTATUS -> Win32 mapping + Unicode wrappers.
replace_once(
    "crates/kernelbase/src/lib.rs",
    "        NtStatus::OBJECT_NAME_NOT_FOUND => ERROR_FILE_NOT_FOUND,\n        NtStatus::OBJECT_NAME_COLLISION => ERROR_FILE_EXISTS,",
    "        NtStatus::OBJECT_NAME_NOT_FOUND => ERROR_FILE_NOT_FOUND,\n        NtStatus::OBJECT_PATH_NOT_FOUND => Win32Error::PATH_NOT_FOUND,\n        NtStatus::NO_MORE_FILES => Win32Error::NO_MORE_FILES,\n        NtStatus::OBJECT_NAME_COLLISION => ERROR_FILE_EXISTS,",
)
replace_once(
    "crates/kernelbase/src/lib.rs",
    '''pub fn remove_directory_w(path_wide: &[u16]) -> Result<(), Win32Error> {\n    let path = wide_to_string(cut_nul_w(path_wide)?)?;\n    let ctx = ntdll::require_context();\n    nt_file::remove_directory(&ctx.fsys, &path).map_err(nt_to_win32)\n}\n''',
    '''pub fn remove_directory_w(path_wide: &[u16]) -> Result<(), Win32Error> {\n    let path = wide_to_string(cut_nul_w(path_wide)?)?;\n    let ctx = ntdll::require_context();\n    nt_file::remove_directory(&ctx.fsys, &path).map_err(nt_to_win32)\n}\n\n/// `FindFirstFileW(pattern)` -> search handle + first result.\npub fn find_first_file_w(\n    pattern_wide: &[u16],\n) -> Result<(WindowsHandle, winabi::Win32FindDataW), Win32Error> {\n    let pattern = wide_to_string(cut_nul_w(pattern_wide)?)?;\n    ntdll::nt_find_first_file(&pattern).map_err(nt_to_win32)\n}\n\n/// `FindNextFileW(handle)` -> next result.\npub fn find_next_file_w(\n    handle: WindowsHandle,\n) -> Result<winabi::Win32FindDataW, Win32Error> {\n    ntdll::nt_find_next_file(handle).map_err(nt_to_win32)\n}\n\n/// `FindClose(handle)`.\npub fn find_close(handle: WindowsHandle) -> Result<(), Win32Error> {\n    ntdll::nt_find_close(handle).map_err(nt_to_win32)\n}\n''',
)

# kernel32 ABI façades + exports.
find_facades = r'''
/// `HANDLE FindFirstFileW(LPCWSTR, LPWIN32_FIND_DATAW)`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn FindFirstFileW_impl(
    lp_file_name: *const u16,
    lp_find_file_data: *mut winabi::Win32FindDataW,
) -> u64 {
    if lp_find_file_data.is_null() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return WindowsHandle::INVALID.0;
    }
    let pattern = match wide_slice(lp_file_name) {
        Ok(p) => p,
        Err(e) => {
            nt_thread::set_last_error(e);
            return WindowsHandle::INVALID.0;
        }
    };
    match kernelbase::find_first_file_w(pattern) {
        Ok((handle, data)) => {
            // SAFETY: output buffer is writable for one WIN32_FIND_DATAW by contract.
            unsafe { *lp_find_file_data = data };
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            handle.0
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            WindowsHandle::INVALID.0
        }
    }
}

/// `BOOL FindNextFileW(HANDLE, LPWIN32_FIND_DATAW)`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "win64" fn FindNextFileW_impl(
    h_find_file: u64,
    lp_find_file_data: *mut winabi::Win32FindDataW,
) -> i32 {
    if lp_find_file_data.is_null() {
        nt_thread::set_last_error(winabi::Win32Error::INVALID_PARAMETER);
        return 0;
    }
    match kernelbase::find_next_file_w(WindowsHandle(h_find_file)) {
        Ok(data) => {
            // SAFETY: output buffer is writable for one WIN32_FIND_DATAW by contract.
            unsafe { *lp_find_file_data = data };
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

/// `BOOL FindClose(HANDLE)`.
pub extern "win64" fn FindClose_impl(h_find_file: u64) -> i32 {
    match kernelbase::find_close(WindowsHandle(h_find_file)) {
        Ok(()) => {
            nt_thread::set_last_error(winabi::Win32Error::SUCCESS);
            1
        }
        Err(e) => {
            nt_thread::set_last_error(e);
            0
        }
    }
}

'''
replace_once(
    "crates/kernel32/src/lib.rs",
    "/// `BOOL MoveFileExW(LPCWSTR, LPCWSTR, DWORD)` — 3 args, sem out-pointer.",
    find_facades + "/// `BOOL MoveFileExW(LPCWSTR, LPCWSTR, DWORD)` — 3 args, sem out-pointer.",
)
replace_once(
    "crates/kernel32/src/lib.rs",
    '    "RemoveDirectoryW",\n    "CloseHandle",',
    '    "RemoveDirectoryW",\n    "FindFirstFileW",\n    "FindNextFileW",\n    "FindClose",\n    "CloseHandle",',
)
replace_once(
    "crates/kernel32/src/lib.rs",
    '        "RemoveDirectoryW" => Some(RemoveDirectoryW_impl as *const () as u64),\n        "CloseHandle" => Some(CloseHandle_impl as *const () as u64),',
    '        "RemoveDirectoryW" => Some(RemoveDirectoryW_impl as *const () as u64),\n        "FindFirstFileW" => Some(FindFirstFileW_impl as *const () as u64),\n        "FindNextFileW" => Some(FindNextFileW_impl as *const () as u64),\n        "FindClose" => Some(FindClose_impl as *const () as u64),\n        "CloseHandle" => Some(CloseHandle_impl as *const () as u64),',
)

print("find-file core implementation applied")
