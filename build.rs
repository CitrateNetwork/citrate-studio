// Compile the Slint UI. `ui/studio.slint` is the single entry that
// imports the rest of the kit (theme, typography, primitives, the
// shell, and every panel). The include-paths let panels import each
// other with bare relative names.
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into()); // base widget style; we override almost everything.
    slint_build::compile_with_config("ui/studio.slint", config)
        .expect("Slint build failed");
}
