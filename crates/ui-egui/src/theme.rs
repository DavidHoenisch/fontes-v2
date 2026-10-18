use eframe::egui::{self, Color32, FontFamily, FontId, RichText};
#[allow(dead_code)]
#[path = "../../../assets/ui/assets.rs"]
pub mod assets;
#[path = "palette.rs"]
mod palettes;
pub use palettes::{Palette, current as palette};
pub fn accent(ctx: &egui::Context) -> Color32 {
    palette(ctx).accent_ink
}
pub fn muted(ctx: &egui::Context) -> Color32 {
    palette(ctx).muted
}
pub fn desk(ctx: &egui::Context) -> Color32 {
    palette(ctx).page
}
pub fn tint(ctx: &egui::Context) -> Color32 {
    palette(ctx).tint
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
        FontFamily::Name("chapter_heading".into()),
        vec!["chapter_heading".into(), "scripture".into()],
    );
    fonts.families.insert(
        FontFamily::Name("original".into()),
        vec!["greek_fallback".into(), "hebrew_fallback".into()],
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
    apply(ctx, fontes_engine::ReadingTheme::default());
    egui_extras::install_image_loaders(ctx);
}
pub fn apply(ctx: &egui::Context, key: fontes_engine::ReadingTheme) {
    let p = Palette::for_theme(key);
    palettes::set(ctx, key);
    let mut style = egui::Style {
        visuals: if p.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        },
        ..Default::default()
    };
    style.visuals.panel_fill = p.page;
    style.visuals.window_fill = p.surface;
    style.visuals.extreme_bg_color = p.surface;
    style.visuals.faint_bg_color = p.surface;
    style.visuals.override_text_color = Some(p.ink);
    style.visuals.hyperlink_color = p.accent_ink;
    style.visuals.error_fg_color = p.error;
    style.visuals.window_stroke = egui::Stroke::new(1.0, p.border);
    style.visuals.window_corner_radius = egui::CornerRadius::same(16);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(16);
    style.visuals.selection.bg_fill = p.tint;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, p.accent_ink);
    style.visuals.widgets.inactive.weak_bg_fill = p.raised;
    style.visuals.widgets.inactive.bg_fill = p.raised;
    style.visuals.widgets.hovered.bg_fill = p.tint;
    style.visuals.widgets.hovered.weak_bg_fill = p.tint;
    style.visuals.widgets.active.bg_fill = p.tint;
    style.visuals.widgets.active.weak_bg_fill = p.tint;
    style.visuals.widgets.open.bg_fill = p.tint;
    style.visuals.widgets.open.weak_bg_fill = p.tint;
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(12);
        widget.bg_stroke = egui::Stroke::NONE;
        widget.fg_stroke = egui::Stroke::new(1.5, p.accent_ink);
        widget.expansion = 0.0;
    }
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.muted);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
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
    // Select the target slot before installing its style. Otherwise a light →
    // dark switch overwrites the previous slot and opens with stale defaults.
    ctx.set_theme(if p.dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    ctx.set_global_style(style);
}
pub fn icon(ctx: &egui::Context, role: &str, size: f32, selected: bool) -> egui::Image<'static> {
    let (_, name, bytes) = assets::ICONS
        .iter()
        .find(|(r, _, _)| *r == role)
        .expect("known icon");
    egui::Image::from_bytes(format!("bytes://{name}.svg"), *bytes)
        .fit_to_exact_size(egui::vec2(size, size))
        .tint(if selected { accent(ctx) } else { muted(ctx) })
}
pub fn nav(ui: &mut egui::Ui, role: &str, text: &str, selected: bool, size: egui::Vec2) -> bool {
    ui.add_sized(
        size,
        egui::Button::image_and_text(
            icon(ui.ctx(), role, 20.0, selected),
            RichText::new(text).color(if selected {
                accent(ui.ctx())
            } else {
                muted(ui.ctx())
            }),
        )
        // Reserve remaining width after the icon/label so sidebar rows align left.
        .right_text(())
        .fill(if selected {
            tint(ui.ctx())
        } else {
            Color32::TRANSPARENT
        }),
    )
    .clicked()
}
pub fn primary(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add_sized(
        [ui.available_width(), 48.0],
        egui::Button::new(RichText::new(text).color(palette(ui.ctx()).on_accent))
            .fill(palette(ui.ctx()).accent)
            .corner_radius(16),
    )
    .clicked()
}
pub fn mobile_nav(ui: &mut egui::Ui, role: &str, text: &str, selected: bool) -> bool {
    let response = ui.add_sized(
        [ui.available_width(), 48.0],
        egui::Button::new("").frame(false),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    let center = response.rect.center().x;
    if selected || response.hovered() {
        let pill = egui::Rect::from_center_size(
            egui::pos2(center, response.rect.top() + 17.0),
            egui::vec2(50.0, 30.0),
        );
        if selected {
            ui.painter().add(
                egui::epaint::Shadow {
                    offset: [0, 3],
                    blur: 8,
                    spread: 0,
                    color: palette(ui.ctx()).scrim.gamma_multiply(0.16),
                }
                .as_shape(pill, egui::CornerRadius::same(16)),
            );
        }
        ui.painter().rect_filled(
            pill,
            16,
            if selected {
                palette(ui.ctx()).accent
            } else {
                tint(ui.ctx())
            },
        );
    }
    icon(ui.ctx(), role, 22.0, selected)
        .tint(if selected {
            palette(ui.ctx()).on_accent
        } else {
            muted(ui.ctx())
        })
        .paint_at(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(center, response.rect.top() + 17.0),
                egui::vec2(22.0, 22.0),
            ),
        );
    ui.painter().text(
        egui::pos2(center, response.rect.top() + 34.0),
        egui::Align2::CENTER_TOP,
        text,
        FontId::new(
            10.0,
            if selected {
                FontFamily::Name("medium".into())
            } else {
                FontFamily::Proportional
            },
        ),
        if selected {
            accent(ui.ctx())
        } else {
            muted(ui.ctx())
        },
    );
    response.clicked()
}
pub fn caption(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(12.0).color(muted(ui.ctx())));
}

/// Paint a rounded, softly graded surface into a reserved background slot.
/// Geometry stays native; no textures, blur passes or runtime assets are needed.
pub fn wash(
    ui: &egui::Ui,
    slot: egui::layers::ShapeIdx,
    rect: egui::Rect,
    radius: f32,
    left: Color32,
    right: Color32,
) {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut mesh = egui::Mesh::default();
    let color = |point: egui::Pos2| {
        let t = ((point.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        left.lerp_to_gamma(right, t)
    };
    mesh.colored_vertex(rect.center(), color(rect.center()));
    for (center, angle) in [
        (rect.min + egui::vec2(radius, radius), std::f32::consts::PI),
        (
            egui::pos2(rect.right() - radius, rect.top() + radius),
            std::f32::consts::PI * 1.5,
        ),
        (rect.max - egui::vec2(radius, radius), 0.0),
        (
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            std::f32::consts::FRAC_PI_2,
        ),
    ] {
        for step in 0..=8 {
            let a = angle + step as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
            let point = center + egui::vec2(a.cos(), a.sin()) * radius;
            mesh.colored_vertex(point, color(point));
        }
    }
    let count = mesh.vertices.len() as u32 - 1;
    for i in 1..=count {
        mesh.add_triangle(0, i, if i == count { 1 } else { i + 1 });
    }
    ui.painter().set(slot, egui::Shape::mesh(mesh));
}

pub fn sheet_handle(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), egui::Sense::hover());
    ui.painter().rect_filled(
        egui::Rect::from_center_size(rect.center(), egui::vec2(36.0, 4.0)),
        2,
        palette(ui.ctx()).border,
    );
    ui.add_space(18.0);
}

pub fn card(ctx: &egui::Context) -> egui::Frame {
    let p = palette(ctx);
    egui::Frame::new()
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(18)
        .inner_margin(20)
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(p.scrim.r(), p.scrim.g(), p.scrim.b(), 16),
        })
}

/// Match the reference's small, tracked section labels without baking text into images.
pub fn eyebrow(ui: &mut egui::Ui, text: impl Into<String>, color: Color32) {
    let mut job = egui::text::LayoutJob::simple(
        text.into(),
        FontId::new(9.0, FontFamily::Name("medium".into())),
        color,
        ui.available_width(),
    );
    job.sections[0].format.extra_letter_spacing = 1.3;
    job.sections[0].format.line_height = Some(12.5);
    ui.add(egui::Label::new(job).wrap().halign(egui::Align::LEFT));
}
pub fn editorial(ui: &mut egui::Ui, text: impl Into<String>, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::simple(
        text.into(),
        FontId::new(size, FontFamily::Name("scripture".into())),
        color,
        ui.available_width(),
    );
    job.sections[0].format.extra_letter_spacing = -size * 0.025;
    job.sections[0].format.line_height = Some(size * 1.18);
    ui.add(egui::Label::new(job).wrap());
}
/// A bounded rounded grid samples an elliptical radial glow, like the HTML reference.
pub fn radial_wash(
    ui: &egui::Ui,
    slot: egui::layers::ShapeIdx,
    rect: egui::Rect,
    radius: f32,
    base: Color32,
    glow: Color32,
    strength: f32,
) {
    const N: u32 = 24;
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut mesh = egui::Mesh::default();
    let mut rows = (0..=N)
        .map(|y| rect.height() * y as f32 / N as f32)
        .collect::<Vec<_>>();
    for step in 0..=12 {
        let edge = r * (1.0 - (step as f32 / 12.0 * std::f32::consts::FRAC_PI_2).cos());
        rows.extend([edge, rect.height() - edge]);
    }
    rows.sort_by(f32::total_cmp);
    rows.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    for row in &rows {
        let yy = rect.top() + row;
        let edge = (*row).min(rect.height() - row);
        let inset = if edge < r {
            r - (r * r - (r - edge).powi(2)).max(0.0).sqrt()
        } else {
            0.0
        };
        for x in 0..=N {
            let xx = rect.left() + inset + (rect.width() - 2.0 * inset) * x as f32 / N as f32;
            let dx = (xx - (rect.left() + rect.width() * 0.95)) / rect.width();
            let dy = (yy - rect.top()) / rect.height();
            let falloff = (1.0 - (dx * dx + dy * dy).sqrt() / 0.95).clamp(0.0, 1.0);
            mesh.colored_vertex(
                egui::pos2(xx, yy),
                base.lerp_to_gamma(glow, strength * falloff),
            );
        }
    }
    for y in 0..rows.len() as u32 - 1 {
        for x in 0..N {
            let a = y * (N + 1) + x;
            mesh.add_triangle(a, a + 1, a + N + 1);
            mesh.add_triangle(a + 1, a + N + 2, a + N + 1);
        }
    }
    ui.painter().set(slot, egui::Shape::mesh(mesh));
}
pub fn word_rings(ui: &egui::Ui, slot: egui::layers::ShapeIdx, rect: egui::Rect) {
    let center = egui::pos2(rect.right() + 42.0, rect.top() + 112.0);
    let mut shapes = Vec::new();
    for (radius, alpha) in [(90.0, 14), (112.0, 9), (135.0, 6)] {
        for step in 0..180 {
            let a = step as f32 / 180.0 * std::f32::consts::TAU;
            let b = (step + 1) as f32 / 180.0 * std::f32::consts::TAU;
            let points = [
                center + egui::vec2(a.cos(), a.sin()) * radius,
                center + egui::vec2(b.cos(), b.sin()) * radius,
            ];
            if points.iter().all(|p| rect.shrink(18.0).contains(*p)) {
                shapes.push(egui::Shape::line_segment(
                    points,
                    egui::Stroke::new(1.0, Color32::from_white_alpha(alpha)),
                ));
            }
        }
    }
    ui.painter().set(slot, egui::Shape::Vec(shapes));
}
pub fn round_close(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let response = ui.add_sized([44.0, 44.0], egui::Button::new("").frame(false));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    ui.painter()
        .circle_filled(response.rect.center(), 18.0, palette(ui.ctx()).tint);
    icon(ui.ctx(), "close", 16.0, false)
        .tint(palette(ui.ctx()).ink)
        .paint_at(
            ui,
            egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0)),
        );
    response.on_hover_text(label)
}
