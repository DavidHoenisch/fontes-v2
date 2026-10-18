//! Exact-text selection and user-owned markup, separate from reading focus.
use super::*;
use egui::{Color32, Stroke};
use std::sync::Arc;

pub(super) struct HighlightDraft {
    pub id: Option<String>,
    pub start: i64,
    pub end: i64,
    pub selection: Option<TextSelection>,
    pub style: HighlightStyle,
    pub selecting: bool,
}
pub(super) struct TextLayout {
    pub verse: Verse,
    pub galley: Arc<egui::Galley>,
    pub pos: egui::Pos2,
    pub rect: egui::Rect,
}
const COLORS: [(MarkColor, &str); 9] = [
    (MarkColor::Yellow, "Yellow"),
    (MarkColor::Blue, "Blue"),
    (MarkColor::Green, "Green"),
    (MarkColor::Orange, "Orange"),
    (MarkColor::Pink, "Pink"),
    (MarkColor::Purple, "Purple"),
    (MarkColor::Red, "Red"),
    (MarkColor::Brown, "Brown"),
    (MarkColor::Gray, "Gray"),
];
fn color(c: MarkColor) -> Color32 {
    match c {
        MarkColor::Yellow => Color32::from_rgb(225, 177, 34),
        MarkColor::Blue => Color32::from_rgb(52, 132, 215),
        MarkColor::Green => Color32::from_rgb(66, 155, 70),
        MarkColor::Orange => Color32::from_rgb(218, 126, 32),
        MarkColor::Pink => Color32::from_rgb(218, 85, 154),
        MarkColor::Purple => Color32::from_rgb(152, 101, 211),
        MarkColor::Red => Color32::from_rgb(213, 69, 66),
        MarkColor::Brown => Color32::from_rgb(158, 113, 71),
        MarkColor::Gray => Color32::from_rgb(139, 143, 151),
    }
}
fn default_style() -> HighlightStyle {
    HighlightStyle {
        kind: MarkKind::Highlighter,
        color: MarkColor::Yellow,
        symbol: None,
    }
}
fn legacy_style(c: Option<&str>) -> HighlightStyle {
    HighlightStyle {
        color: match c {
            Some("blue") => MarkColor::Blue,
            Some("sage") => MarkColor::Green,
            Some("rose") => MarkColor::Pink,
            _ => MarkColor::Yellow,
        },
        ..default_style()
    }
}

pub(super) fn marks(
    s: &Workspace,
    verse: &Verse,
) -> Vec<(String, std::ops::Range<usize>, HighlightStyle)> {
    s.annotations
        .iter()
        .filter(|a| {
            a.kind == AnnotationKind::Highlight
                && verse
                    .refs
                    .iter()
                    .any(|id| *id >= a.start_ref && *id <= a.end_ref)
        })
        .filter_map(|a| {
            let rich = s.highlights.iter().find(|h| h.annotation_id == a.id);
            let range = if let Some(selection) = rich.and_then(|h| h.selection.as_ref()) {
                if selection.translation_id != s.session.translation {
                    return None;
                }
                let span = selection
                    .spans
                    .iter()
                    .find(|span| span.verse_id == verse.id)?;
                if span.start >= span.end || span.end > verse.text.chars().count() {
                    return None;
                }
                if verse
                    .text
                    .chars()
                    .skip(span.start)
                    .take(span.end.saturating_sub(span.start))
                    .collect::<String>()
                    != span.quote
                {
                    return None;
                }
                span.start..span.end
            } else {
                0..verse.text.chars().count()
            };
            Some((
                a.id.clone(),
                range,
                rich.map(|h| h.style.clone())
                    .unwrap_or_else(|| legacy_style(a.color.as_deref())),
            ))
        })
        .collect()
}

pub(super) fn styled_job(
    job: &mut egui::text::LayoutJob,
    s: &Workspace,
    verse: &Verse,
    draft: Option<&HighlightDraft>,
) {
    let mut ranges = marks(s, verse)
        .into_iter()
        .map(|(_, r, style)| (r, style))
        .collect::<Vec<_>>();
    if let Some(draft) = draft.filter(|d| d.selecting)
        && let Some(span) = draft
            .selection
            .as_ref()
            .and_then(|s| s.spans.iter().find(|s| s.verse_id == verse.id))
    {
        ranges.insert(0, (span.start..span.end, default_style()));
    }
    if ranges.is_empty() {
        return;
    }
    let text = job.text.clone();
    let original = job.sections.clone();
    let bytes = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .collect::<Vec<_>>();
    let mut bounds = vec![0, text.len()];
    for (r, _) in &ranges {
        if r.end < bytes.len() {
            bounds.extend([bytes[r.start], bytes[r.end]]);
        }
    }
    for section in &original {
        bounds.extend([section.byte_range.start.0, section.byte_range.end.0]);
    }
    bounds.sort_unstable();
    bounds.dedup();
    job.text.clear();
    job.sections.clear();
    for pair in bounds.windows(2) {
        let mut fmt = original
            .iter()
            .find(|s| s.byte_range.start.0 <= pair[0] && pair[0] < s.byte_range.end.0)
            .map(|s| s.format.clone())
            .unwrap_or_default();
        for (range, style) in ranges.iter().rev() {
            if range.end >= bytes.len()
                || pair[0] < bytes[range.start]
                || pair[0] >= bytes[range.end]
            {
                continue;
            }
            let c = color(style.color);
            match style.kind {
                MarkKind::Highlighter => fmt.background = c.gamma_multiply(0.30),
                MarkKind::Symbol => fmt.background = c.gamma_multiply(0.16),
                MarkKind::Foreground => {
                    fmt.color = if theme::Palette::for_theme(s.session.preferences.theme).dark {
                        c
                    } else {
                        readable_color(style.color)
                    }
                }
                MarkKind::Underline | MarkKind::DoubleUnderline | MarkKind::TripleUnderline => {
                    fmt.underline = Stroke::new(1.5, c)
                }
                MarkKind::Strikethrough => fmt.strikethrough = Stroke::new(1.4, c),
                MarkKind::Italic => fmt.italics = true,
                MarkKind::Bold => {}
                MarkKind::Box
                | MarkKind::Brackets
                | MarkKind::MarginBar
                | MarkKind::WavyUnderline => {}
            }
        }
        job.append(&text[pair[0]..pair[1]], 0.0, fmt);
    }
}

impl FontesApp {
    pub(super) fn begin_highlight(&mut self, verse: &Verse, existing: Option<String>) {
        let s = self.state.as_ref().unwrap();
        let mut draft = HighlightDraft {
            id: existing.clone(),
            start: verse.refs[0],
            end: *verse.refs.last().unwrap(),
            selection: None,
            style: default_style(),
            selecting: false,
        };
        if let Some(id) = existing {
            if let Some(a) = s.annotations.iter().find(|a| a.id == id) {
                draft.start = a.start_ref;
                draft.end = a.end_ref;
                draft.style = legacy_style(a.color.as_deref());
            }
            if let Some(h) = s.highlights.iter().find(|h| h.annotation_id == id) {
                draft.style = h.style.clone();
                draft.selection = h.selection.clone();
            }
        } else if let Some(selected) = self
            .selected_text
            .as_ref()
            .filter(|t| t.verse_id == verse.id)
        {
            draft.selection = Some(TextSelection {
                translation_id: s.session.translation.clone(),
                spans: vec![TextSpan {
                    verse_id: verse.id.clone(),
                    start: selected.range.start,
                    end: selected.range.end,
                    quote: selected.text.clone(),
                }],
            });
        }
        self.selected_text = None;
        self.highlight_draft = Some(draft);
        self.verse_actions = None;
    }
    pub(super) fn draw_highlight_handles(
        &mut self,
        ui: &mut egui::Ui,
        verse: &Verse,
        galley: &Arc<egui::Galley>,
        pos: egui::Pos2,
        rect: egui::Rect,
    ) {
        if !self.highlight_draft.as_ref().is_some_and(|d| d.selecting) {
            return;
        }
        self.highlight_layouts.push(TextLayout {
            verse: verse.clone(),
            galley: galley.clone(),
            pos,
            rect,
        });
        let Some(selection) = self
            .highlight_draft
            .as_ref()
            .and_then(|d| d.selection.as_ref())
        else {
            return;
        };
        for (start, span) in [
            (true, selection.spans.first()),
            (false, selection.spans.last()),
        ] {
            let Some(span) = span.filter(|span| span.verse_id == verse.id) else {
                continue;
            };
            let cursor = galley.pos_from_cursor(egui::text::CCursor::new(if start {
                span.start
            } else {
                span.end
            }));
            let point = pos + cursor.left_bottom().to_vec2();
            let ink = theme::palette(ui.ctx()).accent;
            ui.painter().line_segment(
                [point, point - egui::vec2(0.0, cursor.height())],
                Stroke::new(2.0, ink),
            );
            ui.painter()
                .circle_filled(point + egui::vec2(0.0, 5.0), 6.0, ink);
            let response = ui.interact(
                egui::Rect::from_center_size(point, egui::vec2(44.0, 44.0)),
                egui::Id::new(("highlight-handle", start)),
                egui::Sense::drag(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Slider,
                    true,
                    if start {
                        "Selection start"
                    } else {
                        "Selection end"
                    },
                )
            });
            if response.dragged()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                self.highlight_drag = Some((start, pointer));
                let clip = ui.clip_rect();
                let delta = if pointer.y < clip.top() + 36.0 {
                    8.0
                } else if pointer.y > clip.bottom() - 80.0 {
                    -8.0
                } else {
                    0.0
                };
                if delta != 0.0 {
                    ui.scroll_with_delta(egui::vec2(0.0, delta));
                    ui.ctx().request_repaint();
                }
            }
        }
    }
    pub(super) fn update_highlight_drag(&mut self) {
        let Some((start, pointer)) = self.highlight_drag.take() else {
            return;
        };
        let Some(layout) = self.highlight_layouts.iter().min_by(|a, b| {
            a.rect
                .distance_to_pos(pointer)
                .total_cmp(&b.rect.distance_to_pos(pointer))
        }) else {
            return;
        };
        let cursor = layout
            .galley
            .cursor_from_pos(pointer - layout.pos)
            .index
            .0
            .min(layout.verse.text.chars().count());
        let s = self.state.as_ref().unwrap();
        let passage = &s.passage;
        let Some(draft) = self.highlight_draft.as_mut() else {
            return;
        };
        let Some(selection) = &draft.selection else {
            return;
        };
        let Some(first) = selection.spans.first() else {
            return;
        };
        let last = selection.spans.last().unwrap();
        let Some(a) = passage.iter().position(|v| v.id == first.verse_id) else {
            return;
        };
        let Some(b) = passage.iter().position(|v| v.id == last.verse_id) else {
            return;
        };
        let Some(target) = passage.iter().position(|v| v.id == layout.verse.id) else {
            return;
        };
        let (mut from, mut to) = if start {
            ((target, cursor), (b, last.end))
        } else {
            ((a, first.start), (target, cursor))
        };
        if from > to {
            std::mem::swap(&mut from, &mut to);
        }
        if from == to {
            return;
        }
        let spans = passage[from.0..=to.0]
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let left = if i == 0 { from.1 } else { 0 };
                let right = if from.0 + i == to.0 {
                    to.1
                } else {
                    v.text.chars().count()
                };
                (left < right).then(|| TextSpan {
                    verse_id: v.id.clone(),
                    start: left,
                    end: right,
                    quote: v.text.chars().skip(left).take(right - left).collect(),
                })
            })
            .collect::<Vec<_>>();
        if spans.is_empty() {
            return;
        }
        draft.start = passage[from.0].refs[0];
        draft.end = *passage[to.0].refs.last().unwrap();
        draft.selection = Some(TextSelection {
            translation_id: s.session.translation.clone(),
            spans,
        });
    }
    pub(super) fn highlight_ui(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.highlight_draft.take() else {
            return;
        };
        let mut close = false;
        if draft.selecting {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            egui::Area::new(egui::Id::new("selection-tools"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -72.0])
                .show(ctx, |ui| {
                    theme::card(ctx).show(ui, |ui| {
                        ui.set_width((ctx.content_rect().width() - 64.0).min(380.0));
                        if let Some(sel) = &draft.selection {
                            ui.add(
                                egui::Label::new(
                                    sel.spans
                                        .iter()
                                        .map(|s| s.quote.as_str())
                                        .collect::<Vec<_>>()
                                        .join(" "),
                                )
                                .truncate(),
                            );
                        }
                        let button_width = (ui.available_width() - 8.0) / 2.0;
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [button_width, 48.0],
                                    egui::Button::new(
                                        RichText::new("Add note")
                                            .color(theme::palette(ctx).on_accent),
                                    )
                                    .fill(theme::palette(ctx).accent),
                                )
                                .clicked()
                            {
                                self.note_from_selection(
                                    draft.start,
                                    draft.end,
                                    draft.selection.clone(),
                                );
                                close = true;
                            }
                            if ui
                                .add_sized([button_width, 48.0], egui::Button::new("Highlight"))
                                .clicked()
                            {
                                draft.selecting = false;
                            }
                        });
                        if ui
                            .add_sized(
                                [ui.available_width(), 44.0],
                                egui::Button::new("Done selecting").frame(false),
                            )
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
        } else {
            let bounds = ctx.content_rect();
            let modal = egui::Modal::new(egui::Id::new("highlight-styles"))
                .area(
                    egui::Modal::default_area(egui::Id::new("highlight-styles"))
                        .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO),
                )
                .frame(
                    egui::Frame::new()
                        .fill(theme::palette(ctx).surface)
                        .corner_radius(24)
                        .inner_margin(20),
                )
                .show(ctx, |ui| {
                    let height = (bounds.height() * 0.88).min(720.0);
                    ui.set_width((bounds.width() - 40.0).min(440.0));
                    ui.set_height(height - 40.0);
                    ui.horizontal(|ui| {
                        theme::editorial(ui, "Highlight style", 25.0, theme::palette(ctx).ink);
                        if theme::round_close(ui, "Close highlights").clicked() {
                            close = true;
                        }
                    });
                    ui.add_space(8.0);
                    let quote = draft
                        .selection
                        .as_ref()
                        .map(|s| {
                            s.spans
                                .iter()
                                .map(|s| s.quote.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_else(|| {
                            format!(
                                "{} – {}",
                                label(&self.state.as_ref().unwrap().books, draft.start),
                                label(&self.state.as_ref().unwrap().books, draft.end)
                            )
                        });
                    ui.add(egui::Label::new(RichText::new(&quote).size(14.0)).truncate());
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_sized([144.0, 44.0], egui::Button::new("Adjust selection"))
                            .clicked()
                        {
                            if draft.selection.is_none()
                                && let Some(s) = &self.state
                            {
                                draft.selection = Some(TextSelection {
                                    translation_id: s.session.translation.clone(),
                                    spans: s
                                        .passage
                                        .iter()
                                        .filter(|v| {
                                            v.refs
                                                .iter()
                                                .any(|id| *id >= draft.start && *id <= draft.end)
                                        })
                                        .map(|v| TextSpan {
                                            verse_id: v.id.clone(),
                                            start: 0,
                                            end: v.text.chars().count(),
                                            quote: v.text.clone(),
                                        })
                                        .collect(),
                                });
                            }
                            draft.selecting = true;
                            ctx.request_repaint();
                            self.destination = Destination::Read;
                        }
                        if ui.button("Whole verse").clicked() {
                            draft.selection = None;
                        }
                    });
                    let list_height = (ui.max_rect().bottom()
                        - ui.cursor().min.y
                        - if draft.id.is_some() { 226.0 } else { 174.0 })
                    .max(60.0);
                    egui::ScrollArea::vertical()
                        .id_salt("highlight-style-list")
                        .max_height(list_height)
                        .show(ui, |ui| {
                            let mut recent = Vec::new();
                            if let Some(s) = &self.state {
                                for a in &s.annotations {
                                    if let Some(mark) =
                                        s.highlights.iter().find(|h| h.annotation_id == a.id)
                                        && !recent.contains(&mark.style)
                                    {
                                        recent.push(mark.style.clone());
                                    }
                                    if recent.len() == 5 {
                                        break;
                                    }
                                }
                            }
                            if !recent.is_empty() {
                                ui.collapsing("Recent styles", |ui| {
                                    for style in recent {
                                        if ui
                                            .add_sized(
                                                [ui.available_width(), 44.0],
                                                egui::Button::new(format!(
                                                    "{} · {:?}",
                                                    style_name(style.kind),
                                                    style.color
                                                )),
                                            )
                                            .clicked()
                                        {
                                            draft.style = style;
                                        }
                                    }
                                });
                            }
                            ui.label("Color");
                            ui.horizontal_wrapped(|ui| {
                                for (c, name) in COLORS {
                                    if ui
                                        .add_sized(
                                            [70.0, 44.0],
                                            egui::Button::new(name)
                                                .fill(color(c).gamma_multiply(0.25))
                                                .selected(draft.style.color == c),
                                        )
                                        .clicked()
                                    {
                                        draft.style.color = c;
                                    }
                                }
                            });
                            ui.add_space(12.0);
                            ui.label("Style");
                            for (kind, name) in [
                                (MarkKind::Highlighter, "Highlighter"),
                                (MarkKind::Foreground, "Text color"),
                                (MarkKind::Underline, "Underline"),
                                (MarkKind::DoubleUnderline, "Double underline"),
                                (MarkKind::TripleUnderline, "Triple underline"),
                                (MarkKind::WavyUnderline, "Wavy underline"),
                                (MarkKind::Box, "Box"),
                                (MarkKind::Brackets, "Brackets"),
                                (MarkKind::Bold, "Bold"),
                                (MarkKind::Italic, "Italic"),
                                (MarkKind::Strikethrough, "Strikethrough"),
                                (MarkKind::MarginBar, "Margin bar"),
                            ] {
                                let example = HighlightStyle {
                                    kind,
                                    color: draft.style.color,
                                    symbol: None,
                                };
                                if style_choice(ui, &example, name, draft.style.kind == kind) {
                                    draft.style.kind = kind;
                                    draft.style.symbol = None;
                                }
                            }
                            ui.collapsing("Study symbols", |ui| {
                                for symbol in [
                                    "God",
                                    "God’s love",
                                    "Jesus Christ",
                                    "Holy Spirit",
                                    "Gospel",
                                    "Grace",
                                    "Faith",
                                    "Covenant",
                                    "Prayer",
                                    "Law",
                                    "Kingdom",
                                    "Promise",
                                    "Sin",
                                    "Repentance",
                                    "Redemption",
                                    "Eternal life",
                                    "Question",
                                    "Important",
                                    "Bless",
                                    "Curse",
                                    "Coming of the Lord",
                                    "Day of the Lord",
                                    "Atonement",
                                    "Believe",
                                    "Blood",
                                    "Cry",
                                ] {
                                    let example = HighlightStyle {
                                        kind: MarkKind::Symbol,
                                        color: draft.style.color,
                                        symbol: Some(symbol.into()),
                                    };
                                    if style_choice(
                                        ui,
                                        &example,
                                        symbol,
                                        draft.style.kind == MarkKind::Symbol
                                            && draft.style.symbol.as_deref() == Some(symbol),
                                    ) {
                                        draft.style.kind = MarkKind::Symbol;
                                        draft.style.symbol = Some(symbol.into());
                                    }
                                }
                            });
                        });
                    ui.add_space(8.0);
                    let selected_name = if draft.style.kind == MarkKind::Symbol {
                        draft.style.symbol.as_deref().unwrap_or("Study pattern")
                    } else {
                        style_name(draft.style.kind)
                    };
                    ui.label(
                        RichText::new(format!(
                            "Selected: {selected_name} · {:?}",
                            draft.style.color
                        ))
                        .size(12.0)
                        .color(accent(ctx)),
                    );
                    let preview_quote = draft
                        .selection
                        .as_ref()
                        .map(|s| {
                            s.spans
                                .iter()
                                .map(|s| s.quote.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_else(|| {
                            self.state
                                .as_ref()
                                .unwrap()
                                .passage
                                .iter()
                                .filter(|v| {
                                    v.refs.iter().any(|r| *r >= draft.start && *r <= draft.end)
                                })
                                .map(|v| v.text.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        });
                    live_preview(ui, &preview_quote, &draft.style);
                    ui.add_space(8.0);
                    if let Some(id) = &draft.id
                        && ui
                            .add_sized(
                                [ui.available_width(), 44.0],
                                egui::Button::new("Remove highlight"),
                            )
                            .clicked()
                    {
                        self.dispatch(Command::DeleteAnnotation { id: id.clone() });
                        close = true;
                    }
                    if primary(ui, "Apply highlight") {
                        self.dispatch(Command::SaveHighlight {
                            id: draft.id.clone(),
                            start_ref: draft.start,
                            end_ref: draft.end,
                            style: draft.style.clone(),
                            selection: draft.selection.clone(),
                        });
                        close = true;
                    }
                });
            if modal.should_close() {
                close = true;
            }
        }
        if !close {
            self.highlight_draft = Some(draft);
        }
    }
}

fn segment_rects(
    galley: &egui::Galley,
    range: std::ops::Range<usize>,
    pos: egui::Pos2,
) -> Vec<egui::Rect> {
    let mut rects: Vec<egui::Rect> = Vec::new();
    for i in range {
        let a = galley.pos_from_cursor(egui::text::CCursor::new(i));
        let b = galley.pos_from_cursor(egui::text::CCursor::new(i + 1));
        let right = if (a.top() - b.top()).abs() < 1.0 {
            b.left().max(a.left() + 1.0)
        } else {
            galley
                .rows
                .iter()
                .find(|r| (r.rect().top() - a.top()).abs() < 2.0)
                .map_or(a.left() + 8.0, |r| r.rect().right())
        };
        let rect = egui::Rect::from_min_max(a.left_top(), egui::pos2(right, a.bottom()))
            .translate(pos.to_vec2());
        if let Some(last) = rects
            .last_mut()
            .filter(|r| (r.top() - rect.top()).abs() < 1.0)
        {
            *last = last.union(rect);
        } else {
            rects.push(rect);
        }
    }
    rects
}
pub(super) fn decorations(
    ui: &egui::Ui,
    s: &Workspace,
    verse: &Verse,
    galley: &Arc<egui::Galley>,
    pos: egui::Pos2,
) {
    for (_, range, style) in marks(s, verse) {
        decorate_style(ui, &style, segment_rects(galley, range, pos), galley, pos);
    }
}
fn decorate_style(
    ui: &egui::Ui,
    style: &HighlightStyle,
    rects: Vec<egui::Rect>,
    galley: &Arc<egui::Galley>,
    pos: egui::Pos2,
) {
    let c = color(style.color);
    let stroke = Stroke::new(1.5, c);
    for (index, mut rect) in rects.iter().copied().enumerate() {
        if style.kind == MarkKind::Symbol
            && let Some(next) = rects.get(index + 1)
        {
            rect.max.y = rect.max.y.min(next.min.y - 3.0).max(rect.min.y + 4.0);
        }
        let painter = ui.painter();
        match style.kind {
            MarkKind::DoubleUnderline | MarkKind::TripleUnderline => {
                for n in 1..if style.kind == MarkKind::TripleUnderline {
                    3
                } else {
                    2
                } {
                    painter.hline(rect.x_range(), rect.bottom() + n as f32 * 2.5, stroke);
                }
            }
            MarkKind::WavyUnderline => {
                let n = (rect.width() / 3.0).ceil() as usize;
                for i in 0..n {
                    let x = rect.left() + i as f32 * 3.0;
                    painter.line_segment(
                        [
                            egui::pos2(x, rect.bottom() + if i % 2 == 0 { 1.0 } else { 3.0 }),
                            egui::pos2(
                                (x + 3.0).min(rect.right()),
                                rect.bottom() + if i % 2 == 0 { 3.0 } else { 1.0 },
                            ),
                        ],
                        stroke,
                    );
                }
            }
            MarkKind::Box => {
                painter.rect_stroke(rect.expand(1.0), 2, stroke, egui::StrokeKind::Inside);
            }
            MarkKind::Brackets => {
                for (x, dx) in [(rect.left(), 5.0), (rect.right(), -5.0)] {
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        stroke,
                    );
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x + dx, rect.top())],
                        stroke,
                    );
                    painter.line_segment(
                        [
                            egui::pos2(x, rect.bottom()),
                            egui::pos2(x + dx, rect.bottom()),
                        ],
                        stroke,
                    );
                }
            }
            MarkKind::MarginBar => {
                painter.line_segment(
                    [
                        egui::pos2(rect.left() - 6.0, rect.top()),
                        egui::pos2(rect.left() - 6.0, rect.bottom()),
                    ],
                    Stroke::new(3.0, c),
                );
            }
            MarkKind::Bold => {
                ui.painter()
                    .with_clip_rect(rect.intersect(ui.clip_rect()))
                    .galley(
                        pos + egui::vec2(0.4, 0.0),
                        galley.clone(),
                        theme::palette(ui.ctx()).ink,
                    );
            }
            MarkKind::Symbol => symbol_span(ui, rect, style),
            _ => {}
        }
    }
}

fn style_choice(ui: &mut egui::Ui, style: &HighlightStyle, title: &str, selected: bool) -> bool {
    let response = ui.add_sized(
        [ui.available_width(), 44.0],
        egui::Button::new("").selected(selected),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    if selected {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            8,
            Stroke::new(2.0, accent(ui.ctx())),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            egui::pos2(response.rect.right() - 18.0, response.rect.center().y),
            egui::Align2::CENTER_CENTER,
            "✓",
            egui::FontId::proportional(18.0),
            accent(ui.ctx()),
        );
    }
    let r = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 24.0, response.rect.center().y),
        egui::vec2(30.0, 24.0),
    );
    let c = color(style.color);
    let painter = ui.painter();
    let ink = theme::palette(ui.ctx()).ink;
    if style.kind == MarkKind::Highlighter {
        painter.rect_filled(r, 3, c.gamma_multiply(0.35));
    }
    if style.kind == MarkKind::Box {
        painter.rect_stroke(r, 2, Stroke::new(1.5, c), egui::StrokeKind::Inside);
    }
    if style.kind == MarkKind::MarginBar {
        painter.vline(r.left() - 3.0, r.y_range(), Stroke::new(3.0, c));
    }
    if style.kind == MarkKind::Brackets {
        painter.text(
            r.left_center(),
            egui::Align2::RIGHT_CENTER,
            "[",
            egui::FontId::proportional(22.0),
            c,
        );
        painter.text(
            r.right_center(),
            egui::Align2::LEFT_CENTER,
            "]",
            egui::FontId::proportional(22.0),
            c,
        );
    }
    if style.kind == MarkKind::Symbol {
        painter.rect_filled(r, 2, c.gamma_multiply(0.16));
        symbol_span(ui, r, style);
    }
    let sample = "Aa";
    painter.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        sample,
        egui::FontId::proportional(18.0),
        if style.kind == MarkKind::Foreground {
            c
        } else {
            ink
        },
    );
    if style.kind == MarkKind::Bold {
        painter.text(
            r.center() + egui::vec2(0.5, 0.0),
            egui::Align2::CENTER_CENTER,
            "Aa",
            egui::FontId::proportional(18.0),
            ink,
        );
    }
    if style.kind == MarkKind::Strikethrough {
        painter.hline(r.x_range(), r.center().y, Stroke::new(1.5, c));
    }
    let count = match style.kind {
        MarkKind::Underline => 1,
        MarkKind::DoubleUnderline => 2,
        MarkKind::TripleUnderline => 3,
        _ => 0,
    };
    for n in 0..count {
        painter.hline(
            r.x_range(),
            r.bottom() - 2.0 + n as f32 * 2.0,
            Stroke::new(1.3, c),
        );
    }
    if style.kind == MarkKind::WavyUnderline {
        for n in 0..10 {
            let x = r.left() + n as f32 * 3.0;
            painter.line_segment(
                [
                    egui::pos2(x, r.bottom() + if n % 2 == 0 { 0.0 } else { 2.0 }),
                    egui::pos2(x + 3.0, r.bottom() + if n % 2 == 0 { 2.0 } else { 0.0 }),
                ],
                Stroke::new(1.3, c),
            );
        }
    }
    painter.text(
        egui::pos2(response.rect.left() + 54.0, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        ink,
    );
    response.clicked()
}

fn style_name(kind: MarkKind) -> &'static str {
    match kind {
        MarkKind::Highlighter => "Highlighter",
        MarkKind::Foreground => "Text color",
        MarkKind::Underline => "Underline",
        MarkKind::DoubleUnderline => "Double underline",
        MarkKind::TripleUnderline => "Triple underline",
        MarkKind::WavyUnderline => "Wavy underline",
        MarkKind::Box => "Box",
        MarkKind::Brackets => "Brackets",
        MarkKind::Bold => "Bold",
        MarkKind::Italic => "Italic",
        MarkKind::Strikethrough => "Strikethrough",
        MarkKind::MarginBar => "Margin bar",
        MarkKind::Symbol => "Study symbol",
    }
}

fn readable_color(c: MarkColor) -> Color32 {
    match c {
        MarkColor::Yellow => Color32::from_rgb(132, 93, 0),
        MarkColor::Blue => Color32::from_rgb(25, 91, 158),
        MarkColor::Green => Color32::from_rgb(35, 111, 45),
        MarkColor::Orange => Color32::from_rgb(147, 71, 0),
        MarkColor::Pink => Color32::from_rgb(155, 43, 99),
        MarkColor::Purple => Color32::from_rgb(111, 57, 160),
        MarkColor::Red => Color32::from_rgb(170, 38, 38),
        MarkColor::Brown => Color32::from_rgb(112, 73, 35),
        MarkColor::Gray => Color32::from_rgb(88, 94, 102),
    }
}

/// Original repeating edge motifs follow every selected line, without covering glyphs.
fn symbol_span(ui: &egui::Ui, rect: egui::Rect, style: &HighlightStyle) {
    let painter = ui.painter();
    let c = color(style.color);
    let stroke = Stroke::new(1.4, c);
    let name = style.symbol.as_deref().unwrap_or("Important");
    let r = rect.shrink(1.0);
    painter.vline(r.left(), r.y_range(), stroke);
    painter.vline(r.right(), r.y_range(), stroke);
    for y in [r.top(), r.bottom()] {
        match name {
            "Covenant" | "Promise" | "Eternal life" => {
                painter.hline(r.x_range(), y, stroke);
                painter.hline(
                    r.x_range(),
                    y + if y == r.top() { 2.5 } else { -2.5 },
                    Stroke::new(0.8, c),
                );
            }
            _ => {
                let mut x = r.left();
                let pattern = match name {
                    "God" | "Faith" | "Kingdom" => 0,
                    "God’s love" | "Grace" | "Prayer" => 1,
                    "Jesus Christ" | "Redemption" | "Gospel" => 2,
                    "Holy Spirit" | "Repentance" => 3,
                    "Sin" | "Question" | "Important" => 4,
                    _ => 3,
                };
                while x < r.right() {
                    let end = (x + 10.0).min(r.right());
                    let mid = (x + end) * 0.5;
                    match pattern {
                        0 => {
                            painter
                                .line_segment([egui::pos2(x, y), egui::pos2(mid, y - 2.0)], stroke);
                            painter.line_segment(
                                [egui::pos2(mid, y - 2.0), egui::pos2(end, y)],
                                stroke,
                            );
                        }
                        1 => {
                            painter.circle_filled(egui::pos2(mid, y), 1.6, c);
                            painter.line_segment(
                                [egui::pos2(x, y), egui::pos2((x + 2.0).min(end), y)],
                                stroke,
                            );
                        }
                        2 => {
                            painter.hline(x..=end, y, Stroke::new(0.7, c));
                            painter.line_segment(
                                [egui::pos2(mid, y - 2.5), egui::pos2(mid, y + 2.5)],
                                stroke,
                            );
                        }
                        3 => {
                            let points = (0..=8)
                                .map(|n| {
                                    let t = n as f32 / 8.0;
                                    egui::pos2(
                                        x + (end - x) * t,
                                        y + (t * std::f32::consts::TAU).sin() * 1.6,
                                    )
                                })
                                .collect();
                            painter.add(egui::Shape::line(points, stroke));
                        }
                        _ => {
                            painter.line_segment(
                                [egui::pos2(x, y + 1.5), egui::pos2(mid, y - 1.5)],
                                stroke,
                            );
                            painter.line_segment(
                                [egui::pos2(mid, y - 1.5), egui::pos2(end, y + 1.5)],
                                stroke,
                            );
                        }
                    }
                    x += 10.0;
                }
            }
        }
    }
}

fn live_preview(ui: &mut egui::Ui, quote: &str, style: &HighlightStyle) {
    let p = theme::palette(ui.ctx());
    let c = color(style.color);
    let text = if quote.is_empty() {
        "No text selected".to_owned()
    } else {
        quote.chars().take(180).collect()
    };
    let mut format = egui::TextFormat {
        font_id: egui::FontId::new(17.0, egui::FontFamily::Name("scripture".into())),
        color: p.ink,
        line_height: Some(25.0),
        ..Default::default()
    };
    match style.kind {
        MarkKind::Highlighter => format.background = c.gamma_multiply(0.30),
        MarkKind::Symbol => format.background = c.gamma_multiply(0.16),
        MarkKind::Foreground => {
            format.color = if p.dark {
                c
            } else {
                readable_color(style.color)
            }
        }
        MarkKind::Underline | MarkKind::DoubleUnderline | MarkKind::TripleUnderline => {
            format.underline = Stroke::new(1.5, c)
        }
        MarkKind::Strikethrough => format.strikethrough = Stroke::new(1.4, c),
        MarkKind::Italic => format.italics = true,
        _ => {}
    }
    let mut job = egui::text::LayoutJob::default();
    job.append(&text, 0.0, format);
    job.wrap.max_width = ui.available_width() - 24.0;
    job.wrap.max_rows = 2;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 66.0), egui::Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Highlight preview")
    });
    ui.painter().rect_filled(rect, 8, p.page);
    ui.painter().rect_stroke(
        rect,
        8,
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    let pos = rect.min + egui::vec2(12.0, 8.0);
    ui.painter()
        .with_clip_rect(rect.shrink(3.0))
        .galley(pos, galley.clone(), p.ink);
    let mut preview_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    preview_ui.set_clip_rect(rect.shrink(3.0));
    decorate_style(
        &preview_ui,
        style,
        segment_rects(&galley, 0..text.chars().count(), pos),
        &galley,
        pos,
    );
}
