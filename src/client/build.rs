//! Packs each folder of `templates/` into `<folder>.tar.zst` next to the binaries
//! (`target/<profile>/`), where `mwp init` reads them: `default.tar.zst` is the full project, other
//! packages only carry the files they change. Installers ship them beside `mwp` and `mwp-gui`.

use std::path::PathBuf;

fn main() {
    let templates = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../templates");
    println!("cargo:rerun-if-changed={}", templates.display());
    // OUT_DIR is target/<profile>/build/<crate>/out: three folders up is where the binaries go.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let bin = out.ancestors().nth(3).unwrap();
    for dir in std::fs::read_dir(&templates).expect("templates/").flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        for required in ["manifest.json", "thumbs.png"] {
            assert!(dir.join(required).is_file(), "templates/{name}/{required} is missing");
        }
        let mut tar = tar::Builder::new(Vec::new());
        tar.mode(tar::HeaderMode::Deterministic);
        tar.append_dir_all("", &dir).expect("templates/<package>/");
        let packed = ruzstd::encoding::compress_to_vec(&tar.into_inner().unwrap()[..], ruzstd::encoding::CompressionLevel::Fastest);
        std::fs::write(bin.join(format!("{name}.tar.zst")), packed).expect("write <package>.tar.zst");
    }
}
