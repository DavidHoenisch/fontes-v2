# Abbott-Smith and Easton integration

Reviewed 2026-10-17. The owner selected these public-domain resources instead of Vine’s. Vine’s remains pending; these are distinct works, not substitutes for its text.

## Locked sources and rights

- **Abbott-Smith, A Manual Greek Lexicon of the New Testament (1922)**: `abbott-smith.tei.xml` at upstream commit `8c00cb244761aa23659421a8cbbbf7a3b27b7d59`. The [source project](https://github.com/translatable-exegetical-tools/Abbott-Smith#source-and-copyright) expressly places its XML transcription in the public domain. Its PDF has a separate notice and is not imported. 5,698,817 bytes; SHA-256 `99821bee411626b4aa3e674040297b54b267eff86cb4f9b7ef69a8bdb18b6f90`. Preserve author G. Abbott-Smith and project provenance.
- **Easton, Illustrated Bible Dictionary, third edition (1897)**: [CrossWire Easton](https://www.crosswire.org/sword/modules/ModInfo.jsp?modName=Easton), module 2.0.1. The locked module configuration identifies this edition and declares Public Domain / Copy Freely. The digital module excludes illustrations. 1,437,674 bytes; SHA-256 `f6dd054554764e2e97d5d189a697eb26039054578a9ccf98ce668ab810341c6e`. Preserve author M. G. Easton and CrossWire provenance.

The machine-readable lock and `source-validation.json` record the exact acquisition bytes and evidence. Source-scoped rights review IDs are `research-2026-10-17-abbott-smith` and `research-2026-10-17-easton`.

## Import and linking

Abbott-Smith uses a dedicated offline TEI adapter, not a PDF extraction. It retains all **6,153 entries**, including 257 without an upstream `n` key. The latter use headword plus entry ordinal in this locked transcription; changing that revision requires explicit identity review. Standard numeric G labels are normalized to existing Strong’s IDs and linked as definitions. Extended labels such as `G0901a` remain supplied metadata, without stripping their suffix or inventing a standard-code alignment. Source headwords are dictionary headwords, not inferred lemmas or English-to-token alignments.

Easton uses the existing pinned SWORD 1.9.0 build-only exporter and imports **3,963 entries**. Both adapters retain source text, titles and explicit Scripture citations. `Bible:` is a TEI OSIS work qualifier, accepted before canonical resolution. Missing/noncanonical/reversed/out-of-range targets are diagnostics, not silently repaired links. Valid targets in partly malformed citation lists are retained.

`data/mappings/study-dictionaries-v1.json` independently pins source hashes, record/citation counts and the exact diagnostic baseline: 2,990 Abbott-Smith and 29 Easton citation elements remain unresolved. Many Abbott-Smith references name noncanonical books or LXX material. Some Easton targets in the upstream transcription disagree with surrounding abbreviated prose (for example, Genesis targets amid Revelation citations); supplied targets/text are preserved without a claimed contextual correction. Structural linking is not a full textual collation.

## App integration and compatibility

The complete default (`study`) build includes both sources. The existing release workflow consumes the expanded source lock/profile and checks reproducibility. Current source sets must include both dictionaries; exact previously published 8/9/10-source Study sets remain accepted. Partial new sets are rejected. Source checks and citation-policy digests contribute to build identity. Published SQL schemas, canonical IDs and existing source IDs are unchanged.

Library gives each work its own cover/title, installed count and paged entries. Study → Dictionaries shows entries explicitly citing the current verse; resource search includes their prose. Linked word study offers multiple dictionary definitions, keeping Strong’s as the initial choice and allowing Abbott-Smith where the source supplies that G association. Dictionary selections retain attribution for note collection. Supplied citations preview Scripture in place.

Compiled local data, APK installation and signed public release are separate steps. Do not infer availability in existing installed libraries from an APK build: a new data snapshot must also be installed. Local verification artifacts are not a signed/public release.

## Verification completed locally

- Two offline complete Study builds produced byte-identical SQLite and zstd packages: 74,206 resources; 784,420,864 bytes SQLite, 146,677,702 bytes compressed.
- SQLite SHA-256: `b627632384ce1d940bed2a29268fe0480275261615a4329610993f7ddb3ce562`; package SHA-256: `a965324d153b003f2dd9a71dd80fa251d6e2fbbffbe9d29ec200b53ae2d37594`.
- The real engine resolves G3056 to both Abbott-Smith’s λόγος and Strong’s entries. Source articles and exact attributed excerpts were loaded and saved through `note.save` using an isolated review user database.
- 5,483 explicit standard-code definition links imported for Abbott-Smith. Source counts and unresolved-citation baselines match the reviewed policy.
- Contracts, pipeline and engine tests pass (61 tests, one separate ignored source test); Clippy with warnings denied passes. All 14 frontend tests and the frontend production build pass. Android APK and macOS bundles build.
- Outputs: ignored `dist/study-dictionaries-complete/` and `dist/study-dictionaries-repro/`. Builder commit is the local declared base revision; these uncommitted working-tree builds are local verification artifacts, not signed/public release artifacts. Physical-device installation of this dictionary snapshot has not been verified; ADB had no connected phone at completion.
