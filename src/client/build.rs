//! Packs `templates/` into `templates.tar.zst` next to the binaries (`target/<profile>/`), where
//! `mwp init` reads it. Installers ship that file beside `mwp` and `mwp-gui`.

use std::path::PathBuf;

fn main() {
    let templates = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../templates");
    println!("cargo:rerun-if-changed={}", templates.display());
    let mut tar = tar::Builder::new(Vec::new());
    tar.mode(tar::HeaderMode::Deterministic);
    tar.append_dir_all("", &templates).expect("templates/");
    let packed = ruzstd::encoding::compress_to_vec(&tar.into_inner().unwrap()[..], ruzstd::encoding::CompressionLevel::Fastest);
    // OUT_DIR is target/<profile>/build/<crate>/out: three folders up is where the binaries go.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.ancestors().nth(3).unwrap().join("templates.tar.zst"), packed).expect("write templates.tar.zst");
}
