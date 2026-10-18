//! Reading controls stay close to the passage on mobile.
use super::*;

pub(super) fn theme_choices(ui: &mut egui::Ui, selected_theme: &mut ReadingTheme) {
    ui.label("Color theme");
    caption(ui, "Choose the colors that feel right for your reading.");
    ui.add_space(8.0);
    for key in ReadingTheme::ALL {
        let candidate = theme::Palette::for_theme(key);
        let selected = *selected_theme == key;
        let response = ui.add_sized(
            [ui.available_width(), 52.0],
            egui::Button::new("").selected(selected),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, candidate.name)
        });
        let swatch = egui::Rect::from_min_size(
            response.rect.min + egui::vec2(12.0, 10.0),
            egui::vec2(64.0, 32.0),
        );
        ui.painter().rect_filled(swatch, 8, candidate.page);
        ui.painter().rect_filled(
            egui::Rect::from_min_max(swatch.min + egui::vec2(28.0, 0.0), swatch.max),
            8,
            candidate.feature,
        );
        ui.painter().rect_filled(
            egui::Rect::from_min_size(swatch.min + egui::vec2(5.0, 8.0), egui::vec2(19.0, 16.0)),
            4,
            candidate.selection,
        );
        let galley = ui.painter().layout(
            candidate.name.to_owned(),
            egui::FontId::proportional(13.0),
            theme::palette(ui.ctx()).ink,
            (response.rect.width() - 116.0).max(64.0),
        );
        ui.painter().galley(
            egui::pos2(
                response.rect.left() + 88.0,
                response.rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            theme::palette(ui.ctx()).ink,
        );
        if selected {
            icon(ui.ctx(), "check", 16.0, true).paint_at(
                ui,
                egui::Rect::from_center_size(
                    egui::pos2(response.rect.right() - 16.0, response.rect.center().y),
                    egui::vec2(16.0, 16.0),
                ),
            );
        }
        if response.clicked() {
            *selected_theme = key;
        }
    }
    ui.add_space(24.0);
}

fn typeface_controls(ui: &mut egui::Ui, p: &mut Preferences) {
    theme_choices(ui, &mut p.theme);
    ui.label("Scripture typeface");
    ui.columns(2, |cols| {
        for (col, (serif, title)) in cols
            .iter_mut()
            .zip([(true, "Lora"), (false, "IBM Plex Sans")])
        {
            if col
                .add_sized(
                    [col.available_width(), 44.0],
                    egui::Button::new(title).selected(p.scripture_serif == serif),
                )
                .clicked()
            {
                p.scripture_serif = serif;
            }
        }
    });
    ui.add_space(16.0);
    let label = ui.label("Line spacing");
    ui.spacing_mut().slider_width = ui.available_width();
    ui.add(egui::Slider::new(&mut p.line_spacing, 1.2..=2.2).show_value(false))
        .labelled_by(label.id);
    ui.add_space(16.0);
    ui.checkbox(&mut p.verse_numbers, "Verse numbers");
}
fn text_size(ui: &mut egui::Ui, p: &mut Preferences) {
    ui.horizontal(|ui| {
        let label = ui.add_sized([64.0, 44.0], egui::Label::new("Text size"));
        let palette = theme::palette(ui.ctx());
        let width = ui.available_width();
        let response = ui
            .scope(|ui| {
                let style = ui.style_mut();
                for w in [
                    &mut style.visuals.widgets.inactive,
                    &mut style.visuals.widgets.hovered,
                    &mut style.visuals.widgets.active,
                ] {
                    w.bg_fill = egui::Color32::TRANSPARENT;
                    w.fg_stroke = egui::Stroke::NONE;
                }
                style.visuals.selection.bg_fill = egui::Color32::TRANSPARENT;
                style.spacing.slider_width = width;
                ui.add_sized(
                    [width, 44.0],
                    egui::Slider::new(&mut p.text_size, 16.0..=36.0)
                        .show_value(false)
                        .step_by(1.0)
                        .handle_shape(egui::style::HandleShape::Circle),
                )
                .labelled_by(label.id)
            })
            .inner;
        let radius = response.rect.height() / 2.5;
        let left = response.rect.left() + radius;
        let right = response.rect.right() - radius;
        let center = egui::pos2(
            egui::lerp(left..=right, (p.text_size - 16.0) / 20.0),
            response.rect.center().y,
        );
        let rail =
            egui::Rect::from_center_size(response.rect.center(), egui::vec2(right - left, 7.0));
        ui.painter()
            .rect_filled(rail, 4, palette.border.lerp_to_gamma(palette.surface, 0.35));
        ui.painter().rect_filled(
            egui::Rect::from_min_max(rail.min, egui::pos2(center.x, rail.bottom())),
            4,
            palette.accent,
        );
        ui.painter().circle_filled(center, 8.0, palette.accent);
    });
}
fn preview(ui: &mut egui::Ui, p: &Preferences, s: &Workspace) {
    if let Some(verse) = s
        .passage
        .iter()
        .find(|v| v.refs.contains(&s.session.selected_ref))
    {
        let mut job = egui::text::LayoutJob::simple(
            verse.text.clone(),
            egui::FontId::new(
                p.text_size,
                if p.scripture_serif {
                    egui::FontFamily::Name("scripture".into())
                } else {
                    egui::FontFamily::Proportional
                },
            ),
            theme::palette(ui.ctx()).ink,
            ui.available_width(),
        );
        job.sections[0].format.line_height = Some(p.text_size * p.line_spacing);
        job.sections[0].format.extra_letter_spacing = -0.25;
        ui.add(egui::Label::new(job).wrap());
    }
}
impl FontesApp {
    pub(super) fn card_motion_control(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_sized(
                [ui.available_width(), 44.0],
                egui::Checkbox::new(&mut self.reduce_card_motion, "Reduce word-card motion"),
            )
            .changed()
            && let Err(error) = std::fs::write(
                self.data_dir.join("reduce-card-motion-v1.txt"),
                self.reduce_card_motion.to_string(),
            )
        {
            self.error = Some(format!("Could not save motion preference: {error}"));
        }
    }

    pub(super) fn reading_controls(ui: &mut egui::Ui, p: &mut Preferences, s: &Workspace) {
        typeface_controls(ui, p);
        ui.add_space(24.0);
        text_size(ui, p);
        ui.add_space(24.0);
        caption(ui, "READING PREVIEW");
        preview(ui, p, s);
        ui.add_space(24.0);
    }
    pub(super) fn mobile_reading_settings(&mut self, ctx: &egui::Context) {
        if !self.reading_settings {
            return;
        }
        let Some(state) = self.state.clone() else {
            return;
        };
        let mut preferences = state.session.preferences.clone();
        let bounds = ctx.content_rect();
        let width = bounds.width().min(480.0);
        let height =
            (bounds.height() * 0.88).min(if self.reading_advanced { 680.0 } else { 600.0 });
        let mut manage_library = false;
        let modal = egui::Modal::new(egui::Id::new("reading-settings-sheet"))
            .area(
                egui::Modal::default_area(egui::Id::new("reading-settings-sheet"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO),
            )
            .frame(
                egui::Frame::new()
                    .fill(theme::palette(ctx).surface)
                    .corner_radius(egui::CornerRadius {
                        nw: 28,
                        ne: 28,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(egui::Margin {
                        left: 22,
                        right: 22,
                        top: 10,
                        bottom: 22,
                    }),
            )
            .backdrop_color(theme::palette(ctx).scrim.gamma_multiply(0.26))
            .show(ctx, |ui| {
                ui.set_width(width - 44.0);
                ui.set_height(height - 44.0);
                theme::sheet_handle(ui);
                ui.horizontal(|ui| {
                    caption(ui, "Reading preferences");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::round_close(ui, "Close preferences").clicked() {
                            ui.close();
                        }
                    });
                });
                // Done remains outside the scroll view, including with the keyboard open.
                let done_top = ui.max_rect().bottom() - 48.0;
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .id_salt("reading-settings-scroll")
                    .max_height((done_top - ui.cursor().min.y - 16.0).max(44.0))
                    .show(ui, |ui| {
                        theme::editorial(
                            ui,
                            "Make room for the words.",
                            25.0,
                            theme::palette(ui.ctx()).ink,
                        );
                        ui.add_space(20.0);
                        text_size(ui, &mut preferences);
                        ui.add_space(24.0);
                        preview(ui, &preferences, &state);
                        ui.add_space(24.0);
                        let disclosure = ui.add_sized(
                            [ui.available_width(), 44.0],
                            egui::Button::new(if self.reading_advanced {
                                "Theme & typography  −"
                            } else {
                                "Theme & typography  +"
                            })
                            .frame(false),
                        );
                        disclosure.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                "Theme & typography",
                            )
                        });
                        if disclosure.clicked() {
                            self.reading_advanced = !self.reading_advanced;
                        }
                        // Explicit label remains stable when the disclosure indicator changes.
                        if self.reading_advanced {
                            typeface_controls(ui, &mut preferences);
                            self.card_motion_control(ui);
                            ui.add_space(16.0);
                            manage_library = ui
                                .add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Button::image_and_text(
                                        icon(ui.ctx(), "book", 20.0, false),
                                        "Library settings",
                                    )
                                    .right_text(()),
                                )
                                .clicked();
                        }
                    });
                ui.allocate_space(egui::vec2(0.0, (done_top - ui.cursor().min.y).max(0.0)));
                if primary(ui, "Done") {
                    ui.close();
                }
            });
        if preferences != state.session.preferences {
            self.dispatch(Command::Preferences { preferences });
        }
        if modal.should_close() || manage_library {
            self.reading_settings = false;
        }
        if manage_library {
            self.choose(Destination::Preferences);
        }
    }
}

/// Keep attribution/source access in settings rather than the study reading flow.
pub(super) fn source_credits(ui: &mut egui::Ui) {
    ui.collapsing("Source credits & licenses", |ui| {
        ui.collapsing("Chapter heading typeface", |ui| {
            ui.label("Cormorant Garamond Italic — Christian Thalmann and the Cormorant Project Authors. SIL Open Font License 1.1.");
            ui.hyperlink_to("Typeface source", "https://github.com/google/fonts/tree/main/ofl/cormorantgaramond");
            ui.collapsing("Font license", |ui| { ui.label(include_str!("../../../assets/ui/licenses/CORMORANT-GARAMOND-OFL.txt")); });
        });
        ui.label("Credits for resources distributed with the Study library.");
            ui.collapsing("Hebrew source and license", |ui| {
                ui.label("Original work of the Open Scriptures Hebrew Bible available at https://github.com/openscriptures/morphhb");
                ui.hyperlink_to("Open Scriptures Hebrew Bible", "https://github.com/openscriptures/morphhb");
                ui.hyperlink_to("CC BY 4.0", "https://creativecommons.org/licenses/by/4.0/");
                caption(ui,"Word forms and analyses are retained from OSHB. Verse numbering is adapted to the reading edition; variants in source notes are not shown. No endorsement implied.");
            });
            ui.collapsing("KJV annotation source and license", |ui| {
                ui.label("Strong’s tags: Bible Foundation (OT), KJV2003/CrossWire and Maurice Robinson (NT). Formatting: Michael Paul Johnson/eBible.org. Copyright © 2003–2023 CrossWire Bible Society. Distributed under GNU GPL version 3.");
                ui.hyperlink_to("Source and credits", "https://wiki.crosswire.org/CrossWire_KJV");
                ui.hyperlink_to("Editable USFM source", "https://ebible.org/Scriptures/eng-kjv_usfm.zip?x=1");
                caption(ui, "Tags are checked against this KJV text; mismatched verses are withheld. A dictionary tag does not identify a Hebrew or Greek verse token.");
                ui.collapsing("GNU General Public License", |ui| {
                    ui.label(include_str!("../../../data/rights/GPL-3.0.txt"));
                });
            });
    });
}
