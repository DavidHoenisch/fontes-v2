-- Draft executable model for reviewing ownership and cross-resource linking.
-- Not the complete library v2 schema and not a migration for published v1 data.
-- Run only in a new scratch database. Production token/witness/asset tables and
-- schema migrations remain implementation work.
PRAGMA foreign_keys = ON;

CREATE TABLE packages (
    resource_id TEXT PRIMARY KEY,
    publisher_id TEXT NOT NULL,
    version TEXT NOT NULL,
    build_revision INTEGER NOT NULL CHECK(build_revision > 0),
    payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64)
) STRICT;

CREATE TABLE canonical_refs (
    id INTEGER PRIMARY KEY,
    osis TEXT NOT NULL,
    chapter INTEGER NOT NULL CHECK(chapter > 0),
    verse INTEGER NOT NULL CHECK(verse > 0),
    UNIQUE(osis, chapter, verse)
) STRICT;

CREATE TABLE content_nodes (
    package_id TEXT NOT NULL REFERENCES packages(resource_id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    parent_node_id TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('document','section','paragraph','heading','verse','note','list-item','entry')),
    position INTEGER NOT NULL CHECK(position >= 0),
    text TEXT NOT NULL,
    PRIMARY KEY(package_id, node_id),
    FOREIGN KEY(package_id, parent_node_id) REFERENCES content_nodes(package_id, node_id) ON DELETE CASCADE,
    CHECK(node_id IS NOT parent_node_id)
) STRICT;

CREATE TABLE scripture_links (
    package_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    start_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    end_ref_id INTEGER NOT NULL REFERENCES canonical_refs(id),
    relation TEXT NOT NULL CHECK(relation IN ('text-of','applies-to','cites','quotes','alludes-to')),
    basis TEXT NOT NULL CHECK(basis IN ('author','source','editorial','inferred')),
    provenance TEXT NOT NULL,
    PRIMARY KEY(package_id,node_id,start_ref_id,end_ref_id,relation,basis,provenance),
    FOREIGN KEY(package_id,node_id) REFERENCES content_nodes(package_id,node_id) ON DELETE CASCADE,
    CHECK(start_ref_id <= end_ref_id)
) STRICT;
CREATE INDEX scripture_links_by_start ON scripture_links(start_ref_id,end_ref_id);

CREATE TABLE strongs_codes (
    id TEXT PRIMARY KEY CHECK(substr(id,1,1) IN ('G','H') AND length(id) BETWEEN 5 AND 8 AND substr(id,2) NOT GLOB '*[^0-9]*')
) STRICT;
CREATE TABLE word_links (
    package_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    strongs TEXT NOT NULL REFERENCES strongs_codes(id),
    relation TEXT NOT NULL CHECK(relation IN ('defines','discusses','uses')),
    PRIMARY KEY(package_id,node_id,strongs,relation),
    FOREIGN KEY(package_id,node_id) REFERENCES content_nodes(package_id,node_id) ON DELETE CASCADE
) STRICT;
CREATE INDEX word_links_by_code ON word_links(strongs);

-- Concepts are shared identities. Labels/relationships require authority checks
-- in the trusted importer; package authors cannot edit the central namespace.
CREATE TABLE concepts (
    id TEXT PRIMARY KEY,
    authority TEXT NOT NULL,
    scope_note TEXT NOT NULL
) STRICT;
CREATE TABLE concept_labels (
    concept_id TEXT NOT NULL REFERENCES concepts(id),
    language TEXT NOT NULL,
    label TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('preferred','alternative')),
    PRIMARY KEY(concept_id,language,label)
) STRICT;
CREATE UNIQUE INDEX one_preferred_label ON concept_labels(concept_id,language) WHERE kind='preferred';
CREATE INDEX concept_labels_by_text ON concept_labels(label,language);
CREATE TABLE concept_relations (
    subject_id TEXT NOT NULL REFERENCES concepts(id),
    object_id TEXT NOT NULL REFERENCES concepts(id),
    relation TEXT NOT NULL CHECK(relation IN ('broader','related','close-match')),
    provenance TEXT NOT NULL,
    PRIMARY KEY(subject_id,object_id,relation),
    CHECK(subject_id != object_id)
) STRICT;
-- Only reviewed equivalences go here; broader/related/close-match never collapse.
-- Effective targets are flattened by the registry compiler, with cycle checks.
CREATE TABLE concept_resolutions (
    submitted_id TEXT PRIMARY KEY REFERENCES concepts(id),
    effective_id TEXT NOT NULL REFERENCES concepts(id),
    registry_revision TEXT NOT NULL,
    review_id TEXT NOT NULL,
    CHECK(submitted_id != effective_id)
) STRICT;
CREATE TABLE concept_assertions (
    package_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    submitted_concept_id TEXT NOT NULL REFERENCES concepts(id),
    relation TEXT NOT NULL CHECK(relation IN ('discusses','defines','illustrates')),
    provenance TEXT NOT NULL,
    PRIMARY KEY(package_id,node_id,submitted_concept_id,relation,provenance),
    FOREIGN KEY(package_id,node_id) REFERENCES content_nodes(package_id,node_id) ON DELETE CASCADE
) STRICT;
CREATE VIEW effective_concept_assertions AS
SELECT a.*,coalesce(r.effective_id,a.submitted_concept_id) AS effective_concept_id
FROM concept_assertions a LEFT JOIN concept_resolutions r ON r.submitted_id=a.submitted_concept_id;

CREATE TABLE search_documents (
    id INTEGER PRIMARY KEY,
    package_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    text TEXT NOT NULL,
    UNIQUE(package_id,node_id),
    FOREIGN KEY(package_id,node_id) REFERENCES content_nodes(package_id,node_id) ON DELETE CASCADE
) STRICT;
CREATE VIRTUAL TABLE search_fts USING fts5(text,content='search_documents',content_rowid='id');
-- The library service updates/rebuilds FTS in the same candidate transaction;
-- deleting search_documents alone does not update external-content FTS postings.
