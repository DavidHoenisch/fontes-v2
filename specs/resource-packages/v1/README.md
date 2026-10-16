# Fontes resource package draft contracts

See [SPEC.md](../../../SPEC.md) for the consolidated system requirements, design decisions, flows, implementation status, and schema registry.

These files accompany the [extensible library specification addendum](../../../docs/specifications/library-and-packages-v0.2.md). They define a **proposed** XML authoring format and catalog/delivery metadata contracts. They do not implement a live catalog, the production Rust resource compiler, paid distribution, or the app installer.

The [publishing guides](../../../docs/resource-publishing/README.md) explain authoring, concept proposals, maintainer review/publication, and mobile/desktop integration.

The Rust [authoring CLI](../../../docs/resource-publishing/cli-guide.md) implements `init`, `validate`, `pack`, and `schemas` for source directories and archives, including actionable file/line/column diagnostics. It uses these embedded XSDs plus semantic checks; `xmllint` is a runtime dependency for strict XSD validation.

## Files

| File | Purpose |
| --- | --- |
| `metadata.xsd` | Work identity, contributors, publisher, cover, rights, coverage, and compatibility references |
| `index.xsd` | Ordered documents and optional navigation sections |
| `content.xsd` | Stable document/block identities, restricted inline markup, and typed annotations |
| `concepts.xsd` | Namespaced publisher proposals or a registry snapshot |
| `common.xsd` | Shared types; local schema include, never a package-chosen remote dependency |
| `catalog.schema.json` | New signed library catalog protocol, including card metadata, covers, and package identities |
| `delivery-manifest.schema.json` | Compiled resource payload identity and per-file integrity descriptors |
| `library-model.sql` | Executable ownership/linking model for review; not a complete production schema or migration |
| `taxonomy.example.xml` | Illustrative core concepts, aliases, scope notes, and broader relationships |
| `examples/romans-study/` | A complete small source directory with two documents, six blocks, attribution/license, a 1200 × 1800 PNG cover, and a concept proposal |
| `catalog.example.json`, `delivery-manifest.example.json` | Unsigned synthetic fixtures with example identities, zero hashes, and unavailable URLs; never install or publish them |
| `check_examples.py` | Contract/example checks and deterministic example ZIP generation |
| `submission.schema.json`, `publication-policy.schema.json`, `approval.schema.json` | Implemented source-stage descriptor, trusted repository policy, and exact-hash maintainer approval |
| `source-release-manifest.schema.json` | Implemented signed source-release inventory; separate protocol/domain from app delivery and catalog |

The JSON fixtures are for structural validation only. The example delivery manifest does not describe a real compiled SQLite payload. XSDs do not prove that paths are safe, references exist, licenses are valid, or archive contents are complete. Those checks belong in the semantic validator and editorial workflow.

## Validate and package the example

From the repository root, with Python 3 and `xmllint` installed:

```sh
python3 specs/resource-packages/v1/check_examples.py \
  --pack dist/resource-package-design
```

The command checks the XSDs, indexed files, global node IDs, actual canonical references, proposed concept ownership, concept graph, cover dimensions, and exact fixture file inventory. Negative fixtures check unsupported markup, duplicates, invalid dimensions/references, unsafe paths, and concept namespace/cycle violations. The isolated SQLite model verifies cross-package reference queries, concept resolution without source rewrites, transaction rollback, staged uninstall, search cleanup, and separate user-note preservation. These tests do not simulate app process termination or filesystem activation; production fault-injection tests remain necessary.

It also creates `dist/resource-package-design/org.example.romans-study--1.0.0.fontes-source.zip` with fixed timestamps and file ordering, and checks that repeated packaging yields identical bytes. The source archive contains the files at its root. No private agreement, signing key, generated database, or executable is included.

To additionally verify both JSON Schema 2020-12 schemas and their positive/negative fixtures, use the pinned development-only validator:

```sh
uv run --with jsonschema==4.26.0 python \
  specs/resource-packages/v1/check_examples.py --json \
  --pack dist/resource-package-design
```

This Python checker is a design conformance aid, not a general-purpose production archive validator. The production library and client code remain Rust.

## Semantic rules beyond XSD

| Rule | Required production behavior |
| --- | --- |
| Identity | Verify publisher authority, resource namespace, immutable version bytes, and stable IDs across files/updates |
| Paths and ZIP | Reject traversal, links, duplicates, case-fold collisions, unknown payload files, excessive entries, and actual decompression/XML depth limits |
| Cover and images | Decode approved PNG/JPEG; check media type, dimensions, size, local references, and redistribution rights |
| Reading order | Require every content file once in the index and a matching document/resource ID |
| Text | Present blocks need content; only verse blocks may be explicit omissions; omissions contain no invented text |
| Bible scope | Verse source references and canonical mappings are required; a complete translation proves its declared coverage |
| Scripture | Resolve canonical OSIS IDs against the frozen registry; validate ordered endpoints; preserve relationship and assertion basis |
| Words | Normalize/range-check Strong's IDs; witness-qualified token/lemma data requires an implemented interchange profile |
| Concepts | Resolve IDs in the selected registry or declared publisher namespace, enforce scoped preferred labels, reject broader cycles, and preserve mapping provenance |
| Catalog | Reject duplicate resource IDs, dangling bundle members, incompatible protocols, unsupported required capabilities, and incorrect card/metadata/hash bindings |
| Delivery manifest | Require the complete known payload set, unique paths, hashes/lengths, fixed source identity, approved schema, and valid external signature/trust |
| Rights | A declaration is evidence for review, not proof of ownership; free public catalog entries require offline display/indexing grants and cover permission |

The draft catalog uses `FONTES-LIBRARY-CATALOG-V1` plus one newline byte as its signature domain. Delivery manifests use `FONTES-RESOURCE-PACKAGE-V1` plus one newline. Both detached signatures are 64 raw Ed25519 bytes over domain + original UTF-8 JSON bytes. They are independent of the existing data-release manifest protocol. Verify signatures before trusting parsed fields. Require app-configured repository/channel/key identity and monotonic catalog sequence; a package or catalog cannot authorize its own key.

The implemented source-stage publisher uses the separate `FONTES-RESOURCE-SOURCE-RELEASE-V1` + newline domain and a 64-byte raw Ed25519 signature. Its versioned manifest includes exact source assets/approval/provenance, not compiled SQLite. See the [submission pipeline](../../../docs/resource-publishing/submission-pipeline.md). Schema structure is supplemented by identity, path, complete-inventory, exact-approval, signature, and repository-visibility checks.

## Draft boundaries

The XSDs cover the initial prose/verse authoring profile. A full publisher-facing original-language occurrence/morphology profile, richer tables/footnote navigation, the compiled SQLite exchange schema, complete library v2 migrations, revocation policy, and platform-specific activation remain to be specified or implemented before production. Existing locked source adapters can preserve their structured data through the future internal normalized compiler path; that is not permission to run third-party adapters. The source schemas, catalog protocol, delivery protocol, and library schema have separate version axes.
