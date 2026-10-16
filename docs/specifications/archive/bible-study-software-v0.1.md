<!-- Historical source transcription; current requirements are in /SPEC.md. -->

# Original Bible study software specification v0.1

Historical source: `Bible_Study_Software_Spec_v0.1.docx`, supplied by the project owner. The original document states October 16, 2026. Paragraph text, lists, SQL, and Rust examples below are transcribed from that document; heading/list markup and code fences were added for Markdown. The source Word file is unchanged.

Source SHA-256: `556034d7509451bc37f4581ea50dd440ef1d04884f3b864670bdb3592288d879`.

This is an archival baseline, not the active implementation contract. Use [the current Fontes system specification](../../../SPEC.md), which records subsequent decisions and schema corrections. In particular, the SQL and Rust types below are historical examples and must not replace the current versioned schemas.

## Bible Study Software

Specification v0.1

Date: October 16, 2026

### 1. Overview

A cross-platform, offline-first Bible study application built in pure Rust. It prioritizes a fast, natural reading and navigation experience, consistent and beautiful UI, and study tools (word studies, genealogies, book overviews, AI-assisted search) grounded in public-domain resources. The data model is reference-centric so that translations, original-language data, commentaries, and user annotations all link cleanly to stable canonical references.

### 2. Goals

- Delightful reading surface with excellent typography and minimal chrome.

- Fast, low-friction reference navigation (better than existing apps’ book/chapter/verse pickers).

- Deep but non-bloated study features that stay secondary to the text.

- Fully offline after initial data load; local AI only.

- Consistent visual language across reading view, modals, context menus, and tools.

- Extensible resource model so new public-domain materials can be added without schema changes.

### 3. Functional Requirements

FR1 — Multiple translations

The system shall support one or more public-domain translations (starting with KJV). The user can switch the active translation at any time. All study tools and annotations remain linked to the underlying canonical reference, not to a specific translation’s text.

FR2 — Reference navigation

The system shall provide a reference picker that supports: compact book grid with Old/New Testament grouping and current book highlighted; progressive disclosure of chapter and verse after book selection; persistent quick-jump input that accepts natural forms (“John 3:16”, “j3 16”, book abbreviations); recent/favorite references. The picker shall minimize scrolling and support both mouse/touch and keyboard-driven use.

FR3 — Reading view

The system shall display the active translation’s text for a selected range with clean, highly readable typography (font, size, weight, line spacing, theme); verse numbers that can be shown or hidden; support for user highlights and notes that remain visually consistent with the rest of the UI; smooth scrolling and chapter/verse navigation controls.

FR4 — Word studies

Given a word (or Strong’s number / lemma) in the reading view or via search, the system shall present: original-language surface form, lemma, Strong’s number, and morphology; all occurrences of that lemma/Strong’s across the corpus, with the ability to jump to each; linked lexicon / Strong’s entry content; short gloss and optional aligned translation words.

FR5 — Book overviews

For each book the system shall provide a structured overview containing at least: author, date, audience, purpose, key themes, and a short summary. These overviews are linked to the book’s canonical range and can be opened from the reading view or reference picker.

FR6 — Genealogies

The system shall display biblical family trees (starting with major lines such as Adam → Noah, Abraham → David, etc.). Each person and relationship can be linked back to the canonical reference(s) where the relationship is stated. The view shall be navigable and visually clear.

FR7 — Study resources

Commentaries, Strong’s articles, book summaries, and other public-domain resources shall be linked to canonical references or to Strong’s/lemma identifiers. Selecting a reference or word shall surface relevant linked resources without leaving the primary reading context.

FR8 — AI-assisted search

The system shall support natural-language queries (e.g., “verses about forgiveness”, “what does agape mean in John”). Answers are produced by a local open-source model operating over a hybrid retrieval index (keyword + embeddings) of the verse texts and linked resources. Every answer must cite the canonical references used. The UI must display the actual verse text from the database alongside any model-generated explanation. The model is constrained by a system prompt that forbids inventing verses or references.

FR9 — User annotations

The user can create highlights (with color) and free-text notes attached to any canonical range. Annotations persist across sessions and are visible in the reading view.

FR10 — Consistency of UI chrome

All secondary surfaces (reference picker, context menus, modals, study panels, settings) shall use the same visual language (corner radius, color palette, typography, spacing) as the primary reading view.

### 4. Non-Functional Requirements

NFR1 — Offline-first

All reading, navigation, word studies, genealogies, book overviews, and annotations function without network access after the initial data set is present. AI features use a local model and local index.

NFR2 — Performance

Reference picking and passage loading shall feel instantaneous on modest hardware. Word-study queries that span the corpus shall return results in under one second for typical lemmas.

NFR3 — Platform targets

Primary target is desktop (Windows, macOS, Linux) via egui/eframe. Android is a supported secondary target. iOS is best-effort / experimental.

NFR4 — Data integrity

All links between translations, original-language tokens, resources, annotations, and genealogies are enforced by foreign keys to canonical references (or Strong’s/lemma identifiers). Deleting or updating a canonical reference cascades or is constrained appropriately.

NFR5 — Extensibility

Adding a new public-domain translation, commentary, or lexicon shall require only data import, not schema or code changes to the core linking model.

NFR6 — Resource footprint

The application and its initial public-domain data set shall remain practical for local use (target: under a few hundred MB for text + indexes before a local LLM is added).

### 5. Constraints

- Implementation language: Rust.

- UI toolkit: egui (immediate-mode) with eframe for windowing.

- Persistence: SQLite (or equivalent embedded relational store).

- Initial content: public-domain only (KJV, Strong’s, Matthew Henry, morphhb / equivalent Greek morphology, etc.).

- AI: local open-source model (GGUF or equivalent) + RAG; no cloud dependency required for core or AI features.

- No proprietary Logos or other commercial data formats; all linking is implemented in the open schema.

### 6. Key Behaviors

- Selecting a book in the reference picker immediately reveals chapter (and optionally verse) controls without a full mode switch.

- Clicking a word in the reading view opens a consistent context panel or menu offering: highlight, note, word study, copy, and “ask AI about this”.

- Changing translation updates the displayed text while preserving the current canonical reference and any open study panels.

- AI answers always include a list of cited canonical references; selecting a citation jumps the reading view to that location and shows the real verse text.

- Genealogies and book overviews are reachable both from a dedicated study surface and contextually from the reading view when the current range is relevant.

### 7. Data Model

The system is organized around a stable canonical_refs table. Translations, original-language tokens, resources, genealogies, book overviews, and user annotations all foreign-key to these references (or to Strong’s/lemma identifiers). This mirrors the reference-centric linking approach used by advanced Bible software while remaining open and SQLite-friendly.

#### 7.1 Core Schema (SQL)

```sql
-- Core
CREATE TABLE books (
    id              INTEGER PRIMARY KEY,
    osis            TEXT UNIQUE NOT NULL,
    name            TEXT NOT NULL,
    testament       TEXT NOT NULL,
    sort_order      INTEGER NOT NULL
);

CREATE TABLE canonical_refs (
    id              INTEGER PRIMARY KEY,
    book_id         INTEGER NOT NULL REFERENCES books(id),
    chapter         INTEGER NOT NULL,
    verse_start     INTEGER NOT NULL,
    verse_end       INTEGER,
    UNIQUE(book_id, chapter, verse_start, verse_end)
);

CREATE TABLE translations (
    id              INTEGER PRIMARY KEY,
    abbr            TEXT UNIQUE NOT NULL,
    name            TEXT NOT NULL,
    language        TEXT NOT NULL,
    is_public_domain INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE verse_texts (
    id               INTEGER PRIMARY KEY,
    translation_id   INTEGER NOT NULL REFERENCES translations(id),
    canonical_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    text             TEXT NOT NULL,
    UNIQUE(translation_id, canonical_ref_id)
);

CREATE TABLE original_tokens (
    id               INTEGER PRIMARY KEY,
    canonical_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    language         TEXT NOT NULL,
    surface          TEXT NOT NULL,
    lemma            TEXT NOT NULL,
    strongs          TEXT,
    morph_code       TEXT,
    position         INTEGER NOT NULL,
    gloss            TEXT
);

CREATE TABLE word_alignments (
    id                  INTEGER PRIMARY KEY,
    verse_text_id       INTEGER NOT NULL REFERENCES verse_texts(id),
    original_token_id   INTEGER NOT NULL REFERENCES original_tokens(id),
    word_start          INTEGER,
    word_end            INTEGER
);

CREATE TABLE resources (
    id          INTEGER PRIMARY KEY,
    type        TEXT NOT NULL,
    title       TEXT,
    author      TEXT,
    source      TEXT,
    content     TEXT
);

CREATE TABLE resource_links (
    id               INTEGER PRIMARY KEY,
    resource_id      INTEGER NOT NULL REFERENCES resources(id),
    canonical_ref_id INTEGER REFERENCES canonical_refs(id),
    strongs          TEXT,
    lemma            TEXT,
    link_type        TEXT NOT NULL
);

CREATE TABLE persons (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    strongs     TEXT,
    notes       TEXT
);

CREATE TABLE genealogy_edges (
    id              INTEGER PRIMARY KEY,
    parent_id       INTEGER NOT NULL REFERENCES persons(id),
    child_id        INTEGER NOT NULL REFERENCES persons(id),
    relationship    TEXT,
    canonical_ref_id INTEGER REFERENCES canonical_refs(id),
    notes           TEXT
);

CREATE TABLE book_overviews (
    id          INTEGER PRIMARY KEY,
    book_id     INTEGER NOT NULL REFERENCES books(id),
    author      TEXT,
    date_written TEXT,
    audience    TEXT,
    purpose     TEXT,
    key_themes  TEXT,
    outline     TEXT,
    summary     TEXT NOT NULL
);

CREATE TABLE annotations (
    id               INTEGER PRIMARY KEY,
    canonical_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    type             TEXT NOT NULL,
    color            TEXT,
    content          TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
```

#### 7.2 Corresponding Rust Types

The following Rust structs mirror the tables for use with sqlx or a thin data layer.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CanonicalRef {
    pub id: i64,
    pub book_id: i64,
    pub chapter: i32,
    pub verse_start: i32,
    pub verse_end: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerseText {
    pub id: i64,
    pub translation_id: i64,
    pub canonical_ref_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OriginalToken {
    pub id: i64,
    pub canonical_ref_id: i64,
    pub language: String,
    pub surface: String,
    pub lemma: String,
    pub strongs: Option<String>,
    pub morph_code: Option<String>,
    pub position: i32,
    pub gloss: Option<String>,
}

// Additional structs for Resource, ResourceLink, Person,
// GenealogyEdge, BookOverview, and Annotation follow the same pattern.
```

### 8. Out of Scope (v0.1)

- Proprietary or copyrighted translations and resources.

- Full syntactic/clause parsing or advanced original-language diagramming.

- Cloud sync or multi-user collaboration.

- Mobile-optimized reading plans or social features.

This is a living specification. It captures the functional requirements, non-functional requirements, constraints, key behaviors, and reference-centric data model for the pure-Rust Bible study application.
