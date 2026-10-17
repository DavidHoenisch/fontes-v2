//! Reviewed, bundled theme roles. No mutable process-wide or thread-local theme.
use eframe::egui::{self, Color32};
use fontes_engine::ReadingTheme;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub key: ReadingTheme,
    pub name: &'static str,
    pub dark: bool,
    pub page: Color32,
    pub chrome: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub action_light: Color32,
    pub accent_ink: Color32,
    pub on_accent: Color32,
    pub tint: Color32,
    pub selection: Color32,
    pub feature: Color32,
    pub feature_light: Color32,
    pub glow: Color32,
    pub mark: Color32,
    pub on_feature: Color32,
    pub green: Color32,
    pub error: Color32,
    pub scrim: Color32,
    pub highlights: [Color32; 4],
}
impl Palette {
    pub fn for_theme(key: ReadingTheme) -> Self {
        static PALETTES: OnceLock<Vec<Palette>> = OnceLock::new();
        let palettes = PALETTES.get_or_init(|| {
            let value: serde_json::Value =
                serde_json::from_str(include_str!("../../../assets/ui/themes.json"))
                    .expect("bundled theme JSON");
            assert_eq!(value["theme_version"], 1);
            ReadingTheme::ALL
                .into_iter()
                .map(|key| {
                    let r = value["themes"]
                        .as_array()
                        .expect("theme records")
                        .iter()
                        .find(|v| v["id"] == key.id())
                        .expect("every stable theme has a palette");
                    let c =
                        |role: &str| parse_color(r["colors"][role].as_str().expect("color role"));
                    Palette {
                        key,
                        name: match key {
                            ReadingTheme::SeaGlass => "Sea glass & amber",
                            ReadingTheme::InkSaffron => "Ink & saffron",
                            ReadingTheme::CharcoalRaspberry => "Charcoal & raspberry",
                            ReadingTheme::CobaltCitron => "Cobalt & citron",
                            ReadingTheme::TerracottaLinen => "Terracotta & linen",
                            ReadingTheme::AubergineApricot => "Aubergine & apricot",
                            ReadingTheme::MidnightMint => "Midnight & mint",
                        },
                        dark: r["dark"].as_bool().expect("dark mode"),
                        page: c("page"),
                        chrome: c("chrome"),
                        surface: c("surface"),
                        raised: c("raised"),
                        border: c("border"),
                        ink: c("ink"),
                        muted: c("muted"),
                        accent: c("accent"),
                        action_light: c("action_light"),
                        accent_ink: c("accent_ink"),
                        on_accent: c("on_accent"),
                        tint: c("tint"),
                        selection: c("selection"),
                        feature: c("feature"),
                        feature_light: c("feature_light"),
                        glow: c("glow"),
                        mark: c("mark"),
                        on_feature: c("on_feature"),
                        green: c("green"),
                        error: c("error"),
                        scrim: c("scrim"),
                        highlights: ["amber", "sage", "blue", "rose"].map(|role| {
                            parse_color(r["highlights"][role].as_str().expect("highlight role"))
                        }),
                    }
                })
                .collect()
        });
        *palettes.iter().find(|p| p.key == key).expect("known theme")
    }
}
fn parse_color(hex: &str) -> Color32 {
    assert!(
        hex.starts_with('#') && hex.len() == 7,
        "six-digit sRGB palette color"
    );
    let n = u32::from_str_radix(&hex[1..], 16).expect("hex palette color");
    Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8)
}
pub fn current(ctx: &egui::Context) -> Palette {
    Palette::for_theme(
        ctx.data(|d| d.get_temp::<ReadingTheme>(egui::Id::new("fontes-reading-theme")))
            .unwrap_or_default(),
    )
}
pub fn set(ctx: &egui::Context, theme: ReadingTheme) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("fontes-reading-theme"), theme));
}
