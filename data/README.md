# Fontes data contracts

The current product and architecture requirements are in [SPEC.md](../SPEC.md); its schema registry identifies these versioned SQL/JSON/Rust files as the exact contract authorities.

The pipeline design is in [the architecture document](../docs/architecture/data-pipeline.md). This directory defines the initial content and client update contracts. The shared Rust types are in `crates/fontes-data-contracts`.

- `canonical/registry-v1.json` freezes the initial 66-book reference identities and all chapter lengths. Never regenerate or renumber it during an upstream refresh. Its 31,102 verse anchors were verified from the recorded KJV snapshot.
- `sources.lock.json` records ten inspected input files with exact sizes and SHA-256 digests. Reading consumes KJV and WEB; study also consumes both Strong’s dictionaries, the full Robinson TR archive, Matthew Henry, Nave’s, TSK and owner-approved CC BY 4.0 OSHB Hebrew words/morphology. Rights review IDs refer to dated source research and `rights/oshb-hebrew-2026-10-16.md`.
- `profiles.json` defines required membership and capabilities. A profile with a missing adapter, lock, or required source must fail its build. Both profiles are implemented and published in a shared signed release.
- `schema/content-v1.sql` defines the immutable published SQLite snapshot. `schema/user-v1.sql` defines a separate mutable database for annotations and saved references.
- `examples/release-manifest.json` is an **unsigned illustrative test fixture**. Its sizes, content digests, builder commit, timestamp, and tag do not describe a published release. Never install or publish this fixture.

`mappings/reading-v1.json` contains reviewed WEB verse-numbering and omission rules. `trust/data-release-1.json` contains the trusted release public key; no private seed belongs here.

Run `cargo test --workspace --locked` for contract and pipeline tests. The reading/study compiler, signed publisher, and package verifier are implemented in `crates/fontes-data-pipeline`. The data release workflow compiles real inputs twice, checks reproducibility, and publishes verified release assets. Scheduled source discovery and atomic client installation remain implementation work.

`mappings/study-v1.json` freezes known upstream citation anomalies by source digest and record key. A changed anomaly set fails the build until explicitly reclassified. Invalid targets remain in source text and the release audit; valid targets in the same list still receive links. See the root README for build dependencies and query examples.

`mappings/hebrew-oshb-v1.json` binds the exact OSHB archive digest to reviewed partial-verse boundaries, Psalm superscriptions and a known source omission. Full reference shifts come from the pinned source’s VerseMap.xml. Source Strong’s codes and morphology do not establish English token alignments. The Study capability now includes Hebrew morphology; the updated client also accepts the complete legacy eight-source Study set. Public signed release availability must be recorded separately from local builds.

The current Study profile has ten sources. The separately approved GPL KJV annotations are reviewed in [the rights record](rights/kjv-strongs-2026-10-16.md); the full GPL text and editable USFM archive accompany the build. [The dictionary-tag map](mappings/kjv-strongs-v1.json) withholds 1 John 2:23 rather than attaching tags across a text mismatch. These dictionary associations do not populate original-token alignment tables. Hebrew words retain their separate CC BY 4.0 attribution. Published SQL and canonical IDs are unchanged.
