#[path = "src/icon.rs"]
mod icon;
#[path = "src/icon_ico.rs"]
mod icon_ico;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=src/icon_ico.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    for size in [32, 48, 256] {
        icon_ico::write_png(&out.join(format!("blackout-{size}.png")), size)
            .unwrap_or_else(|e| panic!("write blackout-{size}.png: {e}"));
    }
    let icns = out.join("blackout.icns");
    icon_ico::write_icns(&icns).expect("write blackout.icns");
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let dest = std::path::PathBuf::from(manifest).join("target/blackout.icns");
        let _ = std::fs::copy(&icns, dest);
    }

    #[cfg(windows)]
    embed_windows(&out);
}

#[cfg(windows)]
fn embed_windows(out: &std::path::Path) {
    let ico = out.join("blackout.ico");
    icon_ico::write_ico(&ico).expect("write blackout.ico");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("ico path"));
    res.set("ProductName", "Blackout");
    res.set("FileDescription", "Blackout");
    res.set("CompanyName", "Alex Shink");
    res.set("LegalCopyright", "Copyright (c) 2026 Alex Shink");
    res.set("OriginalFilename", "blackout.exe");
    res.set("InternalName", "blackout");
    res.compile().expect("embed Windows resources");
}
