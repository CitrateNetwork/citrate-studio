// Publish the path to ui/lib.slint so consumers' build.rs can register it as the
// `@citrate-ui-kit` Slint library. (Pure asset crate — consumers compile the
// .slint themselves via slint_build's library_paths.)
use std::{env, path::PathBuf};
fn main() {
    let lib = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("ui")
        .join("lib.slint");
    println!("cargo:UI_KIT={}", lib.display());
    println!("cargo:rerun-if-changed=ui");
    println!("cargo:rerun-if-changed=assets");
}
