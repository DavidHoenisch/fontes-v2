# Resource review and publication

Status: operating design for the future catalog. The existing GitHub Actions workflow publishes whole reading/study snapshots. It does not yet compile or publish modular packages. The proposed dedicated repository is `fontes-library`; its owner, URL, intake contact, trust policy, and maintainers are not provisioned.

The implemented [source-submission pipeline](submission-pipeline.md) now handles PR validation, exact-hash approvals, reproducible previews, and signed **source** prereleases in this repository. The workflow below describes the later compiled delivery/catalog stage. Source releases do not yet contain SQLite or replace the current snapshot pipeline.

## Intake and review

Accept small redistributable sources unpacked in a PR, or a descriptor naming an immutable source ZIP with SHA-256 and byte length. Keep large revisions in release/object storage rather than normal Git history. Preserve the reviewed source identity and semantic diff report. A submitter's URL is input to a bounded fetcher, never permission to execute their build script.

Verify these items before trusted publication:

| Area | Evidence or check |
| --- | --- |
| Publisher identity | Namespace ownership and authorization to submit/update this resource |
| Distribution | Reviewed text license, territories/platform scope, source notices, offline display/indexing grant |
| Artwork | Separate cover/media rights, credits, dimensions, safe decoding, and declared media type |
| Content | Complete file inventory, stable IDs, reading order, coverage, source fidelity, and meaningful diff |
| References | Registry resolution, versification mapping, explicit relationship/basis, and link-quality report |
| Concepts | Existing IDs or publisher-owned proposals, scope notes, reviewed relationships, no cycles |
| Updates | Version progression, immutable prior bytes, note-anchor continuity, required dependency effects |
| Metadata | Accurate card/detail information, no private material, clear limitations and release notes |

Use private intake for uncertain/restricted rights before any public upload. Reference private evidence by a non-secret review identifier; do not include agreements, credentials, or correspondence in public archives. NLT and similar licensed works need actual publisher permission and suitable distribution controls. Free catalog admission does not waive licensing requirements.

## CI boundaries and publication order

1. PR checks run without signing keys, publication credentials, privileged caches, or arbitrary contributor code. Validate archive/XML limits and semantics with fixed Fontes tooling; render a preview and semantic diff. Tool success is separate from rights/editorial approval.
2. After approved merge, rebuild in a fresh trusted job using fixed adapters/toolchain and reviewed hashes. Produce normalized `content.sqlite`, approved metadata, assets, notices, and a complete manifest. Reject unknown database objects and payload files. Demonstrate reproducibility before enabling publication.
3. Sign the exact delivery manifest bytes with the resource domain. Create an immutable version/build release and download/verify every uploaded artifact. Do not expose signing secrets to the compilation of untrusted submissions.
4. Generate catalog records and cover variants from approved compiled metadata. Check repeated fields, resource dependencies, taxonomy snapshot, metadata hash, artifact lengths/hashes, and catalog uniqueness.
5. Publish and verify all referenced assets, including taxonomy and covers, before releasing the signed next catalog sequence. A mutable discovery pointer can advance only to that verified sequence.
6. Retain source/build receipts, approval identity, validation/diff reports, and the previous catalog. Confirm a clean client can verify and install from the published artifacts before announcing availability.

The catalog and delivery protocols have separate signature domains and are independent of today's data-release manifest. See the [contract README](../../specs/resource-packages/v1/README.md) for exact signing bytes. Trust comes from app configuration, not an author-supplied key. Key rotation and revocation policy must be completed before production.

## Corrections, withdrawal, and source refresh

Never replace published bytes under an existing resource version/build identity. Text or rights changes get a new resource version. Compiler-only corrections can increment build revision with the same source hash. Review semantic changes and anchor continuity in both cases. Scheduled upstream discovery should open reviewable update proposals against locked sources; it must not automatically publish changed text or license terms.

For bad content, first omit the resource from the next complete signed catalog snapshot and remove it from bundles. Keep withdrawal evidence and a replacement/correction reference in repository review history; the initial catalog schema has no structured replacement-notice field. Clients stop new installations after accepting the new snapshot; offline clients cannot learn the withdrawal until refresh. Withdrawal does not silently delete installed content or user notes. Signing compromise requires the completed revocation response; ordinary editorial withdrawal is insufficient for that case. Rolling back release contents still uses a **higher catalog sequence**, preserving replay protection; a client downgrade requires an explicit supported policy.

Taxonomy revisions can update mappings independently of source resources. Follow the [concept guide](concept-guide.md), preserve review history, and test that older installed assertions resolve correctly. New tagging of existing prose needs a separately signed enrichment format; that format is not implemented.

## Opening submissions

Publish a verified repository URL, private rights contact, publisher-registration process, supported tooling version, templates, validation error guide, and review expectations before inviting submissions. Provide catalog-independent source downloads for the schemas and taxonomy. The [Rust CLI](cli-guide.md) currently provides scaffolding, source validation, packaging, and schema export. Conversion, preview, semantic diff, and submission preparation remain proposed interfaces.
