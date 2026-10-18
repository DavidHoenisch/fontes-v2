# Fontes

Offline Bible study data and application foundations. The Rust pipeline compiles locked public domain Bible texts, dictionaries, morphology, commentary, topics, and cross-references into a versioned SQLite database with full-text search and permanent canonical references.

Start with **[the Fontes system specification](SPEC.md)** for product requirements, non-functional requirements, user flows, UI decisions, schema authorities, architectural choices, acceptance criteria, and implementation status. [Specification history and supporting documents](docs/specifications/README.md) include a complete Markdown transcription of the original Word baseline.

## Run the application

The first engine-first Rust application is implemented with egui/eframe and wgpu. It reads real reading/study snapshots, supports passage navigation, KJV/WEB switching, local keyword search, persistent notes/highlights, and linked study resources. A toolkit-independent command engine is shared by the UI, CLI, JSON control and MCP stdio.

```sh
cargo run --locked -p fontes
# First launch: Reading or Study → Continue → Download and open.
# Or open a trusted locally compiled reading profile:
cargo run --locked -p fontes -- --library dist/reading/library.sqlite
# Or use a compiled study profile:
cargo run --locked -p fontes -- --library dist/study/library.sqlite
```

See the [application guide](docs/app/README.md) for first-run data, mobile preview, headless commands and Android APK builds, and the [architecture](docs/app/architecture.md) for crate boundaries and compatibility. The responsive UI uses the approved local assets. [Download the development builds](https://github.com/DavidHoenisch/fontes-v2/releases/tag/app-v0.1.0-dev.3.1) for Windows x64, Linux x64, macOS Apple Silicon/Intel, and Android arm64, with [test setup instructions](docs/app/testing-builds.md). All platform Actions jobs passed; physical-device/native-window and accessibility checks remain. [Guided first-run download](docs/app/first-run-setup.md) is implemented; automatic updates, the add-on catalog, AI and curated genealogy/overview features are later milestones.

## Extensible library design

The [resource package and catalog addendum](docs/specifications/library-and-packages-v0.2.md) designs independent downloads, a shared installed library, paragraph-level links, and a growing concept registry. [Draft contracts and a validated example](specs/resource-packages/v1/README.md) include XML XSDs, catalog/delivery JSON Schemas, and an executable linking model. These are design artifacts; the public catalog, resource compiler, and app installer are future work. Current signed reading/study releases remain unchanged.

See the [resource publishing guides](docs/resource-publishing/README.md) for authoring, taxonomy proposals, maintainer review/publication, client integration, and implementation status.

The implemented [resource CLI](docs/resource-publishing/cli-guide.md) scaffolds packages with optional book/publisher metadata, validates XML and referenced assets with file/line/column diagnostics, creates deterministic source ZIPs, and exports its embedded XSDs. Install with `cargo install --path crates/fontes-resource --locked`; strict validation also requires `xmllint` on `PATH`.

The [submission/release pipeline](docs/resource-publishing/submission-pipeline.md) validates `library/submissions/` on PRs, prepares reproducible previews, and publishes hash-approved source packages as signed prereleases from the trusted branch. Its reusable workflows support a separate/private policy and storage repository. Source-stage releases are not app-installable SQLite delivery packages.

## Build the reading data

The pinned Rust toolchain is installed through rustup. The first build downloads approximately 10 MB of source archives and checks their exact SHA-256 digests and sizes.

```sh
cargo test --workspace --locked
cargo build --release --locked -p fontes-data-pipeline
target/release/fontes-data build \
  --profile reading \
  --output dist/reading \
  --builder-commit "$(git rev-parse HEAD)" \
  --mirror-repository DavidHoenisch/fontes-v2
target/release/fontes-data validate dist/reading/library.sqlite
```

The output directory must be new or empty. A verified content-addressed source cache lives in `.data-cache/`. Use `--offline` to rebuild from that cache without any network access. If upstream replaces a locked archive, the pipeline can recover that exact archive from its retained assets on earlier data releases. It fails if the required locked bytes cannot be recovered; it never substitutes a changed download.

The current reading profile contains **31,102 canonical anchors and 62,204 verse rows**, across 66 books and two translations, plus **31,153 TSK cross-reference entries**. Building it now requires the SWORD extraction bridge described below. Legacy signed reading-only packages remain supported; newly compiled TSK data requires a new signed release to reach installed Reading libraries. `verses_by_reference` links source verse records to canonical IDs. The WEB Romans doxology is mapped from source Romans 14:24–26 to canonical Romans 16:25–27. Four empty WEB verses are represented explicitly as source omissions. The reviewed rules are in `data/mappings/reading-v1.json`; no text is invented to fill those entries.

The first adapter imports verse-per-line XML. It preserves verse text after trimming surrounding whitespace, including KJV paragraph symbols, but does not yet import USFM paragraph, poetry, or footnote structure. User notes and highlights belong in a separate database defined by `data/schema/user-v1.sql`.

## Build the complete default free resources

New installations use this complete bundle with one **Install free resources** prompt. The `study` ID is retained internally for compatibility; users do not choose between Reading and Study. Compiler and setup defaults select this full bundle. Explicit reduced-profile builds remain available for historical clients and maintenance.

Before the additional dictionaries below, the study profile included reading plus Strong’s Greek (5,624 entries), Strong’s Hebrew (8,674), Robinson TR Greek morphology (all 7,957 NT verses), Matthew Henry (5,360 articles), Nave’s (5,322 entries), and TSK (31,153 entries). That baseline contains 64,090 resources, 281,240 Greek tokens across both source reading streams, 281,307 analyses, and 385,194 TSK target ranges. The updated Study profile additionally includes 305,507 OSHB Hebrew words and 349,297 edition-bound KJV dictionary-tag spans. The locally verified download is approximately 136 MB; SQLite occupies approximately 733 MB (publication builds record their exact sizes).

Install **SWORD 1.9.0** as a build dependency and compile the small extraction bridge. Applications only need SQLite, zstd, and the shared update contracts.

```sh
# macOS
brew install sword
mkdir -p target
c++ -std=c++17 -O2 -Wall -Wextra -Werror \
  -isystem /opt/homebrew/include/sword tools/sword-export.cpp \
  -L/opt/homebrew/lib -lsword -o target/fontes-sword-export

# Ubuntu 24.04 alternative
# sudo apt-get install libsword-dev pkg-config
# c++ -std=c++17 -O2 -Wno-unused-parameter $(pkg-config --cflags sword) \
#   tools/sword-export.cpp $(pkg-config --libs sword) -o target/fontes-sword-export

target/release/fontes-data build \
  --profile study --output dist/study \
  --builder-commit "$(git rev-parse HEAD)" \
  --mirror-repository DavidHoenisch/fontes-v2
target/release/fontes-data validate dist/study/library.sqlite
```

Set `FONTES_SWORD_EXPORTER` to use another bridge path. Its protocol and SWORD version are checked before importing. Greek first/second reading witnesses preserve the source’s alternatives, spelling variants, multiple analyses, and compound Strong’s associations. Dictionary headwords remain distinct from supplied lemmas; absent lemmas, Hebrew morphology, and English word alignments are not inferred. Greek subscriptions and alternate verse-numbering notes remain in structured source apparatus.

The SWORD adapter visits every key, retains linked commentary coverage, and preserves Nave’s duplicate headings with occurrence suffixes (`SIN#2`, `REVERENCE#2`). Rust extracts plain paragraphs directly from markup, retaining prose that SWORD’s display filters can suppress. TSK’s `Jud` means Judges; `Jude` means Jude. The bridge disambiguates those before parsing relative references.

The locked sources contain **439 citation elements with invalid, ambiguous, reversed, or out-of-registry targets** (353 Matthew Henry, 26 Nave, 60 TSK). Their source text and diagnostics are retained; valid parts of those lists remain linked. The exact baseline in `data/mappings/study-v1.json` makes unexpected diagnostic changes fail the build. `study-imports.json` publishes the audit. These counts describe citation elements, some containing several targets. No textual correction is invented.

## Query study content

Use the canonical anchor as the entry point. This retrieves articles applying to John 3:16 and topics/commentary that cite it:

```sql
SELECT DISTINCT r.id, r.type, r.title, r.content, l.link_type
FROM reference_ranges rr
JOIN resource_links l ON l.range_id = rr.id
JOIN resources r ON r.id = l.resource_id
WHERE rr.start_ref_id <= 43003016 AND rr.end_ref_id >= 43003016;
```

Greek word study joins a specific reading stream to its own analyses and definitions:

```sql
SELECT t.position, t.surface, a.analysis_index, a.morph_code,
       ts.strongs, d.title, d.content
FROM segment_refs sr
JOIN original_tokens t ON t.segment_id = sr.segment_id
JOIN token_analyses a ON a.token_id = t.id
JOIN token_strongs ts ON ts.token_id = a.token_id
                         AND ts.analysis_index = a.analysis_index
JOIN resource_links dl ON dl.strongs = ts.strongs
                          AND dl.link_type = 'definition'
JOIN resources d ON d.id = dl.resource_id
WHERE sr.canonical_ref_id = 43001001
  AND t.witness_id = 'witness:robinson-tr-first'
ORDER BY t.position, a.analysis_index, ts.strongs;
```

For TSK, first select its resource through an `applies_to` link for the origin verse, then read that resource’s `cross_reference` links for target ranges. Keep these link types distinct. Resource search uses `search_fts` joined to `search_documents.resource_id`.

## GitHub releases

[Data releases](https://github.com/DavidHoenisch/fontes-v2/releases) use tags such as `data-v1-stable-1`. Pushing a relevant compiler, source, schema, or workflow change to `main` runs `.github/workflows/data-release.yml`; the workflow can also be dispatched manually.

The workflow tests the workspace, builds the complete default bundle twice, compares SQLite, compressed bytes and audit reports, and passes checked outputs to a separate publishing job. Only that job receives the signing secret and repository write permission. The publisher allocates the next data sequence, signs the manifest, creates a draft release, verifies every uploaded asset, publishes, and downloads/verifies the published assets again. It never overwrites published releases and sets `make_latest=false` to keep data releases separate from app releases. Identical content from the same compiler commit is a no-op.

Published assets include:

- `fontes-study.sqlite.zst`: one complete free-resource snapshot. The internal `study` name is retained for compatibility; no separate Reading download is built by the release workflow.
- `release-manifest.json` and its raw Ed25519 signature, covering the database hashes, lengths, versions, source identities, and capabilities.
- `sources.lock.json`, `NOTICE.txt`, the complete bundle validation report, and `study-imports.json`. The bundle also retains `GPL-3.0.txt` and `KJV-ANNOTATIONS-NOTICE.md`.
- `changes.json`, including verse/resource changes, the default profile and explicit retirement of the separate Reading download.
- All ten exact source inputs, named by SHA-256 (`.xml` dictionaries and `.zip` archives), for future rebuilds.

Verify a downloaded release using the public key committed in this repository:

```sh
gh release download data-v1-stable-2 --repo DavidHoenisch/fontes-v2 --dir dist/downloaded
target/release/fontes-data verify \
  --dir dist/downloaded \
  --trust data/trust/data-release-1.json \
  --repository DavidHoenisch/fontes-v2
```

The verifier authenticates the original manifest bytes before parsing, checks source provenance, caps decompression at 1 GiB per SQLite artifact and 256 MiB compressed, checks both artifact hashes, and validates SQLite references, coverage, integrity, and search. It does not install content or change user data. A future shared app updater will add atomic activation and recovery.

The publisher expects the `FONTES_DATA_SIGNING_KEY` GitHub repository secret. Its value is a 32-byte Ed25519 seed encoded as hex, matching `data/trust/data-release-1.json`. Private keys belong outside Git. Key generation and repository-secret setup have been performed for the initial release; normal runs do not regenerate keys.

## Further development

Reading and study are published together so a new study release preserves the reading update channel. The publisher accepts `publish --dir dist/reading --additional-dir dist/study` with the usual trust and repository arguments. Scheduled source discovery, reviewed update PRs, and mobile/desktop installation are specified in the [architecture](docs/architecture/data-pipeline.md). See [source research](docs/research/public-domain-data-sources.md) for content and rights decisions.

### Additional default dictionaries

New default builds also include **Abbott-Smith’s Greek Lexicon (1922), 6,153 entries**, and **Easton’s Bible Dictionary (1897 third edition), 3,963 entries**. Source bytes, rights statements, import behavior and unresolved-citation limits are documented in [the dictionary integration review](docs/research/abbott-smith-easton.md). Existing published bundles remain compatible; these additions reach existing users through a new data snapshot, not an APK alone.
