-- user.sqlite is mutable and never replaced by a published content snapshot.
PRAGMA foreign_keys = ON;
PRAGMA application_id = 0x464E5553;
PRAGMA user_version = 1;

CREATE TABLE user_schema_info (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    canonical_registry_version INTEGER NOT NULL CHECK (canonical_registry_version = 1)
) STRICT;

-- Seed the same permanent anchor registry locally. SQLite FKs cannot cross databases.
CREATE TABLE canonical_refs (
    id INTEGER PRIMARY KEY,
    osis TEXT NOT NULL,
    chapter INTEGER NOT NULL CHECK (chapter BETWEEN 1 AND 999),
    verse INTEGER NOT NULL CHECK (verse BETWEEN 1 AND 999),
    UNIQUE (osis, chapter, verse)
) STRICT;

CREATE TABLE reference_ranges (
    id TEXT PRIMARY KEY,
    start_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    end_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    UNIQUE (start_ref_id, end_ref_id),
    CHECK (start_ref_id <= end_ref_id)
) STRICT;

CREATE TABLE annotations (
    id TEXT PRIMARY KEY,
    range_id TEXT NOT NULL REFERENCES reference_ranges(id),
    type TEXT NOT NULL CHECK (type IN ('highlight', 'note')),
    color TEXT,
    content TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Optional translation/word anchor supplements the permanent scripture range.
    translation_id TEXT,
    quoted_text TEXT,
    anchor_content_id TEXT
) STRICT;
CREATE INDEX annotations_by_range ON annotations(range_id);

CREATE TABLE saved_references (
    id TEXT PRIMARY KEY,
    range_id TEXT NOT NULL REFERENCES reference_ranges(id),
    type TEXT NOT NULL CHECK (type IN ('favorite', 'recent')),
    last_used_at TEXT NOT NULL
) STRICT;

CREATE TABLE update_state (
    repository TEXT NOT NULL,
    channel TEXT NOT NULL CHECK (channel IN ('stable', 'preview')),
    content_schema_version INTEGER NOT NULL,
    -- This high-water mark is not lowered by a local rollback.
    highest_verified_sequence INTEGER NOT NULL CHECK (highest_verified_sequence > 0),
    PRIMARY KEY (repository, channel, content_schema_version)
) STRICT;
