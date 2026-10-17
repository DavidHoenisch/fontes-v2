//! Reading controls stay close to the passage on mobile.
use super::*;

impl FontesApp {
    pub(super) fn reading_controls(ui: &mut egui::Ui, p: &mut Preferences, s: &Workspace) {
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
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = ui.available_width();
            let label = ui
                .horizontal(|ui| {
                    let label = ui.label("Text size");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} px", p.text_size)).color(muted()));
                    });
                    label
                })
                .inner;
            ui.add(
                egui::Slider::new(&mut p.text_size, 16.0..=36.0)
                    .show_value(false)
                    .step_by(1.0),
            )
            .labelled_by(label.id);
            ui.add_space(12.0);
            let label = ui
                .horizontal(|ui| {
                    let label = ui.label("Line spacing");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.2}×", p.line_spacing)).color(muted()));
                    });
                    label
                })
                .inner;
            ui.add(egui::Slider::new(&mut p.line_spacing, 1.2..=2.2).show_value(false))
                .labelled_by(label.id);
        });
        ui.add_space(16.0);
        ui.checkbox(&mut p.verse_numbers, "Verse numbers");
        ui.add_space(24.0);
        caption(ui, "READING PREVIEW");
        if let Some(verse) = s
            .passage
            .iter()
            .find(|v| v.refs.contains(&s.session.selected_ref))
        {
            egui::Frame::new()
                .fill(theme::desk())
                .corner_radius(16)
                .inner_margin(16)
                .show(ui, |ui| {
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
                        theme::color(assets::COLOR_INK),
                        ui.available_width(),
                    );
                    for section in &mut job.sections {
                        section.format.line_height = Some(p.text_size * p.line_spacing);
                    }
                    ui.add(egui::Label::new(job).wrap());
                });
        }
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
        let height = (bounds.height() * 0.88).min(680.0);
        let mut manage_library = false;
        let modal = egui::Modal::new(egui::Id::new("reading-settings-sheet"))
            .area(
                egui::Modal::default_area(egui::Id::new("reading-settings-sheet"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO),
            )
            .frame(
                egui::Frame::new()
                    .fill(theme::color(assets::COLOR_SURFACE))
                    .corner_radius(egui::CornerRadius {
                        nw: 24,
                        ne: 24,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(24),
            )
            .backdrop_color(theme::color(assets::COLOR_SCRIM).gamma_multiply(0.24))
            .show(ctx, |ui| {
                ui.set_width(width - 48.0);
                ui.set_height(height - 48.0);
                ui.horizontal(|ui| {
                    ui.label(scripture("Reading style", 24.0, true));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized([60.0, 44.0], egui::Button::new("Done").frame(false))
                            .clicked()
                        {
                            ui.close();
                        }
                    });
                });
                ui.add_space(12.0);
                egui::ScrollArea::vertical()
                    .id_salt("reading-settings-scroll")
                    .max_height((height - 116.0).max(44.0))
                    .show(ui, |ui| {
                        Self::reading_controls(ui, &mut preferences, &state);
                        manage_library = ui
                            .add_sized(
                                [ui.available_width(), 44.0],
                                egui::Button::image_and_text(
                                    icon("book", 20.0, false),
                                    "Library settings",
                                )
                                .right_text(()),
                            )
                            .clicked();
                    });
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
