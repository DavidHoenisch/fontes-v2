//! Bundled book artwork and public-domain introductions; independent of installed profile.
use super::*;
mod art {
    include!("book_art.rs");
}
#[derive(serde::Deserialize)]
struct Introduction {
    id: u16,
    name: String,
    title: String,
    paragraphs: Vec<String>,
    attribution: String,
    credit: String,
    source: String,
}
fn introductions() -> &'static Vec<Introduction> {
    static DATA: std::sync::OnceLock<Vec<Introduction>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../assets/ui/book-covers/introductions.json"
        ))
        .expect("validated introductions")
    })
}
impl FontesApp {
    pub(super) fn book_opening_ui(&mut self, ui: &mut egui::Ui, mobile: bool) {
        let Some(id) = self.book_opening else {
            return;
        };
        let Some(book) = introductions().iter().find(|b| b.id == id) else {
            self.book_opening = None;
            return;
        };
        egui::Panel::bottom("book-opening-actions")
            .exact_size(112.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                if primary(ui, "Start reading") {
                    self.destination = Destination::Read;
                    self.select(i64::from(id) * 1_000_000 + 1001);
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 44.0],
                        egui::Button::new("Choose a chapter"),
                    )
                    .clicked()
                {
                    self.open_picker();
                    self.picker_book = Some(id);
                    self.picker_chapter = 1;
                    self.picker_verse = 1;
                    self.picker_stage = PickerStage::Chapter;
                    self.picker_reset_scroll = true;
                    self.book_opening = None;
                }
            });
        let content_width =
            (ui.available_width() - if mobile { 44.0 } else { 80.0 }).clamp(100.0, 680.0);
        egui::ScrollArea::vertical()
            .id_salt(("book-introduction", id))
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(if mobile { 22 } else { 40 })
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.set_max_width(content_width);
                        if ui
                            .add_sized(
                                [160.0, 44.0],
                                egui::Button::new("← Back to reading").frame(false),
                            )
                            .clicked()
                        {
                            self.book_opening = None;
                        }
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(&book.name).font(egui::FontId::new(
                                if book.name.len() > 18 { 34.0 } else { 48.0 },
                                egui::FontFamily::Name("chapter_heading".into()),
                            )));
                            ui.label(RichText::new(&book.title).size(16.0).color(muted(ui.ctx())));
                            ui.add_space(18.0);
                            let width = ui.available_width().min(340.0);
                            ui.add(
                                egui::Image::from_bytes(
                                    format!("bytes://book-cover-{id}.svg"),
                                    art::ART[usize::from(id - 1)],
                                )
                                .fit_to_exact_size(egui::vec2(width, width * 640.0 / 600.0))
                                .corner_radius(20),
                            );
                        });
                        ui.add_space(24.0);
                        theme::eyebrow(ui, "BOOK INTRODUCTION", muted(ui.ctx()));
                        ui.label(RichText::new(&book.attribution).size(15.0));
                        if !book.credit.is_empty() {
                            ui.label(
                                RichText::new(&book.credit)
                                    .size(13.0)
                                    .color(muted(ui.ctx())),
                            );
                        }
                        ui.add_space(16.0);
                        for paragraph in &book.paragraphs {
                            ui.label(RichText::new(paragraph).size(17.0));
                            ui.add_space(14.0);
                        }
                        ui.hyperlink_to("Public-domain introduction · CCEL source", &book.source);
                        ui.add_space(20.0);
                    });
            });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_book_has_bundled_art_and_separate_introduction() {
        assert_eq!(introductions().len(), 66);
        assert_eq!(art::ART.len(), 66);
        for (index, book) in introductions().iter().enumerate() {
            assert_eq!(usize::from(book.id), index + 1);
            assert!(!book.paragraphs.is_empty());
            assert!(book.paragraphs.iter().all(|p| p.len() > 100));
            assert!(book.source.ends_with(".i.html"));
            assert!(
                std::str::from_utf8(art::ART[index])
                    .unwrap()
                    .contains("<svg")
            );
        }
    }
}
