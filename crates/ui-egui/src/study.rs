//! Contextual study presentation; all content and links come from engine view models.
use super::*;
use std::{collections::BTreeSet, ops::Range};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum StudyTab {
    Words,
    Commentary,
    References,
    Topics,
}
#[derive(Clone)]
pub(super) struct SelectedText {
    pub verse_id: String,
    pub ref_id: i64,
    pub range: Range<usize>,
    pub text: String,
}
pub(super) struct WordFocus {
    code: String,
    surface: String,
    witness: String,
}
impl FontesApp {
    pub(super) fn open_study(&mut self, reference: i64, tab: StudyTab) {
        if self.destination != Destination::Study {
            self.scroll_to = Some(reference);
        }
        self.destination = Destination::Study;
        self.study_tab = tab;
        self.resource = None;
        self.word = None;
        self.word_focus = None;
        if self
            .state
            .as_ref()
            .is_some_and(|s| s.session.selected_ref != reference)
        {
            // The selection response requests the context for its new canonical location.
            self.dispatch(Command::Select { ref_id: reference });
        } else if !matches!(self.context, Some(Data::Context { ref_id, .. }) if ref_id == reference)
        {
            self.dispatch(Command::Context { ref_id: reference });
        }
    }
    fn request_word_preview(&mut self, code: &str) {
        if self.word_previews.contains_key(code) || self.preview_requested.contains(code) {
            return;
        }
        if self.preview_requested.len() >= 32 {
            self.preview_requested.clear();
            self.word_previews.clear();
        }
        let before = self.pending;
        self.dispatch(Command::Word { code: code.into() });
        if self.pending > before {
            *self.reply_previews.back_mut().expect("queued request") = true;
            self.preview_requested.insert(code.into());
        }
    }
    pub(super) fn lookup_word(&mut self, code: &str) {
        self.word_card_key.clear();
        self.word_card_back = false;
        self.occurrences_open = false;
        self.occurrence_data = None;
        self.occurrence_book = None;
        self.occurrence_requested = None;
        self.occurrence_return = None;
        self.word_focus = None;
        self.study_tab = StudyTab::Words;
        self.word = None;
        self.resource = None;
        self.word_code = code.into();
        self.dispatch(Command::Word { code: code.into() });
    }
    pub(super) fn verse_text(
        &mut self,
        ui: &mut egui::Ui,
        verse: &Verse,
        mut job: egui::text::LayoutJob,
        mobile: bool,
    ) {
        if verse.text.is_empty() {
            ui.label("[No text supplied by this edition]");
            return;
        }
        // Keep the selected word visible while the popup or study panel owns keyboard focus.
        if let Some(selected) = self
            .selected_text
            .as_ref()
            .filter(|t| t.verse_id == verse.id)
        {
            let offset = |index| {
                verse
                    .text
                    .char_indices()
                    .nth(index)
                    .map_or(verse.text.len(), |(byte, _)| byte)
            };
            let start = offset(selected.range.start);
            let end = offset(selected.range.end);
            let format = job.sections[0].format.clone();
            job.text.clear();
            job.sections.clear();
            job.append(&verse.text[..start], 0.0, format.clone());
            let mut selected_format = format.clone();
            selected_format.background = theme::palette(ui.ctx()).highlights[0];
            job.append(&verse.text[start..end], 0.0, selected_format);
            job.append(&verse.text[end..], 0.0, format);
        }
        highlights::styled_job(
            &mut job,
            self.state.as_ref().unwrap(),
            verse,
            self.highlight_draft.as_ref(),
        );
        // Mobile text must not own drag/keyboard focus: a focused TextEdit steals
        // the next swipe for cursor selection and issues cursor scroll-to requests.
        // Desktop retains native text selection; touch uses tap/long-press lookup.
        let (response, galley, galley_pos, cursor_range) = if mobile {
            job.wrap.max_width = (ui.available_width() - 40.0).max(80.0);
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
            let response = ui.add(
                egui::Label::new(galley.clone())
                    .selectable(false)
                    .sense(egui::Sense::click()),
            );
            let pos = response.rect.min;
            (response, galley, pos, None)
        } else {
            let mut text = verse.text.as_str();
            let mut layouter = |ui: &egui::Ui, _: &dyn egui::TextBuffer, width: f32| {
                let mut job = job.clone();
                job.wrap.max_width = width;
                ui.fonts_mut(|fonts| fonts.layout_job(job))
            };
            let output = egui::TextEdit::multiline(&mut text)
                .id_salt(("verse-text", &verse.id, self.selection_generation))
                .frame(egui::Frame::NONE)
                .margin(0)
                .desired_rows(1)
                .desired_width((ui.available_width() - 40.0).max(80.0))
                .layouter(&mut layouter)
                .show(ui);
            (
                output.response.response,
                output.galley,
                output.galley_pos,
                output.cursor_range,
            )
        };
        highlights::decorations(ui, self.state.as_ref().unwrap(), verse, &galley, galley_pos);
        self.draw_highlight_handles(ui, verse, &galley, galley_pos, response.rect);
        self.draw_note_markers(ui, verse, &galley, galley_pos, response.rect);
        if self.highlight_draft.as_ref().is_some_and(|d| d.selecting) {
            return;
        }
        if !mobile
            && response.clicked()
            && self
                .state
                .as_ref()
                .is_some_and(|s| !verse.refs.contains(&s.session.selected_ref))
        {
            self.dispatch(Command::Select {
                ref_id: verse.refs[0],
            });
        }
        if response.has_focus()
            && let Some(range) = cursor_range
        {
            let range = range.as_sorted_char_range();
            let range = range.start.0..range.end.0;
            if !range.is_empty() {
                self.selected_text = Some(selection(verse, range));
            } else if !response.secondary_clicked()
                && !response.context_menu_opened()
                && self.verse_actions.is_none()
                && !self.reading_settings
            {
                self.selected_text = None;
            }
        }
        // Track the gesture independently of egui's touch-only secondary click.
        // Android pointer delivery and sparse repaint timing must not turn a hold
        // into the primary word-study click when the finger is released.
        let mut held = false;
        if mobile {
            let (pressed, down, released, pos, time) = ui.input(|i| {
                (
                    i.pointer.primary_pressed(),
                    i.pointer.primary_down(),
                    i.pointer.primary_released(),
                    i.pointer.interact_pos(),
                    i.time,
                )
            });
            if pressed
                && response.is_pointer_button_down_on()
                && let Some(pos) = pos
            {
                self.scripture_press = Some((response.id, pos, time));
            }
            if let Some((id, origin, start)) = self.scripture_press
                && id == response.id
            {
                if pos.is_none_or(|p| p.distance(origin) > 8.0) {
                    self.scripture_press = None;
                } else if down || released {
                    if time - start >= 0.5 && !self.suppress_word_tap {
                        held = true;
                    } else if down {
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(50));
                    }
                }
            }
        }
        let secondary = response.secondary_clicked() || held;
        if mobile && secondary {
            self.suppress_word_tap = true;
        }
        if mobile
            && !self.suppress_word_tap
            && response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let cursor = galley.cursor_from_pos(pos - galley_pos).index.0;
            let range = verse
                .strongs_links
                .iter()
                .map(|l| l.start..l.end)
                .chain(verse.word_links.iter().map(|l| l.start..l.end))
                .find(|range| range.contains(&cursor))
                .or_else(|| word_at(&verse.text, cursor));
            if let Some(range) = range {
                if let Some((id, _, _)) = highlights::marks(self.state.as_ref().unwrap(), verse)
                    .into_iter()
                    .find(|(_, r, _)| r.start < range.end && r.end > range.start)
                {
                    self.begin_highlight(verse, Some(id));
                    return;
                }
                let codes = lookup_codes(verse, &range);
                self.selected_text = Some(selection(verse, range));
                self.open_study(verse.refs[0], StudyTab::Words);
                if codes.len() == 1 {
                    self.lookup_word(codes.first().expect("one verified code"));
                }
            }
        }
        if secondary {
            // Right-click without a selection targets the word beneath the pointer.
            if self
                .selected_text
                .as_ref()
                .is_none_or(|t| t.verse_id != verse.id)
                && let Some(pos) = response.interact_pointer_pos()
            {
                let cursor = galley.cursor_from_pos(pos - galley_pos);
                self.selected_text =
                    word_at(&verse.text, cursor.index.0).map(|range| selection(verse, range));
            }
        }
        if mobile {
            if secondary {
                // Holding is a text-selection gesture, not a dictionary/menu action.
                if let Some(pos) = response.interact_pointer_pos() {
                    let cursor = galley.cursor_from_pos(pos - galley_pos);
                    self.selected_text =
                        word_at(&verse.text, cursor.index.0).map(|range| selection(verse, range));
                }
                self.begin_highlight(verse, None);
                if let Some(draft) = &mut self.highlight_draft {
                    draft.selecting = true;
                }
            }
        } else {
            response.context_menu(|ui| {
                ui.set_width(300.0);
                egui::Frame::new().inner_margin(8).show(ui, |ui| {
                    self.verse_action_contents(ui, verse, false);
                });
            });
        }
    }
    pub(super) fn mobile_verse_actions(&mut self, ctx: &egui::Context) {
        let Some(verse) = self.verse_actions.clone() else {
            return;
        };
        let bounds = ctx.content_rect();
        let width = bounds.width().min(480.0);
        let has_selection = self
            .selected_text
            .as_ref()
            .is_some_and(|t| t.verse_id == verse.id);
        let height = (bounds.height() * 0.9).min(if has_selection { 528.0 } else { 344.0 });
        let modal = egui::Modal::new(egui::Id::new("verse-action-sheet"))
            .area(
                egui::Modal::default_area(egui::Id::new("verse-action-sheet"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO),
            )
            .frame(
                egui::Frame::new()
                    .fill(theme::palette(ctx).surface)
                    .corner_radius(egui::CornerRadius {
                        nw: 24,
                        ne: 24,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(egui::Margin {
                        left: 22,
                        right: 22,
                        top: 10,
                        bottom: 27,
                    }),
            )
            .backdrop_color(theme::palette(ctx).scrim.gamma_multiply(0.26))
            .show(ctx, |ui| {
                ui.set_width(width - 44.0);
                ui.set_height(height - 37.0);
                theme::sheet_handle(ui);
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                ui.horizontal(|ui| {
                    caption(
                        ui,
                        format!(
                            "{} · {}",
                            label(&self.state.as_ref().unwrap().books, verse.refs[0]),
                            self.state
                                .as_ref()
                                .unwrap()
                                .translations
                                .iter()
                                .find(|t| t.id == self.state.as_ref().unwrap().session.translation)
                                .map(|t| t.abbr.as_str())
                                .unwrap_or("")
                        ),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close = theme::round_close(ui, "Close actions");
                        if close.clicked() {
                            ui.close();
                        }
                    });
                });
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .id_salt("verse-action-sheet-scroll")
                    .max_height((height - 108.0).max(80.0))
                    .show(ui, |ui| {
                        self.verse_action_contents(ui, &verse, true);
                    });
            });
        if modal.should_close() {
            self.verse_actions = None;
        }
    }

    fn verse_action_contents(&mut self, ui: &mut egui::Ui, verse: &Verse, mobile: bool) {
        let selected = self
            .selected_text
            .clone()
            .filter(|t| t.verse_id == verse.id);
        if let Some(selected) = &selected {
            let mut preview = egui::text::LayoutJob::simple(
                format!("“{}”", selected.text),
                egui::FontId::new(37.0, egui::FontFamily::Name("scripture".into())),
                theme::palette(ui.ctx()).ink,
                ui.available_width(),
            );
            preview.sections[0].format.extra_letter_spacing = -1.0;
            preview.sections[0].format.line_height = Some(44.0);
            preview.wrap.max_rows = 2;
            ui.add(egui::Label::new(preview).wrap().selectable(false));
            ui.add_space(12.0);
            caption(ui, "Explore the word. Stay in the passage.");
            ui.add_space(12.0);
            let codes = lookup_codes(verse, &selected.range);
            if codes.is_empty() {
                if action_row(ui, "search", "Look up original word…", true, mobile).clicked() {
                    self.open_study(verse.refs[0], StudyTab::Words);
                    ui.close();
                }
                ui.add_space(4.0);
                caption(
                    ui,
                    "No word link in this edition. Browse this verse’s original words.",
                );
            } else {
                for code in codes {
                    self.request_word_preview(&code);
                    let headword = self
                        .word_previews
                        .get(&code)
                        .and_then(|v| v.first())
                        .cloned();
                    if lookup_button(ui, &code, headword.as_ref()).clicked() {
                        self.open_study(verse.refs[0], StudyTab::Words);
                        self.lookup_word(&code);
                        ui.close();
                    }
                }
            }
            ui.add_space(12.0);
            if action_row(ui, "copy", "Copy selection", false, mobile).clicked() {
                ui.ctx().copy_text(selected.text.clone());
                ui.close();
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
        }
        if action_row(ui, "book", "Study this verse", false, mobile).clicked() {
            self.open_study(verse.refs[0], StudyTab::Commentary);
            ui.close();
        }
        if action_row(ui, "notes", "Add note", false, mobile).clicked() {
            let selection = self
                .selected_text
                .as_ref()
                .filter(|t| t.verse_id == verse.id)
                .map(|t| TextSelection {
                    translation_id: self.state.as_ref().unwrap().session.translation.clone(),
                    spans: vec![TextSpan {
                        verse_id: verse.id.clone(),
                        start: t.range.start,
                        end: t.range.end,
                        quote: t.text.clone(),
                    }],
                });
            self.note_from_selection(verse.refs[0], *verse.refs.last().unwrap(), selection);
            ui.close();
        }
        if action_row(ui, "bookmark", "Highlight…", false, mobile).clicked() {
            let range = self
                .selected_text
                .as_ref()
                .map(|s| s.range.clone())
                .unwrap_or(0..verse.text.chars().count());
            let existing = highlights::marks(self.state.as_ref().unwrap(), verse)
                .into_iter()
                .find(|(_, r, _)| r.start < range.end && r.end > range.start)
                .map(|(id, _, _)| id);
            self.begin_highlight(verse, existing);
            ui.close();
        }
        if action_row(ui, "copy", "Copy verse", false, mobile).clicked() {
            ui.ctx().copy_text(format!(
                "{} — {}",
                verse.text,
                label(&self.state.as_ref().unwrap().books, verse.refs[0])
            ));
            ui.close();
        }
    }

    pub(super) fn study_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.state.clone().unwrap();
        // Each article opens at its beginning; returning restores the results scroll.
        let article_id = self.resource.as_ref().map(|r| r.id.clone());
        let word_key = self.word.as_ref().and_then(|word| {
            if let Data::Word { code, .. } = word {
                Some(code.clone())
            } else {
                None
            }
        });
        egui::ScrollArea::vertical().scroll_bar_visibility(if ui.available_width()<760.0{egui::scroll_area::ScrollBarVisibility::AlwaysHidden}else{egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded}).id_salt(("study-scroll", self.study_tab as u8, article_id, word_key, self.occurrences_open)).show(ui, |ui| {
            egui::Frame::new().inner_margin(18).show(ui, |ui| {
                let header_top=ui.cursor().min;
                let header_wash=ui.painter().add(egui::Shape::Noop);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let size=if ui.available_width()<310.0{26.0}else{30.0};
                    theme::editorial(ui,"Word study",size,theme::palette(ui.ctx()).ink);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                        if ui.add(egui::Button::new(RichText::new("← Back to reading").size(11.0).color(muted(ui.ctx()))).frame(false)).clicked(){self.choose(Destination::Read);}
                    });
                });
                ui.add_space(19.0);
                ui.columns(4, |cols| {
                    for (col, (tab, title, accessible)) in cols.iter_mut().zip([
                        (StudyTab::Words,"Words","Words"), (StudyTab::Commentary,"Commentary","Commentary"),
                        (StudyTab::References,"References","Cross-references"), (StudyTab::Topics,"Topics","Topics"),
                    ]) {
                        let active=self.study_tab==tab;
                        let text=RichText::new(title).size(11.0).family(if active{egui::FontFamily::Name("medium".into())}else{egui::FontFamily::Proportional}).color(if active{accent(col.ctx())}else{muted(col.ctx())});
                        let response=col.add_sized([col.available_width(),44.0],egui::Button::new(text).frame(false).wrap());
                        response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,true,accessible));
                        if active {let line=egui::Rect::from_min_size(egui::pos2(response.rect.left(),response.rect.bottom()-1.0),egui::vec2(response.rect.width().min(64.0),3.0));col.painter().rect_filled(line,2,accent(col.ctx()));}
                        if response.clicked(){self.study_tab=tab;self.resource=None;}
                    }
                });
                ui.separator();
                let p=theme::palette(ui.ctx());
                theme::radial_wash(ui,header_wash,egui::Rect::from_min_max(header_top-egui::vec2(18.0,18.0),egui::pos2(ui.max_rect().right()+18.0,ui.cursor().min.y)),0.0,p.page,p.glow,0.55);
                ui.add_space(22.0);
                if !s.has_study {
                    ui.label("Open a study library to add Strong’s dictionaries, original Hebrew and Greek, commentary, topics and cross-references.");
                    return;
                }
                if let Some(r) = self.resource.clone() {
                    if ui.button("← Back to results").clicked() {
                        self.resource = None;
                    }
                    ui.add_space(12.0);
                    ui.label(RichText::new(friendly_title(&r.title)).size(20.0).strong());
                    if let Some(author) = &r.author { caption(ui, author); }
                    ui.add_space(12.0);
                    if ui.add_sized([180.0,44.0],egui::Button::new("Add excerpt to note")).clicked(){self.collect_resource(r.clone());}
                    article_body(ui, &r.content);
                    let links: BTreeSet<_> = r.links.iter().filter_map(|l| l.start_ref).collect();
                    if !links.is_empty() {
                        ui.add_space(16.0);
                        caption(ui, "SCRIPTURE REFERENCES");
                        ui.horizontal_wrapped(|ui| {
                            for id in links {
                                if ui.button(label(&s.books, id)).clicked() { self.select(id); }
                            }
                        });
                    }
                    ui.add_space(16.0);
                    ui.collapsing("Source & attribution", |ui| {
                        caption(ui, format!("{} · {}", r.source, r.rights_review));
                    });
                } else if self.study_tab == StudyTab::Words {
                    self.word_panel(ui);
                } else if let Some(Data::Context { resources, cross_references, .. }) = self.context.clone() {
                    if self.study_tab == StudyTab::References {
                        if cross_references.is_empty() { ui.label("No cross-reference targets are installed for this verse."); }
                        for target in &cross_references {
                            ui.push_id(("cross-reference",target.start_ref,target.end_ref),|ui|{
                                theme::card(ui.ctx()).show(ui,|ui|{
                                    ui.set_min_width(ui.available_width());
                                    theme::eyebrow(ui,"REFERENCE",muted(ui.ctx()));
                                    ui.add_space(8.0);
                                    if ui.add_sized([ui.available_width(),44.0],egui::Button::new(scripture(&target.label,22.0,true).color(theme::palette(ui.ctx()).ink)).frame(false).right_text(()).wrap()).clicked(){self.select(target.start_ref);}
                                    ui.add_space(12.0);
                                    if let Some(preview)=&target.preview{
                                        let mut job=egui::text::LayoutJob::simple(preview.clone(),egui::FontId::proportional(13.0),muted(ui.ctx()),ui.available_width());
                                        job.sections[0].format.line_height=Some(21.5);
                                        ui.add(egui::Label::new(job).wrap());
                                    }else{caption(ui,"No verse text supplied by this edition.");}
                                    if target.start_ref!=target.end_ref{ui.add_space(8.0);caption(ui,"Preview of the first verse");}
                                    ui.add_space(12.0);
                                    if ui.add_sized([160.0,44.0],egui::Button::new("Add passage to note")).clicked(){self.notes_state.collecting=self.note.is_none();self.notes_state.insert=self.note.is_some();self.dispatch(Command::NotePassage{ref_id:target.start_ref});}
                                    if ui.add_sized([144.0,44.0],egui::Button::new("Source article")).clicked(){self.dispatch(Command::Resource{id:target.resource_id.clone()});}
                                });
                                ui.add_space(16.0);
                            });
                        }
                        if cross_references.len() == 100 { caption(ui, "Showing the first 100 distinct reference ranges."); }
                        ui.add_space(16.0);
                        ui.collapsing("Reference articles", |ui| {
                            for r in resources.iter().filter(|r| r.kind == "cross_reference" && r.basis == "applies_to") { self.resource_card(ui, r); }
                        });
                        return;
                    }
                    let kind = match self.study_tab { StudyTab::Commentary => "commentary", StudyTab::References => "cross_reference", _ => "topic" };
                    if self.study_tab != StudyTab::Topics {
                        ui.checkbox(&mut self.include_mentions, "Include mentions elsewhere");
                        ui.add_space(12.0);
                    }
                    let mut seen = BTreeSet::new();
                    let matches: Vec<_> = resources.iter().filter(|r| r.kind == kind && (self.include_mentions || self.study_tab == StudyTab::Topics || r.basis == "applies_to") && seen.insert(&r.id)).collect();
                    if matches.is_empty() { ui.label("No matching resources in your installed library for this verse."); }
                    for r in matches { self.resource_card(ui, r); }
                    if resources.len() == 100 { caption(ui, "Showing the first 100 matching articles. More may be available in your library."); }
                } else {
                    ui.label("Loading resources for this verse…");
                }
            });
        });
    }
    fn word_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(Data::Word {
            code,
            headwords,
            definitions,
            occurrences: _,
            total,
        }) = self.word.clone()
        {
            if ui
                .add_sized(
                    [ui.available_width(), 44.0],
                    egui::Button::new("← Words in this verse").frame(false),
                )
                .clicked()
            {
                self.word = None;
                self.word_focus = None;
                return;
            }
            ui.add_space(8.0);
            ui.columns(2, |cols| {
                if cols[0]
                    .add_sized(
                        [cols[0].available_width(), 44.0],
                        egui::Button::new("Meaning").selected(!self.occurrences_open),
                    )
                    .clicked()
                {
                    self.occurrences_open = false;
                }
                if cols[1]
                    .add_sized(
                        [cols[1].available_width(), 44.0],
                        egui::Button::new(format!("Occurrences · {total}"))
                            .selected(self.occurrences_open),
                    )
                    .clicked()
                {
                    self.occurrences_open = true;
                    if self.occurrence_data.is_none() && self.occurrence_requested.is_none() {
                        self.request_occurrences(&code, None, 0);
                    }
                }
            });
            ui.add_space(12.0);
            if self.occurrences_open {
                self.occurrence_panel(ui, &code, headwords.first().map(|h| h.text.as_str()));
                return;
            }
            if self.word_card_key != code {
                self.word_card_key = code.clone();
                self.word_card_back = false;
            }
            let motion = !self.reduce_card_motion;
            let progress = ui.ctx().animate_bool_with_time(
                ui.id().with(("word-flip", &code)),
                self.word_card_back,
                if motion { 0.32 } else { 0.18 },
            );
            let reverse = progress >= 0.5;
            let paint_start = ui.ctx().graphics_mut(|g| g.entry(ui.layer_id()).next_idx());
            let mut full_entry = None;
            let linked = self.selected_text.as_ref().and_then(|selected| {
                self.state
                    .as_ref()?
                    .passage
                    .iter()
                    .find(|v| v.id == selected.verse_id)?
                    .word_links
                    .iter()
                    .find(|link| {
                        link.strongs == code
                            && link.start < selected.range.end
                            && link.end > selected.range.start
                    })
            });
            let focus = self.word_focus.as_ref().filter(|focus| focus.code == code);
            let p = theme::palette(ui.ctx());
            let mut slots = None;
            let has_form = focus.is_some() || linked.is_some();
            let flip = ui.interact(
                egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 366.0)),
                ui.id().with(("flip-word-card", &code)),
                egui::Sense::click(),
            );
            let hero = egui::Frame::new()
                .corner_radius(22)
                .inner_margin(23)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 24,
                    spread: 0,
                    color: p.scrim.gamma_multiply(0.30),
                })
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(320.0);
                    slots = Some((
                        ui.painter().add(egui::Shape::Noop),
                        ui.painter().add(egui::Shape::Noop),
                    ));
                    ui.visuals_mut().override_text_color = Some(p.on_feature);
                    if reverse {
                        theme::eyebrow(ui, format!("STRONG’S {code} · MEANING"), p.on_feature);
                        ui.add_space(16.0);
                        if let Some(d) = definitions.first() {
                            let text = self
                                .word_entries
                                .get(&d.id)
                                .map(|e| e.content.as_str())
                                .unwrap_or(&d.excerpt);
                            let mut excerpt = egui::text::LayoutJob::simple(
                                text.to_owned(),
                                egui::FontId::proportional(15.0),
                                p.on_feature,
                                ui.available_width(),
                            );
                            excerpt.sections[0].format.line_height = Some(23.0);
                            excerpt.wrap.max_rows = 6;
                            excerpt.wrap.break_anywhere = false;
                            ui.add(egui::Label::new(excerpt).selectable(false));
                            ui.add_space(12.0);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Button::new(
                                        RichText::new("Read full entry →").color(p.feature),
                                    )
                                    .fill(p.on_feature),
                                )
                                .clicked()
                            {
                                full_entry = Some(d.id.clone());
                            }
                            if definitions.len() > 1 {
                                feature_caption(
                                    ui,
                                    format!(
                                        "{} dictionary sources available below",
                                        definitions.len()
                                    ),
                                );
                            }
                        } else {
                            ui.label(
                                RichText::new("No dictionary entry installed for this number.")
                                    .color(p.on_feature),
                            );
                        }
                        ui.add_space(8.0);
                        if ui
                            .add_sized(
                                [ui.available_width(), 44.0],
                                egui::Button::new(
                                    RichText::new("Back to word").color(p.on_feature),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.word_card_back = false;
                        }
                    } else {
                        ui.columns(2, |cols| {
                            let language = if code.starts_with('H') {
                                "HEBREW"
                            } else {
                                "GREEK"
                            };
                            let kind = if focus.is_some() {
                                "VERSE WORD"
                            } else if linked.is_some() {
                                "LINKED SOURCE WORD"
                            } else {
                                "DICTIONARY HEADWORD"
                            };
                            theme::eyebrow(
                                &mut cols[0],
                                format!("{language} {kind}"),
                                p.on_feature,
                            );
                            let reference = self
                                .state
                                .as_ref()
                                .unwrap()
                                .reference_label()
                                .to_uppercase();
                            cols[1].with_layout(
                                egui::Layout::right_to_left(egui::Align::TOP),
                                |ui| {
                                    ui.set_max_width(72.0);
                                    theme::eyebrow(ui, reference, p.on_feature);
                                },
                            );
                        });
                        ui.add_space(12.0);
                        if let Some(focus) = focus {
                            original_word_heading(ui, &focus.surface);
                        } else if let Some(link) = linked {
                            original_word_heading(ui, &link.surface);
                        } else if let Some(headword) = headwords.first() {
                            original_word_heading(ui, &headword.text);
                        } else {
                            theme::editorial(ui, format!("Strong’s {code}"), 30.0, p.on_feature);
                        }
                        if let Some(headword) = headwords.first() {
                            if has_form {
                                feature_caption(
                                    ui,
                                    format!("Dictionary headword · {}", headword.text),
                                );
                            }
                            if let Some(transliteration) = &headword.transliteration {
                                ui.label(
                                    scripture(transliteration, 16.0, true).color(p.on_feature),
                                );
                            }
                        }
                        for headword in headwords.iter().skip(1) {
                            feature_caption(ui, format!("Dictionary headword · {}", headword.text));
                        }
                        if let Some(focus) = focus {
                            feature_caption(ui, witness_title(&focus.witness));
                        }
                        if !has_form && !headwords.is_empty() {
                            // The label remains explicit for assistive technology and source distinction.
                            let response =
                                ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover());
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Label,
                                    true,
                                    "Dictionary headword",
                                )
                            });
                        }
                        ui.add_space(15.0);
                        let (line, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().hline(
                            line.x_range(),
                            line.center().y,
                            egui::Stroke::new(1.0, p.on_feature.gamma_multiply(0.18)),
                        );
                        ui.add_space(8.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 28.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                let text = self
                                    .selected_text
                                    .as_ref()
                                    .map(|t| format!("Reading selection: “{}”", t.text))
                                    .unwrap_or_else(|| {
                                        if has_form {
                                            "Supplied source form".into()
                                        } else {
                                            "Dictionary entry".into()
                                        }
                                    });
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(text).size(10.0).color(p.on_feature),
                                    )
                                    .wrap(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        egui::Frame::new()
                                            .fill(p.on_feature.gamma_multiply(0.12))
                                            .corner_radius(6)
                                            .inner_margin(egui::Margin::symmetric(8, 5))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(&code)
                                                        .size(11.0)
                                                        .color(p.on_feature),
                                                );
                                            });
                                    },
                                );
                            },
                        );
                        ui.add_space(12.0);
                        feature_caption(ui, "Tap for meaning ↻");
                    }
                });
            let (slot, rings) = slots.expect("word card background");
            theme::radial_wash(
                ui,
                slot,
                hero.response.rect,
                22.0,
                p.feature,
                p.feature_light,
                0.60,
            );
            theme::word_rings(ui, rings, hero.response.rect);
            // Tessellate just this card, then compress its vertices around the
            // vertical axis. Layout and text wrapping remain fixed during the flip.
            let paint_end = ui.ctx().graphics_mut(|g| g.entry(ui.layer_id()).next_idx());
            if progress > 0.0 && progress < 1.0 {
                let shapes = ui.ctx().graphics_mut(|g| {
                    let list = g.entry(ui.layer_id());
                    let shapes = list
                        .all_entries()
                        .skip(paint_start.0)
                        .take(paint_end.0 - paint_start.0)
                        .cloned()
                        .collect::<Vec<_>>();
                    for i in paint_start.0..paint_end.0 {
                        list.reset_shape(egui::layers::ShapeIdx(i));
                    }
                    shapes
                });
                let scale = (progress * std::f32::consts::PI).cos().abs().max(0.025);
                let center = hero.response.rect.center().x;
                for primitive in ui.ctx().tessellate(shapes, ui.ctx().pixels_per_point()) {
                    if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
                        for vertex in &mut mesh.vertices {
                            if motion {
                                vertex.pos.x = center + (vertex.pos.x - center) * scale;
                            } else {
                                vertex.color =
                                    vertex.color.gamma_multiply((progress * 2.0 - 1.0).abs());
                            }
                        }
                        ui.painter()
                            .with_clip_rect(primitive.clip_rect)
                            .add(egui::Shape::mesh(mesh));
                    }
                }
            }
            flip.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    if reverse {
                        "Flip word card"
                    } else {
                        "Tap for meaning"
                    },
                )
            });
            if full_entry.is_none() && flip.clicked() {
                self.word_card_back = !self.word_card_back;
                ui.ctx().request_repaint();
            }
            if let Some(id) = full_entry {
                self.dispatch(Command::Resource { id });
            }
            if reverse && definitions.len() > 1 {
                ui.add_space(12.0);
                ui.collapsing("Other dictionary sources", |ui| {
                    for d in definitions.iter().skip(1) {
                        if ui
                            .add_sized([ui.available_width(), 44.0], egui::Button::new(&d.title))
                            .clicked()
                        {
                            self.dispatch(Command::Resource { id: d.id.clone() });
                        }
                    }
                });
            }
            ui.add_space(16.0);
        } else {
            if let Some(selected) = &self.selected_text {
                caption(ui, format!("Reading selection: “{}”", selected.text));
                let verse = self
                    .state
                    .as_ref()
                    .unwrap()
                    .passage
                    .iter()
                    .find(|v| v.id == selected.verse_id);
                let codes = verse
                    .map(|v| lookup_codes(v, &selected.range))
                    .unwrap_or_default();
                if codes.is_empty() {
                    ui.label("This edition has no word-level link for this selection. Choose the original word below; these words belong to the whole verse.");
                } else {
                    let surfaces: BTreeSet<_> = verse
                        .into_iter()
                        .flat_map(|v| &v.word_links)
                        .filter(|link| {
                            link.start < selected.range.end && link.end > selected.range.start
                        })
                        .map(|link| link.surface.as_str())
                        .collect();
                    for surface in surfaces {
                        let mut job = egui::text::LayoutJob::simple(
                            surface.to_owned(),
                            egui::FontId::new(40.0, egui::FontFamily::Name("original".into())),
                            theme::palette(ui.ctx()).ink,
                            ui.available_width(),
                        );
                        job.wrap.max_rows = 3;
                        ui.add(egui::Label::new(job).wrap().selectable(false));
                    }
                    caption(ui, "Linked source word");
                    ui.horizontal_wrapped(|ui| {
                        for code in codes {
                            if ui.button(format!("Strong’s {code}")).clicked() {
                                self.lookup_word(&code);
                            }
                        }
                    });
                }
                ui.add_space(16.0);
            } else {
                ui.label("Choose an original word to open its Strong’s definition.");
                caption(
                    ui,
                    "Direct lookup uses edition-specific dictionary tags or verified original-word alignments.",
                );
                ui.add_space(16.0);
            }
            if let Some(Data::Context { tokens, .. }) = self.context.clone() {
                if tokens.is_empty() {
                    ui.label("No original-language word data is installed for this verse.");
                } else {
                    let witnesses: BTreeSet<_> = tokens.iter().map(|t| t.witness.clone()).collect();
                    if !witnesses.contains(&self.witness) {
                        self.witness = witnesses.first().unwrap().clone();
                    }
                    egui::ComboBox::from_id_salt("source-witness")
                        .width(ui.available_width().min(260.0))
                        .selected_text(witness_title(&self.witness))
                        .show_ui(ui, |ui| {
                            for witness in witnesses {
                                ui.selectable_value(
                                    &mut self.witness,
                                    witness.clone(),
                                    witness_title(&witness),
                                );
                            }
                        });
                    ui.add_space(12.0);
                    let mut positions = BTreeSet::new();
                    let witness = self.witness.clone();
                    for t in tokens.iter().filter(|t| t.witness == witness) {
                        if !positions.insert(t.position) {
                            continue;
                        }
                        let analyses: Vec<_> = tokens
                            .iter()
                            .filter(|a| a.witness == t.witness && a.position == t.position)
                            .collect();
                        let codes: BTreeSet<_> = analyses
                            .iter()
                            .filter_map(|a| a.strongs.as_deref())
                            .collect();
                        ui.horizontal_wrapped(|ui| {
                            if codes.len() == 1 {
                                if ui
                                    .add_sized(
                                        [ui.available_width().min(180.0), 48.0],
                                        egui::Button::new(scripture(&t.surface, 28.0, true))
                                            .frame(false),
                                    )
                                    .clicked()
                                {
                                    let code = *codes.first().unwrap();
                                    self.lookup_word(code);
                                    self.word_focus = Some(WordFocus {
                                        code: code.into(),
                                        surface: t.surface.clone(),
                                        witness: t.witness.clone(),
                                    });
                                }
                            } else {
                                ui.label(scripture(&t.surface, 28.0, true));
                            }
                            for code in codes {
                                if ui.button(code).clicked() {
                                    self.lookup_word(code);
                                    self.word_focus = Some(WordFocus {
                                        code: code.into(),
                                        surface: t.surface.clone(),
                                        witness: t.witness.clone(),
                                    });
                                }
                            }
                        });
                        if let Some(lemma) = &t.lemma {
                            caption(ui, lemma);
                        }
                        ui.collapsing(format!("Word details · {}", t.position), |ui| {
                            for a in &analyses {
                                caption(
                                    ui,
                                    format!(
                                        "{} {} · analysis {}",
                                        a.morph_system.as_deref().unwrap_or("Source"),
                                        a.morph_code
                                            .as_deref()
                                            .unwrap_or("Morphology not supplied"),
                                        a.analysis_index
                                    ),
                                );
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
            } else {
                ui.label("Loading original words…");
            }
        }
        ui.add_space(16.0);
        ui.collapsing("Look up a Strong’s number", |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.word_code)
                    .hint_text("G25 or H7225")
                    .desired_width(ui.available_width()),
            );
            if ui.button("Look up").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.lookup_word(&self.word_code.clone());
            }
        });
    }
    fn resource_card(&mut self, ui: &mut egui::Ui, r: &ResourceSummary) {
        ui.push_id(&r.id, |ui| {
            theme::card(ui.ctx()).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                theme::eyebrow(ui, r.kind.replace('_', " ").to_uppercase(), muted(ui.ctx()));
                ui.add_space(8.0);
                theme::editorial(
                    ui,
                    friendly_title(&r.title),
                    22.0,
                    theme::palette(ui.ctx()).ink,
                );
                if let Some(author) = &r.author {
                    ui.add_space(8.0);
                    caption(ui, author);
                }
                ui.add_space(16.0);
                let mut excerpt = egui::text::LayoutJob::simple(
                    r.excerpt.clone(),
                    egui::FontId::proportional(13.0),
                    muted(ui.ctx()),
                    ui.available_width(),
                );
                excerpt.sections[0].format.line_height = Some(21.5);
                ui.add(egui::Label::new(excerpt).wrap());
                ui.add_space(8.0);
                caption(
                    ui,
                    match r.basis.as_str() {
                        "applies_to" => "On this passage",
                        "citation" => "Mentions this passage",
                        _ => "Related to this passage",
                    },
                );
                ui.add_space(16.0);
                let p = theme::palette(ui.ctx());
                if ui
                    .add_sized(
                        [160.0, 44.0],
                        egui::Button::image_and_text(
                            icon(ui.ctx(), "arrow", 16.0, false).tint(p.on_accent),
                            RichText::new("Open article").color(p.on_accent),
                        )
                        .fill(p.accent)
                        .corner_radius(12),
                    )
                    .clicked()
                {
                    self.dispatch(Command::Resource { id: r.id.clone() });
                }
            });
            ui.add_space(16.0);
        });
    }
}
fn selection(verse: &Verse, range: Range<usize>) -> SelectedText {
    SelectedText {
        verse_id: verse.id.clone(),
        ref_id: verse.refs[0],
        text: verse
            .text
            .chars()
            .skip(range.start)
            .take(range.len())
            .collect(),
        range,
    }
}
fn lookup_codes(verse: &Verse, range: &Range<usize>) -> BTreeSet<String> {
    verse
        .word_links
        .iter()
        .filter(|l| l.start < range.end && l.end > range.start)
        .map(|l| l.strongs.clone())
        .chain(
            verse
                .strongs_links
                .iter()
                .filter(|l| l.start < range.end && l.end > range.start)
                .map(|l| l.strongs.clone()),
        )
        .collect()
}
fn word_at(text: &str, cursor: usize) -> Option<Range<usize>> {
    let chars: Vec<_> = text.chars().collect();
    let mut start = cursor.min(chars.len().saturating_sub(1));
    if !chars.get(start).is_some_and(|c| c.is_alphanumeric()) {
        return None;
    }
    let mut end = start + 1;
    while start > 0
        && (chars[start - 1].is_alphanumeric()
            || chars[start - 1] == '’'
            || chars[start - 1] == '\'')
    {
        start -= 1;
    }
    while end < chars.len()
        && (chars[end].is_alphanumeric() || chars[end] == '’' || chars[end] == '\'')
    {
        end += 1;
    }
    Some(start..end)
}
fn friendly_title(title: &str) -> String {
    if let Some((prefix, reference)) = title.split_once(": ") {
        let name = match prefix {
            "MHC" => "Matthew Henry",
            "TSK" => "Treasury of Scripture Knowledge",
            _ => return title.into(),
        };
        let parts: Vec<_> = reference.split('.').collect();
        let reference = match parts.as_slice() {
            [book, chapter, verse] => format!("{book} {chapter}:{verse}"),
            [book, chapter] => format!("{book} {chapter}"),
            _ => reference.into(),
        };
        return format!("{name} · {reference}");
    }
    title.into()
}
fn witness_title(witness: &str) -> String {
    match witness {
        "witness:robinson-tr-first" => "Greek Textus Receptus · first reading".into(),
        "witness:robinson-tr-second" => "Greek Textus Receptus · second reading".into(),
        "witness:oshb-wlc" => "Hebrew / Aramaic · OSHB".into(),
        _ => witness.into(),
    }
}

/// `plain_paragraphs` stores each supplied source paragraph/line on a new line.
/// Preserve their order; do not infer headings or split historical prose by punctuation.
fn article_body(ui: &mut egui::Ui, text: &str) {
    ui.scope(|ui| {
        ui.set_max_width(ui.available_width().min(640.0));
        for paragraph in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                paragraph,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(16.0),
                    color: theme::palette(ui.ctx()).ink,
                    line_height: Some(25.0),
                    ..Default::default()
                },
            );
            ui.add(egui::Label::new(job).wrap().selectable(true));
            ui.add_space(12.0);
        }
    });
}

/// Full-width, left-aligned actions share a stable icon column and touch-sized rows.
fn action_row(
    ui: &mut egui::Ui,
    role: &str,
    title: &str,
    emphasized: bool,
    _mobile: bool,
) -> egui::Response {
    if emphasized {
        return ui.add_sized(
            [ui.available_width(), 64.0],
            egui::Button::image_and_text(
                icon(ui.ctx(), role, 20.0, false).tint(theme::palette(ui.ctx()).on_accent),
                RichText::new(title)
                    .size(14.0)
                    .color(theme::palette(ui.ctx()).on_accent),
            )
            .fill(theme::palette(ui.ctx()).accent)
            .corner_radius(15)
            .right_text(())
            .wrap(),
        );
    }
    let p = theme::palette(ui.ctx());
    let response = ui.add_sized(
        [ui.available_width(), 48.0],
        egui::Button::new("").frame(false),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    if response.hovered() {
        ui.painter().rect_filled(response.rect, 9, p.tint);
    }
    let is_highlight = title.starts_with("Highlight verse");
    let actual_role = if is_highlight {
        "spark"
    } else if title == "Study this verse" {
        "tree"
    } else {
        role
    };
    icon(ui.ctx(), actual_role, 19.0, false).paint_at(
        ui,
        egui::Rect::from_center_size(
            egui::pos2(response.rect.left() + 14.0, response.rect.center().y),
            egui::vec2(19.0, 19.0),
        ),
    );
    let text = if is_highlight {
        "Highlight verse"
    } else {
        title
    };
    ui.painter().text(
        egui::pos2(response.rect.left() + 38.0, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(13.0),
        p.ink,
    );
    if is_highlight {
        ui.painter().circle_filled(
            egui::pos2(response.rect.right() - 49.0, response.rect.center().y),
            6.0,
            egui::Color32::from_rgb(231, 179, 72),
        );
        ui.painter().text(
            egui::pos2(response.rect.right() - 4.0, response.rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "Amber",
            egui::FontId::proportional(10.0),
            p.muted,
        );
    } else if title == "Study this verse" {
        icon(ui.ctx(), "chevron", 16.0, false).paint_at(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(response.rect.right() - 8.0, response.rect.center().y),
                egui::vec2(16.0, 16.0),
            ),
        );
    }
    response
}
fn lookup_button(
    ui: &mut egui::Ui,
    code: &str,
    headword: Option<&DictionaryHeadword>,
) -> egui::Response {
    let p = theme::palette(ui.ctx());
    let shadow = ui.painter().add(egui::Shape::Noop);
    let slot = ui.painter().add(egui::Shape::Noop);
    let response = ui.add_sized(
        [ui.available_width(), 64.0],
        egui::Button::new("").frame(false),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Look up Strong’s {code}"),
        )
    });
    ui.painter().set(
        shadow,
        egui::epaint::Shadow {
            offset: [0, 5],
            blur: 12,
            spread: 0,
            color: p.scrim.gamma_multiply(0.18),
        }
        .as_shape(response.rect, egui::CornerRadius::same(15)),
    );
    theme::wash(
        ui,
        slot,
        response.rect,
        15.0,
        p.accent,
        p.accent.lerp_to_gamma(p.action_light, 0.65),
    );
    let title = if code.starts_with('H') {
        "Look up the Hebrew word"
    } else {
        "Look up the Greek word"
    };
    let available =
        (response.rect.width() - if headword.is_some() { 98.0 } else { 32.0 }).max(80.0);
    let title = ui.painter().layout(
        title.into(),
        egui::FontId::new(13.0, egui::FontFamily::Name("medium".into())),
        p.on_accent,
        available,
    );
    ui.painter().galley(
        response.rect.min + egui::vec2(16.0, 12.0),
        title,
        p.on_accent,
    );
    ui.painter().text(
        response.rect.min + egui::vec2(16.0, 39.0),
        egui::Align2::LEFT_TOP,
        format!("Strong’s {code} · Dictionary entry"),
        egui::FontId::proportional(10.0),
        p.on_accent,
    );
    if let Some(headword) = headword {
        let galley = ui.painter().layout(
            headword.text.clone(),
            egui::FontId::new(27.0, egui::FontFamily::Name("original".into())),
            p.on_accent,
            76.0,
        );
        ui.painter().galley(
            egui::pos2(
                response.rect.right() - 16.0 - galley.size().x,
                response.rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            p.on_accent,
        );
    }
    response
}

/// Keep each original-language word in its own shaping run, separate from Latin labels.
fn original_word_heading(ui: &mut egui::Ui, text: &str) {
    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        egui::FontId::new(53.0, egui::FontFamily::Name("original".into())),
        theme::palette(ui.ctx()).on_feature,
        ui.available_width(),
    );
    job.sections[0].format.line_height = Some(64.0);
    job.wrap.max_rows = 3;
    ui.add(egui::Label::new(job).wrap().selectable(false));
}
fn feature_caption(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text)
            .size(12.0)
            .color(theme::palette(ui.ctx()).on_feature),
    );
}
