# Maintainer source review and publication handoff

Status: local development implementation (D054). This extends [publisher preparation](resource-preparation.md) and uses the existing [source-submission pipeline](../resource-publishing/submission-pipeline.md), [submission](../../specs/resource-packages/v1/submission.schema.json) and [approval](../../specs/resource-packages/v1/approval.schema.json) contracts without changing their versions or meaning.

## Submit the approved package

Both personal source preparations and organization publisher reviews offer **Submit package to Fontes**. Require a current publisher approval; organization submissions additionally require a generated package bound to it. The publisher supplies release notes and explicitly authorizes sharing this package with Fontes maintainers for public-free publication consideration. An organization draft never enters the platform queue simply because it was prepared/approved. Other private drafts remain inaccessible to maintainers.

Submission verifies the approved upstream bytes, strictly validates and normalizes them twice with the trusted Rust packer, compares canonical bytes and retains both upstream and canonical hashes. Normalizing the ZIP container preserves its file payloads; the reviewed source hash is the canonical hash required by the existing release contract. A frozen readable preview retains ordered source IDs, text, document/block/inline assertions, metadata, all coverage ranges/contributor roles, rights and artwork inventories. Presentation is flattened and labeled; exact ZIP and individual inventory files remain available for full review. No uploaded markup executes and no external asset is fetched.

Submission rechecks ownership/publishing membership, origin/resource/review revisions under the same locks as authoring and membership commands. The record is immutable with respect to source bytes. Duplicate submissions of the same origin revision fail with `already_submitted`; it is safe to open the existing review, not repeat a POST automatically. Replacement publisher revisions require a newly approved submission. The public review is separate from legacy metadata-only status.

## Review and decide

**Review queue → Source packages** shows active submissions; **Metadata only** retains the former metadata workflow. Publishing teams can follow feedback and withdraw their submissions. Fontes maintainers can read only packages explicitly submitted to this queue, without access to unrelated organization drafts. An independent maintainer cannot be the submitter, publisher approver or creator of that origin draft. Organization membership does not itself confer a platform maintainer role.

| State/action | Result |
| --- | --- |
| Submitted → feedback | Append a comment and advance optimistic review revision |
| Submitted/approved/handoff ready → request changes | Record mandatory feedback, clear current approval/handoff, retain decision history |
| Submitted → approve exact source package | Require complete publisher identity/namespace, text rights, artwork rights, content/fidelity/coverage and reference/concept review attestations; record non-secret rights/publisher review IDs, reviewer/time and canonical hash |
| Approved → prepare publication handoff | Revalidate/repack exact canonical bytes, refuse tool drift, reserve local resource/version/hash and retain deterministic unsigned handoff bytes |
| Handoff ready → repeat handoff | Return the existing verified receipt/bytes without creating another decision |
| Active → publisher withdrawal | Retain authorized audit metadata; block new source/document/file/handoff downloads and further decisions |
| Any publisher origin update | Mark earlier active reviews superseded and clear current approval/handoff; new package approval/submission required |

Decisions require expected review revision and exact canonical hash. Writes lock organization (when present), resource and review in the same order as publisher edits. Review status/revision, decision and audit event commit atomically. Source edits and competing decisions cannot attach approval to a stale package. Immutable event details retain prior approval evidence IDs and handoff hashes after later changes/withdrawal; the screen displays latest 100 audit events. Withdrawn/superseded records appear in a separate bounded history list. Withdrawal here does not claim to retract an already published external release.

Review attestations are records of the maintainer's substantive checks, not an automated legal or identity verification service. Do not use fictional fixtures as production publisher registrations or rights evidence. No private agreement should be included in exported files; evidence identifiers must be safe for the eventual release artifacts.

## Move toward source publication

The handoff ZIP contains:

- `submissions/<resource-id>/source.fontes-source.zip`: exact reviewed canonical source bytes.
- `submissions/<resource-id>/submission.json`: existing strict descriptor, repository-local path and release notes; build revision starts at 1.
- `approvals/<resource-id>--<version>.json`: existing strict public-free exact-source approval with rights/publisher evidence IDs.
- `review-receipt.json` and `README.txt`: local publisher/maintainer evidence and the trusted workflow handoff instructions.

It contains no policy allowlist change, private signing key, signed manifest, compiled app payload or catalog listing. Copy the source/approval paths into the separately trusted submission repository, verify/register the actual publisher with its real policy, check existing destination versions/builds, run the normal trusted check, then review/merge through protected source-release CI. The pipeline independently rechecks policy/approval, reproducibility, target/repository identity and signing/publication conditions. Browser approval cannot alter that trusted policy or authorize a signer. Production namespace registry, CI dispatch/PR transport and release-status reconciliation are not implemented by this local handoff.

Local handoff identity reservations prevent different source bytes under an already handed-off resource/version, including after local withdrawal. Use a new resource version for changed source. This is a prototype collision guard, not a production registry or proof a release exists. The external trusted pipeline remains authoritative for previously published identities. Source prereleases still do not establish app installation/default-bundle inclusion.

## Operating bounds and evidence

Migrations 008–010 add source-review records, append-only events/decision snapshots, local handoff-version reservations, source-edit invalidation triggers and serialized active-intake capacity. Existing personal/organization metadata status and published contracts remain unchanged. The older preparation migrations are not rewritten. Auth uses the existing guarded login-free local personas or verified managed identity; operations are disabled outside development until isolated storage/workers are available.

Book bytes, canonical ZIPs, document previews and handoff archives remain private hash-addressed local files in ignored `.local/org-epub`. Postgres stores identities, source/snapshot pointers, review decisions and non-secret audit evidence. Reads/downloads recheck current publishing access or the platform maintainer role; removed publishers lose subsequent access. Details retain audit metadata for inactive submissions but content/download routes reject them. All responses are private/no-store; mutation Origin and 16 KiB JSON limits apply. Scoped file lookup uses retained inventory entries, not arbitrary filesystem paths.

The active queue is capped at 100 globally with serialized inserts; list limits are 100 active accessible records or 100 most recent inactive records. Archive limits remain 20 MiB input/64 MiB expansion/2,000 files/8 MiB per file; readable review supports at most 256 documents, 25,000 blocks, 8 million text characters and 40 displayed blocks per response. Larger packages use repository intake. Subprocesses have 30-second limits and receive no database/auth/signing credentials. A configured trusted taxonomy is passed to validation and packing; examples are not production registries. Disk retention/quota cleanup, hosted job isolation, reviewer MFA, policy/namespace operations and live release transport remain production work.

`npm run test:source-review` exercises actual HTTP/Postgres/Rust preparation, explicit consent, canonicalization of a noncanonical publisher ZIP, frozen payloads/assertions, independence, stale decisions, feedback/change requests, resubmission/supersession, public-intake scope for organization drafts, audit snapshots, withdrawal, version collisions, deterministic handoff and consumption of its descriptor/approval by the existing CLI. CLI handoff testing uses a separately isolated synthetic policy/registry/trust fixture; it does not modify the live policy or publish. Browser checks verify default HeroUI controls and the original labeled review demo. Production probes include every new route and reject development cookies. No live signed release or device installation is inferred from these tests.
