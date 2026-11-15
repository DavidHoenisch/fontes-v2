# Fontes publisher workspace

Working Next.js/TypeScript dashboard with metadata review and local source preparation. Use `npm run dev` to prototype **without signing in**. HeroUI 3.2.6 owns the default appearance of buttons, inputs, selects, cards, tabs, tables, dialogs, avatars and chips; application CSS handles layout and editorial artwork, not component retheming.

Dashboard branding uses the approved [Wellspring identity](../../assets/brand/wellspring/README.md). `public/brand` contains unchanged copies of the horizontal logo and rounded icon masters, plus the outlined wordmark's font license. Refresh those copies when the masters change. The same icon appears on sign-in and in the browser favicon; this presentation change requires no data or API migration.

## Run locally

```sh
cd apps/fontes-site
npm ci
npm run dev
```

Open [the local workspace](http://127.0.0.1:3000). This checkout already has an ignored `.env.local` configured for the dedicated development project. Keep it private. On another checkout, copy `.env.example` to `.env.local`, supply the development database/auth settings, then run `npm run db:migrate` and `npm run db:seed`. The migration runner records a digest and refuses changed applied migrations. Seed is idempotent and never resets existing draft edits.

The development project is `fontes-development` (`round-brook-87136723`), AWS us-west-2, Postgres 17. Its `main` branch is `br-winter-mountain-arnbp7nn`; this is a dedicated development project, not production. Managed Neon Better Auth is enabled in `neon_auth`, email/password and OTP verification are enabled, and the two localhost origins are trusted. The application schema is separately named `fontes_cloud`. No other Neon projects were changed. Connection credentials and the random cookie secret are stored only in ignored `.env.local` with mode 0600.

The development starter seeds fictional Alex Morgan (publisher), Morgan Ellis (reviewer), and four explicitly labeled synthetic records. They are database identities, not Neon Auth accounts or verified publisher identities. **Settings → Development playground** switches the local persona. Reviewers cannot approve their own work, and cannot see another person's editable drafts. Publisher, maintainer and ordinary-reader accounts remain one identity model with scoped roles.

## Implemented scope

- Responsive dashboard, resource table/filter/search, resource detail, new/edit forms, metadata review queue, activity, settings and live read-only API explorer.
- Create/edit owned resource metadata in Postgres; submit metadata, request changes with feedback, revise/resubmit, and approve metadata with an independent reviewer.
- Atomic resource transitions and activity writes; revision checks reject stale edits/decisions and concurrent competing decisions. Stable owner/resource identities are server allocated.
- Zod-validated requests and serialized response shapes, request IDs, structured errors, bounded JSON bodies and mutation Origin checks. API lists expose their 100-resource/30-event limits.
- Neon Auth server/client proxy and optional signup, email/password sign-in, OTP verification/resend and sign-out UI. Verified-email gate precedes managed-account workspace access. Auth subjects map to stable Fontes account/namespace records; signup never assigns a maintainer role.

**New resource** now offers two paths: an offline-prepared `.fontes-source.zip`, or a guided wizard for metadata, contributor/publisher, text/artwork rights, cover upload, prose EPUB import and readable publisher review. The importer follows EPUB spine order, preserves supported text blocks, and proposes conservative literal Scripture references for explicit accept/reject. It does not call an AI service. Publisher approval follows strict Rust validation/packing and binds both the draft snapshot and exact source ZIP bytes; edits clear it. See [the preparation design and limits](../../docs/architecture/resource-preparation.md).

**Metadata or preparation approval does not mean rights cleared, signed, published, or app-installable.** D054 adds local exact-source maintainer review and unsigned release handoff. Hosted isolated workers/storage, signing/release transport, release/catalog publishing, app delivery, private source publication, payment fulfillment and hosted AI remain unimplemented. Preparation runs only in local development, with private hash-addressed files in ignored `.local/intake` and versioned snapshots in Postgres. Organization creation, roles, invitation links, membership management and private metadata drafts now persist in Postgres; see the [organization guide](../../docs/architecture/organizations.md). Nothing changes the phone app, signed data snapshots or local user notes.

Build the trusted Rust CLI with `cargo build -p fontes-resource` at repository root and ensure `xmllint` is on PATH before importing prepared sources or approving/exporting guided content. The optional `FONTES_RESOURCE_CLI` and `FONTES_RESOURCE_TAXONOMY` settings are trusted local operator configuration, never package input. No illustrative taxonomy is automatically selected as a production registry.

## Auth modes

`npm run dev` binds to `127.0.0.1:3000` and defaults `FONTES_AUTH_MODE` to `development` without requiring an auth session. An explicit shell environment override wins:

```sh
FONTES_AUTH_MODE=neon npm run dev
```

In that mode, open `/auth` for managed authentication. Local demo personas and managed accounts do not share an identity. Verified Neon users receive a new empty personal workspace. Maintainer role assignment is an explicit server/database operation; there is no self-service elevation or client role header.

Bypass is rejected unless `NODE_ENV` is `development`/`test`, the request targets loopback and common deployment flags are absent. Production and unknown auth modes fail closed; an auth outage never falls back to a developer account. The dev-persona cookie is allowlisted and accepted only after this guard. `.env.local` deliberately selects development mode; do not copy it into hosting environments.

The configured application Origin must exactly match the browser URL. Use the printed `127.0.0.1` URL rather than switching between localhost aliases. JSON mutations require that Origin and `application/json`: metadata is limited to 16 KiB, preparation saves to 2 MiB. Preparation uploads use bounded raw bytes with the expected resource revision header (20 MiB archives; 4 MiB cover). No native token/CORS integration exists yet.

For a local production-mode check:

```sh
npm run build
FONTES_AUTH_MODE=neon npm start
```

This is **not a production launch configuration**. Review MFA and privileged-session requirements, recovery flows, auth provider availability, rate limiting, least-privilege database roles, audit retention, deployment Origin settings and secret rotation before deployment. Shared development SMTP is not a production email service. Production startup with `FONTES_AUTH_MODE=development` returns a structured 503 instead of granting access.

## API contract

See [the draft contract guide](../../specs/server-api/v1/README.md) and the generated [OpenAPI endpoint](http://127.0.0.1:3000/api/v1/openapi). Runtime schemas live in `src/lib/contracts.ts`; OpenAPI reuses them. The dashboard talks to the same HTTP endpoints as another web client would.

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/api/v1/workspace` | Actor, scoped resources/activity and explicit unavailable capabilities |
| GET/POST | `/api/v1/resources` | Bounded resource list or new owned metadata draft |
| GET/PATCH | `/api/v1/resources/{id}` | Scoped detail or version-checked owned draft edit |
| POST | `/api/v1/resources/{id}/transitions` | `submit`, `request_changes`, `approve_metadata` with expected revision |
| GET/PUT | `/api/v1/resources/{id}/intake` | Read/save scoped preparation fields and reference decisions; retain source/history |
| POST | `/api/v1/resources/{id}/intake/{operation}` | Local `epub`, `prepared`, `cover` upload or publisher `approve` |
| GET | `/api/v1/resources/{id}/intake/{operation}` | Scoped `cover` preview or approval-bound source `package` download |
| GET | `/api/v1/openapi` | Draft OpenAPI 3.1 document; no credentials |
| POST | `/api/v1/dev/persona` | Local-only allowlisted persona switch; absent as a supported production capability |
| GET/POST | `/api/auth/{path}` | Neon SDK-managed web auth proxy |

Lists are bounded, not fully paginated. Resource creation is not yet idempotent; do not automatically retry it. Review actions are protected by optimistic revisions and leave exactly one event per accepted transition. Publication and native OAuth are intentionally absent rather than implied by placeholder endpoints.

## Verification

```sh
npm run test
npm run typecheck
npm run build
# With npm run dev serving on 127.0.0.1:3000:
npm run test:api
npm run test:intake
npm run test:organizations
# Requires the production build; briefly owns 127.0.0.1:3001:
npm run test:production-auth
```

Verification adds EPUB/spine/text and adversarial archive tests to the 25 original boundary tests. `test:api` checks metadata/permissions/concurrency and the Neon session proxy; `test:intake` exercises real imports, human decisions, Rust validation, repeatable generated ZIP bytes, unchanged prepared bytes, approval invalidation and retained derivation history. These create/remove only their own synthetic database records; uploaded fixture bytes remain in ignored local storage. Production probes require managed-session access and reject bypass cookies. Browser QA is recorded independently of schema/build results. No test claims that AI or signed app publication exists.

Signup/OTP delivery to a real inbox, password recovery, a verified managed-user session, maintainer MFA, production deployment and native sign-in have **not** been demonstrated. Build/schema tests do not establish those behaviors. Upstream Neon auth SDK is pinned to `0.6.0-beta`; its dependency graph currently emits peer warnings. Keep the lockfile and rerun auth/integration checks when upgrading.

Bundled IBM Plex and Lora fonts retain their OFL notices in `public/licenses/`. The decorative Romans illustration is copied from the repository's Covenant Forms set; see [its provenance](../../assets/ui/book-covers/README.md). Typographic covers are demo presentation, not licensed book artwork or published packages.

## Organization workspaces

Open **Organizations** to create a workspace, invite collaborators, manage roles, prepare private metadata drafts, choose a future offline-policy preference and inspect activity. Controls use HeroUI defaults. Owners manage ownership; admins manage publishers/members; publishers prepare drafts; members cannot see unpublished drafts. Fontes reviewer status grants no organization access.

Invitations use copyable links, not email delivery. Invite `reviewer@example.invalid` from Publisher, copy the link, switch to Reviewer in Workspace settings, then **Organizations → Open invitation**. Matching verified-email checks also apply in managed-auth mode. Links are single-use, expire in seven days and are invalidated by renewal/revocation. Only token hashes live in Postgres.

Organization drafts are separate from personal resources and public metadata review. D053 adds local publisher approval and validated source export. Private publishing, member feeds/downloads, native access and offline-grant enforcement remain planned. Settings record policy preferences with history; they do not issue a license. See the [organization operating boundaries](../../docs/architecture/organizations.md) for capacity/rate limits and remaining production work.

Run `npm run test:organizations` against the dedicated development server for live HTTP/database authorization, invitation and concurrency checks. The test removes only the synthetic organizations created by that invocation. Migration 004 is additive; `npm run db:migrate` preserves earlier migrations and their digests.

Current organization verification: 52 unit tests, 64 organization HTTP/Postgres checks, the existing 22 metadata and 22 intake checks, TypeScript and production build pass. The local browser demonstration contains a labeled Wellspring demo team, two synthetic members and a metadata-only study-guide draft; these are not verified publishers or released content.

## Real-book EPUB evaluation

Within an organization resource choose **Prepare EPUB** or **Read source draft**. D052 adds local, paged full-book review with original chapter order, safe text/emphasis, source footnote/index links, cover preview, conversion/rights report and unapproved Scripture candidates. The [Calvin/Beveridge evaluation](../../docs/research/calvin-epub-evaluation.md) records exact source bytes and measured results. D053 extends it with corrected review, publisher preparation approval and validated source ZIP generation; it does not share a book with ordinary members.

Book bytes/prose remain in private ignored `.local/org-epub`; Postgres stores only report/inventory/hash metadata and history. Owner/admin/publisher membership is required on every request; no platform reviewer override. File operations are disabled outside development. Original EPUB artwork and edition notices retain their source claims; no redistribution or AI permission is inferred.

Run `npm run test:epub-review` against the dedicated development server for synthetic live scope/pagination/link/history checks. To audit an explicitly supplied local book without database ingestion, run `FONTES_EPUB_FIXTURE=/path/to/book.epub npm test -- tests/epub-review.test.ts`. The actual book is not a committed fixture.


### Publisher corrections, approval and package generation

In **Read source draft**, use **Next unreviewed reference → Edit text & references** to compare original text, correct inline text, accept/reject/reset/remove candidates or add exact-evidence OSIS references. A text correction resets that block's references and approval. **Publication details & rights** collects coverage and separate text/artwork authorization. Upload a conforming cover, resolve references, then **Approve reviewed draft → Generate Fontes source package → Download source ZIP**. Strict Rust validation runs during generation. Metadata edits/reversions, source replacement and content/rights/artwork edits cannot reuse the former approval. Current rich review preserves note links; XML v1 export retains their text with disclosed formatting/navigation limits.

Run `npm run test:publisher-review` for synthetic end-to-end Postgres/Rust checks. Migrations 006–007 retain immutable local snapshot pointers/history and a metadata epoch. Local prose and output ZIPs stay ignored under `.local/org-epub`; existing development auth/storage guards apply. This is publisher source preparation, separate from rights/namespace verification, signed release, compiled package, member delivery or phone integration. See the [review design](../../docs/architecture/resource-preparation.md#organization-publisher-review-d053).


### Maintainer source review and publication handoff (D054)

After publisher approval/package generation, choose **Submit package to Fontes**, add release notes and explicitly authorize public-free maintainer review. Both personal preparations and organization packages are supported. In **Review queue → Source packages**, maintainers inspect the frozen exact package, rights, artwork, assertions and file hashes; record feedback or request changes; and approve with identity/rights evidence IDs. The submitter, origin creator and publisher approver cannot make their own maintainer decision. Metadata-only review remains in its own tab.

**Prepare publication handoff** then **Download publication handoff** exports the existing source/descriptor/approval layout for the trusted repository pipeline. This is an unsigned, unpublished source-stage artifact. It does not change real publisher policy, sign, dispatch CI or create a live release. Later source edits supersede the review; withdrawal and supersession block content downloads, while change requests clear current approval/handoff. Review event snapshots retain evidence after later transitions. Migrations 008–010 are additive; run `npm run db:migrate` then `npm run test:source-review`. See [flow, limits and verification](../../docs/architecture/maintainer-review.md).
