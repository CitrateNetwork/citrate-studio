//! Minimal second shell — renders citrate-studio-ui-kit components, proving the
//! kit is consumable by an app other than Studio.
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    Gallery::new()?.run()
}
