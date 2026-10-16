//! Embedded Fontes UI assets and design constants. No network access required.
//! Include this file as a module; colors are sRGB RGBA bytes.

pub const COLOR_DESK: [u8; 4] = [28, 32, 30, 255];
pub const COLOR_SIDEBAR: [u8; 4] = [21, 25, 23, 255];
pub const COLOR_SURFACE: [u8; 4] = [37, 42, 39, 255];
pub const COLOR_RAISED: [u8; 4] = [44, 50, 46, 255];
pub const COLOR_BORDER: [u8; 4] = [60, 68, 62, 255];
pub const COLOR_INK: [u8; 4] = [236, 237, 229, 255];
pub const COLOR_MUTED: [u8; 4] = [171, 179, 170, 255];
pub const COLOR_QUIET: [u8; 4] = [130, 142, 131, 255];
pub const COLOR_ACCENT: [u8; 4] = [220, 188, 131, 255];
pub const COLOR_TINT: [u8; 4] = [56, 55, 43, 255];
pub const COLOR_INVERSE: [u8; 4] = [28, 33, 27, 255];
pub const COLOR_GREEN: [u8; 4] = [178, 197, 171, 255];
pub const DESKTOP_SIDEBAR_WIDTH: f32 = 208.0;
pub const DESKTOP_NAVIGATION_ROW_HEIGHT: f32 = 36.0;
pub const DESKTOP_SIDEBAR_ITEM_GAP: f32 = 4.0;
pub const DESKTOP_NAVIGATION_ICON_GAP: f32 = 8.0;
pub const DESKTOP_NAVIGATION_PADDING_VERTICAL: f32 = 8.0;
pub const DESKTOP_SAVED_PASSAGE_PADDING_VERTICAL: f32 = 6.0;
pub const DESKTOP_SAVED_PASSAGE_CONTENT_GAP: f32 = 2.0;
pub const DESKTOP_NAVIGATION_SECTION_SPACER: f32 = 16.0;
pub const DESKTOP_SMALL_SECTION_SPACER: f32 = 4.0;
pub const DESKTOP_TOOLBAR_HEIGHT: f32 = 72.0;
pub const DESKTOP_WINDOW_BAR_HEIGHT: f32 = 64.0;
pub const DESKTOP_READER_WIDTH: f32 = 800.0;
pub const DESKTOP_STUDY_PANEL_WIDTH: f32 = 432.0;
pub const DESKTOP_ICON_SIZE: f32 = 20.0;
pub const MOBILE_HORIZONTAL_PADDING: f32 = 24.0;
pub const MOBILE_CONTENT_WIDTH: f32 = 342.0;
pub const MOBILE_TAP_TARGET_MINIMUM: f32 = 44.0;
pub const MOBILE_ACTION_HEIGHT: f32 = 48.0;
pub const MOBILE_NAVIGATION_ROW_HEIGHT: f32 = 84.0;
pub const MOBILE_ICON_SIZE: f32 = 24.0;
pub const MOBILE_REFERENCE_CELL_SIZE: f32 = 44.0;
pub const MOBILE_REFERENCE_CELL_GAP: f32 = 5.0;

/// Stable role, Phosphor name, SVG bytes. White fills support runtime tinting.
pub static ICONS: &[(&str, &str, &[u8])] = &[
    (
        "book",
        "book-open",
        include_bytes!("icons/phosphor/regular/book-open.svg"),
    ),
    (
        "search",
        "magnifying-glass",
        include_bytes!("icons/phosphor/regular/magnifying-glass.svg"),
    ),
    (
        "notes",
        "note-pencil",
        include_bytes!("icons/phosphor/regular/note-pencil.svg"),
    ),
    (
        "tree",
        "tree-structure",
        include_bytes!("icons/phosphor/regular/tree-structure.svg"),
    ),
    (
        "settings",
        "sliders-horizontal",
        include_bytes!("icons/phosphor/regular/sliders-horizontal.svg"),
    ),
    (
        "chevron",
        "caret-right",
        include_bytes!("icons/phosphor/regular/caret-right.svg"),
    ),
    (
        "down",
        "caret-down",
        include_bytes!("icons/phosphor/regular/caret-down.svg"),
    ),
    ("close", "x", include_bytes!("icons/phosphor/regular/x.svg")),
    (
        "arrow",
        "arrow-right",
        include_bytes!("icons/phosphor/regular/arrow-right.svg"),
    ),
    (
        "back",
        "arrow-left",
        include_bytes!("icons/phosphor/regular/arrow-left.svg"),
    ),
    (
        "bookmark",
        "bookmark-simple",
        include_bytes!("icons/phosphor/regular/bookmark-simple.svg"),
    ),
    (
        "spark",
        "sparkle",
        include_bytes!("icons/phosphor/regular/sparkle.svg"),
    ),
    (
        "check",
        "check",
        include_bytes!("icons/phosphor/regular/check.svg"),
    ),
    (
        "copy",
        "copy",
        include_bytes!("icons/phosphor/regular/copy.svg"),
    ),
    (
        "plus",
        "plus",
        include_bytes!("icons/phosphor/regular/plus.svg"),
    ),
    (
        "minus",
        "minus",
        include_bytes!("icons/phosphor/regular/minus.svg"),
    ),
    (
        "wifi",
        "wifi-high",
        include_bytes!("icons/phosphor/regular/wifi-high.svg"),
    ),
    (
        "battery",
        "battery-full",
        include_bytes!("icons/phosphor/regular/battery-full.svg"),
    ),
];

/// Font family, weight, role, TrueType bytes.
pub static FONTS: &[(&str, u16, &str, &[u8])] = &[
    (
        "IBM Plex Sans",
        400,
        "ui_body",
        include_bytes!("fonts/IBMPlexSans-Regular.ttf"),
    ),
    (
        "IBM Plex Sans",
        500,
        "ui_label",
        include_bytes!("fonts/IBMPlexSans-Medium.ttf"),
    ),
    (
        "Lora",
        400,
        "scripture",
        include_bytes!("fonts/Lora-Regular.ttf"),
    ),
    (
        "Noto Serif",
        400,
        "greek_fallback",
        include_bytes!("fonts/NotoSerif-Regular.ttf"),
    ),
    (
        "Noto Serif Hebrew",
        400,
        "hebrew_fallback",
        include_bytes!("fonts/NotoSerifHebrew-Regular.ttf"),
    ),
];
