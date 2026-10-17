# OSHB Hebrew source review · 2026-10-16

Owner decision in this implementation session: **Allow attribution-licensed data**. This revises the public-domain-only annotation policy for reviewed CC BY 4.0 Hebrew data; it is not permission to infer that other datasets or annotation layers are public domain.

- Source: Open Scriptures Hebrew Bible (`openscriptures/morphhb`).
- Pinned revision: `3d15126fb1ef74867fc1434be1942e837932691f`.
- Archive SHA-256: `f56c150708b5d74719ecb709c712c31eae9855bca7b111fd82ec91b5d177b4c7`; 20,870,584 bytes.
- [Pinned license](https://github.com/openscriptures/morphhb/blob/3d15126fb1ef74867fc1434be1942e837932691f/LICENSE.md): CC BY 4.0, permitting redistribution and adaptation, including commercial use, with attribution. The underlying WLC text remains public domain.
- Required attribution: “Original work of the Open Scriptures Hebrew Bible available at https://github.com/openscriptures/morphhb”.

Import scope: primary direct verse words, stable source word IDs, supplied augmented Strong’s codes and OSHM morphology from `wlc/*.xml`; explicit full mappings from `wlc/VerseMap.xml` and reviewed partial-word boundaries in `data/mappings/hebrew-oshb-v1.json`. Preserve Unicode sequences without NFC normalization. Note-contained variant readings remain in the retained source archive and are not mixed into the primary token stream. Prefix-only lemma components do not become invented Strong’s entries or dictionary headwords.

Modification notice: converted OSIS into SQLite/JSONL, explicitly mapped MT references to canonical KJV anchors and split source segments at reviewed partial-verse boundaries. Psalm superscriptions retain their source identity and map to canonical verse 1, where the locked English KJV includes their text. Nehemiah 7:68 has no source words and remains an explicit omission. No English-to-Hebrew token alignment or endorsement is claimed.

The exact source archive is retained with build outputs. Attribution, license URL and modification notice are emitted in `NOTICES.txt` and made available in the app’s Hebrew source/credits disclosure. The new Study source set needs the updated bootstrap client; previously published eight-source Study snapshots remain accepted explicitly. Published content/user SQL and canonical IDs are unchanged.
