//! Thin egui presentation. All library queries and durable actions dispatch engine commands.
#[cfg(test)]
mod tests;

mod study;
mod theme;

use study::{SelectedText, StudyTab};

use eframe::egui::{self, RichText};

use fontes_engine::{
    worker::{self, Host, Request},
    *,
};

use std::path::PathBuf;

use theme::{accent, assets, caption, icon, muted, nav, primary, scripture};

#[derive(Clone, Copy, PartialEq)]
enum Destination {
    Read,
    Search,
    Notes,
    Study,
    Preferences,
}
struct NoteDraft {
    id: Option<String>,
    start: i64,
    end: i64,
    text: String,
    color: Option<String>,
    kind: AnnotationKind,
}
pub struct FontesApp {
    host: Host,
    state: Option<Workspace>,
    data_dir: PathBuf,
    library_path: String,
    opening_path: Option<String>,
    close_after_work: bool,
    destination: Destination,
    pending: usize,
    error: Option<String>,
    reference: String,
    picker: bool,
    picker_book: Option<u16>,
    picker_chapter: u16,
    picker_verse: u16,
    testament: String,
    query: String,
    hits: Vec<SearchHit>,
    search_offset: u32,
    search_more: bool,
    context: Option<Data>,
    resource: Option<ResourceDetail>,
    study_tab: StudyTab,
    include_mentions: bool,
    witness: String,
    selected_text: Option<SelectedText>,
    selection_generation: u64,
    word_code: String,
    word: Option<Data>,
    note: Option<NoteDraft>,
    scroll_to: Option<i64>,
    save_pending: bool,
}
impl FontesApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        library: Option<PathBuf>,
        data_dir: PathBuf,
    ) -> Self {
        Self::with_context(&cc.egui_ctx, library, data_dir)
    }
    pub fn with_context(ctx: &egui::Context, library: Option<PathBuf>, data_dir: PathBuf) -> Self {
        theme::install(ctx);

        let library = library.or_else(|| {
            std::fs::read_to_string(data_dir.join("library-path.txt"))
                .ok()
                .filter(|s| !s.trim().is_empty() && s.len() <= 4096)
                .map(|s| PathBuf::from(s.trim()))
        });

        let wake = ctx.clone();

        let host = worker::spawn(move || wake.request_repaint());

        let library_path = library
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| data_dir.join("library.sqlite").display().to_string());

        let mut app = Self {
            host,
            state: None,
            data_dir,
            library_path,
            opening_path: None,
            close_after_work: false,
            destination: Destination::Read,
            pending: 0,
            error: None,
            reference: String::new(),
            picker: false,
            picker_book: None,
            picker_chapter: 1,
            picker_verse: 1,
            testament: "NT".into(),
            query: String::new(),
            hits: vec![],
            search_offset: 0,
            search_more: false,
            context: None,
            resource: None,
            study_tab: StudyTab::Commentary,
            include_mentions: false,
            witness: String::new(),
            selected_text: None,
            selection_generation: 0,
            word_code: String::new(),
            word: None,
            note: None,
            scroll_to: None,
            save_pending: false,
        };

        if library.is_some() {
            app.open();
        }
        app
    }
    fn open(&mut self) {
        if self.pending > 0 {
            self.error = Some("Wait for the current library operation to finish.".into());

            return;
        }
        if let Err(e) = std::fs::create_dir_all(&self.data_dir) {
            self.error = Some(e.to_string());

            return;
        }
        self.opening_path = Some(self.library_path.trim().to_owned());

        self.send(Request::Open {
            library: PathBuf::from(self.library_path.trim()),
            user_db: self.data_dir.join("user.sqlite"),
        });
    }
    fn send(&mut self, request: Request) {
        match self.host.requests.try_send(request) {
            Ok(()) => {
                self.pending += 1;

                self.error = None;
            }
            Err(e) => self.error = Some(format!("The library is busy; retry this action. {e}")),
        }
    }
    fn dispatch(&mut self, command: Command) {
        self.send(Request::Dispatch(command));
    }
    fn receive(&mut self) {
        while let Ok(result) = self.host.events.try_recv() {
            self.pending = self.pending.saturating_sub(1);

            match result {
                Ok(response) => {
                    if let Some(path) = self.opening_path.take() {
                        let tmp = self.data_dir.join("library-path.tmp");

                        if let Err(e) = std::fs::write(&tmp, path).and_then(|()| {
                            std::fs::rename(&tmp, self.data_dir.join("library-path.txt"))
                        }) {
                            self.error = Some(format!(
                                "Library opened, but its path could not be remembered: {e}"
                            ));
                        }
                    }
                    let content_changed = self
                        .state
                        .as_ref()
                        .is_some_and(|s| s.content_id != response.state.content_id);
                    let changed = content_changed
                        || self.state.as_ref().is_none_or(|s| {
                            s.session.selected_ref != response.state.session.selected_ref
                        });
                    if content_changed {
                        self.word = None;
                        self.hits.clear();
                        self.search_offset = 0;
                        self.search_more = false;
                    }

                    let translation_changed = self.state.as_ref().is_some_and(|s| {
                        s.session.translation != response.state.session.translation
                    });

                    if content_changed || translation_changed {
                        self.selection_generation = self.selection_generation.wrapping_add(1);
                        self.selected_text = None;
                    }
                    if translation_changed {
                        self.scroll_to = Some(response.state.session.selected_ref);
                        self.hits.clear();
                        self.search_more = false;
                        if self.destination == Destination::Search && !self.query.trim().is_empty()
                        {
                            self.dispatch(Command::Search {
                                query: self.query.clone(),
                                limit: 30,
                                offset: 0,
                            });
                        }
                    }
                    if changed {
                        self.word = None;
                        if self
                            .selected_text
                            .as_ref()
                            .is_some_and(|t| t.ref_id != response.state.session.selected_ref)
                        {
                            self.selected_text = None;
                        }
                        self.scroll_to = Some(response.state.session.selected_ref);

                        self.context = None;

                        self.resource = None;

                        if self.destination == Destination::Study {
                            self.dispatch(Command::Context {
                                ref_id: response.state.session.selected_ref,
                            });
                        }
                    }
                    if translation_changed && !changed {
                        self.context = None;
                        if self.destination == Destination::Study {
                            self.dispatch(Command::Context {
                                ref_id: response.state.session.selected_ref,
                            });
                        }
                    }
                    self.state = Some(response.state);

                    match response.data {
                        d @ Data::Context { ref_id, .. } => {
                            if self
                                .state
                                .as_ref()
                                .is_some_and(|s| s.session.selected_ref == ref_id)
                            {
                                self.context = Some(d);

                                self.resource = None;
                            }
                        }
                        d @ Data::Word { .. } => {
                            self.word = Some(d);

                            self.resource = None;
                        }
                        Data::Resource { resource } => self.resource = Some(resource),
                        Data::Search {
                            hits,
                            offset,
                            has_more,
                        } => {
                            self.hits = hits;

                            self.search_offset = offset;

                            self.search_more = has_more;
                        }
                        Data::SavedAnnotation { .. } => {
                            self.note = None;

                            self.save_pending = false;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    self.opening_path = None;

                    self.error = Some(e);

                    self.save_pending = false;
                }
            }
        }
    }
    fn choose(&mut self, d: Destination) {
        if (d == Destination::Study || self.destination == Destination::Study)
            && let Some(s) = &self.state
        {
            self.scroll_to = Some(s.session.selected_ref);
        }
        self.destination = d;

        if d == Destination::Study
            && let Some(s) = &self.state
        {
            self.dispatch(Command::Context {
                ref_id: s.session.selected_ref,
            });
        }
    }
    fn select(&mut self, id: i64) {
        self.dispatch(Command::Select { ref_id: id });

        if self.destination != Destination::Study {
            self.destination = Destination::Read;
        }

        self.picker = false;
    }
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.receive();

        let mobile = ui.max_rect().width() < 760.0;

        ui.spacing_mut().interact_size.y = if mobile { 44.0 } else { 36.0 };

        if self.state.is_none() {
            self.welcome(ui);

            return;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            self.picker = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.picker = false;

            if !self.save_pending {
                self.note = None;
            }
        }
        if !mobile {
            egui::Panel::left("navigation")
                .exact_size(assets::DESKTOP_SIDEBAR_WIDTH)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(theme::color(assets::COLOR_SIDEBAR))
                        .inner_margin(16),
                )
                .show(ui, |ui| self.sidebar(ui));
        } else if self.note.is_none() && !self.picker {
            egui::Panel::bottom("mobile-navigation")
                .exact_size(84.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::color(assets::COLOR_SIDEBAR))
                        .inner_margin(8),
                )
                .show(ui, |ui| {
                    ui.columns(4, |cols| {
                        for (col, (d, role, title)) in cols.iter_mut().zip(destinations()) {
                            col.vertical_centered(|ui| {
                                if theme::mobile_nav(ui, role, title, self.destination == d) {
                                    self.choose(d);
                                }
                            });
                        }
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::desk()).inner_margin(0))
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(24, 12))
                    .show(ui, |ui| self.toolbar(ui, mobile));

                ui.separator();

                if let Some(error) = self.error.clone() {
                    egui::Frame::new().inner_margin(12).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(230, 175, 159), error);

                            if ui.small_button("Dismiss").clicked() {
                                self.error = None;
                            }
                        });
                    });
                }
                if self.pending > 0 {
                    ui.horizontal(|ui| {
                        ui.spinner();

                        caption(ui, "Working with your local library…");
                    });
                }
                if self.note.is_some() {
                    self.note_editor(ui);
                } else if self.picker {
                    self.reference_picker(ui, mobile);
                } else {
                    match self.destination {
                        Destination::Read => self.reader(ui, mobile),
                        Destination::Search => self.search_ui(ui),
                        Destination::Notes => self.notes_ui(ui),
                        Destination::Study if !mobile => self.reader(ui, false),
                        Destination::Study => self.study_ui(ui),
                        Destination::Preferences => self.preferences(ui),
                    }
                }
            });
    }
    fn welcome(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::desk()).inner_margin(24)).show(ui,|ui|{
            ui.add_space(64.0);
ui.label(scripture("A place for deeper reading.",32.0,true));

            ui.add_space(16.0);
ui.label("Open your Fontes reading or study library to begin. Everything you read and write stays on this device.");

            ui.add_space(24.0);
caption(ui,"LOCAL SQLITE LIBRARY");

            ui.add(egui::TextEdit::singleline(&mut self.library_path).desired_width(f32::INFINITY).hint_text("Path to library.sqlite"));

            if primary(ui,"Open library"){self.open();
}
            if self.pending>0{ui.spinner();
}
            if let Some(e)=&self.error{ui.colored_label(egui::Color32::LIGHT_RED,e);
}
            ui.add_space(24.0);
caption(ui,"No library yet? Build the reading profile with fontes-data, or verify a signed data release using the instructions in docs/app/README.md. Automatic downloads are coming later.");

        });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(16.0);

        ui.label(scripture("fontes", 32.0, true).color(accent()));

        caption(ui, "YOUR STUDY DESK");

        ui.add_space(32.0);

        ui.spacing_mut().item_spacing.y = 4.0;

        for (d, role, title) in destinations() {
            if nav(
                ui,
                role,
                title,
                self.destination == d,
                egui::vec2(ui.available_width(), 36.0),
            ) {
                self.choose(d);
            }
        }
        ui.add_space(16.0);

        ui.separator();

        ui.add_space(16.0);

        caption(ui, "SAVED PASSAGES");

        if let Some(s) = self.state.clone() {
            let saved = s
                .saved
                .iter()
                .filter(|r| r.kind == "favorite")
                .take(8)
                .collect::<Vec<_>>();

            if saved.is_empty() {
                caption(ui, "Bookmark a verse to keep it here.");
            }
            for r in saved {
                if ui
                    .add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new(&r.label).right_text(()).frame(false),
                    )
                    .clicked()
                {
                    self.select(r.ref_id);
                }
            }
        }
        ui.add_space(24.0);

        if nav(
            ui,
            "settings",
            "Reading preferences",
            self.destination == Destination::Preferences,
            egui::vec2(ui.available_width(), 36.0),
        ) {
            self.choose(Destination::Preferences);
        }
        ui.add_space(16.0);

        caption(ui, "ON THIS DEVICE");

        if let Some(s) = &self.state {
            caption(ui, format!("{} library · Offline", s.profile));
        }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui, mobile: bool) {
        let s = self.state.clone().expect("loaded");

        let compact = mobile && ui.available_width() < 312.0;

        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::image_and_text(
                    icon("book", 20.0, true),
                    s.reference_label(),
                ))
                .clicked()
            {
                self.picker = !self.picker;
            }
            let mut translation = s.session.translation.clone();

            egui::ComboBox::from_id_salt("translation")
                .width(if mobile { 65.0 } else { 110.0 })
                .selected_text(
                    s.translations
                        .iter()
                        .find(|t| t.id == translation)
                        .map(|t| t.abbr.as_str())
                        .unwrap_or("Bible"),
                )
                .show_ui(ui, |ui| {
                    for t in &s.translations {
                        ui.selectable_value(&mut translation, t.id.clone(), &t.abbr);
                    }
                });

            if translation != s.session.translation {
                self.dispatch(Command::Translation { id: translation });
            }
            let enabled = s
                .saved
                .iter()
                .any(|r| r.kind == "favorite" && r.ref_id == s.session.selected_ref);

            if !compact
                && ui
                    .add(egui::Button::image(icon("bookmark", 20.0, enabled)))
                    .on_hover_text("Save passage")
                    .clicked()
            {
                self.dispatch(Command::Favorite {
                    ref_id: s.session.selected_ref,
                    enabled: !enabled,
                });
            }
            if mobile
                && ui
                    .add(egui::Button::image(icon("settings", 20.0, false)))
                    .on_hover_text("Reading preferences")
                    .clicked()
            {
                self.choose(Destination::Preferences);
            }
        });
    }
    fn reader(&mut self, ui: &mut egui::Ui, mobile: bool) {
        let s = self.state.clone().expect("loaded");

        if !mobile && self.destination == Destination::Study {
            let panel_width = (ui.available_width() * 0.42).clamp(260.0, 420.0);
            egui::Panel::right("study-panel")
                .default_size(panel_width)
                .min_size(240.0)
                .max_size((ui.available_width() - 240.0).max(240.0))
                .resizable(true)
                .frame(
                    egui::Frame::new()
                        .fill(theme::color(assets::COLOR_SURFACE))
                        .inner_margin(0),
                )
                .show(ui, |ui| self.study_ui(ui));
        } else if !mobile && ui.available_width() > 1050.0 {
            egui::Panel::right("context-panel")
                .exact_size(320.0)
                .resizable(false)
                .frame(egui::Frame::new().fill(theme::color(assets::COLOR_SURFACE)).inner_margin(24))
                .show(ui, |ui| {
                    caption(ui, "ALONGSIDE YOUR READING");
                    ui.add_space(12.0);
                    ui.heading(s.reference_label());
                    ui.add_space(16.0);
                    ui.label("Select a word and right-click to study it, or explore this verse’s resources.");
                    ui.add_space(16.0);
                    if primary(ui, "Study this verse") {
                        self.open_study(s.session.selected_ref, StudyTab::Commentary);
                    }
                    if ui.add_sized([ui.available_width(), 48.0], egui::Button::image_and_text(icon("notes", 20.0, false), "Add a note")).clicked() {
                        self.new_note(s.session.selected_ref);
                    }
                    ui.add_space(24.0);
                    for note in s.annotations.iter().filter(|a| a.kind == AnnotationKind::Note && a.start_ref <= s.session.selected_ref && a.end_ref >= s.session.selected_ref) {
                        ui.label(note.content.as_deref().unwrap_or(""));
                        ui.add_space(12.0);
                    }
                });
        }
        egui::ScrollArea::vertical().id_salt("passage-scroll").show(ui,|ui|{
            let width=ui.available_width().min(if mobile{600.0}else{720.0});

            let inset=((ui.available_width()-width)/2.0).max(if mobile{24.0}else{32.0});

            egui::Frame::new().inner_margin(egui::Margin::symmetric(inset as i8,24)).show(ui,|ui|{
                ui.set_max_width(width-2.0*inset.min(32.0));

                let book=s.books.iter().find(|b|i64::from(b.id)==s.session.selected_ref/1_000_000).expect("valid book");

                caption(ui,format!("{} · AVAILABLE OFFLINE",book.name.to_uppercase()));

                ui.add_space(12.0);
ui.label(scripture(format!("{} {}",book.name,s.session.selected_ref%1_000_000/1000),if mobile{28.0}else{38.0},true));

                ui.add_space(8.0);
caption(ui,s.translations.iter().find(|t|t.id==s.session.translation).map(|t|t.name.clone()).unwrap_or_default());

                ui.add_space(28.0);

                if s.passage.is_empty(){ui.label("This chapter has no supplied text in this translation. Try another translation.");
}
                for verse in &s.passage{
                    let selected=verse.refs.contains(&s.session.selected_ref);

                    let highlight=s.annotations.iter().find(|a|a.color.is_some() && verse.refs.iter().any(|id|*id>=a.start_ref && *id<=a.end_ref));

                    let fill=highlight.and_then(|a|a.color.as_deref()).map(highlight_color).unwrap_or_else(||if selected{theme::tint()}else{egui::Color32::TRANSPARENT});

                    let row=egui::Frame::new().fill(fill).corner_radius(6).inner_margin(egui::Margin::symmetric(8,6)).show(ui,|ui|{
                        ui.horizontal_top(|ui|{
                            if s.session.preferences.verse_numbers{ui.add_sized([25.0,32.0],egui::Label::new(RichText::new(verse.refs.iter().map(|r|(r%1000).to_string()).collect::<Vec<_>>().join("–")).size(12.0).color(if selected{accent()}else{muted()})));
}
                            let text=if verse.text.is_empty(){"[No text supplied by this edition]"}else{&verse.text};

                            let mut job=egui::text::LayoutJob::default();

                            job.append(text,0.0,egui::TextFormat{font_id:egui::FontId::new(s.session.preferences.text_size,if s.session.preferences.scripture_serif{egui::FontFamily::Name("scripture".into())}else{egui::FontFamily::Proportional}),color:theme::color(assets::COLOR_INK),line_height:Some(s.session.preferences.text_size*s.session.preferences.line_spacing),..Default::default()});

                            self.verse_text(ui, verse, job, mobile);

                        });

                        for note in s.annotations.iter().filter(|a|a.kind==AnnotationKind::Note && verse.refs.iter().any(|id|*id>=a.start_ref && *id<=a.end_ref)){caption(ui,format!("Note · {}",note.content.as_deref().unwrap_or("")));
}
                    });

                    if self.scroll_to.is_some_and(|id|verse.refs.contains(&id)){row.response.scroll_to_me(Some(egui::Align::Center));
self.scroll_to=None;
}
                    ui.add_space(8.0);

                }
                ui.add_space(24.0);
ui.horizontal(|ui|{
                    if ui.add_sized([100.0,48.0],egui::Button::new("← Previous")).clicked(){self.dispatch(Command::Chapter{delta:-1});
}
                    if ui.add_sized([100.0,48.0],egui::Button::new("Next →")).clicked(){self.dispatch(Command::Chapter{delta:1});
}
                });

                ui.add_space(12.0);

                if primary(ui,"Add a note to selected verse"){self.new_note(s.session.selected_ref);
}
            });

        });
    }
    fn reference_picker(&mut self, ui: &mut egui::Ui, mobile: bool) {
        let s = self.state.clone().unwrap();

        if let Some(book) = self
            .picker_book
            .and_then(|id| s.books.iter().find(|b| b.id == id))
        {
            egui::Panel::bottom("picker-confirm")
                .exact_size(80.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::desk())
                        .inner_margin(egui::Margin::symmetric(24, 16)),
                )
                .show(ui, |ui| {
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::new().inner_margin(24).show(ui, |ui| {
                ui.spacing_mut().button_padding.x = 6.0;

                ui.horizontal(|ui| {
                    ui.heading("Go to a passage");

                    if ui.button("Close").clicked() {
                        self.picker = false;
                    }
                });

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.reference)
                            .hint_text("John 3:16 or Rom 1:3")
                            .desired_width((ui.available_width() - 70.0).max(100.0)),
                    );

                    if (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui.button("Go").clicked()
                    {
                        self.dispatch(Command::Navigate {
                            reference: self.reference.clone(),
                        });

                        self.picker = false;
                    }
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.testament, "OT".into(), "Old Testament");

                    ui.selectable_value(&mut self.testament, "NT".into(), "New Testament");
                });

                let columns = ((ui.available_width() + 5.0) / if mobile { 55.0 } else { 81.0 })
                    .floor()
                    .max(1.0)
                    .min(if mobile { 5.0 } else { 8.0 }) as usize;

                egui::Grid::new("book-grid")
                    .spacing([5.0, 5.0])
                    .show(ui, |ui| {
                        for (i, b) in s
                            .books
                            .iter()
                            .filter(|b| b.testament == self.testament)
                            .enumerate()
                        {
                            if ui
                                .add_sized(
                                    [if mobile { 50.0 } else { 76.0 }, 44.0],
                                    egui::Button::new(RichText::new(&b.osis).size(12.0))
                                        .truncate()
                                        .selected(
                                            self.picker_book.unwrap_or(
                                                (s.session.selected_ref / 1_000_000) as u16,
                                            ) == b.id,
                                        ),
                                )
                                .on_hover_text(&b.name)
                                .clicked()
                            {
                                self.picker_book = Some(b.id);

                                self.picker_chapter = 1;

                                self.picker_verse = 1;
                            }
                            if (i + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });

                if let Some(book) = self
                    .picker_book
                    .and_then(|id| s.books.iter().find(|b| b.id == id))
                {
                    ui.add_space(24.0);

                    caption(ui, format!("{} · CHAPTER", book.name));

                    number_grid(
                        ui,
                        "chapters",
                        book.verse_counts.len() as u16,
                        &mut self.picker_chapter,
                        if mobile { 6 } else { 10 },
                    );

                    let count = book.verse_counts[(self.picker_chapter - 1) as usize];

                    self.picker_verse = self.picker_verse.min(count);

                    ui.add_space(16.0);

                    caption(ui, "VERSE");

                    number_grid(
                        ui,
                        "verses",
                        count,
                        &mut self.picker_verse,
                        if mobile { 6 } else { 10 },
                    );
                }
                ui.add_space(24.0);

                caption(ui, "FAVORITE PASSAGES");

                for r in s.saved.iter().filter(|r| r.kind == "favorite").take(8) {
                    if ui
                        .add_sized([ui.available_width(), 44.0], egui::Button::new(&r.label))
                        .clicked()
                    {
                        self.select(r.ref_id);
                    }
                }
                ui.add_space(16.0);

                caption(ui, "RECENT PASSAGES");

                for r in s.saved.iter().filter(|r| r.kind == "recent").take(8) {
                    if ui
                        .add_sized([ui.available_width(), 44.0], egui::Button::new(&r.label))
                        .clicked()
                    {
                        self.select(r.ref_id);
                    }
                }
            });
        });
    }
    fn search_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui,|ui|{egui::Frame::new().inner_margin(24).show(ui,|ui|{
            ui.heading("Search your library");
caption(ui,"ON THIS DEVICE · KEYWORD SEARCH");
ui.add_space(16.0);

            let edit=ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search words, for example forgiveness").desired_width(f32::INFINITY));

            if primary(ui,"Search") || (edit.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter))){self.dispatch(Command::Search{query:self.query.clone(),limit:30,offset:0});
}
            ui.add_space(16.0);
caption(ui,"Results come from the selected translation and installed study articles. Local AI answers require a future model and retrieval layer.");

            for hit in self.hits.clone(){ui.add_space(16.0);
if ui.add_sized([ui.available_width(),44.0],egui::Button::new(RichText::new(&hit.title).color(accent())).frame(false)).clicked(){if let Some(id)=hit.ref_id{self.select(id);
}else{self.dispatch(Command::Resource{id:hit.id});
self.destination=Destination::Study;
}}ui.label(hit.excerpt);
ui.separator();
}
            ui.horizontal(|ui|{if self.search_offset>0 && ui.button("← Previous results").clicked(){self.dispatch(Command::Search{query:self.query.clone(),limit:30,offset:self.search_offset.saturating_sub(30)});
}
if self.search_more && ui.button("More results →").clicked(){self.dispatch(Command::Search{query:self.query.clone(),limit:30,offset:self.search_offset+30});
}});

        });
});
    }
    fn new_note(&mut self, reference: i64) {
        self.note = Some(NoteDraft {
            id: None,
            start: reference,
            end: reference,
            text: String::new(),
            color: None,
            kind: AnnotationKind::Note,
        });
    }
    fn note_editor(&mut self, ui: &mut egui::Ui) {
        let s = self.state.clone().unwrap();

        egui::Panel::bottom("note-actions")
            .exact_size(80.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::desk())
                    .inner_margin(egui::Margin::symmetric(24, 16)),
            )
            .show(ui, |ui| {
                ui.add_enabled_ui(!self.save_pending, |ui| {
                    ui.columns(2, |cols| {
                        let width = cols[0].available_width();

                        if cols[0]
                            .add_sized([width, 48.0], egui::Button::new("Cancel"))
                            .clicked()
                        {
                            self.note = None;
                        }
                        if primary(&mut cols[1], "Save")
                            && let Some(draft) = &self.note
                        {
                            let command = Command::SaveAnnotation {
                                id: draft.id.clone(),
                                start_ref: draft.start,
                                end_ref: draft.end,
                                kind: draft.kind.clone(),
                                color: draft.color.clone(),
                                content: Some(draft.text.clone()),
                            };

                            self.save_pending = true;

                            self.dispatch(command);
                        }
                    });
                });
            });

        if self.note.is_none() {
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::new().inner_margin(24).show(ui, |ui| {
                let draft = self.note.as_mut().unwrap();

                ui.heading(if draft.kind == AnnotationKind::Note {
                    "Your note"
                } else {
                    "Your highlight"
                });

                caption(
                    ui,
                    format!(
                        "ATTACHED TO {} · PRIVATE ON THIS DEVICE",
                        label(&s.books, draft.start)
                    ),
                );

                ui.add_space(16.0);

                ui.horizontal_wrapped(|ui| {
                    caption(ui, "COLOR");

                    if ui.selectable_label(draft.color.is_none(), "None").clicked() {
                        draft.color = None;
                    }
                    for c in ["amber", "sage", "blue", "rose"] {
                        if ui
                            .add_sized(
                                [48.0, 44.0],
                                egui::Button::new(c)
                                    .fill(highlight_color(c))
                                    .selected(draft.color.as_deref() == Some(c)),
                            )
                            .clicked()
                        {
                            draft.color = Some(c.into());
                        }
                    }
                });

                ui.add_space(16.0);

                if draft.kind == AnnotationKind::Note {
                    ui.add(
                        egui::TextEdit::multiline(&mut draft.text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(12)
                            .hint_text("What stands out in this passage?"),
                    );
                }
                ui.add_space(16.0);

                caption(ui, "Follows the passage across translations.");
            });
        });
    }
    fn notes_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.state.clone().unwrap();

        egui::ScrollArea::vertical().show(ui,|ui|{egui::Frame::new().inner_margin(24).show(ui,|ui|{
            ui.heading("Notes & highlights");
caption(ui,"PRIVATE ON THIS DEVICE");
ui.add_space(16.0);

            if primary(ui,"Add a note to selected verse"){self.new_note(s.session.selected_ref);
}
            if ui.add_sized([ui.available_width(),48.0],egui::Button::new("Highlight selected verse")).clicked(){self.note=Some(NoteDraft{id:None,start:s.session.selected_ref,end:s.session.selected_ref,text:String::new(),color:Some("amber".into()),kind:AnnotationKind::Highlight});
}
            let favorite=s.saved.iter().any(|r|r.kind=="favorite" && r.ref_id==s.session.selected_ref);

            if ui.add_sized([ui.available_width(),48.0],egui::Button::new(if favorite{"Remove passage bookmark"}else{"Bookmark selected passage"})).clicked(){self.dispatch(Command::Favorite{ref_id:s.session.selected_ref,enabled:!favorite});
}
            if s.annotations.is_empty(){ui.add_space(24.0);
ui.label("Your observations belong alongside the passage. Select a verse and add your first note.");
}
            for note in &s.annotations{ui.add_space(24.0);
ui.colored_label(accent(),label(&s.books,note.start_ref));
ui.label(note.content.as_deref().unwrap_or("Highlight"));
if let Some(c)=&note.color{caption(ui,format!("Color · {c}"));
}
                ui.horizontal_wrapped(|ui|{if ui.button("Read passage").clicked(){self.select(note.start_ref);
}
if ui.button("Edit").clicked(){self.note=Some(NoteDraft{id:Some(note.id.clone()),start:note.start_ref,end:note.end_ref,text:note.content.clone().unwrap_or_default(),color:note.color.clone(),kind:note.kind.clone()});
}});

                // Deletion deliberately uses a confirmation to protect user-authored text.
                ui.menu_button("Delete…",|ui|{if ui.button("Permanently delete this annotation").clicked(){self.dispatch(Command::DeleteAnnotation{id:note.id.clone()});
ui.close();
}});
ui.separator();

            }
        });
});
    }
    fn preferences(&mut self, ui: &mut egui::Ui) {
        let s = self.state.clone().unwrap();

        let mut p = s.session.preferences.clone();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::new().inner_margin(24).show(ui, |ui| {
                caption(ui, "MAKE ROOM FOR READING");

                ui.label(scripture("Your reading rhythm", 28.0, true));

                ui.add_space(24.0);

                ui.label("Scripture typeface");

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut p.scripture_serif, true, "Lora");

                    ui.selectable_value(&mut p.scripture_serif, false, "IBM Plex Sans");
                });

                ui.add_space(16.0);

                ui.add(egui::Slider::new(&mut p.text_size, 16.0..=36.0).text("Text size"));

                ui.add_space(16.0);

                ui.add(egui::Slider::new(&mut p.line_spacing, 1.2..=2.2).text("Line spacing"));

                ui.add_space(16.0);

                ui.checkbox(&mut p.verse_numbers, "Verse numbers");

                ui.add_space(24.0);

                caption(ui, "READING PREVIEW");

                if let Some(v) = s
                    .passage
                    .iter()
                    .find(|v| v.refs.contains(&s.session.selected_ref))
                {
                    ui.label(scripture(&v.text, p.text_size, p.scripture_serif));
                }
                ui.add_space(24.0);

                if primary(ui, "Back to reading") {
                    self.destination = Destination::Read;
                }
                ui.add_space(24.0);

                caption(ui, "LOCAL LIBRARY");

                ui.add(
                    egui::TextEdit::singleline(&mut self.library_path).desired_width(f32::INFINITY),
                );

                if ui
                    .add_sized(
                        [ui.available_width(), 48.0],
                        egui::Button::new("Open library"),
                    )
                    .clicked()
                {
                    self.open();
                }
            });
        });

        if p.text_size != s.session.preferences.text_size
            || p.line_spacing != s.session.preferences.line_spacing
            || p.verse_numbers != s.session.preferences.verse_numbers
            || p.scripture_serif != s.session.preferences.scripture_serif
        {
            self.dispatch(Command::Preferences { preferences: p });
        }
    }
}
impl eframe::App for FontesApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive();

        if ctx.input(|i| i.viewport().close_requested()) && self.pending > 0 {
            self.close_after_work = true;

            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.close_after_work && self.pending == 0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
fn destinations() -> [(Destination, &'static str, &'static str); 4] {
    [
        (Destination::Read, "book", "Read"),
        (Destination::Search, "search", "Search"),
        (Destination::Notes, "notes", "Notes"),
        (Destination::Study, "tree", "Study"),
    ]
}
fn number_grid(ui: &mut egui::Ui, id: &str, count: u16, selected: &mut u16, columns: u16) {
    let columns = columns.min(((ui.available_width() + 5.0) / 49.0).floor().max(1.0) as u16);

    egui::Grid::new(id).spacing([5.0, 5.0]).show(ui, |ui| {
        for n in 1..=count {
            if ui
                .add_sized(
                    [44.0, 44.0],
                    egui::Button::new(n.to_string()).selected(*selected == n),
                )
                .clicked()
            {
                *selected = n;
            }
            if n % columns == 0 {
                ui.end_row();
            }
        }
    });
}
fn highlight_color(name: &str) -> egui::Color32 {
    match name {
        "sage" => egui::Color32::from_rgb(48, 65, 45),
        "blue" => egui::Color32::from_rgb(43, 59, 66),
        "rose" => egui::Color32::from_rgb(65, 47, 47),
        _ => theme::tint(),
    }
}
