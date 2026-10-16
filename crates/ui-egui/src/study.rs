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
impl FontesApp {
    pub(super) fn open_study(&mut self, reference: i64, tab: StudyTab) {
        if self.destination != Destination::Study {
            self.scroll_to = Some(reference);
        }
        self.destination = Destination::Study;
        self.study_tab = tab;
        self.resource = None;
        self.word = None;
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
    fn lookup_word(&mut self, code: &str) {
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
            selected_format.background = accent().gamma_multiply(0.45);
            job.append(&verse.text[start..end], 0.0, selected_format);
            job.append(&verse.text[end..], 0.0, format);
        }
        // An immutable TextBuffer supplies native drag/double-click selection and copy.
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
            .min_size(egui::vec2(0.0, if mobile { 44.0 } else { 0.0 }))
            .desired_width(ui.available_width())
            .layouter(&mut layouter)
            .show(ui);
        let response = &output.response.response;
        if response.clicked()
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
            && let Some(range) = output.cursor_range
        {
            let range = range.as_sorted_char_range();
            let range = range.start.0..range.end.0;
            if !range.is_empty() {
                self.selected_text = Some(selection(verse, range));
            } else if !response.secondary_clicked() && !response.context_menu_opened() {
                self.selected_text = None;
            }
        }
        if response.secondary_clicked() {
            // Right-click without a selection targets the word beneath the pointer.
            if self
                .selected_text
                .as_ref()
                .is_none_or(|t| t.verse_id != verse.id)
                && let Some(pos) = response.interact_pointer_pos()
            {
                let cursor = output.galley.cursor_from_pos(pos - output.galley_pos);
                self.selected_text =
                    word_at(&verse.text, cursor.index.0).map(|range| selection(verse, range));
            }
        }
        response.context_menu(|ui| {
            ui.set_max_width(300.0);
            let selected = self.selected_text.clone().filter(|t| t.verse_id == verse.id);
            if let Some(selected) = &selected {
                ui.label(RichText::new(format!("“{}”", selected.text)).strong());
                let codes = aligned_codes(verse, &selected.range);
                if codes.is_empty() {
                    if ui.button("Look up original word…").clicked() {
                        self.open_study(verse.refs[0], StudyTab::Words);
                        ui.close();
                    }
                    caption(ui, "This edition has no Strong’s link for this selection. Choose a source word in the study panel.");
                } else {
                    for code in codes {
                        if ui.button(format!("Look up Strong’s {code}")).clicked() {
                            self.open_study(verse.refs[0], StudyTab::Words);
                            self.lookup_word(&code);
                            ui.close();
                        }
                    }
                }
                if ui.button("Copy selection").clicked() {
                    ui.ctx().copy_text(selected.text.clone());
                    ui.close();
                }
                ui.separator();
            }
            if ui.button("Study this verse").clicked() {
                self.open_study(verse.refs[0], StudyTab::Commentary);
                ui.close();
            }
            if ui.button("Add note").clicked() {
                self.new_note(verse.refs[0]);
                ui.close();
            }
            if ui.button("Highlight verse · amber").clicked() {
                self.dispatch(Command::SaveAnnotation { id: None, start_ref: verse.refs[0], end_ref: *verse.refs.last().unwrap(), kind: AnnotationKind::Highlight, color: Some("amber".into()), content: None });
                ui.close();
            }
            if ui.button("Copy verse").clicked() {
                ui.ctx().copy_text(format!("{} — {}", verse.text, label(&self.state.as_ref().unwrap().books, verse.refs[0])));
                ui.close();
            }
        });
        if mobile && response.double_clicked() {
            self.open_study(verse.refs[0], StudyTab::Words);
        }
    }
    pub(super) fn study_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.state.clone().unwrap();
        // Each article opens at its beginning; returning restores the results scroll.
        let article_id = self.resource.as_ref().map(|r| r.id.clone());
        egui::ScrollArea::vertical().id_salt(("study-scroll", self.study_tab as u8, article_id)).show(ui, |ui| {
            egui::Frame::new().inner_margin(20).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Study");
                    if ui.button("Back to reading").clicked() {
                        self.choose(Destination::Read);
                    }
                });
                caption(ui, s.reference_label());
                ui.add_space(16.0);
                for row in [[(StudyTab::Words, "Words"), (StudyTab::Commentary, "Commentary")], [(StudyTab::References, "Cross-references"), (StudyTab::Topics, "Topics")]] {
                    ui.columns(2, |cols| {
                        for (col, (tab, title)) in cols.iter_mut().zip(row) {
                            if col.add_sized([col.available_width(), col.spacing().interact_size.y], egui::Button::selectable(self.study_tab == tab, title)).clicked() {
                                self.study_tab = tab;
                                self.resource = None;
                            }
                        }
                    });
                }
                ui.separator();
                ui.add_space(12.0);
                if !s.has_study {
                    ui.label("Open a study library to add Strong’s dictionaries, original Greek, commentary, topics and cross-references.");
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
                            ui.push_id(("cross-reference", target.start_ref, target.end_ref), |ui| {
                                if ui.add_sized([ui.available_width(), ui.spacing().interact_size.y], egui::Button::new(RichText::new(&target.label).color(accent())).right_text(()).wrap()).clicked() { self.select(target.start_ref); }
                                if let Some(preview) = &target.preview { ui.label(preview); } else { caption(ui, "No verse text supplied by this edition."); }
                                if target.start_ref != target.end_ref { caption(ui, "Preview of the first verse"); }
                                if ui.small_button("Source article").clicked() { self.dispatch(Command::Resource { id: target.resource_id.clone() }); }
                                ui.add_space(12.0);
                                ui.separator();
                                ui.add_space(12.0);
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
            definitions,
            occurrences,
            total,
        }) = self.word.clone()
        {
            if ui.button("← Words in this verse").clicked() {
                self.word = None;
            }
            ui.add_space(12.0);
            ui.heading(format!("Strong’s {code}"));
            if definitions.is_empty() {
                ui.label("No dictionary entry installed for this number.");
            }
            for d in definitions {
                ui.add_space(12.0);
                ui.add(egui::Label::new(&d.excerpt).wrap().selectable(true));
                if ui.button("Read full dictionary entry").clicked() {
                    self.dispatch(Command::Resource { id: d.id });
                }
                if let Some(author) = d.author {
                    caption(ui, author);
                }
            }
            ui.add_space(16.0);
            ui.collapsing(format!("Occurrences · {total} verses"), |ui| {
                caption(ui, "Installed source tokens · first 100 distinct verses");
                ui.horizontal_wrapped(|ui| {
                    for r in occurrences {
                        if ui.button(r.label).clicked() {
                            self.select(r.ref_id);
                        }
                    }
                });
            });
        } else {
            if let Some(selected) = &self.selected_text {
                ui.label(RichText::new(format!("“{}”", selected.text)).size(20.0));
                let verse = self
                    .state
                    .as_ref()
                    .unwrap()
                    .passage
                    .iter()
                    .find(|v| v.id == selected.verse_id);
                let codes = verse
                    .map(|v| aligned_codes(v, &selected.range))
                    .unwrap_or_default();
                if codes.is_empty() {
                    ui.label("This edition has no word-level link for this selection. Choose the original word below; these words belong to the whole verse.");
                } else {
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
                    "English word links appear when the edition supplies verified alignments.",
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
                            ui.label(scripture(&t.surface, 22.0, true));
                            for code in codes {
                                if ui.button(code).clicked() {
                                    self.lookup_word(code);
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
            ui.label(
                RichText::new(friendly_title(&r.title))
                    .strong()
                    .color(accent()),
            );
            if let Some(author) = &r.author {
                caption(ui, author);
            }
            caption(
                ui,
                match r.basis.as_str() {
                    "applies_to" => "On this passage",
                    "citation" => "Mentions this passage",
                    _ => "Related to this passage",
                },
            );
            ui.add_space(8.0);
            ui.label(&r.excerpt);
            if ui.button("Open article").clicked() {
                self.dispatch(Command::Resource { id: r.id.clone() });
            }
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);
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
fn aligned_codes(verse: &Verse, range: &Range<usize>) -> BTreeSet<String> {
    verse
        .word_links
        .iter()
        .filter(|l| l.start < range.end && l.end > range.start)
        .map(|l| l.strongs.clone())
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
                    color: theme::color(assets::COLOR_INK),
                    line_height: Some(25.0),
                    ..Default::default()
                },
            );
            ui.add(egui::Label::new(job).wrap().selectable(true));
            ui.add_space(12.0);
        }
    });
}
