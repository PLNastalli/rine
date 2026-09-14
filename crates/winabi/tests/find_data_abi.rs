use std::mem::{align_of, offset_of, size_of};
use winabi::{FileTime, Win32FindDataW};

#[test]
fn win32_find_data_w_matches_windows_x64_layout() {
    assert_eq!(size_of::<FileTime>(), 8);
    assert_eq!(align_of::<FileTime>(), 4);

    assert_eq!(size_of::<Win32FindDataW>(), 592);
    assert_eq!(align_of::<Win32FindDataW>(), 4);
    assert_eq!(offset_of!(Win32FindDataW, dw_file_attributes), 0);
    assert_eq!(offset_of!(Win32FindDataW, ft_creation_time), 4);
    assert_eq!(offset_of!(Win32FindDataW, ft_last_access_time), 12);
    assert_eq!(offset_of!(Win32FindDataW, ft_last_write_time), 20);
    assert_eq!(offset_of!(Win32FindDataW, n_file_size_high), 28);
    assert_eq!(offset_of!(Win32FindDataW, n_file_size_low), 32);
    assert_eq!(offset_of!(Win32FindDataW, dw_reserved0), 36);
    assert_eq!(offset_of!(Win32FindDataW, dw_reserved1), 40);
    assert_eq!(offset_of!(Win32FindDataW, c_file_name), 44);
    assert_eq!(offset_of!(Win32FindDataW, c_alternate_file_name), 564);
}
