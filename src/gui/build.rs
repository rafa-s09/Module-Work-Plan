//! Puts assets/icone/icon.ico in mwp-gui.exe, so Explorer and shortcuts show it.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icone/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/icone/icon.ico");
    // Without the Windows SDK resource compiler the exe only lacks its icon; the build goes on.
    if let Err(e) = res.compile() {
        println!("cargo:warning=mwp-gui.exe built without its icon: {e}");
    }
}
