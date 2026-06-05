//! Citrate Studio UI kit — the shared Slint design system (theme, typography,
//! icons, primitives, fonts) for Citrate native apps, at Slint 1.16.
//!
//! This crate ships the `.slint` source + fonts and publishes the path to
//! `ui/lib.slint`. Consumers register it as the `@citrate-ui-kit` library in
//! their `build.rs` (via `DEP_CITRATE_STUDIO_UI_KIT_UI_KIT`, or a relative path
//! to this crate's `ui/lib.slint`) and `import { Theme, Btn, … } from
//! "@citrate-ui-kit";` in their `.slint`. See citrate-studio + examples/kit-gallery.

/// The shared UI-kit components, by export name (single source of truth for
/// what `@citrate-ui-kit` provides).
pub const COMPONENTS: &[&str] = &[
    "Theme", "Settings", "SelectableText", "Label", "Eyebrow", "Title", "Mono",
    "Editorial", "AppIcon", "CapGlyph", "Palettes", "RiskDot", "RiskBadge",
    "DataChip", "RoleGlyph", "SurfBadge", "SevDot", "AnchorMark", "Keyframe",
    "Hairline", "Btn", "IconButton", "Card",
];
