fn main() {
    let bytes = pe::builder::build_suite_exe();
    let emu =
        runtime::Emulator::load(runtime::Capsule::default(), &bytes, "suite.exe").expect("load");
    let base = emu.image_base();
    let names = [
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
    let expect = [
        kernel32::GetStdHandle_impl as *const () as u64,
        kernel32::WriteFile_impl as *const () as u64,
        kernel32::CreateFileA_impl as *const () as u64,
        kernel32::ReadFile_impl as *const () as u64,
        kernel32::CloseHandle_impl as *const () as u64,
        kernel32::VirtualAlloc_impl as *const () as u64,
        kernel32::VirtualFree_impl as *const () as u64,
        kernel32::VirtualProtect_impl as *const () as u64,
        kernel32::GetCommandLineW_impl as *const () as u64,
        kernel32::TlsAlloc_impl as *const () as u64,
        kernel32::TlsFree_impl as *const () as u64,
        kernel32::TlsGetValue_impl as *const () as u64,
        kernel32::TlsSetValue_impl as *const () as u64,
        kernel32::GetLastError_impl as *const () as u64,
        kernel32::Sleep_impl as *const () as u64,
        kernel32::ExitProcess_impl as *const () as u64,
    ];
    let img = pe::Image::parse(&bytes).unwrap();
    let (_d, syms) = img.imports().unwrap();
    for s in &syms {
        let v = unsafe { *((base + s.iat_rva as u64) as *const u64) };
        let n = s.name.clone().unwrap_or("?".to_string());
        let mut ok = "???".to_string();
        for (a, e) in expect.iter().zip(names.iter()) {
            if *a == v {
                ok = e.to_string();
            }
        }
        println!("{n:16} iat_rva={:#X} -> {ok}", s.iat_rva);
    }
}
