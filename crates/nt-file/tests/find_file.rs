use nt_file::{find_close, find_first_file, find_next_file, DriveMap};
use nt_object::HandleTable;
use std::collections::HashMap;
use winabi::{NtStatus, Win32FindDataW};

fn decode_name(data: &Win32FindDataW) -> String {
    let end = data
        .c_file_name
        .iter()
        .position(|&u| u == 0)
        .unwrap_or(data.c_file_name.len());
    String::from_utf16(&data.c_file_name[..end]).unwrap()
}

#[test]
fn find_file_wildcards_are_case_insensitive_and_handles_close() {
    let root = std::env::temp_dir().join(format!(
        "rine-find-file-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    std::fs::write(root.join("B.TXT"), b"bb").unwrap();
    std::fs::write(root.join("skip.bin"), b"bin").unwrap();

    let mut drives = HashMap::new();
    drives.insert('T', root.to_string_lossy().into_owned());
    let fsys = DriveMap::new(drives, None);
    let table = HandleTable::new();

    let (handle, first) = find_first_file(&table, &fsys, "T:\\*.txt").unwrap();
    let second = find_next_file(&table, handle).unwrap();
    let mut names = vec![decode_name(&first), decode_name(&second)];
    names.sort_by_key(|s| s.to_ascii_lowercase());
    assert_eq!(names, ["a.txt", "B.TXT"]);
    assert_eq!(find_next_file(&table, handle), Err(NtStatus::NO_MORE_FILES));

    find_close(&table, handle).unwrap();
    assert_eq!(
        find_next_file(&table, handle),
        Err(NtStatus::INVALID_HANDLE)
    );
    assert_eq!(find_close(&table, handle), Err(NtStatus::INVALID_HANDLE));

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn find_file_reports_no_match_and_missing_directory() {
    let root = std::env::temp_dir().join(format!("rine-find-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let mut drives = HashMap::new();
    drives.insert('T', root.to_string_lossy().into_owned());
    let fsys = DriveMap::new(drives, None);
    let table = HandleTable::new();

    assert_eq!(
        find_first_file(&table, &fsys, "T:\\*.txt"),
        Err(NtStatus::OBJECT_NAME_NOT_FOUND)
    );
    assert_eq!(
        find_first_file(&table, &fsys, "T:\\missing\\*"),
        Err(NtStatus::OBJECT_PATH_NOT_FOUND)
    );

    std::fs::remove_dir_all(&root).unwrap();
}
