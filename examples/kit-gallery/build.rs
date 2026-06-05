use std::collections::HashMap;
use std::path::PathBuf;
fn main() {
    let kit = std::env::var("DEP_CITRATE_STUDIO_UI_KIT_UI_KIT")
        .unwrap_or_else(|_| "../../ui-kit/ui/lib.slint".to_string());
    let mut libs: HashMap<String, PathBuf> = HashMap::new();
    libs.insert("citrate-ui-kit".to_string(), PathBuf::from(kit));
    let config = slint_build::CompilerConfiguration::new().with_library_paths(libs);
    slint_build::compile_with_config("ui/gallery.slint", config).expect("gallery slint compile");
}
