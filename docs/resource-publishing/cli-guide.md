# Resource authoring CLI

`fontes-resource` implements scaffolding, strict source validation, deterministic source ZIP packaging, and schema export. It supports the draft prose/verse format in this repository. Delivery SQLite compilation, preview/conversion, catalog publication, and app installation are separate future work.

It also implements `submissions check`, `submissions build`, `submissions publish`, and `submissions verify` for the [GitHub source-submission pipeline](submission-pipeline.md). Source-release publication is distinct from the app delivery/catalog stages.

## Install

From a checkout of this repository with the [pinned Rust toolchain](../../rust-toolchain.toml):

```sh
cargo install --path crates/fontes-resource --locked
fontes-resource --help
```

Strict XSD validation requires libxml2's `xmllint` on `PATH`. On Ubuntu/WSL, install `libxml2-utils`. On macOS, check `xmllint --version`; if unavailable, install libxml2 and expose its `bin` directory on `PATH`. A native Windows installation must also provide `xmllint.exe`, or use WSL with the same Linux instructions. The CLI embeds the exact XSDs and canonical registry used at build time, so it can operate outside the repository without network access. It does not download schemas selected by the package. See the [xmllint manual](https://gnome.pages.gitlab.gnome.org/libxml2/xmllint.html) for the external validator.

For development without installing the binary, use `cargo run --locked -p fontes-resource -- <command> ...`.

## Scaffold a package

```sh
fontes-resource init dist/my-romans-study \
  --id org.example.my-romans-study \
  --publisher-id org.example \
  --kind study-guide \
  --title "A Study of Romans" \
  --author "Example Author" \
  --publisher "Example Press" \
  --publication-date 2026-10-15 \
  --language en \
  --start Rom.1.1 --end Rom.1.7
```

Only the destination, resource ID, and publisher ID are required. The example namespace is fictional. `--kind` supports translation, commentary, lexicon, word-study, study-guide, devotional, and book. Repeat `--author` for multiple authors. Additional options include `--subtitle`, `--description`, `--edition`, `--version`, `--source-versification`, `--license-id`, `--rights-holder`, `--license-file`, and `--attribution-file`. Publication dates accept a year, year-month, or date as defined by the metadata XSD. Run `fontes-resource init --help` for all options.

The command creates `metadata.xml`, `index.xml`, an initial content document, cover artwork, rights files, and empty `media/` and `taxonomy/` directories. The initial selected coverage and block annotation use the supplied anchors. A translation gets a verse block with source identity and a `text-of` mapping. XML-sensitive characters in titles/names are escaped correctly. Invalid parameters fail before creating a package, and an existing destination is refused.

By default the artwork is a CC0 placeholder. Supply a real cover with `--cover PATH`, `--cover-license-id`, `--cover-rights-holder`, `--cover-attribution`, and `--cover-alt`. Custom cover bytes must decode as PNG/JPEG at 1200 × 1800, no larger than 4 MiB.

Scaffolding deliberately leaves text permissions as `deny`, rights statements as TODOs, and unfinished content marked `FONTES-TODO`. Copying a license file does not infer permission flags. Edit the actual text, rights, attribution, and metadata before submission. A structurally valid scaffold is still an unfinished draft.

## Validate a directory or ZIP

```sh
fontes-resource validate dist/my-romans-study

fontes-resource validate specs/resource-packages/v1/examples/romans-study \
  --taxonomy specs/resource-packages/v1/taxonomy.example.xml \
  --deny-warnings
```

The second command checks the complete repository example. The taxonomy fixture is illustrative, not a published authoritative registry. A package using central concept IDs needs an explicit local registry via `--taxonomy`; the CLI never silently trusts a package-supplied registry. Publisher concepts belong in its own `taxonomy/concepts.xml`.

Validation checks the XSD vocabulary, attributes, values and element order, UTF-8/XML syntax, resource/index identities, globally unique local IDs, referenced content/media, cover bytes/dimensions, safe archive paths, actual expanded byte limits, file inventory, scripture existence/range order, verse mappings/complete translation coverage, normalized Strong's codes, publisher concept ownership, labels, targets, and broader cycles. References use the [frozen canonical registry](../../data/canonical/registry-v1.json); valid syntax alone is insufficient.

Image references are relative to the **package root**, not the content file's directory. A missing image produces output like:

```text
content/rom/001.xml:27:18: error[MISSING_FILE]: Referenced file 'media/map.png' does not exist in the package.
  help: Add the file at this exact case-sensitive path, or correct/remove the XML reference.
Package validation failed.
```

Locations are one-based. Semantic XML errors point to the relevant attribute or element. Malformed XML includes parser line/column; XSD errors retain the validator's line and a column when the element can be located. Binary, archive-wide, and missing-root-file errors have no invented XML location. Semantic errors are collected together after XML/schema checks pass; fix structural errors first, then rerun for cross-file checks.

For editor/CI integration:

```sh
fontes-resource validate dist/my-romans-study \
  --format json --report dist/my-romans-study-validation.json
```

The JSON report has `report_version: 1`, `valid`, optional resource identity/version, and diagnostics containing `severity`, `code`, `file`, `line`, `column`, `message`, and `help`. Keep reports outside the source tree; an existing report path is refused. Exit codes for `validate` are 0 for a valid package, 1 for validation failure, and 2 for invocation/operational failure. `--deny-warnings` turns unfinished templates and other warnings into a failed result. Other commands return 2 on failure.

## Package and export schemas

```sh
fontes-resource pack dist/my-romans-study \
  --output dist/org.example.my-romans-study--1.0.0.fontes-source.zip \
  --deny-warnings

fontes-resource schemas --output dist/fontes-resource-schemas
```

Pass `--taxonomy PATH` to `pack` when central concepts are used. Packaging validates a private snapshot and packages those exact bytes, with sorted file order, fixed timestamps, and fixed permissions. It prints the archive path, SHA-256, and length as JSON. The output filename must match metadata identity/version. Invalid sources create no output ZIP; an existing ZIP is never overwritten. Keep output outside the source tree. The same CLI build and inputs produce identical bytes; the pinned toolchain/lockfile define that build.

Schema export creates a new directory containing the embedded XSDs, including local includes. It refuses an existing directory. This is useful for XML editor completion; full validation still requires the CLI's semantic checks.

## Common errors

| Code | Fix |
| --- | --- |
| `XML_PARSE`, `XML_ENCODING`, `XML_DEPTH`, `XML_INSTRUCTION` | Correct syntax/encoding or reduce nesting; remove DTD/entity declarations and processing instructions |
| `XML_SCHEMA`, `XML_NAMESPACE` | Follow the named element/attribute type and order in the bundled XSD |
| `MISSING_FILE`, `REFERENCE_PATH` | Add the exact referenced file or correct its root-relative path; images belong under `media/` |
| `REQUIRED_FILE`, `UNLISTED_FILE` | Restore required files; index content, reference media, or remove extras |
| `RESOURCE_NAMESPACE`, `RESOURCE_MISMATCH`, `DOCUMENT_IDENTITY`, `IDENTITY_WHITESPACE` | Make publisher, resource, and indexed document identities agree without whitespace |
| `EMPTY_VALUE` | Supply meaningful descriptive text or image alternative text |
| `DUPLICATE_ID`, `DUPLICATE_PATH` | Use unique stable IDs and paths, including when paths differ only by case |
| `SCRIPTURE_RANGE`, `VERSE_MAPPING`, `COVERAGE_SCOPE`, `INCOMPLETE_COVERAGE` | Use real ordered canonical anchors and explicit verse mappings/omissions; correct coverage declarations |
| `STRONGS_CODE` | Use `G0001`–`G5624` or `H0001`–`H8674` with canonical padding |
| `UNKNOWN_CONCEPT`, `CONCEPT_AUTHORITY`, `CONCEPT_RELATION`, `CONCEPT_CYCLE`, `PREFERRED_LABEL` | Select a registry, declare owned concepts, correct labels/targets, and remove parent cycles |
| `COVER_DIMENSIONS`, `IMAGE_SIZE`, `IMAGE_DECODE` | Supply actual valid images; a metadata declaration cannot repair invalid bytes |
| `UNSAFE_PATH`, `FILE_TYPE`, `ARCHIVE_ENTRY`, `SIZE_LIMIT`, `ENTRY_LIMIT` | Remove links/special files, unsafe names, encryption, or excessive input |
| `PLACEHOLDER`, `PUBLIC_CATALOG_RIGHTS`, `WARNINGS_DENIED` | Finish the draft and obtain the necessary grants; never change rights merely to silence a warning |
| `XMLLINT_UNAVAILABLE` | Install `xmllint` and ensure it is on `PATH` |

Content PNG/JPEG images are limited to 16 MiB, 8192 pixels per dimension, and 16,777,216 total pixels; covers use the stricter rule above. ZIP/directory limits are 10,000 entries, 16 MiB/XML file, 1 GiB expanded total, and at most 32 path components. DTDs, external entities, symlinks, special files, case collisions, executable payloads, and unlisted files are rejected.

This validator checks authoring conformance. It cannot establish publisher ownership, legal permission, source fidelity, or editorial approval. Passing validation creates no signed delivery package and publishes nothing.
