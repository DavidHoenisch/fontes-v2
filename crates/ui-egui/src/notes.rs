//! Notes notebook, anchored notes sheet, and versioned scrapbook editor.
use super::*;
#[derive(Default)]
pub(super) struct NotesState {
    pub word_code: String,
    pub insert_kind: u8,
    pub collecting: bool,
    pub undo: Option<(usize, NoteBlock)>,
    pub query: String,
    pub filter: u8,
    pub oldest: bool,
    pub modal_ids: Vec<String>,
    pub modal_anchor: Option<(i64, i64, Option<TextSelection>)>,
    pub clip_request: Option<String>,
    pub passage_clip: Option<NoteBlock>,
    pub insert: bool,
    pub insert_at: Option<usize>,
    pub clip: Option<ResourceDetail>,
    pub excerpt: String,
    pub source: Option<NoteBlock>,
    pub confirm_discard: bool,
}
fn empty_doc() -> NoteDocument {
    NoteDocument {
        title: String::new(),
        pinned: false,
        blocks: vec![],
        selection: None,
    }
}
impl FontesApp {
    pub(super) fn new_note(&mut self, reference: i64) {
        self.notes_state.undo = None;
        self.notes_state.clip = None;
        self.notes_state.passage_clip = None;
        self.notes_state.excerpt.clear();
        self.notes_state.source = None;
        self.notes_state.insert = false;
        self.note = Some(NoteDraft {
            id: None,
            start: reference,
            end: reference,
            text: String::new(),
            document: empty_doc(),
        });
    }
    pub(super) fn edit_note(&mut self, id: &str) {
        let s = self.state.as_ref().unwrap();
        let Some(a) = s.annotations.iter().find(|a| a.id == id) else {
            return;
        };
        let mut document = s
            .notes
            .iter()
            .find(|n| n.annotation_id == id)
            .map(|n| n.document.clone())
            .unwrap_or_else(empty_doc);
        let text = if let Some(NoteBlock::Thought { text }) = document.blocks.first() {
            text.clone()
        } else if s.notes.iter().any(|n| n.annotation_id == id) {
            String::new()
        } else {
            a.content.clone().unwrap_or_default()
        };
        if matches!(document.blocks.first(), Some(NoteBlock::Thought { .. })) {
            document.blocks.remove(0);
        }
        self.notes_state.undo = None;
        self.notes_state.clip = None;
        self.notes_state.passage_clip = None;
        self.notes_state.excerpt.clear();
        self.notes_state.source = None;
        self.notes_state.insert = false;
        self.note = Some(NoteDraft {
            id: Some(id.into()),
            start: a.start_ref,
            end: a.end_ref,
            text,
            document,
        });
        self.notes_state.modal_ids.clear();
    }
    pub(super) fn note_from_selection(
        &mut self,
        start: i64,
        end: i64,
        selection: Option<TextSelection>,
    ) {
        self.new_note(start);
        if let Some(d) = &mut self.note {
            d.end = end;
            d.document.selection = selection;
        }
        self.highlight_draft = None;
    }
    fn note_document(&self) -> Option<NoteDocument> {
        self.note.as_ref().map(|d| {
            let mut doc = d.document.clone();
            if !d.text.trim().is_empty() {
                doc.blocks.insert(
                    0,
                    NoteBlock::Thought {
                        text: d.text.clone(),
                    },
                );
            }
            doc.blocks
                .retain(|b| !matches!(b,NoteBlock::Thought{text} if text.trim().is_empty()));
            doc
        })
    }
    pub(super) fn notes_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.state.clone().unwrap();
        let p = theme::palette(ui.ctx());
        let has_notes = s.annotations.iter().any(|a| a.kind == AnnotationKind::Note);
        egui::ScrollArea::vertical()
            .id_salt("notes-notebook")
            .show(ui, |ui| {
                egui::Frame::new().inner_margin(24).show(ui, |ui| {
                    ui.label(
                        RichText::new("Your notebook")
                            .font(egui::FontId::new(
                                40.0,
                                egui::FontFamily::Name("chapter_heading".into()),
                            ))
                            .color(p.ink),
                    );
                    caption(ui, "Thoughts to keep. Passages to return to.");
                    ui.add_space(20.0);
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(egui::Stroke::new(1.0, p.border))
                        .corner_radius(16)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), 32.0],
                                egui::TextEdit::singleline(&mut self.notes_state.query)
                                    .frame(egui::Frame::NONE)
                                    .hint_text("Find a thought or passage"),
                            );
                        });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        for (i, title) in ["Notes", "Highlights", "Bookmarks"].iter().enumerate() {
                            let selected = self.notes_state.filter == i as u8;
                            let r = ui.add_sized(
                                [ui.available_width().min(96.0), 44.0],
                                egui::Button::new(
                                    RichText::new(*title).size(13.0).color(if selected {
                                        p.accent
                                    } else {
                                        p.muted
                                    }),
                                )
                                .frame(false),
                            );
                            if selected {
                                ui.painter().hline(
                                    r.rect.x_range(),
                                    r.rect.bottom(),
                                    egui::Stroke::new(2.0, p.accent),
                                );
                            }
                            if r.clicked() {
                                self.notes_state.filter = i as u8;
                            }
                        }
                    });
                    ui.add_space(20.0);
                    let q = self.notes_state.query.to_lowercase();
                    let mut entries = s
                        .annotations
                        .iter()
                        .filter(|a| {
                            (a.kind == AnnotationKind::Note) == (self.notes_state.filter == 0)
                        })
                        .filter(|a| {
                            format!(
                                "{} {}",
                                label(&s.books, a.start_ref),
                                a.content.as_deref().unwrap_or("")
                            )
                            .to_lowercase()
                            .contains(&q)
                        })
                        .collect::<Vec<_>>();
                    if self.notes_state.oldest {
                        entries.reverse();
                    }
                    entries.sort_by_key(|a| {
                        !s.notes
                            .iter()
                            .any(|n| n.annotation_id == a.id && n.document.pinned)
                    });
                    let count = if self.notes_state.filter == 2 {
                        s.saved
                            .iter()
                            .filter(|r| r.kind == "favorite" && r.label.to_lowercase().contains(&q))
                            .count()
                    } else {
                        entries.len()
                    };
                    ui.horizontal(|ui| {
                        caption(
                            ui,
                            format!(
                                "{count} {}",
                                ["notes", "highlights", "bookmarks"]
                                    [self.notes_state.filter as usize]
                            ),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_sized(
                                    [110.0, 44.0],
                                    egui::Button::new(if self.notes_state.oldest {
                                        "Oldest first"
                                    } else {
                                        "Recent first"
                                    })
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.notes_state.oldest = !self.notes_state.oldest;
                            }
                        });
                    });
                    if count == 0 {
                        ui.add_space(36.0);
                        theme::editorial(
                            ui,
                            if q.is_empty() {
                                "Let a thought take root."
                            } else {
                                "A little further?"
                            },
                            28.0,
                            p.ink,
                        );
                        ui.label(if q.is_empty() {
                            "Keep a question, a prayer, or something you noticed in the passage."
                        } else {
                            "Try another word or passage reference."
                        });
                        ui.add_space(16.0);
                        if !has_notes && q.is_empty() {
                            if primary(ui, "Write your first note") {
                                self.new_note(s.session.selected_ref);
                            }
                        } else if !q.is_empty() && ui.button("Clear search").clicked() {
                            self.notes_state.query.clear();
                        }
                    }
                    if self.notes_state.filter == 2 {
                        for r in s
                            .saved
                            .iter()
                            .filter(|r| r.kind == "favorite" && r.label.to_lowercase().contains(&q))
                        {
                            let mut menu_action = false;
                            let card = theme::card(ui.ctx()).show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.colored_label(p.accent, &r.label);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.menu_button("More", |ui| {
                                                if ui.button("Remove bookmark").clicked() {
                                                    self.dispatch(Command::Favorite {
                                                        ref_id: r.ref_id,
                                                        enabled: false,
                                                    });
                                                    menu_action = true;
                                                    ui.close();
                                                }
                                            });
                                        },
                                    );
                                });
                                if let Some(v) =
                                    s.passage.iter().find(|v| v.refs.contains(&r.ref_id))
                                {
                                    ui.label(RichText::new(&v.text).size(19.0));
                                } else {
                                    caption(ui, "A passage to return to ↗");
                                }
                                ui.add_space(12.0);
                            });
                            if card.response.interact(egui::Sense::click()).clicked()
                                && !menu_action
                            {
                                self.select(r.ref_id);
                            }
                            ui.add_space(12.0);
                        }
                    } else {
                        for a in entries {
                            let rich = s.notes.iter().find(|n| n.annotation_id == a.id);
                            let mut menu_action = false;
                            let card = theme::card(ui.ctx()).show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.colored_label(p.accent, label(&s.books, a.start_ref));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.menu_button("More", |ui| {
                                                if ui
                                                    .add_sized(
                                                        [140.0, 44.0],
                                                        egui::Button::new("Read passage"),
                                                    )
                                                    .clicked()
                                                {
                                                    self.destination = Destination::Read;
                                                    self.select(a.start_ref);
                                                    menu_action = true;
                                                    ui.close();
                                                }
                                                if a.kind == AnnotationKind::Note
                                                    && ui
                                                        .add_sized(
                                                            [140.0, 44.0],
                                                            egui::Button::new("Edit note"),
                                                        )
                                                        .clicked()
                                                {
                                                    self.edit_note(&a.id);
                                                    menu_action = true;
                                                    ui.close();
                                                }
                                                ui.menu_button("Delete…", |ui| {
                                                    if ui
                                                        .button(
                                                            "Permanently delete this annotation",
                                                        )
                                                        .clicked()
                                                    {
                                                        self.dispatch(Command::DeleteAnnotation {
                                                            id: a.id.clone(),
                                                        });
                                                        menu_action = true;
                                                        ui.close();
                                                    }
                                                });
                                            });
                                        },
                                    );
                                });
                                ui.add_space(8.0);
                                if let Some(n) = rich {
                                    if !n.document.title.is_empty() {
                                        ui.label(
                                            RichText::new(&n.document.title).size(18.0).strong(),
                                        );
                                        ui.add_space(8.0);
                                    }
                                    let preview = n.document.blocks.iter().find_map(|b| {
                                        if let NoteBlock::Thought { text } = b {
                                            Some(text.as_str())
                                        } else {
                                            None
                                        }
                                    });
                                    if let Some(text) = preview {
                                        ui.label(
                                            RichText::new(
                                                text.chars().take(150).collect::<String>(),
                                            )
                                            .size(15.0)
                                            .color(p.muted),
                                        );
                                    }
                                    if let Some(sel) = &n.document.selection {
                                        ui.add_space(12.0);
                                        ui.label(
                                            RichText::new(
                                                sel.spans
                                                    .iter()
                                                    .map(|s| s.quote.as_str())
                                                    .collect::<Vec<_>>()
                                                    .join(" ")
                                                    .chars()
                                                    .take(130)
                                                    .collect::<String>(),
                                            )
                                            .size(18.0)
                                            .italics(),
                                        );
                                    }
                                    let pieces = n
                                        .document
                                        .blocks
                                        .iter()
                                        .filter(|b| !matches!(b, NoteBlock::Thought { .. }))
                                        .count();
                                    if pieces > 0 {
                                        ui.add_space(12.0);
                                        caption(ui, format!("{pieces} collected pieces"));
                                    }
                                    if n.document.pinned {
                                        caption(ui, "Pinned observation");
                                    }
                                } else {
                                    ui.label(
                                        RichText::new(
                                            a.content
                                                .as_deref()
                                                .unwrap_or("Highlight")
                                                .chars()
                                                .take(160)
                                                .collect::<String>(),
                                        )
                                        .size(16.0),
                                    );
                                }
                                if a.kind == AnnotationKind::Highlight
                                    && let Some(h) =
                                        s.highlights.iter().find(|h| h.annotation_id == a.id)
                                    && let Some(sel) = &h.selection
                                {
                                    ui.label(
                                        sel.spans
                                            .iter()
                                            .map(|s| s.quote.as_str())
                                            .collect::<Vec<_>>()
                                            .join(" "),
                                    );
                                }
                                ui.add_space(16.0);
                            });
                            let response = card.response.interact(egui::Sense::click());
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    format!("Open note {}", a.id),
                                )
                            });
                            if response.clicked() && !menu_action {
                                if a.kind == AnnotationKind::Note {
                                    self.notes_state.modal_ids = vec![a.id.clone()];
                                    ui.ctx().request_repaint();
                                } else {
                                    self.select(a.start_ref);
                                    self.destination = Destination::Read;
                                }
                            }
                            ui.add_space(12.0);
                        }
                    }
                    ui.add_space(96.0);
                });
            });
        if has_notes && self.notes_state.modal_ids.is_empty() && self.notes_state.source.is_none() {
            let rect = ui.max_rect();
            egui::Area::new(egui::Id::new("new-note-fab"))
                .fixed_pos(egui::pos2(rect.right() - 80.0, rect.bottom() - 76.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let r = ui.add_sized(
                        [56.0, 56.0],
                        egui::Button::new("").fill(p.accent).corner_radius(28),
                    );
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "New note")
                    });
                    let c = r.rect.center();
                    let paint = ui.painter();
                    for points in [
                        vec![[-1.0, -16.0], [2.0, -17.0], [0.0, 17.0], [-3.0, 16.0]],
                        vec![[-15.0, 1.0], [-14.0, -2.0], [16.0, -3.0], [14.0, 0.0]],
                    ] {
                        paint.add(egui::Shape::convex_polygon(
                            points
                                .into_iter()
                                .map(|a| c + egui::vec2(a[0], a[1]))
                                .collect(),
                            p.on_accent,
                            egui::Stroke::NONE,
                        ));
                    }
                    if r.clicked() {
                        self.new_note(s.session.selected_ref);
                        ui.ctx().request_repaint();
                    }
                });
        }
    }
    pub(super) fn note_editor(&mut self, ui: &mut egui::Ui) {
        let Some(mut draft) = self.note.take() else {
            return;
        };
        let s = self.state.clone().unwrap();
        let mut save = false;
        let mut cancel = false;
        egui::Panel::bottom("scrapbook-actions")
            .exact_size(76.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_sized([110.0, 48.0], egui::Button::new("Add to note"))
                        .clicked()
                    {
                        self.notes_state.insert = true;
                    }
                    ui.add_enabled_ui(!self.save_pending, |ui| {
                        save = primary(ui, "Save note");
                    });
                });
            });
        egui::ScrollArea::vertical()
            .id_salt(("scrapbook-editor", draft.id.clone()))
            .show(ui, |ui| {
                egui::Frame::new().inner_margin(24).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized([100.0, 44.0], egui::Button::new("Cancel"))
                            .clicked()
                        {
                            cancel = true;
                        }
                        ui.checkbox(&mut draft.document.pinned, "Pin note");
                    });
                    caption(ui, format!("A NOTE ON {}", label(&s.books, draft.start)));
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.document.title)
                            .hint_text("Give this thought a title")
                            .font(egui::FontId::proportional(24.0))
                            .desired_width(ui.available_width()),
                    );
                    if let Some(sel) = &draft.document.selection {
                        theme::card(ui.ctx()).show(ui, |ui| {
                            ui.label(
                                sel.spans
                                    .iter()
                                    .map(|s| s.quote.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            );
                            caption(ui, "Attached to these words");
                        });
                    }
                    ui.add_space(16.0);
                    ui.add(
                        egui::TextEdit::multiline(&mut draft.text)
                            .desired_rows(4)
                            .desired_width(ui.available_width())
                            .hint_text("What are you noticing?"),
                    );
                    ui.add_space(16.0);
                    let mut action = None;
                    for (i, b) in draft.document.blocks.iter_mut().enumerate() {
                        ui.push_id(i, |ui| {
                            theme::card(ui.ctx()).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    caption(
                                        ui,
                                        match b {
                                            NoteBlock::Thought { .. } => "YOUR THOUGHT",
                                            NoteBlock::Passage { .. } => "SCRIPTURE",
                                            NoteBlock::Resource { resource_id, .. }
                                                if resource_id
                                                    .to_lowercase()
                                                    .contains("strong") =>
                                            {
                                                "WORD STUDY"
                                            }
                                            NoteBlock::Resource { .. } => "RESOURCE EXCERPT",
                                        },
                                    );
                                    if ui
                                        .add_sized([44.0, 44.0], egui::Button::new("↑"))
                                        .on_hover_text("Move up")
                                        .clicked()
                                    {
                                        action = Some((i, 0));
                                    }
                                    if ui
                                        .add_sized([44.0, 44.0], egui::Button::new("↓"))
                                        .on_hover_text("Move down")
                                        .clicked()
                                    {
                                        action = Some((i, 1));
                                    }
                                    if ui
                                        .add_sized([44.0, 44.0], egui::Button::new("×"))
                                        .on_hover_text("Remove piece")
                                        .clicked()
                                    {
                                        action = Some((i, 2));
                                    }
                                });
                                match b {
                                    NoteBlock::Thought { text } => {
                                        ui.add(
                                            egui::TextEdit::multiline(text)
                                                .desired_rows(3)
                                                .desired_width(ui.available_width())
                                                .hint_text("Connect these pieces…"),
                                        );
                                    }
                                    _ => self.note_block(ui, b),
                                }
                            });
                            if ui
                                .add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Button::new("+ Insert here"),
                                )
                                .clicked()
                            {
                                self.notes_state.insert_at = Some(i + 1);
                                self.notes_state.insert = true;
                            }
                            ui.add_space(16.0);
                        });
                    }
                    if let Some((i, kind)) = action {
                        match kind {
                            0 if i > 0 => draft.document.blocks.swap(i, i - 1),
                            1 if i + 1 < draft.document.blocks.len() => {
                                draft.document.blocks.swap(i, i + 1)
                            }
                            2 => {
                                let removed = draft.document.blocks.remove(i);
                                self.notes_state.undo = Some((i, removed));
                            }
                            _ => {}
                        }
                    }
                    if self.notes_state.undo.is_some()
                        && ui
                            .add_sized([140.0, 44.0], egui::Button::new("Undo removal"))
                            .clicked()
                        && let Some((i, b)) = self.notes_state.undo.take()
                    {
                        draft
                            .document
                            .blocks
                            .insert(i.min(draft.document.blocks.len()), b);
                    }
                    caption(
                        ui,
                        "Private on this device · Sources stay with their excerpts",
                    );
                });
            });
        self.note = Some(draft);
        if save {
            let d = self.note.as_ref().unwrap();
            let command = Command::SaveNote {
                id: d.id.clone(),
                start_ref: d.start,
                end_ref: d.end,
                document: self.note_document().unwrap(),
            };
            self.save_pending = true;
            self.dispatch(command);
        }
        if cancel {
            self.notes_state.confirm_discard = true;
        }
    }
    fn note_block(&mut self, ui: &mut egui::Ui, block: &NoteBlock) {
        match block {
            NoteBlock::Thought { text } => {
                ui.label(RichText::new(text).size(16.0));
            }
            NoteBlock::Passage {
                ref_id,
                translation_id,
                quote,
            } => {
                ui.label(RichText::new(quote).size(20.0));
                if ui
                    .add_sized(
                        [ui.available_width(), 44.0],
                        egui::Button::new(format!(
                            "{} · {} ↗",
                            label(&self.state.as_ref().unwrap().books, *ref_id),
                            translation_id
                        )),
                    )
                    .clicked()
                {
                    self.notes_state.passage_clip = None;
                    self.notes_state.source = Some(block.clone());
                }
            }
            NoteBlock::Resource {
                quote,
                title,
                author,
                ..
            } => {
                ui.label(RichText::new(quote).size(17.0));
                caption(ui, author.as_deref().unwrap_or("Source excerpt"));
                if ui
                    .add_sized(
                        [ui.available_width(), 44.0],
                        egui::Button::new(format!("{} ↗", title)),
                    )
                    .clicked()
                {
                    self.notes_state.passage_clip = None;
                    self.notes_state.source = Some(block.clone());
                }
            }
        }
    }
}
impl FontesApp {
    fn insert_note_block(&mut self, block: NoteBlock) {
        if let Some(d) = &mut self.note {
            let i = self
                .notes_state
                .insert_at
                .take()
                .unwrap_or(d.document.blocks.len())
                .min(d.document.blocks.len());
            d.document.blocks.insert(i, block);
        }
        self.notes_state.insert = false;
        self.notes_state.clip = None;
        self.notes_state.passage_clip = None;
    }
    pub(super) fn collect_resource(&mut self, r: ResourceDetail) {
        self.notes_state.clip = Some(r);
        self.notes_state.excerpt.clear();
        if self.note.is_some() {
            self.notes_state.insert = true;
        } else {
            self.notes_state.collecting = true;
        }
    }
    pub(super) fn notes_modal(&mut self, ctx: &egui::Context) {
        if self.notes_state.collecting {
            let state = self.state.clone().unwrap();
            let mut target = None;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("collect-note-target")).show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 48.0).min(480.0));
                theme::editorial(ui, "Keep this in a note", 28.0, theme::palette(ctx).ink);
                if primary(ui, "Create a new note") {
                    target = Some(None);
                }
                egui::ScrollArea::vertical()
                    .max_height(ctx.content_rect().height() * 0.5)
                    .show(ui, |ui| {
                        for a in state
                            .annotations
                            .iter()
                            .filter(|a| a.kind == AnnotationKind::Note)
                        {
                            let title = state
                                .notes
                                .iter()
                                .find(|n| n.annotation_id == a.id)
                                .map(|n| n.document.title.as_str())
                                .filter(|s| !s.is_empty())
                                .unwrap_or("Untitled note");
                            if ui
                                .add_sized(
                                    [ui.available_width(), 48.0],
                                    egui::Button::new(format!(
                                        "{} · {}",
                                        title,
                                        label(&state.books, a.start_ref)
                                    )),
                                )
                                .clicked()
                            {
                                target = Some(Some(a.id.clone()));
                            }
                        }
                    });
                if ui
                    .add_sized([120.0, 44.0], egui::Button::new("Cancel collection"))
                    .clicked()
                {
                    cancel = true;
                }
            });
            if let Some(id) = target {
                let clip = self.notes_state.clip.clone();
                let passage = self.notes_state.passage_clip.clone();
                let excerpt = self.notes_state.excerpt.clone();
                if let Some(id) = id {
                    self.edit_note(&id);
                } else {
                    self.new_note(state.session.selected_ref);
                }
                self.notes_state.clip = clip;
                self.notes_state.passage_clip = passage;
                self.notes_state.excerpt = excerpt;
                self.notes_state.collecting = false;
                self.notes_state.insert = true;
            }
            if cancel {
                self.notes_state.collecting = false;
            }
            return;
        }
        if self.notes_state.confirm_discard {
            egui::Modal::new(egui::Id::new("discard-note")).show(ctx, |ui| {
                ui.heading("Discard unsaved changes?");
                ui.label("Your saved note will be kept.");
                if primary(ui, "Keep writing") {
                    self.notes_state.confirm_discard = false;
                }
                if ui
                    .add_sized([180.0, 44.0], egui::Button::new("Discard changes"))
                    .clicked()
                {
                    self.note = None;
                    self.notes_state.confirm_discard = false;
                }
            });
            return;
        }
        if let Some(block) = self.notes_state.source.clone() {
            let mut close = false;
            egui::Modal::new(egui::Id::new("note-source")).show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 48.0).min(520.0));
                theme::editorial(ui, "Source context", 28.0, theme::palette(ctx).ink);
                egui::ScrollArea::vertical()
                    .max_height(ctx.content_rect().height() * 0.55)
                    .show(ui, |ui| match &block {
                        NoteBlock::Passage {
                            ref_id,
                            translation_id,
                            quote,
                        } => {
                            ui.label(quote);
                            caption(
                                ui,
                                format!(
                                    "{} · {}",
                                    label(&self.state.as_ref().unwrap().books, *ref_id),
                                    translation_id
                                ),
                            );
                            if ui.button("View current edition text").clicked() {
                                self.dispatch(Command::NotePassage { ref_id: *ref_id });
                            }
                            if let Some(NoteBlock::Passage { quote: current, .. }) =
                                &self.notes_state.passage_clip
                            {
                                ui.separator();
                                ui.label(current);
                            }
                        }
                        NoteBlock::Resource {
                            quote,
                            title,
                            author,
                            source,
                            content_id,
                            resource_id,
                        } => {
                            ui.label(quote);
                            caption(ui, title);
                            caption(ui, author.as_deref().unwrap_or(""));
                            caption(ui, format!("Retained excerpt · {source} · {content_id}"));
                            if ui.button("View full source").clicked() {
                                self.dispatch(Command::Resource {
                                    id: resource_id.clone(),
                                });
                            }
                            if let Some(r) = &self.resource
                                && r.id == *resource_id
                            {
                                ui.separator();
                                ui.label(&r.content);
                                caption(
                                    ui,
                                    format!(
                                        "Source {} · Rights review {}",
                                        r.source, r.rights_review
                                    ),
                                );
                            }
                        }
                        _ => {}
                    });
                if primary(ui, "Return to note") {
                    close = true;
                }
            });
            if close {
                ctx.request_repaint();
                self.notes_state.source = None;
                self.notes_state.passage_clip = None;
            }
            return;
        }
        if self.notes_state.insert && self.note.is_some() {
            let mut close = false;
            let response = egui::Modal::new(egui::Id::new("note-insert")).show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 48.0).min(520.0));
                theme::editorial(ui, "Add to your note", 28.0, theme::palette(ctx).ink);
                if ui
                    .add_sized(
                        [ui.available_width(), 44.0],
                        egui::Button::new("Close insertion menu"),
                    )
                    .clicked()
                {
                    close = true;
                }
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height()*0.5).max(120.0))
                    .show(ui, |ui| {
                        if primary(ui, "Your thought") {
                            self.insert_note_block(NoteBlock::Thought {
                                text: String::new(),
                            });
                        }
                        ui.horizontal_wrapped(|ui|{
                            for (i,title) in ["Passage / resource","Word definition"].iter().enumerate(){if ui.add_sized([140.0,44.0],egui::Button::new(*title).selected(self.notes_state.insert_kind==i as u8)).clicked(){self.notes_state.insert_kind=i as u8;self.notes_state.clip=None;self.notes_state.passage_clip=None;}}
                        });
                        if let Some(r) = self.notes_state.clip.clone() {
                            ui.label(&r.title);
                            caption(ui, r.author.as_deref().unwrap_or(""));
                            ui.collapsing("Choose a source paragraph",|ui|{
                                egui::ScrollArea::vertical().max_height(180.0).id_salt("clip-paragraphs").show(ui,|ui|{for (i,para) in r.content.lines().filter(|s|!s.trim().is_empty()).enumerate(){if ui.add(egui::Button::new(para.chars().take(140).collect::<String>()).wrap()).clicked(){self.notes_state.excerpt=para.to_owned();}
if i>200{break;}}});
                            });
                            ui.label("Exact excerpt to keep (select a paragraph above, or paste source text):");
                            ui.add(
                                egui::TextEdit::multiline(&mut self.notes_state.excerpt)
                                    .desired_rows(4)
                                    .desired_width(ui.available_width()),
                            );
                            if ui.button("Choose another piece").clicked() {
                                self.notes_state.clip = None;
                            }
                        } else if let Some(block) = self.notes_state.passage_clip.clone() {
                            if let NoteBlock::Passage { quote, .. } = &block {
                                ui.label(RichText::new(quote).size(20.0));
                            }

                            if ui.button("Choose another piece").clicked() {
                                self.notes_state.passage_clip = None;
                            }
                        } else if self.notes_state.insert_kind==1 {
                            ui.add(egui::TextEdit::singleline(&mut self.notes_state.word_code).hint_text("Strong’s number, e.g. H3068"));
                            if ui.add_enabled(!self.notes_state.word_code.trim().is_empty(),egui::Button::new("Look up definition").min_size(egui::vec2(160.0,44.0))).clicked(){self.dispatch(Command::Word{code:self.notes_state.word_code.clone()});}
                            if let Some(Data::Word{code,definitions,..})=self.word.clone(){
                                caption(ui,format!("Definitions for {code}"));
                                for entry in definitions{if ui.add_sized([ui.available_width(),44.0],egui::Button::new(entry.title)).clicked(){
                                    if let Some(r)=self.word_entries.get(&entry.id).cloned(){self.notes_state.excerpt=r.content.clone();self.notes_state.clip=Some(r);}else{self.notes_state.clip_request=Some(entry.id.clone());self.dispatch(Command::Resource{id:entry.id});}
                                }}
                            }
                        } else {
                            if ui
                                .add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Button::new("Current passage"),
                                )
                                .clicked()
                            {
                                self.dispatch(Command::NotePassage {
                                    ref_id: self.state.as_ref().unwrap().session.selected_ref,
                                });
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text("Find a passage, commentary, or word"),
                            );
                            if ui
                                .add_enabled(
                                    !self.query.trim().is_empty() && self.query.len() <= 512,
                                    egui::Button::new("Search library")
                                        .min_size(egui::vec2(160.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.dispatch(Command::Search {
                                    query: self.query.clone(),
                                    offset: 0,
                                    limit: 20,
                                });
                            }
                            for hit in self.hits.clone() {
                                theme::card(ctx).show(ui, |ui| {
                                    ui.label(&hit.title);
                                    ui.label(&hit.excerpt);
                                    if ui
                                        .add_sized(
                                            [140.0, 44.0],
                                            egui::Button::new("Preview piece"),
                                        )
                                        .clicked()
                                    {
                                        if let Some(ref_id) = hit.ref_id {
                                            self.dispatch(Command::NotePassage { ref_id });
                                        } else {
                                            self.notes_state.clip_request = Some(hit.id.clone());
                                            self.dispatch(Command::Resource { id: hit.id });
                                        }
                                    }
                                });
                            }
                        }
                    });
                if let Some(r)=self.notes_state.clip.clone(){
                            let valid = !self.notes_state.excerpt.trim().is_empty()
                                && r.content.contains(&self.notes_state.excerpt);
                            if ui
                                .add_enabled(
                                    valid,
                                    egui::Button::new("Add resource excerpt")
                                        .min_size(egui::vec2(180.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.insert_note_block(NoteBlock::Resource {
                                    resource_id: r.id,
                                    quote: self.notes_state.excerpt.clone(),
                                    title: r.title,
                                    author: r.author,
                                    source: r.source,
                                    content_id: self.state.as_ref().unwrap().content_id.clone(),
                                });
                            }
                }else if let Some(block)=self.notes_state.passage_clip.clone() && primary(ui,"Add passage"){self.insert_note_block(block);}
            });
            if close || response.should_close() {
                ctx.request_repaint();
                self.notes_state.insert = false;
            }
            return;
        }
        if !self.notes_state.modal_ids.is_empty() {
            let s = self.state.clone().unwrap();
            let ids = self.notes_state.modal_ids.clone();
            let mut close = false;
            let response = egui::Modal::new(egui::Id::new("anchored-notes"))
                .area(
                    egui::Modal::default_area(egui::Id::new("anchored-notes"))
                        .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO),
                )
                .frame(
                    egui::Frame::new()
                        .fill(theme::palette(ctx).surface)
                        .corner_radius(28)
                        .inner_margin(24),
                )
                .show(ctx, |ui| {
                    ui.set_width((ctx.content_rect().width() - 48.0).min(580.0));
                    ui.horizontal_top(|ui| {
                        let width = ui.available_width() - 48.0;
                        ui.allocate_ui(egui::vec2(width, 48.0), |ui| {
                            theme::editorial(
                                ui,
                                &if ids.len() == 1 {
                                    "Your note".into()
                                } else {
                                    format!("{} notes on these words", ids.len())
                                },
                                24.0,
                                theme::palette(ctx).ink,
                            );
                        });
                        if theme::round_close(ui, "Close notes").clicked() {
                            close = true;
                        }
                    });
                    egui::ScrollArea::vertical()
                        .max_height((ctx.content_rect().height() * 0.65 - 120.0).max(140.0))
                        .show(ui, |ui| {
                            for id in &ids {
                                let Some(a) = s.annotations.iter().find(|a| &a.id == id) else {
                                    continue;
                                };
                                ui.push_id(id, |ui| {
                                    caption(ui, label(&s.books, a.start_ref));
                                    if let Some(r) = s.notes.iter().find(|n| &n.annotation_id == id)
                                    {
                                        if let Some(sel) = &r.document.selection {
                                            ui.label(
                                                RichText::new(
                                                    sel.spans
                                                        .iter()
                                                        .map(|s| s.quote.as_str())
                                                        .collect::<Vec<_>>()
                                                        .join(" "),
                                                )
                                                .italics(),
                                            );
                                        }
                                        theme::editorial(
                                            ui,
                                            &r.document.title,
                                            24.0,
                                            theme::palette(ctx).ink,
                                        );
                                        for b in &r.document.blocks {
                                            self.note_block(ui, b);
                                            ui.add_space(16.0);
                                        }
                                    } else {
                                        ui.label(a.content.as_deref().unwrap_or(""));
                                    }
                                    if ui
                                        .add_sized([120.0, 44.0], egui::Button::new("Edit note"))
                                        .clicked()
                                    {
                                        self.edit_note(id);
                                    }
                                    ui.separator();
                                });
                            }
                        });
                    if primary(ui, "+ Add another note") {
                        let anchor = self.notes_state.modal_anchor.clone().or_else(|| {
                            ids.first().and_then(|id| {
                                s.annotations.iter().find(|a| &a.id == id).map(|a| {
                                    (
                                        a.start_ref,
                                        a.end_ref,
                                        s.notes
                                            .iter()
                                            .find(|n| &n.annotation_id == id)
                                            .and_then(|n| n.document.selection.clone()),
                                    )
                                })
                            })
                        });
                        if let Some((start, end, sel)) = anchor {
                            self.note_from_selection(start, end, sel);
                        }
                        close = true;
                    }
                });
            if close || response.should_close() {
                ctx.request_repaint();
                self.notes_state.modal_ids.clear();
            }
        }
    }
}
impl FontesApp {
    pub(super) fn draw_note_markers(
        &mut self,
        ui: &mut egui::Ui,
        verse: &Verse,
        galley: &std::sync::Arc<egui::Galley>,
        pos: egui::Pos2,
        rect: egui::Rect,
    ) {
        let s = self.state.as_ref().unwrap();
        let mut groups: std::collections::BTreeMap<usize, Vec<String>> = Default::default();
        for a in s.annotations.iter().filter(|a| {
            a.kind == AnnotationKind::Note
                && verse
                    .refs
                    .iter()
                    .any(|r| *r >= a.start_ref && *r <= a.end_ref)
        }) {
            let selection = s
                .notes
                .iter()
                .find(|n| n.annotation_id == a.id)
                .and_then(|n| n.document.selection.as_ref());
            let at = if let Some(sel) = selection {
                if sel.translation_id != s.session.translation {
                    continue;
                }
                let Some(span) = sel.spans.iter().find(|span| span.verse_id == verse.id) else {
                    continue;
                };
                if verse
                    .text
                    .chars()
                    .skip(span.start)
                    .take(span.end.saturating_sub(span.start))
                    .collect::<String>()
                    != span.quote
                {
                    continue;
                }
                // One marker at the final selected verse; notes remain reachable in the notebook.
                if sel
                    .spans
                    .last()
                    .is_none_or(|last| last.verse_id != verse.id)
                {
                    continue;
                }
                span.end
            } else {
                verse.text.chars().count()
            };
            groups.entry(at).or_default().push(a.id.clone());
        }
        let p = theme::palette(ui.ctx());
        for (end, ids) in groups {
            let cursor = galley.pos_from_cursor(egui::text::CCursor::new(end));
            let x = rect.right() + 4.0;
            let y = pos.y + cursor.center().y - 12.0;
            let r = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(28.0, 28.0));
            let response = ui.interact(
                r.expand(8.0),
                egui::Id::new(("inline-notes", &verse.id, end)),
                egui::Sense::click(),
            );
            let title = format!("Open {} notes on this selection", ids.len());
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &title));
            ui.painter().rect_filled(r, 7, p.surface);
            ui.painter().rect_stroke(
                r.shrink(5.0),
                2,
                egui::Stroke::new(1.2, p.accent),
                egui::StrokeKind::Inside,
            );
            ui.painter().line_segment(
                [
                    r.min + egui::vec2(8.0, 11.0),
                    r.min + egui::vec2(19.0, 11.0),
                ],
                egui::Stroke::new(1.0, p.accent),
            );
            if ids.len() > 1 {
                ui.painter().text(
                    r.right_top(),
                    egui::Align2::RIGHT_TOP,
                    ids.len().to_string(),
                    egui::FontId::proportional(10.0),
                    p.accent,
                );
            }
            if response.clicked() {
                self.notes_state.modal_ids = ids;
                self.highlight_draft = None;
                self.suppress_word_tap = true;
            }
        }
    }
}
