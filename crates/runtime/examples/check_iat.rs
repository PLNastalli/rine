fn main() {
    let bytes = pe::builder::build_minimal_hello();
    let emu =
        runtime::Emulator::load(runtime::Capsule::default(), &bytes, "hello.exe").expect("load");
    let base = emu.image_base();
    for (i, name) in ["GetStdHandle", "WriteFile", "ExitProcess"]
        .iter()
        .enumerate()
    {
        let v = unsafe { *((base + 0x2090 + i as u64 * 8) as *const u64) };
        eprintln!("IAT[{i}] {name} = {v:#X}");
    }
    eprintln!(
        "GetStdHandle_impl = {:#X}",
        kernel32::GetStdHandle_impl as *const () as u64
    );
    eprintln!(
        "WriteFile_impl    = {:#X}",
        kernel32::WriteFile_impl as *const () as u64
    );
    eprintln!(
        "ExitProcess_impl  = {:#X}",
        kernel32::ExitProcess_impl as *const () as u64
    );
    eprintln!(
        "RtlExit           = {:#X}",
        ntdll::RtlExitUserProcess_impl as *const () as u64
    );
    eprintln!("entry = {:#X}", emu.entry_point() as u64);
}
