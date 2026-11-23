//! Windows only: gives Frostfire.exe its icon and version information.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/Frostfire.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/Frostfire.ico")
        .set("ProductName", "Frostfire")
        .set("FileDescription", "Frostfire")
        .set("InternalName", "Frostfire")
        .set("OriginalFilename", "Frostfire.exe")
        .set("LegalCopyright", "MIT License");
    if let Err(e) = res.compile() {
        // Building on Windows needs the SDK's rc.exe (part of the MSVC build tools) and fails
        // loudly without it. A check from another OS needs llvm-rc; without one the exe just
        // has no icon.
        if cfg!(windows) {
            panic!("could not compile Windows resources: {e}");
        }
        println!("cargo:warning=Windows resources skipped (no llvm-rc on PATH): {e}");
    }
}
