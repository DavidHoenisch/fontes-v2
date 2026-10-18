//! In-place reference selection: only one grid scrolls at a time.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum PickerStage {
    Book,
    Chapter,
    Verse,
}

impl FontesApp {
    pub(super) fn open_picker(&mut self) {
        if let Some(state) = &self.state {
            let reference = state.session.selected_ref;
            self.picker_book = Some((reference / 1_000_000) as u16);
            self.picker_chapter = (reference % 1_000_000 / 1000) as u16;
            self.picker_verse = (reference % 1000) as u16;
            if let Some(book) = state.books.iter().find(|b| Some(b.id) == self.picker_book) {
                self.testament = book.testament.clone();
            }
        }
        self.picker_stage = PickerStage::Book;
        self.picker_reset_scroll = true;
        self.picker = true;
    }

    fn picker_stage(&mut self, stage: PickerStage) {
        self.picker_stage = stage;
        self.picker_reset_scroll = true;
    }

    pub(super) fn reference_picker(&mut self, ui: &mut egui::Ui, mobile: bool) {
        let state = self.state.clone().unwrap();
        let book = self
            .picker_book
            .and_then(|id| state.books.iter().find(|b| b.id == id));
        let width = (ui.available_width().min(ui.clip_rect().width()) - 48.0).max(44.0);
        if let Some(book) = book {
            egui::Panel::bottom("picker-confirm")
                .exact_size(80.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::desk(ui.ctx()))
                        .inner_margin(egui::Margin::symmetric(24, 16)),
                )
                .show(ui, |ui| {
                    ui.set_width(width);
                    if primary(
                        ui,
                        &format!(
                            "Open {} {}:{}",
                            book.name, self.picker_chapter, self.picker_verse
                        ),
                    ) {
                        self.select(
                            i64::from(book.id) * 1_000_000
                                + i64::from(self.picker_chapter) * 1000
                                + i64::from(self.picker_verse),
                        );
                    }
                });
        }

        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 16,
                bottom: 8,
            })
            .show(ui, |ui| {
                ui.set_width(width);
                ui.spacing_mut().button_padding.x = 6.0;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Go to a passage").size(20.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized([56.0, 44.0], egui::Button::new("Close").frame(false))
                            .clicked()
                        {
                            self.picker = false;
                        }
                    });
                });
                if let Some(book) = book
                    && ui
                        .add_sized(
                            [ui.available_width(), 44.0],
                            egui::Button::new(format!(
                                "About {} · artwork & introduction",
                                book.name
                            )),
                        )
                        .clicked()
                {
                    self.book_opening = Some(book.id);
                    self.destination = Destination::Read;
                    self.picker = false;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let input = ui.add_sized(
                        [(ui.available_width() - 56.0).max(44.0), 44.0],
                        egui::TextEdit::singleline(&mut self.reference)
                            .hint_text("John 3:16 or Rom 1:3"),
                    );
                    if (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui
                            .add_sized([48.0, 44.0], egui::Button::new("Go"))
                            .clicked()
                    {
                        self.dispatch(Command::Navigate {
                            reference: self.reference.clone(),
                        });
                        self.picker = false;
                    }
                });
                ui.add_space(12.0);
                ui.columns(3, |cols| {
                    for (col, (stage, title, value)) in cols.iter_mut().zip([
                        (
                            PickerStage::Book,
                            "Book",
                            book.map(|b| b.osis.clone())
                                .unwrap_or_else(|| "Choose".into()),
                        ),
                        (
                            PickerStage::Chapter,
                            "Chapter",
                            self.picker_chapter.to_string(),
                        ),
                        (PickerStage::Verse, "Verse", self.picker_verse.to_string()),
                    ]) {
                        let button = egui::Button::new(format!("{title}\n{value}"))
                            .selected(self.picker_stage == stage)
                            .min_size(egui::vec2(col.available_width(), 52.0));
                        if col
                            .add_enabled(stage == PickerStage::Book || book.is_some(), button)
                            .clicked()
                        {
                            self.picker_stage(stage);
                        }
                    }
                });
            });
        ui.separator();

        // Stage and canonical parent identify independent scroll positions. Advancing always
        // starts the next grid at its beginning, even after scrolling Psalms or a long chapter.
        let scroll = egui::ScrollArea::vertical().id_salt((
            "reference-grid",
            self.picker_stage,
            self.picker_book,
            self.picker_chapter,
        ));
        let scroll = if std::mem::take(&mut self.picker_reset_scroll) {
            scroll.vertical_scroll_offset(0.0)
        } else {
            scroll
        };
        scroll.show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(24, 12))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().button_padding.x = 6.0;
                    match self.picker_stage {
                        PickerStage::Book => {
                            ui.columns(2, |cols| {
                                for (col, (testament, title)) in cols
                                    .iter_mut()
                                    .zip([("OT", "Old Testament"), ("NT", "New Testament")])
                                {
                                    if col
                                        .add_sized(
                                            [col.available_width(), 44.0],
                                            egui::Button::new(title)
                                                .selected(self.testament == testament),
                                        )
                                        .clicked()
                                    {
                                        self.testament = testament.into();
                                        self.picker_reset_scroll = true;
                                    }
                                }
                            });
                            ui.add_space(8.0);
                            let cell_width = if mobile { 56.0 } else { 76.0 };
                            let columns = ((ui.available_width() + 5.0) / (cell_width + 5.0))
                                .floor()
                                .clamp(1.0, if mobile { 5.0 } else { 8.0 })
                                as usize;
                            let cell_width = if mobile {
                                (ui.available_width() - (columns - 1) as f32 * 5.0) / columns as f32
                            } else {
                                cell_width
                            };
                            let testament = self.testament.clone();
                            egui::Grid::new("book-grid")
                                .spacing([5.0, 5.0])
                                .show(ui, |ui| {
                                    for (i, book) in state
                                        .books
                                        .iter()
                                        .filter(|b| b.testament == testament)
                                        .enumerate()
                                    {
                                        if ui
                                            .add_sized(
                                                [cell_width, 44.0],
                                                egui::Button::new(
                                                    RichText::new(&book.osis).size(12.0),
                                                )
                                                .truncate()
                                                .selected(self.picker_book == Some(book.id)),
                                            )
                                            .on_hover_text(&book.name)
                                            .clicked()
                                        {
                                            if self.picker_book != Some(book.id) {
                                                self.picker_chapter = 1;
                                                self.picker_verse = 1;
                                            }
                                            self.picker_book = Some(book.id);
                                            self.picker_stage(PickerStage::Chapter);
                                        }
                                        if (i + 1) % columns == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                            ui.add_space(24.0);
                            for (kind, title) in [
                                ("favorite", "FAVORITE PASSAGES"),
                                ("recent", "RECENT PASSAGES"),
                            ] {
                                caption(ui, title);
                                for saved in state.saved.iter().filter(|r| r.kind == kind).take(8) {
                                    if ui
                                        .add_sized(
                                            [ui.available_width(), 44.0],
                                            egui::Button::new(&saved.label),
                                        )
                                        .clicked()
                                    {
                                        self.select(saved.ref_id);
                                    }
                                }
                                ui.add_space(16.0);
                            }
                        }
                        PickerStage::Chapter => {
                            if let Some(book) = book {
                                ui.label(RichText::new(&book.name).size(20.0));
                                caption(ui, "CHOOSE A CHAPTER");
                                ui.add_space(12.0);
                                let previous = self.picker_chapter;
                                if number_grid(
                                    ui,
                                    "chapters",
                                    book.verse_counts.len() as u16,
                                    &mut self.picker_chapter,
                                    if mobile { 6 } else { 10 },
                                ) {
                                    if previous != self.picker_chapter {
                                        self.picker_verse = 1;
                                    }
                                    self.picker_stage(PickerStage::Verse);
                                }
                            }
                        }
                        PickerStage::Verse => {
                            if let Some(book) = book {
                                ui.label(
                                    RichText::new(format!("{} {}", book.name, self.picker_chapter))
                                        .size(20.0),
                                );
                                caption(ui, "CHOOSE A VERSE TO OPEN");
                                ui.add_space(12.0);
                                let count = book.verse_counts[(self.picker_chapter - 1) as usize];
                                if number_grid(
                                    ui,
                                    "verses",
                                    count,
                                    &mut self.picker_verse,
                                    if mobile { 6 } else { 10 },
                                ) {
                                    self.select(
                                        i64::from(book.id) * 1_000_000
                                            + i64::from(self.picker_chapter) * 1000
                                            + i64::from(self.picker_verse),
                                    );
                                }
                            }
                        }
                    }
                });
        });
    }
}
