//! A complete book-distribution overview paired with small, navigable verse pages.
use super::*;
const PAGE: u32 = 6;
pub(super) struct OccurrenceReturn {
    pub code: String,
    pub book: Option<u16>,
    pub offset: u32,
}
impl FontesApp {
    pub(super) fn request_occurrences(&mut self, code: &str, book: Option<u16>, offset: u32) {
        self.occurrence_scroll_results = self.occurrence_data.is_some();
        self.occurrence_book = book;
        self.occurrence_requested = Some((code.into(), book, offset));
        self.dispatch(Command::Occurrences {
            code: code.into(),
            book_id: book,
            offset,
            limit: PAGE,
        });
    }
    pub(super) fn occurrence_return_link(&mut self, ui: &mut egui::Ui) {
        if self.destination == Destination::Read
            && self.occurrence_return.is_some()
            && ui
                .add_sized(
                    [ui.available_width(), 44.0],
                    egui::Button::new("← Return to word study").frame(false),
                )
                .clicked()
        {
            let back = self.occurrence_return.take().unwrap();
            let reference = self.state.as_ref().unwrap().session.selected_ref;
            self.open_study(reference, StudyTab::Words);
            self.lookup_word(&back.code);
            self.occurrences_open = true;
            self.request_occurrences(&back.code, back.book, back.offset);
        }
    }
    pub(super) fn occurrence_panel(
        &mut self,
        ui: &mut egui::Ui,
        code: &str,
        headword: Option<&str>,
    ) {
        let p = theme::palette(ui.ctx());
        if let Some(headword) = headword {
            ui.label(
                RichText::new(headword)
                    .font(egui::FontId::new(
                        30.0,
                        egui::FontFamily::Name("original".into()),
                    ))
                    .color(p.ink),
            );
        }
        caption(ui, format!("STRONG’S {code}"));
        let Some(data) = self.occurrence_data.clone().filter(|d| d.code == code) else {
            ui.add_space(16.0);
            ui.spinner();
            ui.label("Finding where this word appears…");
            return;
        };
        ui.add_space(12.0);
        theme::card(ui.ctx()).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            theme::editorial(ui,"Where it appears",24.0,p.ink);
            ui.horizontal_wrapped(|ui|{ui.label(scripture(data.total.to_string(),28.0,true));ui.label("verses");caption(ui,format!("· {} books",data.books.len()));});
            ui.add_space(8.0);
            ui.label(RichText::new("Distinct verses in installed original-language sources, not individual word repetitions.").size(11.0).color(p.muted));
            if data.total==0 {ui.add_space(12.0);ui.label("No source-token occurrences are installed for this word. Its dictionary entry may still be available.");return;}
            egui::CollapsingHeader::new("Distribution by book").id_salt(("word-distribution",code)).default_open(true).show(ui,|ui|{
                let mut ranked=data.books.clone();ranked.sort_by_key(|b|(std::cmp::Reverse(b.verses),b.book_id));
                let max=ranked.first().map_or(1,|b|b.verses).max(1) as f32;
                for book in ranked.iter().take(5){
                    let response=ui.add_sized([ui.available_width(),44.0],egui::Button::new("").frame(false));
                    let label=format!("Filter occurrences: {}, {} verses",book.label,book.verses);
                    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,true,&label));
                    let rect=response.rect;
                    if self.occurrence_book==Some(book.book_id){ui.painter().rect_filled(rect,8,p.tint);}
                    let label_width=(rect.width()*0.36).min(112.0);
                    let title=ui.painter().layout(book.label.clone(),egui::FontId::proportional(12.0),p.ink,label_width-8.0);
                    ui.painter().galley(egui::pos2(rect.left()+4.0,rect.center().y-title.size().y/2.0),title,p.ink);
                    let track=egui::Rect::from_min_max(egui::pos2(rect.left()+label_width,rect.center().y-5.0),egui::pos2(rect.right()-36.0,rect.center().y+5.0));
                    ui.painter().rect_filled(track,5,p.border.gamma_multiply(0.45));
                    let bar=egui::Rect::from_min_size(track.min,egui::vec2((track.width()*book.verses as f32/max).max(2.0),track.height()));
                    ui.painter().rect_filled(bar,5,p.accent);
                    ui.painter().text(egui::pos2(rect.right()-4.0,rect.center().y),egui::Align2::RIGHT_CENTER,book.verses.to_string(),egui::FontId::proportional(12.0),p.ink);
                    if response.clicked(){self.request_occurrences(code,Some(book.book_id),0);}
                }
                if data.books.len()>5{caption(ui,"Top 5 books · the filter includes every book with matches.");}
            });
        });
        ui.add_space(16.0);
        if data.total == 0 {
            return;
        }
        let mut book = self.occurrence_book;
        ui.horizontal_wrapped(|ui| {
            ui.label("Book");
            egui::ComboBox::from_id_salt("occurrence-book-filter")
                .width((ui.available_width() - 64.0).max(100.0))
                .selected_text(
                    book.and_then(|id| {
                        data.books
                            .iter()
                            .find(|b| b.book_id == id)
                            .map(|b| b.label.clone())
                    })
                    .unwrap_or_else(|| "All books".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut book, None, "All books");
                    for b in &data.books {
                        ui.selectable_value(
                            &mut book,
                            Some(b.book_id),
                            format!("{} · {}", b.label, b.verses),
                        );
                    }
                });
            if book.is_some()
                && ui
                    .add_sized([52.0, 44.0], egui::Button::new("Clear"))
                    .clicked()
            {
                book = None;
            }
        });
        if book != self.occurrence_book {
            self.request_occurrences(code, book, 0);
        }
        if self.occurrence_requested.is_some() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Updating verses…");
            });
            return;
        }
        let heading = ui.label(
            RichText::new(
                data.book_id
                    .and_then(|id| {
                        data.books
                            .iter()
                            .find(|b| b.book_id == id)
                            .map(|b| format!("Verses in {}", b.label))
                    })
                    .unwrap_or_else(|| "Matching verses".into()),
            )
            .size(15.0)
            .strong(),
        );
        if self.occurrence_scroll_results {
            heading.scroll_to_me(Some(egui::Align::TOP));
            self.occurrence_scroll_results = false;
        }
        self.occurrence_pager(ui, &data);
        if data.verses.is_empty() {
            ui.label("No verses on this page. Choose another book or return to the previous page.");
        }
        theme::card(ui.ctx()).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            for (index, verse) in data.verses.iter().enumerate() {
                if index > 0 {
                    ui.add_space(8.0);
                    ui.separator();
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 44.0],
                        egui::Button::new(
                            RichText::new(format!("{}  →", verse.label))
                                .size(15.0)
                                .color(p.accent_ink),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    self.occurrence_return = Some(OccurrenceReturn {
                        code: code.into(),
                        book: data.book_id,
                        offset: data.offset,
                    });
                    self.select(verse.ref_id);
                    self.destination = Destination::Read;
                }
                if let Some(text) = &verse.preview {
                    let mut job = egui::text::LayoutJob::simple(
                        text.clone(),
                        egui::FontId::new(14.0, egui::FontFamily::Name("scripture".into())),
                        p.ink,
                        ui.available_width(),
                    );
                    job.wrap.max_rows = 3;
                    job.sections[0].format.line_height = Some(22.0);
                    ui.add(egui::Label::new(job).wrap());
                } else {
                    ui.label(
                        RichText::new("No text supplied in the selected translation.")
                            .size(12.0)
                            .color(p.muted),
                    );
                }
                ui.add_space(12.0);
            }
        });
        self.occurrence_pager(ui, &data);
    }
    fn occurrence_pager(&mut self, ui: &mut egui::Ui, data: &WordOccurrences) {
        ui.add_space(12.0);
        if !data.verses.is_empty() {
            caption(
                ui,
                format!(
                    "{}–{} of {} verses",
                    data.offset + 1,
                    data.offset + data.verses.len() as u32,
                    data.filtered_total
                ),
            );
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    data.offset > 0,
                    egui::Button::new("Previous verses").min_size(egui::vec2(110.0, 44.0)),
                )
                .clicked()
            {
                self.request_occurrences(
                    &data.code,
                    data.book_id,
                    data.offset.saturating_sub(PAGE),
                );
            }
            if ui
                .add_enabled(
                    data.has_more,
                    egui::Button::new("Next verses").min_size(egui::vec2(100.0, 44.0)),
                )
                .clicked()
            {
                self.request_occurrences(&data.code, data.book_id, data.offset + PAGE);
            }
        });
        ui.add_space(12.0);
    }
}
