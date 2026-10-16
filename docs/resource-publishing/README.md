# Resource publishing documentation

The current requirements, flows, architectural choices, and implementation gates are consolidated in [SPEC.md](../../SPEC.md). These guides explain how to apply the resource-related contracts.

Fontes is designed to support a default offline collection and additional resources selected from a free catalog. Resources ship independently and join one installed library, so a verse lookup can find relevant paragraphs across all installed works.

This documentation describes the package system. The current reading/study data pipeline publishes signed SQLite snapshots, and source-submission validation/publication is implemented under `library/` in this repository. The modular SQLite delivery compiler, catalog, client installer, and custom publisher portal are not implemented yet.

| Audience | Start here |
| --- | --- |
| Authors and publishers | [Authoring a resource](authoring-guide.md): archive layout, metadata, artwork, references, validation, and updates |
| CLI users and CI authors | [CLI reference](cli-guide.md): scaffold, validate, package, export schemas, and interpret diagnostics |
| Submission and release operators | [Submission pipeline](submission-pipeline.md): trusted approvals, PR feedback, signed source releases, reusable workflows, and private storage |
| Authors and taxonomy reviewers | [Concepts and topics](concept-guide.md): reuse, proposals, aliases, hierarchy, and reviewed mappings |
| Catalog maintainers | [Review and publication](maintainer-guide.md): intake, rights, CI, release order, and corrections |
| Mobile, desktop, and pipeline developers | [Library integration](developer-guide.md): ownership, queries, updates, recovery, migration, and acceptance tests |

The [specification addendum](../specifications/library-and-packages-v0.2.md) records the architecture decisions. The [contract directory](../../specs/resource-packages/v1/README.md) contains XSDs, JSON Schemas, an executable SQLite ownership model, and a complete small source example. If a guide and contract disagree, treat that as a defect to resolve before freezing version 1.

## What works today

Run these commands from the repository root:

```sh
python3 specs/resource-packages/v1/check_examples.py \
  --pack dist/resource-package-design

uv run --with jsonschema==4.26.0 python \
  specs/resource-packages/v1/check_examples.py --json
```

Python 3 and `xmllint` are required; the second command additionally uses `uv` to supply a pinned JSON Schema validator. These commands check the checked-in examples and isolated SQL model. The first generates a deterministic demonstration source ZIP. To scaffold or validate your own source directory/ZIP, use the implemented Rust [CLI](cli-guide.md). Neither tool compiles resource SQLite payloads, publishes a catalog, or installs resources.

## Delivery milestones

| Stage | Status and completion requirement |
| --- | --- |
| Existing corpus pipeline | Implemented; see [data pipeline architecture](../architecture/data-pipeline.md) and the repository README |
| Package design | Draft contracts, example source, conformance checks, and these guides exist |
| Publisher tooling | Rust `init`, `validate`, `pack`, and `schemas` implemented; resource SQLite compiler, converters, preview, diff, and submission tools planned |
| Submission operation | Implemented in `library/`: read-only PR checks, hash approvals, reproducible previews and signed source-release publisher; dedicated intake/catalog services remain planned |
| Catalog operation | Catalog publication/client, production publisher registry, private rights contact and taxonomy governance remain planned |
| Client integration | Planned library schema v2, legacy anchor migration, shared installer, catalog UI, and recovery testing |
| Paid distribution | Later design phase; requires publisher agreements and entitlement/delivery infrastructure |

Before calling this a supported publishing format, complete the compiled SQLite exchange schema, original-language occurrence profile, anchor-remapping contract, signing-key rotation/revocation policy, and platform activation implementation. The [implementation gates](../specifications/library-and-packages-v0.2.md#implementation-sequence-and-acceptance-gates) define the order and required demonstrations.
