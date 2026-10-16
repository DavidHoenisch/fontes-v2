# Fontes extensible library and resource packages

Read this detailed design with [SPEC.md](../../SPEC.md), the consolidated system implementation baseline. The system specification records product requirements, original requirement traceability, cross-system decisions, implementation status, and remaining release gates.

Status: proposed specification addendum to Bible Study Software Specification v0.1. The authoring schemas and example in this repository are reviewable draft contracts. The Rust source authoring CLI implements scaffolding, validation, packaging, and schema export. The catalog service, publisher portal, delivery SQLite compiler, and app installer described here are implementation work. Existing data releases remain valid and unchanged.

The subsequent [source-submission pipeline](../resource-publishing/submission-pipeline.md) implements trusted validation, exact-hash approval, and signed source-stage prereleases under this repository's `library/`, with reusable workflows for future separate/private repositories. This is separate from the compiled delivery/catalog stages below; see the current system specification for revision/status tracking.

See the [publishing documentation](../resource-publishing/README.md) for author, taxonomy, maintainer, and client integration guides, including executable example checks and implementation status.

Fontes will offer a default offline library plus independently installable resources. Publishers submit a documented XML source package; Fontes validates and compiles it into a signed delivery package. The app browses a small catalog, downloads only selected resources, and imports them into one shared installed content database. Every paragraph can contribute scripture, word-study, and topic links to the same library index.

The first catalog distributes free resources with verified redistribution permission. Paid purchases are a later phase. Package identity, publisher identity, and rights metadata must support that phase without assuming that a free download is public domain or that a public package can enforce paid access.

## Decisions and scope

| Question | Initial decision |
| --- | --- |
| Default versus optional resources | Reading and study become install presets listing independent resource IDs |
| Publisher submission | ZIP archive with XML metadata, an ordered index, content shards, cover, attribution, and license |
| Authoring format | Fontes XML with published XSDs and semantic validation; adapters for other formats |
| One huge XML document versus chapter files | An index plus shards; a chapter per file is recommended, not a filename-derived identity |
| App download | A compiled resource package produced by trusted Fontes CI |
| Installed storage | One managed `library.sqlite` for all active content; separate `user.sqlite` for user-owned data |
| Browsing | A signed catalog with separately hosted covers; no package download to inspect a card |
| Installation | Build and validate a candidate library generation, then atomically activate it |
| Linking | Stable canonical scripture anchors, stable block IDs, qualified word identifiers, and concept IDs |
| Topics | A versioned concept registry with labels, scope notes, relationships, proposals, and reviewed mappings |
| Submission repository | Proposed separate `fontes-library` repository; not provisioned by this addendum |
| Commercial titles | A future private intake and entitlement service; never place restricted content in a public PR |

The original specification explicitly excludes proprietary/copyrighted translations. This addendum expands the architecture to support appropriately licensed resources, while keeping the default corpus's public-domain policy. NLT is an example of a possible future licensed translation, not a currently distributable package. Tyndale describes permission requirements for substantial NLT use; listing or distributing a full translation requires the applicable publisher agreement. [Tyndale permissions](https://www.tyndale.com/permissions).

## Three separate artifacts

1. **Source package**: editable XML and assets submitted by a publisher or Fontes maintainer. The same rules apply to both.
2. **Delivery package**: normalized, indexed data compiled and signed by Fontes. This is what the app installs.
3. **Catalog**: discoverable descriptions, covers, compatibility, sizes, rights, and verified download identities. It describes resources whether or not they are installed.

A resource is a logical work or edition. A package version is a distributable version of that work. A content block is a paragraph, verse, heading, note, or entry within it. These are separate identities; the current schema's `resources` table holds articles, so its rows must not be confused with catalog-level works.

A Bible edition is normally one resource containing many books and chapters. A multi-volume work can use ordered parts in its index. Different translations or independently maintained editions use different resource IDs. Collections are bundles of resource IDs; they do not duplicate the underlying packages.

## Resource identity and versions

A verified publisher receives a namespace such as `org.example`. Its resource might be `org.example.romans-study`. The example namespace is illustrative and is not a registered Fontes publisher. Only that publisher or an explicitly authorized maintainer can update that identity. A title, author's display name, cover, or ISBN never establishes ownership.

Each source has a numeric `major.minor.patch` content version, with no leading zeros. The initial stable format does not accept prerelease versions. Released source bytes under an identity/version are immutable. Delivery builds add a positive `build_revision`, allowing a compiler correction to rebuild the same source without overwriting an artifact. Clients compare structured version components and build revision, not filename ordering.

Document and block IDs are publisher-assigned, stable within the resource, and unique across its content files. Moving `rom-001-p03` into another file does not change its identity. Inserted paragraphs get new IDs. A changed heading does not rename its ID. An update that splits, merges, or retires blocks includes a reviewed anchor map in the release process; no nearest-paragraph guess is treated as a stable note anchor.

| Version axis | Purpose |
| --- | --- |
| Source format version | XML vocabulary and archive rules |
| Resource version | Publisher's content revision |
| Delivery build revision | Fontes compilation revision of those source bytes |
| Catalog protocol and sequence | Discovery, authenticity, compatibility, and replay resistance |
| Canonical registry version | Scripture identities and versification mapping target |
| Taxonomy snapshot | Concepts and reviewed mappings used to build derived indexes |
| Library schema version | Installed tables and relationships understood by the app |

The draft XML namespace is `urn:fontes:resource:1`. It is a proposed stable identifier, not an online schema download location. Schemas are distributed with the tooling and are never fetched from a package-chosen URL. Adding required elements requires a negotiated format version.

## Source archive contract

Filename: `<resource-id>--<version>.fontes-source.zip`, for example `org.example.romans-study--1.0.0.fontes-source.zip`. The files below are at the archive root, without an extra enclosing directory.

```text
metadata.xml
index.xml
rights/
  LICENSE.txt
  ATTRIBUTION.txt
assets/
  cover.png                 # alternatively cover.jpg
content/
  rom/
    001.xml
    002.xml
  introductions/
    romans.xml              # optional; listed in index.xml if present
taxonomy/
  concepts.xml              # optional publisher concept proposals
media/
  figure-01.png             # optional local images referenced by content
```

Required root files are `metadata.xml`, `index.xml`, `rights/LICENSE.txt`, and `rights/ATTRIBUTION.txt`. Exactly one cover is declared by metadata. Content files live below `content/` and are enumerated once in `index.xml`. An optional proposal file has the fixed path `taxonomy/concepts.xml`. Optional images live below `media/`. The compiler rejects unlisted payloads; build scripts, executables, fonts, plugins, external stylesheets, and SQL are outside this source contract.

The ordered index provides reading order, sections, navigation labels, and file identity. Paths organize source files; paths and alphabetical order do not establish reading order. A short article can use a single content file. A Bible should normally use `content/<osis-book>/<chapter>.xml`. Dictionaries can shard entries into sensible groups; one file per word is unnecessary. Shards must not split an individual block. The initial limits are 16 MiB per XML file, 10,000 archive entries, nesting depth 32 in XML and 8 in navigation, and 1 GiB total expanded source. The validator streams/counts actual bytes; declared ZIP sizes are not sufficient. These are configurable review limits, not a requirement that the app fit a gigabyte library.

All paths use relative, case-sensitive names with `/` separators and the portable ASCII subset defined by `common.xsd`: letters, digits, underscores, and hyphens, with dots additionally permitted within components after the first. Reject absolute paths, `.` or `..` components, backslashes, duplicate names, case-fold collisions, links, device files, encryption, and ZIP entries escaping the archive. XML text is UTF-8 and may contain non-Latin scripts; DTDs, external entities, XInclude, remote assets, and arbitrary HTML are forbidden. Allowed images are decoded and checked against declared media type, dimensions, and size. Unsupported features fail explicitly rather than silently discarding content.

The current authoring validator additionally caps path nesting at 32 components and content PNG/JPEG assets at 16 MiB, 8192 pixels per dimension, and 16,777,216 total pixels. The stricter cover limits below apply independently.

### Metadata

`metadata.xml` contains:

- Resource ID, content version, format version, resource kind, language, title, subtitle, description, edition, and publication date.
- Author/editor/translator identities and display names; verified publisher namespace and display name. Dates or names are attributed publisher metadata.
- Cover path, media type, dimensions, and accessibility description.
- Source versification and canonical registry version. A non-scripture book still declares which canonical registry its links target.
- License identifier (SPDX where applicable, otherwise a reviewed `LicenseRef-*`), copyright/attribution statement, rights holder, and local license/attribution paths.
- Declared permission scope for offline display, local keyword search, export, printing, embeddings, and AI context. Absence is not a permission grant. The first catalog requires offline display and keyword indexing. Additional capabilities are enabled only after rights review.
- Optional source/provenance URLs and identifiers such as ISBN. A URL is descriptive metadata, not executable input or an automatic fetch instruction.

Rights review also records territorial and platform restrictions; a worldwide grant is never inferred from “free.” If an agreement needs enforcement beyond the initial public-free mechanism, withhold the catalog entry until that mechanism exists. Existing exceptions, including the KJV Crown-rights notice, must survive conversion from current sources.

Availability, price, entitlement, approval status, and trust are catalog decisions. A submitter cannot authorize itself through metadata. Catalog descriptions are bounded plain text, never remote HTML. Do not include private license agreements, access tokens, customer information, or private correspondence in a public archive; provide a private evidence reference to maintainers instead.

### Cover images

Require a **1200 × 1800 pixel** portrait master cover in PNG or JPEG, at most 4 MiB. An old cover of a different aspect ratio should be placed on a compliant canvas without stretching. This creates consistent cards while preserving recognizable artwork. The cover's redistribution rights must be verified independently of the underlying book's text.

Publication produces a 200 × 300 card thumbnail and a 600 × 900 detail image. The catalog binds their media types, dimensions, lengths, and hashes, and carries the cover license, rights holder, and attribution so the detail view can credit artwork before installation. The app uses those assets while browsing and can cache them independently of the book. No catalog request needs to extract a large ZIP. The sample cover is an illustrative placeholder, not proposed product artwork.

### Content and navigation

`index.xml` orders documents and optionally groups them into named sections. Each document declares a stable ID and consists of typed blocks. Each block has its own stable ID, a supported type, text with a small inline vocabulary, and optional annotations.

Initial resource kinds: `translation`, `commentary`, `lexicon`, `word-study`, `study-guide`, `devotional`, and `book`. Initial block kinds: `paragraph`, `heading`, `verse`, `note`, `list-item`, and `entry`. Inline content supports emphasis, scripture references, Strong's links, concept terms, and local images with alternate text. Unknown markup is a validation error. Detailed original-language token/morphology interchange needs a separate profile before third parties can publish occurrence-level analyses; the existing core source adapters retain their full token data and are not flattened into prose.

A translation's verse block declares its original source reference and reviewed canonical mapping. The default target is the permanent registry already committed in this repository. Missing source verses remain explicit omissions. Merged and split verses keep source identity and all applicable canonical anchors. A whole Bible claiming complete coverage must demonstrate that coverage; a partial resource must declare its scope in catalog metadata and validation output.

The example at `specs/resource-packages/v1/examples/romans-study/` exercises an introduction, an indexed chapter shard, inline citation, an attributed allusion, a Strong's annotation, a canonical concept, and a publisher-proposed concept.

## Scripture and word links

Links belong to the smallest useful block, usually a paragraph or verse. A chapter can carry a broad `applies-to` range for discovery, but that is not a replacement for paragraph annotations. Inline scripture references are indexed automatically and render as navigable references. Authors can also annotate implicit discussion without changing their prose:

```xml
<block xmlns="urn:fontes:resource:1" id="rom-001-p03" kind="paragraph">
  <text>The paragraph discusses the Messiah's human ancestry.</text>
  <annotations>
    <scripture start="Rom.1.3" relation="alludes-to" basis="author"/>
    <concept id="urn:fontes:concept:incarnation" relation="discusses"/>
  </annotations>
</block>
```

Use `text-of` for a translation's canonical text, `applies-to` for commentary coverage, `cites` for an explicit reference, `quotes` for a quotation, and `alludes-to` for an attributed implicit connection. Start/end OSIS references are checked against the permanent registry, then resolved to foreign keys. Source reference strings remain available for audit. Verse-numbering differences require mapping; they are not normalized by guessing. Quotation text keeps its stated translation/edition when known and is not silently labeled as the user's active translation.

Strong's IDs normalize to `G0001` or `H0001` and up, as the current pipeline does. Their shared identity does not require a dictionary to be installed. Definition resources appear when installed. Lemmas and occurrences are witness-specific; the same spelling in two editions is not enough to merge tokens or establish an English word alignment.

Publisher-supplied links, imported source links, editorial additions, machine suggestions, and user tags retain different provenance. Inference produces a suggestion or a separately marked assertion with method/version/confidence; it does not manufacture an author citation. Retrieval can include authored allusions by default, show topic-related material separately, and expose optional inferred matches with their basis.

A context menu at Romans 1:3 searches the shared index across all installed works and reports **why** each hit matched: commentary coverage, citation, quotation, authored allusion, or related topic. The result opens the exact paragraph, with surrounding context and its work's attribution. A missing optional resource never breaks scripture identity or another work's content.

## Concepts and taxonomy

Use **concept** for the stable metadata object and **topic** for its user-facing browsing label. Concept IDs are persistent URIs; labels are editable and localizable. A concept contains a preferred label per language, alternative labels, a scope note, relationships, provenance, and status. A label alone is never an identity.

The model follows the useful distinctions in [W3C SKOS](https://www.w3.org/TR/skos-primer/): labels, broader/narrower relationships, related concepts, and mappings between vocabularies. Fontes can store this in SQLite and publish XML/JSON; authors do not need to learn RDF.

| Relationship | Meaning and search behavior |
| --- | --- |
| Preferred and alternative labels | Different names for the same concept; resolve to one ID |
| Broader/narrower | Different concepts in a hierarchy; expansion is explicit and attributable |
| Related | Useful association without equivalence or hierarchy |
| Reviewed equivalence mapping | A publisher concept maps to a central concept; preserve the original assertion |
| Close match | Similar meanings; suggest for review, do not automatically merge |

“The Flood” and “Noah's Flood” can label the same narrative concept, scoped to Genesis 6–9. “Global Flood interpretation” expresses a particular interpretation. It can be a narrower or related topic, with “global flood” as an alternative label of that interpretation concept. It must not become an unconditional synonym asserting that every work about the Flood endorses global extent. Likewise, the desert of Sin and the concept of sin must remain distinct even if labels overlap; the imported Nave entries illustrate that problem.

A search for a child concept shows exact/alias matches first and offers the broader Flood topic for expanded results. A broader-topic search can include narrower topics with a visible expansion indicator. Topic association adds retrieval context; it does not imply a scripture citation or an author's endorsement. Several parents are permitted, cycles are forbidden, and traversal deduplicates concepts. Do not use a single-parent folder tree as the concept model.

### Author workflow for existing and new concepts

1. Search the published taxonomy by label, aliases, language, and scope note. Select the stable concept ID when the meaning fits.
2. If no concept fits, declare a publisher concept, for example `urn:fontes:publisher:org.example:concept:messianic-ancestry`, in `taxonomy/concepts.xml`.
3. Supply preferred label, language, definition/scope, suggested parents or related IDs, and representative scripture evidence where relevant. Provisional IDs must be in the submitting publisher's namespace.
4. Validation accepts structurally valid declarations and reports likely duplicates as suggestions. A string similarity match never rewrites the submission.
5. Maintainers accept a new canonical concept, add an alias to an existing one, approve an equivalence mapping, or keep meanings distinct. The proposal and decision remain reviewable in the catalog repository.
6. Publish a signed taxonomy revision. Existing installed assertions retain their submitted IDs and receive an effective resolution through the approved mapping. Authors can adopt the canonical ID in their next revision; discovery does not depend on everyone republishing immediately.

Packages must not inject new definitions or relationships into the central namespace. Locally imported unpublished material can retain provisional concepts, clearly marked as local. Publisher-authored statements and Fontes editorial enrichment are separate assertion layers. Fontes can tag existing resources with a new concept through a signed enrichment package containing evidence, target block IDs, target resource/content hashes, and editorial provenance. It never rewrites an author's immutable source archive to add its own interpretation.

Deprecation and merging preserve old IDs with redirects. Labels can change without ID changes. Mappings are versioned, auditable, and reversible; close matches are not transitively collapsed. Conflicting definitions require editorial review. An uninstall removes that package's assertions, not shared concepts or mappings still used by other works. User-created private tags remain in `user.sqlite` and do not enter the public taxonomy automatically.

## Catalog and browsing

The first implementation is a static signed catalog hosted from the dedicated catalog repository and its release/CDN assets. Proposed repository name: `fontes-library`. Its organization/owner, final URL, maintainers, and intake contact must be provisioned before the app advertises submission links. No fictional live endpoint is required by this draft.

Publication extracts approved metadata once and generates catalog JSON plus cover variants. A card displays title, cover, authors, kind, language, and installed/update state. The detail view includes description, edition, publisher, dates, scope/coverage, license and attribution, download and installed-size estimates, supported study features, validation limitations, version, and release notes. “Install” changes to “Update,” “Installed,” or “Requires a newer app” as appropriate. A free catalog needs no purchase account for publicly distributable content.

The catalog lists all available resources; “your library” lists installed resources. Default bundles and recommendations do not silently install new books. Users choose updates or enable an explicit resource-update preference, and can pin a version. Download, install, remove, and update states are recoverable after interruption.

The signed catalog envelope uses a new protocol, distinct from the current data-release manifest. Its signature covers the exact UTF-8 `catalog.json` bytes prefixed by `FONTES-LIBRARY-CATALOG-V1` and one newline byte; `catalog.json.sig` is a detached 64-byte Ed25519 signature. It binds a monotonic sequence, channel, timestamp, taxonomy snapshot, resource metadata record hashes, cover hashes/lengths, and immutable package download identities. Each package record includes resource version, build revision, compressed/expanded lengths, hashes, format/library-schema compatibility, and required versus optional dependencies. A mutable latest pointer is only discovery; the app authenticates the referenced catalog and applies its high-water mark. Do not add unrecognized fields to the existing strict v1 manifest.

The catalog’s `metadata_sha256` hashes the exact compiled `metadata.json` bytes in the delivery package; publication checks that repeated card fields agree with that metadata. The compact initial catalog can contain all metadata in one response. A future sharded catalog uses signed hashes for its pages so browsing can remain lazy without downloading resource archives. Metadata/cover caches work offline. The initial catalog is a complete availability snapshot with one advertised version/build per resource ID. Withdrawal omits the resource from the next signed snapshot and removes it from bundles; clients that accept that snapshot stop offering installation but preserve installed content and user data. Approval, withdrawal reasons, and replacement decisions remain in repository review history; structured replacement notices need a later contract extension. An offline client cannot learn a new withdrawal until it refreshes the catalog. A user's cached verified package remains subject to its actual license terms.

## Submission and publication

The public catalog repository should version small manifests, package descriptors, source/build recipes, open taxonomy proposals, reviews, and validation summaries. Do not store every large ZIP revision in ordinary Git history. GitHub documents its large-file limits; release/object assets are a better fit for immutable archives. [GitHub large files](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github).

For an already redistributable work, a submission PR contains its resource directory or an immutable source-package URL plus expected SHA-256 and length, verified publisher identity, rights declaration, and proposed taxonomy changes. Small open XML sources can be reviewed unpacked in Git; larger sources live as hash-addressed submission/release assets. Maintainers fetch through the fixed ingestion service with size limits, not by executing a PR-supplied script. Semantic reports expose text/link changes even when the submitted object is a ZIP.

Works with uncertain or restricted rights use private intake before any public upload. A prospective paid title cannot enter a public source repository or public release asset merely because a publisher intends to sell it later. Its public catalog metadata can be separate from the restricted payload once an actual agreement permits that.

The PR flow is:

```mermaid
flowchart LR
    A[Author XML and cover] --> B[Local validation and preview]
    B --> C[Source archive and submission PR]
    C --> D[Unprivileged structural checks]
    D --> E[Publisher rights and editorial review]
    E --> F[Trusted build after merge]
    F --> G[Signed resource and catalog release]
    G --> H[App downloads selected resource]
    H --> I[Validated library generation]
```

Untrusted submissions run without publication credentials, signing keys, privileged caches, or arbitrary author code. Only reviewed changes on the trusted branch enter a fresh build with fixed Fontes adapters. Approval checks textual rights, cover rights, required attribution, scope, link quality, concept changes, and publisher ownership. The compiler can validate structure; it cannot prove that the submitter owns a book.

The publisher creates immutable delivery assets, downloads and verifies them, and only then publishes a catalog sequence pointing to them. Bad content is withdrawn or corrected through a new sequence/version. Existing published bytes and signatures are never overwritten. Public and later restricted distribution use separate storage and authorization policies.

## Delivery package

Filename: `<resource-id>--<version>--b<build-revision>.fontes.zip`.

```text
manifest.json
manifest.json.sig
metadata.json
content.sqlite
assets/cover.png
media/...
NOTICE.txt
```

The delivery manifest binds publisher/resource identity, source archive hash, compiler commit, canonical/taxonomy versions, library payload schema, required capabilities, and every entry's path, media type, length, and SHA-256. Its detached signature is 64 raw Ed25519 bytes and covers the exact manifest bytes prefixed by the `FONTES-RESOURCE-PACKAGE-V1` domain and one newline byte. Entries cover all payload files except the manifest and its signature; duplicate paths, unlisted files, and missing required payloads are errors. Trust roots ship with the app; a package cannot introduce its own trusted signing key. Reproducible compilation excludes volatile timestamps from content identity. Record a payload hash separately from catalog publication sequence.

`content.sqlite` is a validated exchange payload, not a database to attach permanently and not a migration script to run against the user's library. Trusted importer code reads only allowed tables and inserts parameterized records into the app's known schema. Extra SQL, triggers, schema objects, and executable extensions are rejected. Images are stored outside SQLite by content hash; the library has verified asset records. Reusing an asset requires compatible redistribution rights as well as identical bytes.

A standalone resource may cite other books, concepts, or Strong's codes without requiring their full resources to be installed. Cross-resource links are qualified targets with availability checks; missing optional works do not produce dangling mandatory content foreign keys. True required dependencies are explicit, cycle checked, and shown to the user with their size before installation. The initial public content types should avoid hard dependencies wherever canonical shared identities suffice.

## Installed database and recovery

Separate package archives do not require separate installed databases. Multiple SQLite files can be queried together, but foreign keys cannot cross schema boundaries. One authoritative installed database keeps paragraph, verse, concept, token, and ownership joins straightforward; it also avoids attaching an arbitrary number of resource files. [SQLite foreign keys](https://www.sqlite.org/foreignkeys.html#fk_unsupported), [ATTACH](https://www.sqlite.org/lang_attach.html).

The proposed **library schema v2** adds package ownership and paragraph identities. `specs/resource-packages/v1/library-model.sql` is an executable model of those constraints, not the complete schema or a production migration. It is not an in-place reinterpretation of the published schema v1. The main relationships are:

| Entity | Essential identity and relationships |
| --- | --- |
| Installed packages | Resource ID, publisher, version, build revision, source/payload hashes, rights and metadata |
| Content nodes | Package ID + stable local ID; ordered document/block hierarchy and text |
| Scripture links | Content-node FK, canonical start/end FKs, relation, basis, source assertion/provenance |
| Word links | Node/token FK, shared Strong's ID or qualified witness/lemma identity |
| Concept assertions | Node FK, submitted concept FK, relation, contributor and evidence |
| Concept registry | Stable IDs, labels, scope notes, relationships, status and signed registry revision |
| Concept resolutions | Submitted ID, effective ID, mapping type, review identity and revision |
| Search documents | Package/node identity; FTS and any separately permitted derived indexes |
| Package assets | Verified image identity and file hash, with package ownership |

A composed library has no single public-domain or export-permitted flag. Attribution, display/export permissions, and AI eligibility follow each package’s owned records. Installing works together does not automatically authorize exporting their combined database.

All publisher-owned translations, witnesses, source segments, tokens, articles, and assertions have explicit ownership. Shared canonical anchors and Strong's identities are never deleted when a package is removed. Shared source snapshots and concepts are garbage-collected only when no surviving records depend on them. User annotations remain in `user.sqlite`, anchored to canonical ranges and optionally to stable resource/block IDs plus a quoted fallback. If a removed resource is referenced by a note, the note survives with an unavailable-resource state.

Use the existing generation-pointer design, extended to a composed library:

```text
user.sqlite
library/
  active.json
  previous.json
  generations/<local-generation-id>/library.sqlite
  generations/<local-generation-id>/installed-set.json
  downloads/<package-sha256>.part
  staging/<job-id>/...
assets/<sha256>...
```

A Fontes signature authenticates downloaded inputs. A composed local library has a locally generated content hash and receipts for those verified inputs; it must not pretend to be the original publisher-signed SQLite snapshot.

### Installation or resource update

1. Verify catalog trust, selected identity, compatibility, permissions, dependencies, and download/expanded-size bounds. Reserve disk space for the candidate database, staging, journals, and rollback generation. An existing previous generation can mean three full library copies during an update; report the actual estimate.
2. Download to a partial file and verify its exact signed length/hash before extraction. Extract in staging, apply archive limits, verify every manifest entry and signature, and validate the payload. Failure leaves the active library untouched.
3. Acquire the library mutation lock. Create a consistent candidate copy of the active database with SQLite's backup API, or initialize a new one for first install. Freeze other library mutations during that copy; ordinary readers can continue. Copying just an open `.sqlite` file while ignoring its journal/WAL is not a backup strategy. [SQLite backup API](https://www.sqlite.org/backup.html).
4. Inside one explicit transaction in the candidate, replace only the selected package's owned records, import its new data and assertions, apply reviewed concept resolutions, update FTS, validate coverage and all foreign keys, and record the installed version/receipts. Detect any surviving hard dependency before removing old rows. Do not rely on a cascade that deletes another package's content. [SQLite transactions](https://www.sqlite.org/lang_transaction.html).
5. Run whole-candidate integrity, linking, search, ownership, and package-count checks. Close/checkpoint the candidate; flush files and directories using platform primitives. Verify its installed-set manifest and referenced assets are complete.
6. Atomically switch the active-generation pointer and preserve the previous verified pointer. New queries open the new generation; existing readers finish on the old handle. The pointer and its database's installed-package set are the installed-state authority; avoid a second independently committed installed-version list in `user.sqlite`.
7. On startup, validate activation and recover from an incomplete pointer/staging operation using the previous generation or bundled seed. Keep one known-good previous generation until health checks and reader release permit cleanup. Garbage-collect unreferenced assets only after accounting for active, rollback, staging, and in-flight readers.

Uninstall follows the same staged process, deleting owned content and derived indexes, preserving shared identities and user data, and either refusing removal of required dependencies or asking for an explicit dependent removal selection. A failed install/upgrade/uninstall is a discarded candidate, not restoration over a live database. Disk exhaustion, cancellation, process termination, malformed input, incompatible schema, and failed activation must all preserve the previous usable library.

This initial approach spends local disk I/O to make recovery simple. It does not redownload unrelated books. Future transaction-only incremental installation can reduce copy costs after equivalent fault-injection coverage exists; callers should depend on the shared library API, not a particular copy mechanism.

## Moving from current snapshots to packages

The current reading/study releases are useful bootstrap and verification fixtures. Do not replace a user's composed library with the next whole-profile snapshot: that would erase add-ons. Keep the released v1 bytes immutable while adding a separate v2 package output path.

The first package compiler should produce separate KJV, WEB, Greek Strong's, Hebrew Strong's, Robinson TR, Matthew Henry, Nave, and TSK packages from the same locked sources. Shared source provenance remains intact. Maintainer-approved locked source adapters may feed the common normalized compiler model directly, preserving structured token data that the initial third-party prose XML profile does not yet express. They still pass the same ownership, rights, integrity, and linking gates; third parties cannot submit executable adapters. `reading` becomes a bundle selecting KJV + WEB; `study` selects all eight. Both source reading streams remain in the Robinson TR package. A user can add just a commentary or dictionary later without switching a monolithic profile.

Bootstrap converts a verified v1 snapshot into package-owned v2 records through trusted code, preserving a legacy-ID-to-new-anchor map. Alternatively, a new app can seed the package-native library directly. Schema migration, legacy anchor preservation, package update tests, and mobile disk budgets must pass before this becomes the production install path. Adding design schemas in this repository does not change current release manifests or installation behavior.

## Author and maintainer tooling

The `crates/fontes-resource` Rust library and CLI implement `init`, `validate`, `pack`, and `schemas`; see the [CLI guide](../resource-publishing/cli-guide.md) for working commands. Scaffolding accepts optional book/publisher details, provides an editable cover and rights templates, and refuses existing destinations. Validation covers directories or ZIPs using embedded XSDs through `xmllint`, then checks identities, indexes, media references, decoded images, canonical mappings, coverage, word codes, and concepts. Reports contain stable error codes, file paths, one-based XML locations, a description, and corrective instructions; JSON output supports CI/editors. Non-XML/global errors have no invented line. `--deny-warnings` rejects unfinished templates and other warnings. Packing uses a frozen input snapshot and never overwrites an output archive.

Future conversion, preview, diff, submission, and delivery compilation should share the Rust core. These remaining command names are proposed interfaces:

```text
fontes-resource import --from usfm|osis|markdown <input>
fontes-resource concepts search "salvation"
fontes-resource concepts propose <source-directory>
fontes-resource preview <source-directory>
fontes-resource diff <old-source-or-payload> <new-source-or-payload>
fontes-resource submission prepare <archive>
```

Planned importers preserve source identity and flag missing metadata or uncertain mappings. Current validation checks declarations and structure; publisher authority, legal clearance, source fidelity, probable duplicate concept suggestions, and editorial approval still require future tooling or review. Planned `preview` shows reading order, cover/card/detail metadata, citation navigation, paragraph-level context results, and rights/attribution. Submission preparation will produce reviewable descriptors and a PR template; no automatic public upload occurs before the contributor selects an authorized destination.

Publisher documentation must include the full format, stable-ID/update rules, cover guidance, licensing checklist, type-specific examples, reference/versification guide, searchable taxonomy with scope notes, proposal rules, and validation error explanations. An accessible web editor can later wrap the same tools for authors unfamiliar with XML. Fontes maintainers use the same path for newly discovered resources.

## Implementation sequence and acceptance gates

1. Review this contract and the schema/example bundle; register publisher/namespace and trust-key policy. Provision the catalog repository and public versus private submission channels.
2. Extend the implemented Rust authoring validator, semantic registry checks, scaffold templates, and archive packer with preview, converters, and submission preparation. Treat XSD validation alone as insufficient.
3. Implement library schema v2, ownership, composed generations, migration from current fixtures, and cross-package context queries. Test two independent resources referring to Romans 1:3 and verify that uninstalling one preserves the other and user notes.
4. Split the existing compiler's eight sources into independently signed delivery packages; validate text and link equivalence with the current study snapshot. Publish a test catalog and taxonomy snapshots.
5. Implement catalog cards/detail/install/update/remove in both clients through a shared Rust library service. Add interruption, insufficient-space, rollback, dependency, corrupt package, and catalog replay tests.
6. Establish the concept review/enrichment workflow. Demonstrate that an approved concept mapping improves both old installed packages without changing their source archives.
7. Add paid resources only after publisher agreements, platform purchase requirements, entitlement restoration, restricted delivery, offline license terms, and permitted AI/export behavior have a separate reviewed design.

The format succeeds when an independent author can create and validate a package, a maintainer can review and publish it, and an offline client can install it so the same verse and concept queries immediately discover its paragraphs. Resource removal and updates must be equally well defined.
