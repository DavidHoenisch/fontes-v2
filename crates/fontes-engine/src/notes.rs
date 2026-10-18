//! Versioned, app-owned note documents. Plain annotation text remains a fallback.
use crate::*;
use schemars::JsonSchema;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NoteBlock {
    Thought {
        text: String,
    },
    Passage {
        ref_id: i64,
        translation_id: String,
        quote: String,
    },
    Resource {
        resource_id: String,
        quote: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        source: String,
        #[serde(default)]
        content_id: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoteDocument {
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    pub blocks: Vec<NoteBlock>,
    pub selection: Option<TextSelection>,
}
impl NoteDocument {
    pub fn plain_text(&self) -> String {
        std::iter::once(self.title.clone())
            .chain(self.blocks.iter().map(|b| match b {
                NoteBlock::Thought { text } => text.clone(),
                NoteBlock::Passage {
                    ref_id,
                    translation_id,
                    quote,
                } => format!("{quote}\n[{translation_id} · {ref_id}]"),
                NoteBlock::Resource {
                    quote,
                    title,
                    author,
                    source,
                    ..
                } => format!(
                    "{quote}\n[{title} · {} · {source}]",
                    author.as_deref().unwrap_or("")
                ),
            }))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RichNote {
    pub annotation_id: String,
    pub document: NoteDocument,
}
impl Engine {
    pub(crate) fn save_note(
        &mut self,
        id: Option<String>,
        start_ref: i64,
        end_ref: i64,
        mut document: NoteDocument,
    ) -> Result<Data> {
        self.validate_ref(start_ref)?;
        self.validate_ref(end_ref)?;
        if end_ref < start_ref
            || document.title.len() > 512
            || document.blocks.is_empty()
            || document.blocks.len() > 200
            || serde_json::to_vec(&document)?.len() > 100_000
        {
            return Err(Error::invalid(
                "A note needs 1–200 pieces, a title up to 512 bytes, and at most 100,000 bytes.",
            ));
        }
        let previous = if let Some(id) = &id {
            let kind: Option<String> = self
                .user
                .query_row("SELECT type FROM annotations WHERE id=?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            if kind.as_deref() != Some("note") {
                return Err(Error::invalid("Choose an existing note to edit."));
            }
            self.user
                .query_row(
                    "SELECT document_json FROM fontes_notes_v1 WHERE annotation_id=?1",
                    [id],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
                .map(|s| serde_json::from_str::<NoteDocument>(&s))
                .transpose()?
        } else {
            None
        };
        // A stored anchor can survive a missing source only at its original canonical range.
        let same_range = if let Some(id) = &id {
            self.user.query_row("SELECT r.start_ref_id=?2 AND r.end_ref_id=?3 FROM annotations a JOIN reference_ranges r ON r.id=a.range_id WHERE a.id=?1",params![id,start_ref,end_ref],|r|r.get::<_,bool>(0))?
        } else {
            false
        };
        if !same_range
            || previous
                .as_ref()
                .is_none_or(|p| p.selection != document.selection)
        {
            self.validate_text_selection(start_ref, end_ref, document.selection.as_ref())?;
        }
        for block in &mut document.blocks {
            if previous.as_ref().is_some_and(|p| p.blocks.contains(block)) {
                continue;
            }
            match block {
                NoteBlock::Thought { text } => {
                    if text.trim().is_empty() {
                        return Err(Error::invalid("A thought cannot be empty."));
                    }
                }
                NoteBlock::Passage {
                    ref_id,
                    translation_id,
                    quote,
                } => {
                    self.validate_ref(*ref_id)?;
                    let text:Option<String>=self.content.query_row("SELECT text FROM verses_by_reference WHERE canonical_ref_id=?1 AND translation_id=?2 LIMIT 1",params![*ref_id,&*translation_id],|r|r.get(0)).optional()?;
                    if quote.trim().is_empty() || text.is_none_or(|s| !s.contains(quote.as_str())) {
                        return Err(Error::invalid(
                            "Choose an exact excerpt from an installed passage.",
                        ));
                    }
                }
                NoteBlock::Resource {
                    resource_id,
                    quote,
                    title,
                    author,
                    source,
                    content_id,
                } => {
                    let r = self.resource(resource_id)?;
                    let (t, a, text, src) = (r.title, r.author, r.content, r.source);
                    if quote.trim().is_empty() || !text.contains(quote.as_str()) {
                        return Err(Error::invalid("Choose an exact excerpt from the resource."));
                    }
                    *title = t;
                    *author = a;
                    *source = src;
                    *content_id = self.content_id.clone();
                }
            }
        }
        let plain = document.plain_text();
        if plain.len() > 100_000 || serde_json::to_vec(&document)?.len() > 150_000 {
            return Err(Error::invalid("This note is too large."));
        }
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let encoded = serde_json::to_string(&document)?;
        let quote = document.selection.as_ref().map(|s| {
            s.spans
                .iter()
                .map(|s| s.quote.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        });
        let tx = self.user.transaction()?;
        let range = ensure_range(&tx, start_ref, end_ref)?;
        tx.execute("INSERT INTO annotations(id,range_id,type,content,created_at,updated_at,translation_id,quoted_text,anchor_content_id) VALUES(?1,?2,'note',?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'),?4,?5,?6) ON CONFLICT(id) DO UPDATE SET range_id=excluded.range_id,content=excluded.content,updated_at=excluded.updated_at,translation_id=excluded.translation_id,quoted_text=excluded.quoted_text,anchor_content_id=excluded.anchor_content_id",params![id,range,plain,document.selection.as_ref().map(|s|&s.translation_id),quote,self.content_id])?;
        tx.execute("INSERT INTO fontes_notes_v1(annotation_id,document_json) VALUES(?1,?2) ON CONFLICT(annotation_id) DO UPDATE SET document_json=excluded.document_json",params![id,encoded])?;
        tx.commit()?;
        Ok(Data::SavedAnnotation { id })
    }
}
