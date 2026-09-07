use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // Prefer the kit's published path (git-dep friendly); fall back to the
    // workspace-relative path for a local path dependency.
    let kit = std::env::var("DEP_CITRATE_STUDIO_UI_KIT_UI_KIT")
        .unwrap_or_else(|_| "ui-kit/ui/lib.slint".to_string());
    let mut libs: HashMap<String, PathBuf> = HashMap::new();
    libs.insert("citrate-ui-kit".to_string(), PathBuf::from(kit));

    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_library_paths(libs);
    slint_build::compile_with_config("ui/studio.slint", config).expect("Slint build failed");

    // ST-B-011: the `dev-filebacked` guard in signing.rs keys on `debug_assertions`,
    // but that is a per-profile knob any `--config profile.release.debug-assertions=true`
    // can flip — re-opening the plaintext-key seed path in a distributed release binary.
    // Emit a cfg keyed on the RELEASE profile itself (the property that actually defines
    // a shipping build, and which a debug-assertions override cannot mask), so signing.rs
    // can also `compile_error!` on a release dev-filebacked build. Note: this repo's dev
    // profile is opt-level 1, so we key on PROFILE (release vs debug), not OPT_LEVEL —
    // demo screenshots are still built in debug and compile fine.
    println!("cargo:rustc-check-cfg=cfg(shipping_profile)");
    if std::env::var("PROFILE").map(|p| p == "release").unwrap_or(false) {
        println!("cargo:rustc-cfg=shipping_profile");
    }
}
