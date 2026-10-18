//! App-owned rich highlights; the published user-v1 schema stays unchanged.
use crate::*;
use schemars::JsonSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarkKind {
    Highlighter,
    Foreground,
    Underline,
    DoubleUnderline,
    TripleUnderline,
    WavyUnderline,
    Box,
    Brackets,
    Bold,
    Italic,
    Strikethrough,
    MarginBar,
    Symbol,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarkColor {
    Yellow,
    Blue,
    Green,
    Orange,
    Pink,
    Purple,
    Red,
    Brown,
    Gray,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HighlightStyle {
    pub kind: MarkKind,
    pub color: MarkColor,
    pub symbol: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    pub verse_id: String,
    pub start: usize,
    pub end: usize,
    pub quote: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextSelection {
    pub translation_id: String,
    pub spans: Vec<TextSpan>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RichHighlight {
    pub annotation_id: String,
    pub style: HighlightStyle,
    pub selection: Option<TextSelection>,
}

impl Engine {
    pub(crate) fn save_highlight(
        &mut self,
        id: Option<String>,
        start_ref: i64,
        end_ref: i64,
        style: HighlightStyle,
        selection: Option<TextSelection>,
    ) -> Result<Data> {
        self.validate_ref(start_ref)?;
        self.validate_ref(end_ref)?;
        if end_ref < start_ref {
            return Err(Error::invalid("The highlight ends before it starts."));
        }
        if style.kind == MarkKind::Symbol
            && style
                .symbol
                .as_deref()
                .is_none_or(|s| s.trim().is_empty() || s.chars().count() > 40)
            || style.kind != MarkKind::Symbol && style.symbol.is_some()
        {
            return Err(Error::invalid("Choose a valid highlight symbol."));
        }
        self.validate_text_selection(start_ref, end_ref, selection.as_ref())?;
        if let Some(id) = &id {
            let kind: Option<String> = self
                .user
                .query_row("SELECT type FROM annotations WHERE id=?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            if kind.as_deref() != Some("highlight") {
                return Err(Error::invalid("Choose an existing highlight to edit."));
            }
        }
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let style_json = serde_json::to_string(&style)?;
        let selection_json = selection.as_ref().map(serde_json::to_string).transpose()?;
        let quote = selection.as_ref().map(|s| {
            s.spans
                .iter()
                .map(|s| s.quote.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        });
        let color = match style.color {
            MarkColor::Blue => "blue",
            MarkColor::Green => "sage",
            MarkColor::Pink | MarkColor::Purple | MarkColor::Red => "rose",
            _ => "amber",
        };
        let tx = self.user.transaction()?;
        let range = ensure_range(&tx, start_ref, end_ref)?;
        tx.execute("INSERT INTO annotations(id,range_id,type,color,created_at,updated_at,translation_id,quoted_text,anchor_content_id) VALUES(?1,?2,'highlight',?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'),?4,?5,?6) ON CONFLICT(id) DO UPDATE SET range_id=excluded.range_id,color=excluded.color,updated_at=excluded.updated_at,translation_id=excluded.translation_id,quoted_text=excluded.quoted_text,anchor_content_id=excluded.anchor_content_id",params![id,range,color,selection.as_ref().map(|s|&s.translation_id),quote,self.content_id])?;
        tx.execute("INSERT INTO fontes_highlights_v1(annotation_id,style_json,selection_json) VALUES(?1,?2,?3) ON CONFLICT(annotation_id) DO UPDATE SET style_json=excluded.style_json,selection_json=excluded.selection_json",params![id,style_json,selection_json])?;
        tx.commit()?;
        Ok(Data::SavedAnnotation { id })
    }
}

impl Engine {
    pub(crate) fn validate_text_selection(
        &self,
        start_ref: i64,
        end_ref: i64,
        selection: Option<&TextSelection>,
    ) -> Result<()> {
        if let Some(selected) = selection {
            if selected.spans.is_empty()
                || selected.spans.len() > 200
                || selected.spans.iter().map(|s| s.quote.len()).sum::<usize>() > 100_000
            {
                return Err(Error::invalid(
                    "Select between one character and 200 verses (up to 100,000 bytes).",
                ));
            }
            let rows=self.content.prepare("SELECT v.id,v.text,min(v.canonical_ref_id) FROM verses_by_reference v WHERE v.translation_id=?1 AND v.canonical_ref_id BETWEEN ?2 AND ?3 GROUP BY v.id,v.text ORDER BY min(v.canonical_ref_id) LIMIT 201")?.query_map(params![selected.translation_id,start_ref,end_ref],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
            if rows.len() != selected.spans.len() {
                return Err(Error::invalid(
                    "The selected text no longer matches this edition.",
                ));
            }
            for (i, ((verse, text), span)) in rows.iter().zip(&selected.spans).enumerate() {
                let n = text.chars().count();
                if verse != &span.verse_id
                    || span.start >= span.end
                    || span.end > n
                    || i > 0 && span.start != 0
                    || i + 1 < rows.len() && span.end != n
                    || text
                        .chars()
                        .skip(span.start)
                        .take(span.end - span.start)
                        .collect::<String>()
                        != span.quote
                {
                    return Err(Error::invalid(
                        "The selected text no longer matches this edition.",
                    ));
                }
            }
        }
        Ok(())
    }
}
