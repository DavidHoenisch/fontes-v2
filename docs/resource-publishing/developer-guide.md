# Integrating packages into the installed library

Status: design guidance. [library-model.sql](../../specs/resource-packages/v1/library-model.sql) exercises ownership and links in a scratch database. It is not the complete library schema v2, a delivery exchange schema, or a migration for current releases.

## Data and query ownership

Store active content together in `library.sqlite`; keep user notes, private tags, and preferences in `user.sqlite`. Each publisher-owned node, witness, token, link, derived search row, and asset reference needs explicit package ownership. Shared canonical anchors, Strong's identities, and concepts survive individual package removal. Use `(package_id, node_id)` for content anchors so unrelated publishers can use the same local paragraph ID.

Import a signed delivery payload through trusted parameterized code into known app tables. Source XML is a publisher/CI interface; clients install normalized packages. Do not execute payload SQL or attach each package as a permanent separate library. The complete exchange schema and allowed SQLite-object policy must be frozen and tested before client work depends on them.

The scratch model supports a single range query across installed packages:

```sql
SELECT DISTINCT n.package_id, n.node_id, n.text, l.relation, l.basis
FROM scripture_links AS l
JOIN content_nodes AS n
  ON n.package_id = l.package_id AND n.node_id = l.node_id
WHERE l.start_ref_id <= :verse_id AND l.end_ref_id >= :verse_id;
```

Resolve `Rom.1.3` through the frozen canonical registry before binding `:verse_id`. Return match relationship and provenance so the client can explain each result. Ancestor/document coverage may contribute discovery results while paragraph links provide precise navigation; broad coverage must not be presented as an explicit paragraph citation.

Concept lookup uses the resolution view, preserving the submitted identity:

```sql
SELECT a.package_id, a.node_id, a.submitted_concept_id, a.provenance
FROM effective_concept_assertions AS a
WHERE a.effective_concept_id = :concept_id;
```

Handle hierarchical expansion separately, with cycle checks, deduplication, and a visible expansion indicator. Do not combine related-topic matches with direct scripture citations without preserving the reason. Word links use shared codes; occurrence/lemma identity additionally requires the specific witness. Missing optional dictionaries or works should show unavailable targets without breaking the current text.

## Catalog and compatibility

Authenticate exact catalog bytes using the configured repository/channel/key and monotonic sequence before trusting their contents. Retain the last verified catalog for offline browsing. Each catalog is a complete availability snapshot; an omitted resource stops being offered for installation after refresh, while installed content remains. Download separately hashed cover variants; browsing never requires downloading resource ZIPs. Compare structured resource versions and build revisions, required capabilities, package/library schemas, canonical registry, and dependencies before offering installation.

Treat public rights metadata as constraints per resource. Display, search, export, printing, embeddings, and AI-context use cannot inherit permission from other installed works. A composed library has no universal public-domain flag. Permission enforcement and attribution must apply to query results and derived indexes as well as exports.

## Install, update, and remove

The shared Rust library service should own mutation and recovery for both desktop and mobile. UI code requests an operation and renders its state; it does not maintain a separate installed-version authority.

| State | Durable effect and recovery |
| --- | --- |
| Resolve | Verify selection, rights, compatibility, dependency closure, and disk budget; no library change |
| Download | Write hash-addressed partial file, verify signed bounds/hash; discard corrupt or incomplete inputs |
| Stage | Extract with actual byte/path/depth limits; verify signature, inventory, assets, and payload structure |
| Compose | Under one mutation lock, use SQLite backup API to clone active content into a candidate |
| Import | In a candidate transaction, replace only owned records, preserve shared identities, rebuild/update FTS and receipts |
| Validate | Check integrity, all foreign keys, package counts, links, coverage, search, permissions, and asset availability |
| Activate | Close/checkpoint and flush candidate; atomically replace the active-generation pointer while retaining previous |
| Recover/clean | Validate pointer and installed-set receipt on startup; recover previous verified generation; clean only after reader release |

The database's package set and the generation receipt must agree. Existing readers can finish on their old database handles; new readers open the new active generation. Flush behavior and pointer replacement need platform-specific implementations and failure tests. Never copy only a live SQLite main file while ignoring its journal/WAL. At peak, active, previous, and candidate generations may require three full library copies, plus staging/journals/assets; estimate actual space before mutation.

A failure before activation discards the candidate. A failed activation recovers the last verified pointer. Never restore a backup over a database still serving readers. Removal follows the same candidate lifecycle, with explicit handling of required dependents and FTS cleanup. Asset garbage collection accounts for active, previous, staging, and in-flight readers.

User notes retain canonical anchors, stable package/block identity, and a quoted fallback. If a resource disappears, preserve the note with an unavailable-resource state. Splits/merges require an explicit reviewed anchor map; the interchange contract remains pending. Do not guess a replacement paragraph and silently move notes.

## Migration and release gates

Current signed reading/study snapshots remain immutable. Introduce an independent package output path and a trusted v1-to-v2 migration with legacy anchor mappings. Split the eight existing sources into KJV, WEB, Greek Strong's, Hebrew Strong's, Robinson TR, Matthew Henry, Nave, and TSK packages; preserve both Greek reading streams and source provenance. Reading/study become selection bundles. Never replace a user's composed library with a newer whole-profile snapshot.

Before production, test on both client platforms:

- Independent resources referencing Romans 1:3 appear together; uninstall/update of one preserves the other and user notes.
- A taxonomy equivalence improves existing queries without rewriting source assertions; mapping reversal is recoverable.
- Rejected signature/hash, unknown SQLite objects, unsafe archives, incompatible schema, and dependency cycles leave active content usable.
- Cancellation, disk exhaustion, and process termination at every durable transition recover the last verified generation.
- Crash timing between pointer replacement and cleanup preserves readers, receipts, assets, and an intact rollback generation.
- Migrated text, witness/token relationships, links, search, attributions, and note anchors match the verified v1 fixtures.

The current example checker covers the isolated SQL relationships, transaction rollback, backup-based staging, uninstall/search cleanup, and note preservation. It does not implement or test filesystem activation, network behavior, full migration, or app crash recovery. Those are release gates rather than completed guarantees.
