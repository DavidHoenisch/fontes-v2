use eframe::egui::{self, Color32, FontFamily, FontId, RichText};
#[allow(dead_code)]
#[path = "../../../assets/ui/assets.rs"]
pub mod assets;
pub fn color(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}
pub fn accent() -> Color32 {
    color(assets::COLOR_ACCENT)
}
pub fn muted() -> Color32 {
    color(assets::COLOR_MUTED)
}
pub fn desk() -> Color32 {
    color(assets::COLOR_DESK)
}
pub fn tint() -> Color32 {
    color(assets::COLOR_TINT)
}
pub fn scripture(text: impl Into<String>, size: f32, serif: bool) -> RichText {
    RichText::new(text).font(FontId::new(
        size,
        if serif {
            FontFamily::Name("scripture".into())
        } else {
            FontFamily::Proportional
        },
    ))
}
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::empty();
    for (family, _, role, bytes) in assets::FONTS {
        fonts
            .font_data
            .insert((*role).into(), egui::FontData::from_static(bytes).into());
        let _ = family;
    }
    fonts.families.insert(
        FontFamily::Proportional,
        vec![
            "ui_body".into(),
            "greek_fallback".into(),
            "hebrew_fallback".into(),
        ],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![
            "ui_body".into(),
            "greek_fallback".into(),
            "hebrew_fallback".into(),
        ],
    );
    fonts.families.insert(
        FontFamily::Name("scripture".into()),
        vec![
            "scripture".into(),
            "greek_fallback".into(),
            "hebrew_fallback".into(),
        ],
    );
    fonts.families.insert(
        FontFamily::Name("medium".into()),
        vec![
            "ui_label".into(),
            "greek_fallback".into(),
            "hebrew_fallback".into(),
        ],
    );
    ctx.set_fonts(fonts);
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    style.visuals.panel_fill = desk();
    style.visuals.window_fill = color(assets::COLOR_SURFACE);
    style.visuals.extreme_bg_color = color(assets::COLOR_SIDEBAR);
    style.visuals.faint_bg_color = color(assets::COLOR_SURFACE);
    style.visuals.override_text_color = Some(color(assets::COLOR_INK));
    style.visuals.selection.bg_fill = tint();
    style.visuals.selection.stroke = egui::Stroke::new(1.0, accent());
    style.visuals.widgets.inactive.weak_bg_fill = color(assets::COLOR_RAISED);
    style.visuals.widgets.inactive.bg_fill = color(assets::COLOR_RAISED);
    style.visuals.widgets.hovered.bg_fill = tint();
    style.visuals.widgets.active.bg_fill = tint();
    style.visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, color(assets::COLOR_BORDER));
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(15.0));
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(14.0, FontFamily::Name("medium".into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(24.0, FontFamily::Name("medium".into())),
    );
    ctx.set_global_style(style);
    ctx.set_theme(egui::Theme::Dark);
    egui_extras::install_image_loaders(ctx);
}
pub fn icon(role: &str, size: f32, selected: bool) -> egui::Image<'static> {
    let (_, name, bytes) = assets::ICONS
        .iter()
        .find(|(r, _, _)| *r == role)
        .expect("known icon");
    egui::Image::from_bytes(format!("bytes://{name}.svg"), *bytes)
        .fit_to_exact_size(egui::vec2(size, size))
        .tint(if selected { accent() } else { muted() })
}
pub fn nav(ui: &mut egui::Ui, role: &str, text: &str, selected: bool, size: egui::Vec2) -> bool {
    ui.add_sized(
        size,
        egui::Button::image_and_text(
            icon(role, 20.0, selected),
            RichText::new(text).color(if selected { accent() } else { muted() }),
        )
        .fill(if selected {
            tint()
        } else {
            Color32::TRANSPARENT
        }),
    )
    .clicked()
}
pub fn primary(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add_sized(
        [ui.available_width(), 48.0],
        egui::Button::new(RichText::new(text).color(color(assets::COLOR_INVERSE))).fill(accent()),
    )
    .clicked()
}
pub fn mobile_nav(ui: &mut egui::Ui, role: &str, text: &str, selected: bool) -> bool {
    let response = ui.add_sized(
        [ui.available_width(), 64.0],
        egui::Button::new("").fill(if selected {
            tint()
        } else {
            Color32::TRANSPARENT
        }),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    let center = response.rect.center().x;
    icon(role, 24.0, selected).paint_at(
        ui,
        egui::Rect::from_center_size(
            egui::pos2(center, response.rect.top() + 20.0),
            egui::vec2(24.0, 24.0),
        ),
    );
    ui.painter().text(
        egui::pos2(center, response.rect.top() + 40.0),
        egui::Align2::CENTER_TOP,
        text,
        FontId::proportional(12.0),
        if selected { accent() } else { muted() },
    );
    response.clicked()
}
pub fn caption(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(12.0).color(muted()));
}
