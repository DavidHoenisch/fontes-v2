//! Fontes' toolkit-independent application engine. Content is read-only; user writes are transactional.
pub mod bootstrap;
mod commands;
mod highlights;
mod occurrences;
pub use highlights::*;
mod notes;
pub use notes::*;
pub use occurrences::*;
mod model;
mod queries;
pub mod transport;
pub mod worker;

pub use commands::{Command, command_catalog};
use fontes_data_contracts::{CANONICAL_REGISTRY_JSON, SQLITE_APPLICATION_ID, USER_SCHEMA_SQL};
pub use model::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path, time::Duration};

#[derive(Debug, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_input",
            message: message.into(),
        }
    }
    pub fn missing(message: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: message.into(),
        }
    }
    fn incompatible(message: impl Into<String>) -> Self {
        Self {
            code: "incompatible_data",
            message: message.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self {
            code: "storage",
            message: e.to_string(),
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::invalid(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

pub struct Engine {
    content: Connection,
    user: Connection,
    books: Vec<Book>,
    translations: Vec<Translation>,
    session: Session,
    content_id: String,
    profile: String,
    has_study: bool,
}
impl Engine {
    /// Open an explicitly trusted local v1 snapshot. This is not a download/signature verifier.
    pub fn open(content_path: &Path, user_path: &Path) -> Result<Self> {
        if content_path
            .canonicalize()
            .ok()
            .zip(user_path.canonicalize().ok())
            .is_some_and(|(a, b)| a == b)
        {
            return Err(Error::invalid(
                "Content and user database must be different files.",
            ));
        }
        let content = Connection::open_with_flags(
            content_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        content.execute_batch(
            "PRAGMA trusted_schema=OFF; PRAGMA query_only=ON; PRAGMA foreign_keys=ON;",
        )?;
        if content.query_row("PRAGMA application_id", [], |r| r.get::<_, u32>(0))?
            != SQLITE_APPLICATION_ID
            || content.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))? != 1
        {
            return Err(Error::incompatible(
                "Expected a Fontes content-v1 snapshot. Compile/verify a reading or study profile first.",
            ));
        }
        #[derive(Deserialize)]
        struct Registry {
            books: Vec<Book>,
        }
        let books = serde_json::from_str::<Registry>(CANONICAL_REGISTRY_JSON)?.books;
        // Do not let a content file redefine the permanent registry used by notes or navigation.
        let mut expected = Vec::new();
        for b in &books {
            for (c, count) in b.verse_counts.iter().enumerate() {
                for v in 1..=*count {
                    expected
                        .push(i64::from(b.id) * 1_000_000 + (c as i64 + 1) * 1000 + i64::from(v));
                }
            }
        }
        let actual = content
            .prepare("SELECT id FROM canonical_refs ORDER BY id")?
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if actual != expected {
            return Err(Error::incompatible(
                "Content canonical registry differs from the embedded permanent registry.",
            ));
        }
        let (content_id, profile, registry_version) = content.query_row("SELECT content_id,profile,canonical_registry_version FROM build_info WHERE singleton=1", [], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u32>(2)?)))?;
        if registry_version != 1 {
            return Err(Error::incompatible(
                "Unsupported canonical registry version.",
            ));
        }
        let translations = content
            .prepare("SELECT id,abbr,name FROM translations ORDER BY abbr")?
            .query_map([], |r| {
                Ok(Translation {
                    id: r.get(0)?,
                    abbr: r.get(1)?,
                    name: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if translations.is_empty() {
            return Err(Error::incompatible("The snapshot has no translations."));
        }
        let has_study =
            content.query_row("SELECT EXISTS(SELECT 1 FROM resources)", [], |r| r.get(0))?;
        let mut user = Connection::open(user_path)?;
        user.busy_timeout(Duration::from_secs(2))?;
        user.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")?;
        let app_id: u32 = user.query_row("PRAGMA application_id", [], |r| r.get(0))?;
        let tables: u32 = user.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table'",
            [],
            |r| r.get(0),
        )?;
        if app_id == 0 && tables == 0 {
            let tx = user.transaction()?;
            tx.execute_batch(USER_SCHEMA_SQL)?;
            tx.execute("INSERT INTO user_schema_info VALUES(1,1)", [])?;
            {
                let mut insert = tx.prepare("INSERT INTO canonical_refs VALUES(?1,?2,?3,?4)")?;
                for b in &books {
                    for (c, n) in b.verse_counts.iter().enumerate() {
                        for v in 1..=*n {
                            insert.execute(params![
                                i64::from(b.id) * 1_000_000 + (c as i64 + 1) * 1000 + i64::from(v),
                                b.osis,
                                c + 1,
                                v
                            ])?;
                        }
                    }
                }
            }
            tx.commit()?;
        } else if app_id != 0x464E5553
            || user.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))? != 1
        {
            return Err(Error::incompatible(
                "Refusing to modify an unrelated or unsupported user database.",
            ));
        }
        let user_refs = user
            .prepare("SELECT id FROM canonical_refs ORDER BY id")?
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if user_refs != expected
            || user.query_row(
                "SELECT canonical_registry_version FROM user_schema_info WHERE singleton=1",
                [],
                |r| r.get::<_, u32>(0),
            )? != 1
        {
            return Err(Error::incompatible(
                "User canonical registry does not match this engine.",
            ));
        }
        // App-owned additive extension: preserves the published user-v1 tables and version.
        user.execute_batch("CREATE TABLE IF NOT EXISTS fontes_app_session_v1 (singleton INTEGER PRIMARY KEY CHECK(singleton=1), json TEXT NOT NULL) STRICT; CREATE TABLE IF NOT EXISTS fontes_notes_v1 (annotation_id TEXT PRIMARY KEY REFERENCES annotations(id) ON DELETE CASCADE,document_json TEXT NOT NULL) STRICT; CREATE TABLE IF NOT EXISTS fontes_highlights_v1 (annotation_id TEXT PRIMARY KEY REFERENCES annotations(id) ON DELETE CASCADE,style_json TEXT NOT NULL,selection_json TEXT) STRICT;")?;
        let saved: Option<String> = user
            .query_row(
                "SELECT json FROM fontes_app_session_v1 WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let session = if let Some(json) = saved {
            serde_json::from_str(&json)?
        } else {
            Session {
                selected_ref: 43003016,
                translation: translations[0].id.clone(),
                preferences: Preferences::default(),
            }
        };
        let mut engine = Self {
            content,
            user,
            books,
            translations,
            session,
            content_id,
            profile,
            has_study,
        };
        engine.validate_ref(engine.session.selected_ref)?;
        validate_preferences(&engine.session.preferences)?;
        if !engine
            .translations
            .iter()
            .any(|t| t.id == engine.session.translation)
        {
            engine.session.translation = engine.translations[0].id.clone();
        }
        Ok(engine)
    }

    pub fn dispatch(&mut self, command: Command) -> Result<Response> {
        let mut next = self.session.clone();
        let mut recent = false;
        let data = match command {
            Command::Workspace {} => Data::Workspace,
            Command::Preview { start_ref, end_ref } => {
                self.validate_ref(start_ref)?;
                self.validate_ref(end_ref)?;
                if end_ref < start_ref {
                    return Err(Error::invalid("Invalid preview range."));
                }
                let mut verses=self.content.prepare("SELECT min(canonical_ref_id),text FROM verses_by_reference WHERE translation_id=?1 AND canonical_ref_id BETWEEN ?2 AND ?3 GROUP BY id ORDER BY min(canonical_ref_id),id LIMIT 101")?.query_map(params![self.session.translation,start_ref,end_ref],|r|{let reference=r.get(0)?;Ok(PreviewVerse{ref_id:reference,label:label(&self.books,reference),text:r.get(1)?})})?.collect::<std::result::Result<Vec<_>,_>>()?;
                let has_more = verses.len() > 100;
                verses.truncate(100);
                Data::Preview { verses, has_more }
            }
            Command::LibraryCatalog { source_id, offset } => {
                self.library_catalog(source_id.as_deref(), offset)?
            }
            Command::Navigate { reference } => {
                next.selected_ref = self.resolve(&reference)?;
                recent = true;
                Data::Workspace
            }
            Command::Select { ref_id } => {
                self.validate_ref(ref_id)?;
                next.selected_ref = ref_id;
                recent = true;
                Data::Workspace
            }
            Command::Chapter { delta } => {
                if delta != -1 && delta != 1 {
                    return Err(Error::invalid("Chapter delta must be -1 or 1."));
                }
                let current = self.session.selected_ref / 1000 * 1000;
                let sql = if delta == 1 {
                    "SELECT min(id) FROM canonical_refs WHERE id>?1"
                } else {
                    "SELECT max(id)/1000*1000+1 FROM canonical_refs WHERE id<?1"
                };
                let boundary = if delta == 1 {
                    current + 999
                } else {
                    current + 1
                };
                next.selected_ref = self
                    .content
                    .query_row(sql, [boundary], |r| r.get::<_, Option<i64>>(0))?
                    .ok_or_else(|| {
                        Error::missing("You have reached the end of the available canon.")
                    })?;
                recent = true;
                Data::Workspace
            }
            Command::Translation { id } => {
                if !self.translations.iter().any(|t| t.id == id) {
                    return Err(Error::missing("Translation is not installed."));
                }
                next.translation = id;
                Data::Workspace
            }
            Command::Search {
                query,
                limit,
                offset,
            } => self.search(&query, limit, offset)?,
            Command::Context { ref_id } => {
                self.validate_ref(ref_id)?;
                self.context(ref_id)?
            }
            Command::Resource { id } => Data::Resource {
                resource: self.resource(&id)?,
            },
            Command::Word { code } => self.word(&code)?,
            Command::Occurrences {
                code,
                book_id,
                offset,
                limit,
            } => self.occurrences(&code, book_id, offset, limit)?,
            Command::SaveHighlight {
                id,
                start_ref,
                end_ref,
                style,
                selection,
            } => self.save_highlight(id, start_ref, end_ref, style, selection)?,
            Command::NotePassage { ref_id } => {
                self.validate_ref(ref_id)?;
                let quote:String=self.content.query_row("SELECT text FROM verses_by_reference WHERE canonical_ref_id=?1 AND translation_id=?2 LIMIT 1",params![ref_id,self.session.translation],|r|r.get(0)).optional()?.ok_or_else(||Error::missing("No supplied text in this edition."))?;
                Data::NotePassage {
                    block: NoteBlock::Passage {
                        ref_id,
                        translation_id: self.session.translation.clone(),
                        quote,
                    },
                }
            }
            Command::SaveNote {
                id,
                start_ref,
                end_ref,
                document,
            } => self.save_note(id, start_ref, end_ref, document)?,
            Command::SaveAnnotation {
                id,
                start_ref,
                end_ref,
                kind,
                color,
                content,
            } => {
                self.validate_ref(start_ref)?;
                self.validate_ref(end_ref)?;
                if end_ref < start_ref {
                    return Err(Error::invalid("The range ends before it starts."));
                }
                if let Some(c) = &color
                    && !["amber", "sage", "blue", "rose"].contains(&c.as_str())
                {
                    return Err(Error::invalid(
                        "Highlight color must be amber, sage, blue or rose.",
                    ));
                }
                if kind == AnnotationKind::Highlight && color.is_none() {
                    return Err(Error::invalid("Choose a highlight color."));
                }
                if kind == AnnotationKind::Note
                    && content.as_deref().is_none_or(|s| s.trim().is_empty())
                {
                    return Err(Error::invalid("A note needs text."));
                }
                if content.as_ref().is_some_and(|s| s.len() > 100_000) {
                    return Err(Error::invalid(
                        "Notes may contain at most 100,000 UTF-8 bytes.",
                    ));
                }
                if let Some(id) = &id
                    && !self.user.query_row(
                        "SELECT EXISTS(SELECT 1 FROM annotations WHERE id=?1)",
                        [id],
                        |r| r.get::<_, bool>(0),
                    )?
                {
                    return Err(Error::missing("Annotation does not exist."));
                }
                let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let tx = self.user.transaction()?;
                let range = ensure_range(&tx, start_ref, end_ref)?;
                tx.execute("INSERT INTO annotations(id,range_id,type,color,content,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(id) DO UPDATE SET range_id=excluded.range_id,type=excluded.type,color=excluded.color,content=excluded.content,updated_at=excluded.updated_at",params![id,range,kind.as_str(),color,content])?;
                tx.execute(
                    "DELETE FROM fontes_highlights_v1 WHERE annotation_id=?1",
                    [&id],
                )?;
                tx.execute("DELETE FROM fontes_notes_v1 WHERE annotation_id=?1", [&id])?;
                tx.commit()?;
                Data::SavedAnnotation { id }
            }
            Command::DeleteAnnotation { id } => {
                if self
                    .user
                    .execute("DELETE FROM annotations WHERE id=?1", [id])?
                    == 0
                {
                    return Err(Error::missing("Annotation does not exist."));
                }
                Data::Workspace
            }
            Command::Favorite { ref_id, enabled } => {
                self.validate_ref(ref_id)?;
                let tx = self.user.transaction()?;
                if enabled {
                    let range = ensure_range(&tx, ref_id, ref_id)?;
                    tx.execute("INSERT INTO saved_references VALUES(?1,?2,'favorite',datetime('now')) ON CONFLICT(id) DO NOTHING",params![format!("favorite:{ref_id}"),range])?;
                } else {
                    tx.execute(
                        "DELETE FROM saved_references WHERE id=?1",
                        [format!("favorite:{ref_id}")],
                    )?;
                }
                tx.commit()?;
                Data::Workspace
            }
            Command::Preferences { preferences } => {
                validate_preferences(&preferences)?;
                next.preferences = preferences;
                Data::Workspace
            }
        };
        if recent || next != self.session {
            let tx = self.user.transaction()?;
            if recent {
                let range = ensure_range(&tx, next.selected_ref, next.selected_ref)?;
                tx.execute("INSERT INTO saved_references VALUES(?1,?2,'recent',strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(id) DO UPDATE SET last_used_at=excluded.last_used_at",params![format!("recent:{}",next.selected_ref),range])?;
                tx.execute("DELETE FROM saved_references WHERE type='recent' AND id NOT IN (SELECT id FROM saved_references WHERE type='recent' ORDER BY last_used_at DESC,id LIMIT 20)",[])?;
            }
            tx.execute("INSERT INTO fontes_app_session_v1 VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET json=excluded.json",[serde_json::to_string(&next)?])?;
            tx.commit()?;
        }
        self.session = next;
        Ok(Response {
            protocol_version: 1,
            state: self.workspace()?,
            data,
        })
    }

    pub fn validate_ref(&self, id: i64) -> Result<()> {
        let exists = self
            .books
            .iter()
            .find(|b| i64::from(b.id) == id / 1_000_000)
            .and_then(|b| {
                b.verse_counts
                    .get((id % 1_000_000 / 1000).checked_sub(1)? as usize)
            })
            .is_some_and(|n| id % 1000 >= 1 && id % 1000 <= i64::from(*n));
        if exists {
            Ok(())
        } else {
            Err(Error::invalid(
                "That canonical book/chapter/verse does not exist.",
            ))
        }
    }

    /// Exact names/OSIS/USFM, then unique prefixes. No silent ambiguous-book guesses.
    pub fn resolve(&self, input: &str) -> Result<i64> {
        if input.len() > 256 {
            return Err(Error::invalid("References may contain at most 256 bytes."));
        }
        let input = input.trim().to_lowercase();
        let boundary = input
            .char_indices()
            .find(|(i, c)| {
                *i > 0 && c.is_ascii_digit() && input[..*i].chars().any(char::is_alphabetic)
            })
            .map(|(i, _)| i);
        let (book, numbers) = boundary.map(|i| input.split_at(i)).unwrap_or((&input, "1"));
        let key = normalize(book);
        let key = match key.as_str() {
            "j" | "jn" => "john".into(),
            "psalm" => "ps".into(),
            _ => key,
        };
        let exact = self
            .books
            .iter()
            .filter(|b| {
                [&b.name, &b.osis, &b.usfm]
                    .iter()
                    .any(|s| normalize(s) == key || (b.osis == "Ps" && key == "psalm"))
            })
            .collect::<Vec<_>>();
        let matches = if exact.is_empty() {
            self.books
                .iter()
                .filter(|b| {
                    !key.is_empty()
                        && [&b.name, &b.osis, &b.usfm]
                            .iter()
                            .any(|s| normalize(s).starts_with(&key))
                })
                .collect::<Vec<_>>()
        } else {
            exact
        };
        if matches.len() != 1 {
            return Err(Error::invalid(if matches.is_empty() {
                "Unknown book. Try John 3:16 or Rom 1:3.".into()
            } else {
                format!(
                    "Ambiguous book: {}. Type more of its name.",
                    matches
                        .iter()
                        .map(|b| b.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }));
        }
        let components = numbers
            .split(|c: char| c == ':' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if components.is_empty() || components.len() > 2 {
            return Err(Error::invalid(
                "Use a book, chapter and optional verse, for example John 3:16.",
            ));
        }
        let chapter = components[0]
            .parse::<u16>()
            .map_err(|_| Error::invalid("Chapter must be a number."))?;
        let verse = components
            .get(1)
            .unwrap_or(&"1")
            .parse::<u16>()
            .map_err(|_| Error::invalid("Verse must be a number."))?;
        if !(1..=999).contains(&chapter) || !(1..=999).contains(&verse) {
            return Err(Error::invalid("Chapter and verse must be in 1–999."));
        }
        let id =
            i64::from(matches[0].id) * 1_000_000 + i64::from(chapter) * 1000 + i64::from(verse);
        self.validate_ref(id)?;
        Ok(id)
    }
}
fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}
fn validate_preferences(p: &Preferences) -> Result<()> {
    if !(16.0..=36.0).contains(&p.text_size) || !(1.2..=2.2).contains(&p.line_spacing) {
        Err(Error::invalid(
            "Text size must be 16–36; line spacing must be 1.2–2.2.",
        ))
    } else {
        Ok(())
    }
}
fn ensure_range(tx: &rusqlite::Transaction<'_>, start: i64, end: i64) -> Result<String> {
    tx.execute("INSERT INTO reference_ranges VALUES(?1,?2,?3) ON CONFLICT(start_ref_id,end_ref_id) DO NOTHING",params![format!("range:{start}-{end}"),start,end])?;
    Ok(tx.query_row(
        "SELECT id FROM reference_ranges WHERE start_ref_id=?1 AND end_ref_id=?2",
        params![start, end],
        |r| r.get(0),
    )?)
}
