-- Published snapshots: immutable, built from scratch, without user annotations.
PRAGMA foreign_keys = ON;
PRAGMA application_id = 0x464E5453;
PRAGMA user_version = 1;

CREATE TABLE build_info (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    content_id TEXT NOT NULL,
    profile TEXT NOT NULL,
    content_schema_version INTEGER NOT NULL CHECK (content_schema_version = 1),
    canonical_registry_version INTEGER NOT NULL CHECK (canonical_registry_version = 1),
    source_lock_sha256 TEXT NOT NULL,
    builder_commit TEXT NOT NULL
) STRICT;

CREATE TABLE books (
    id INTEGER PRIMARY KEY CHECK (id BETWEEN 1 AND 999),
    osis TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    testament TEXT NOT NULL CHECK (testament IN ('OT', 'NT', 'DC')),
    sort_order INTEGER UNIQUE NOT NULL
) STRICT;

-- Atomic anchors only. Book IDs are allocated permanently in the canonical registry.
CREATE TABLE canonical_refs (
    id INTEGER PRIMARY KEY,
    book_id INTEGER NOT NULL REFERENCES books(id),
    chapter INTEGER NOT NULL CHECK (chapter BETWEEN 1 AND 999),
    verse INTEGER NOT NULL CHECK (verse BETWEEN 1 AND 999),
    UNIQUE (book_id, chapter, verse),
    CHECK (id = book_id * 1000000 + chapter * 1000 + verse)
) STRICT;

CREATE TABLE reference_ranges (
    id TEXT PRIMARY KEY,
    start_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    end_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    UNIQUE (start_ref_id, end_ref_id),
    CHECK (start_ref_id <= end_ref_id)
) STRICT;

CREATE TABLE source_snapshots (
    id TEXT PRIMARY KEY,
    source_id TEXT UNIQUE NOT NULL,
    source_sha256 TEXT NOT NULL,
    source_url TEXT NOT NULL,
    upstream_revision TEXT,
    rights_review_id TEXT NOT NULL,
    adapter_version TEXT NOT NULL
) STRICT;

CREATE TABLE text_witnesses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    language TEXT NOT NULL,
    edition TEXT NOT NULL,
    versification TEXT NOT NULL,
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id)
) STRICT;

-- A source segment can map to several canonical verses, and vice versa.
CREATE TABLE source_segments (
    id TEXT PRIMARY KEY,
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    witness_id TEXT REFERENCES text_witnesses(id),
    source_ref TEXT NOT NULL,
    source_record_key TEXT NOT NULL,
    UNIQUE (source_snapshot_id, source_record_key),
    UNIQUE (id, witness_id)
) STRICT;

CREATE TABLE segment_refs (
    segment_id TEXT NOT NULL REFERENCES source_segments(id),
    canonical_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    relation TEXT NOT NULL CHECK (relation IN ('exact', 'split', 'merged', 'part')),
    PRIMARY KEY (segment_id, canonical_ref_id)
) STRICT;
CREATE INDEX segment_refs_by_ref ON segment_refs(canonical_ref_id, segment_id);

CREATE TABLE translations (
    id TEXT PRIMARY KEY,
    abbr TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    language TEXT NOT NULL,
    witness_id TEXT UNIQUE NOT NULL REFERENCES text_witnesses(id)
) STRICT;

CREATE TABLE verse_texts (
    id TEXT PRIMARY KEY,
    translation_id TEXT NOT NULL REFERENCES translations(id),
    segment_id TEXT NOT NULL REFERENCES source_segments(id),
    text TEXT NOT NULL,
    -- Structured paragraphs, poetry, supplied words, notes; never arbitrary executable HTML.
    display_json TEXT,
    UNIQUE (translation_id, segment_id)
) STRICT;

CREATE VIEW verses_by_reference AS
SELECT v.id, v.translation_id, r.canonical_ref_id, v.text, v.display_json,
       v.segment_id, r.relation
FROM verse_texts v JOIN segment_refs r ON r.segment_id = v.segment_id;

CREATE TABLE strongs_codes (
    id TEXT PRIMARY KEY,
    language TEXT NOT NULL CHECK (language IN ('he', 'grc')),
    CHECK (length(id) BETWEEN 5 AND 8
       AND substr(id, 1, 1) IN ('H', 'G')
       AND substr(id, 2) NOT GLOB '*[^0-9]*')
) STRICT;

CREATE TABLE lemmas (
    id TEXT PRIMARY KEY,
    witness_id TEXT NOT NULL REFERENCES text_witnesses(id),
    source_key TEXT NOT NULL,
    lemma TEXT NOT NULL,
    UNIQUE (witness_id, source_key)
) STRICT;

CREATE TABLE original_tokens (
    id TEXT PRIMARY KEY,
    witness_id TEXT NOT NULL REFERENCES text_witnesses(id),
    segment_id TEXT NOT NULL REFERENCES source_segments(id),
    surface TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    gloss TEXT,
    UNIQUE (witness_id, segment_id, position),
    FOREIGN KEY (segment_id, witness_id) REFERENCES source_segments(id, witness_id)
) STRICT;

CREATE TABLE token_analyses (
    token_id TEXT NOT NULL REFERENCES original_tokens(id),
    analysis_index INTEGER NOT NULL CHECK (analysis_index >= 0),
    lemma_id TEXT REFERENCES lemmas(id),
    morph_system TEXT,
    morph_code TEXT,
    PRIMARY KEY (token_id, analysis_index),
    CHECK ((morph_system IS NULL) = (morph_code IS NULL))
) STRICT;
CREATE INDEX token_analyses_by_lemma ON token_analyses(lemma_id, token_id);

CREATE TABLE token_strongs (
    token_id TEXT NOT NULL,
    analysis_index INTEGER NOT NULL,
    strongs TEXT NOT NULL REFERENCES strongs_codes(id),
    PRIMARY KEY (token_id, analysis_index, strongs),
    FOREIGN KEY (token_id, analysis_index) REFERENCES token_analyses(token_id, analysis_index)
) STRICT;
CREATE INDEX token_strongs_by_code ON token_strongs(strongs, token_id);

CREATE TABLE word_alignments (
    id TEXT PRIMARY KEY,
    verse_text_id TEXT NOT NULL REFERENCES verse_texts(id),
    original_token_id TEXT NOT NULL REFERENCES original_tokens(id),
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    -- Unicode scalar offsets in verse_texts.text; zero-based and end-exclusive.
    word_start INTEGER NOT NULL CHECK (word_start >= 0),
    word_end INTEGER NOT NULL CHECK (word_end > word_start),
    alignment_method TEXT NOT NULL CHECK (alignment_method IN ('source', 'reviewed'))
) STRICT;

CREATE TABLE resources (
    id TEXT PRIMARY KEY,
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    source_record_key TEXT NOT NULL,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    author TEXT,
    content TEXT NOT NULL,
    display_json TEXT,
    UNIQUE (source_snapshot_id, source_record_key)
) STRICT;

CREATE TABLE resource_links (
    id TEXT PRIMARY KEY,
    resource_id TEXT NOT NULL REFERENCES resources(id),
    range_id TEXT REFERENCES reference_ranges(id),
    strongs TEXT REFERENCES strongs_codes(id),
    lemma_id TEXT REFERENCES lemmas(id),
    link_type TEXT NOT NULL,
    CHECK ((range_id IS NOT NULL) + (strongs IS NOT NULL) + (lemma_id IS NOT NULL) = 1)
) STRICT;
CREATE INDEX resource_links_by_range ON resource_links(range_id, resource_id);
CREATE INDEX resource_links_by_strongs ON resource_links(strongs, resource_id);
CREATE INDEX reference_ranges_by_start ON reference_ranges(start_ref_id, end_ref_id);

CREATE TABLE persons (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    notes TEXT,
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    source_record_key TEXT NOT NULL,
    UNIQUE (source_snapshot_id, source_record_key)
) STRICT;

CREATE TABLE genealogy_edges (
    id TEXT PRIMARY KEY,
    parent_id TEXT NOT NULL REFERENCES persons(id),
    child_id TEXT NOT NULL REFERENCES persons(id),
    relationship TEXT NOT NULL CHECK (relationship IN ('parent', 'ancestor', 'legal_parent')),
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    notes TEXT,
    CHECK (parent_id <> child_id)
) STRICT;

CREATE TABLE genealogy_edge_refs (
    edge_id TEXT NOT NULL REFERENCES genealogy_edges(id),
    range_id TEXT NOT NULL REFERENCES reference_ranges(id),
    PRIMARY KEY (edge_id, range_id)
) STRICT;

CREATE TABLE book_overviews (
    id TEXT PRIMARY KEY,
    book_id INTEGER NOT NULL REFERENCES books(id),
    source_snapshot_id TEXT NOT NULL REFERENCES source_snapshots(id),
    author TEXT,
    date_written TEXT,
    audience TEXT,
    purpose TEXT,
    key_themes_json TEXT,
    outline_json TEXT,
    summary TEXT NOT NULL,
    source_resource_id TEXT REFERENCES resources(id),
    editorial_status TEXT NOT NULL CHECK (editorial_status = 'reviewed')
) STRICT;

-- Deterministically ordered document IDs; vector indexes refer to these IDs plus content_id.
CREATE TABLE search_documents (
    id INTEGER PRIMARY KEY,
    verse_text_id TEXT REFERENCES verse_texts(id),
    resource_id TEXT REFERENCES resources(id),
    text TEXT NOT NULL,
    CHECK ((verse_text_id IS NOT NULL) + (resource_id IS NOT NULL) = 1)
) STRICT;
CREATE VIRTUAL TABLE search_fts USING fts5(
    text, content='search_documents', content_rowid='id', tokenize='unicode61'
);
