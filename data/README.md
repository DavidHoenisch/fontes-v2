# Fontes data contracts

The current product and architecture requirements are in [SPEC.md](../SPEC.md); its schema registry identifies these versioned SQL/JSON/Rust files as the exact contract authorities.

The pipeline design is in [the architecture document](../docs/architecture/data-pipeline.md). This directory defines the initial content and client update contracts. The shared Rust types are in `crates/fontes-data-contracts`.

- `canonical/registry-v1.json` freezes the initial 66-book reference identities and all chapter lengths. Never regenerate or renumber it during an upstream refresh. Its 31,102 verse anchors were verified from the recorded KJV snapshot.
- `sources.lock.json` records eight inspected input files with exact sizes and SHA-256 digests. Reading consumes KJV and WEB; study also consumes both Strong’s dictionaries, the full Robinson TR archive, Matthew Henry, Nave’s, and TSK. Rights review IDs refer to the dated source research.
- `profiles.json` defines required membership and capabilities. A profile with a missing adapter, lock, or required source must fail its build. Both profiles are implemented and published in a shared signed release.
- `schema/content-v1.sql` defines the immutable published SQLite snapshot. `schema/user-v1.sql` defines a separate mutable database for annotations and saved references.
- `examples/release-manifest.json` is an **unsigned illustrative test fixture**. Its sizes, content digests, builder commit, timestamp, and tag do not describe a published release. Never install or publish this fixture.

`mappings/reading-v1.json` contains reviewed WEB verse-numbering and omission rules. `trust/data-release-1.json` contains the trusted release public key; no private seed belongs here.

Run `cargo test --workspace --locked` for contract and pipeline tests. The reading/study compiler, signed publisher, and package verifier are implemented in `crates/fontes-data-pipeline`. The data release workflow compiles real inputs twice, checks reproducibility, and publishes verified release assets. Scheduled source discovery and atomic client installation remain implementation work.

`mappings/study-v1.json` freezes known upstream citation anomalies by source digest and record key. A changed anomaly set fails the build until explicitly reclassified. Invalid targets remain in source text and the release audit; valid targets in the same list still receive links. See the root README for build dependencies and query examples.
