use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Book {
    pub id: u16,
    pub osis: String,
    pub usfm: String,
    pub name: String,
    pub testament: String,
    pub verse_counts: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Translation {
    pub id: String,
    pub abbr: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verse {
    pub id: String,
    /// All canonical anchors for this source segment, in display order.
    pub refs: Vec<i64>,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub text_size: f32,
    pub line_spacing: f32,
    pub verse_numbers: bool,
    pub scripture_serif: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            text_size: 22.0,
            line_spacing: 1.65,
            verse_numbers: true,
            scripture_serif: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub selected_ref: i64,
    pub translation: String,
    pub preferences: Preferences,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationKind {
    Note,
    Highlight,
}
impl AnnotationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Highlight => "highlight",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub start_ref: i64,
    pub end_ref: i64,
    pub kind: AnnotationKind,
    pub color: Option<String>,
    pub content: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedReference {
    pub ref_id: i64,
    pub label: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub content_id: String,
    pub profile: String,
    pub session: Session,
    pub books: Vec<Book>,
    pub translations: Vec<Translation>,
    pub passage: Vec<Verse>,
    pub annotations: Vec<Annotation>,
    pub saved: Vec<SavedReference>,
    pub has_study: bool,
}
impl Workspace {
    pub fn reference_label(&self) -> String {
        label(&self.books, self.session.selected_ref)
    }
}
pub fn label(books: &[Book], reference: i64) -> String {
    books
        .iter()
        .find(|b| i64::from(b.id) == reference / 1_000_000)
        .map(|b| {
            format!(
                "{} {}:{}",
                b.name,
                reference % 1_000_000 / 1000,
                reference % 1000
            )
        })
        .unwrap_or_else(|| reference.to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub ref_id: Option<i64>,
    pub title: String,
    pub excerpt: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub kind: String,
    pub basis: String,
    pub excerpt: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceDetail {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub content: String,
    pub source: String,
    pub rights_review: String,
    pub links: Vec<ResourceLink>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceLink {
    pub start_ref: Option<i64>,
    pub end_ref: Option<i64>,
    pub strongs: Option<String>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenAnalysis {
    pub witness: String,
    pub position: u32,
    pub surface: String,
    pub analysis_index: u32,
    pub lemma: Option<String>,
    pub morph_system: Option<String>,
    pub morph_code: Option<String>,
    pub strongs: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Data {
    Workspace,
    Search {
        hits: Vec<SearchHit>,
        offset: u32,
        has_more: bool,
    },
    Context {
        ref_id: i64,
        resources: Vec<ResourceSummary>,
        tokens: Vec<TokenAnalysis>,
    },
    Resource {
        resource: ResourceDetail,
    },
    Word {
        code: String,
        definitions: Vec<ResourceSummary>,
        occurrences: Vec<SavedReference>,
        total: u32,
    },
    SavedAnnotation {
        id: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    pub protocol_version: u32,
    pub state: Workspace,
    pub data: Data,
}
