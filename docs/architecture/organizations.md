# Organization workspaces

Status: **implemented dashboard/API management and private metadata drafts; private publication and app delivery planned**, 2026-10-17. D050 extends [the server plan](server-components.md). The authoritative development contracts are [organization schemas](../../apps/fontes-site/src/lib/organization-contracts.ts), [services](../../apps/fontes-site/src/lib/organizations.ts), [migration 004](../../apps/fontes-site/db/004-organizations.sql) and the generated [HTTP API](../../specs/server-api/v1/README.md).

## Ownership and roles

Creating an organization creates its first owner, stable server-assigned UUID/namespace, initial policy-preference revision and audit event atomically. Its namespace is an unverified publishing identity, not proof of public publisher authority. A single account can hold independent roles in several organizations. Fontes platform reviewer status grants no organization access.

| Role | Organization capabilities |
| --- | --- |
| Owner | Settings, invitations, membership and ownership changes, private metadata drafts |
| Admin | Settings and private metadata drafts; invite/manage publishers and members; cannot invite an admin or change/remove owners/admins |
| Publisher | Read/create/edit private metadata drafts and view roster/activity |
| Member | View organization details, redacted roster and activity; future released-resource access, with no unpublished draft access |

Any member can leave; owners can remove/change other owners only while at least one owner remains. Ownership is explicitly assigned to an existing member rather than embedded in an invitation. Every organization mutation serializes on the organization row, checks current membership/role and expected organization revision, and appends an event in the same transaction. Membership acceptance additionally locks the accepting account to enforce the membership cap across organizations. Concurrent demotion/removal cannot leave zero owners. The SQL function uses the caller's database privileges (`SECURITY INVOKER`); it is not a signing or publication privilege.

Roster emails and invitation history are visible only to owners/admins. All current members see names/roles and the latest 30 action/actor/time events. Those events contain no invitation tokens, manuscript text or email body. An inaccessible organization returns the same 404 as an unknown one. Organization reads assemble roster, invitations, resources and audit in one membership-scoped SQL snapshot. Removal ends subsequent API access; it cannot retract bytes a member has already read/downloaded.

## Invitations

The current transport is **copy a link**, not email. Creation/renewal returns a 256-bit random base64url token once; only its SHA-256 hash is stored. The URL puts it in a fragment (`/?view=organizations#invite=...`), and the dashboard consumes/removes the fragment before making authenticated same-origin JSON requests. Tokens never belong in API paths/query parameters, audit data or logs. Invitation links expire after seven days. Renewal rotates the token and expiry; revocation invalidates a pending link. A unique pending invitation per organization/normalized email prevents accidental duplicates. Expired pending invitations can be renewed/revoked rather than recreated under a second identity.

The recipient must be signed in with the matching verified email; development personas provide the same matching-account check without a login. Preview verifies the token and email before showing organization/inviter/role. Explicit acceptance checks token status/expiry and capacity, creates one unique membership, consumes the invitation and records an event atomically. Wrong email sees 404; expired/revoked/replayed tokens return 409. Existing membership is not silently modified by an invite. Never autojoin by email domain. Changing the recipient or invited role requires revoking and creating an invitation.

For local prototyping, invite `reviewer@example.invalid` from Alex's publisher persona, copy the link, switch to Reviewer in Workspace settings, then use **Organizations → Open invitation** or open the copied URL. Morgan's platform reviewer role remains independent of the newly accepted organization role. No email is sent to either synthetic address.

The server does not persist raw invitation tokens for later retrieval. If a creator loses a response/link, refresh organization state and renew the link. Creation is not automatically idempotent; retrying with stale revisions fails rather than creating another invite. Email-provider delivery/outbox, inbox/recovery proof, stronger abuse controls, full idempotency/cursor pagination and account-deletion/retention policies remain production work.

## Organization-owned drafts

Private metadata drafts live in a separate `organization_resources` table with a foreign key to the owning organization and a creator attribution. They do not become the creator's personal resource and survive that creator leaving. Owner/admin/publisher access is checked on every read/write. Ordinary members do not receive draft metadata in the API. Generic personal resource, intake and maintainer-review endpoints cannot find these IDs. They never enter the public metadata/review lists; there is no visibility toggle or private-to-public promotion shortcut.

The dashboard collects the existing metadata fields and supports version-checked editing. Each edit requires both organization revision and resource revision. Current status is only `draft`. D052 adds [local EPUB evaluation](../research/calvin-epub-evaluation.md), with uploads, paged reading, source links/footnotes and conversion/rights reports available only to the publishing team. D053 adds publisher correction/approval and local validated source export. Substantive rights review, approved ordinary-member previews, private release authorization, isolated compilation/signing, scoped feeds/covers/payloads, download grants and app installation remain unimplemented. Personal source preparation remains available under D049; copying that public-free preparer into a private workflow would not establish the missing authorization and grant contracts.

## Offline policy preferences

Owners/admins can record **Keep downloaded copies** or **Renewable offline lease** as a preference for future releases. Each actual preference change appends an immutable policy revision; renaming an organization does not create a policy revision. The UI and API label these as preferences and report `offline_grants=false`. Neither option issues a grant or changes current device behavior. No lease duration is invented here. Actual grant signature/version, beneficiary/resource binding, duration/clock behavior, compatibility for durable grants and app enforcement remain O022/AC16 work.

## Prototype bounds and verification

All mutations use authenticated sessions or the guarded loopback development actor, exact Origin, strict schemas and the existing 16 KiB JSON limit. Results are private/no-store. Hard prototype bounds keep lists complete within capacity: 20 memberships per account, 100 members, 100 lifetime invitation records and 100 metadata drafts per organization. Creation is limited to five organizations per account/day; invite creation/renewal to 20 per organization/hour. No automatic deletion occurs on reaching a bound. Latest audit history is a bounded 30-event window; the database retains earlier events. Retention/pruning/quotas and stronger account/IP abuse defenses require production policy.

Migration 004 is additive and applies only to `fontes_cloud`; existing accounts, personal drafts, `neon_auth`, source XML/XSD, signed releases and native/local user data are unchanged. The migration runner now splits standard quoted strings, comments and dollar-quoted procedural bodies correctly while keeping every migration and its digest record in one transaction. Applied migrations remain immutable.

Verification includes schema/migration tests and live HTTP/Postgres checks for email mismatch, token hashing/rotation/expiry/replay, concurrent acceptance, concurrent owner demotion, role escalation, cross-tenant and personal/reviewer isolation, draft editing, policy history and immediate API denial after removal. Browser form/layout checks are separate evidence. Real email delivery, managed-account signup/inbox acceptance, private publication/downloads, native sign-in and offline devices are not demonstrated by these tests.

## Local EPUB evaluation extension (D052)

The [evaluation schemas](../../apps/fontes-site/src/lib/epub-review-contracts.ts), [parser](../../apps/fontes-site/src/lib/epub-review.ts), [service](../../apps/fontes-site/src/lib/organization-epub.ts) and [migration 005](../../apps/fontes-site/db/005-organization-epub-review.sql) define this separate review profile. Artifacts are private local hash-addressed files with integrity checks. Postgres stores summary/inventory and immutable import history, not book prose. An import checks current role before reading bytes, then rechecks role and both optimistic revisions under the organization row lock before committing. Metadata edits do not approve the book or rewrite its retained source; source replacement creates a new preparation revision.

Bounds: existing 20 MiB uploads/64 MiB expansion/2,000 entries/8 MiB per archive file; evaluation permits 256 reading-order sections, 25,000 blocks, 8 million text characters, 64,000 characters per block and 200,000 nodes/64 levels per XHTML document. Each derived document is at most 8 MiB and inventory at most 256 KiB. Image decode permits upright single-frame PNG/JPEG, at most 4 MiB and 16 million pixels; these preview bounds do not redefine package-cover requirements. Each reading response contains at most 40 blocks. Block jumps resolve only within the retained section inventory; image reads require a listed current asset digest. Every request rechecks publishing-team membership and is private/no-store.

The immutable extraction profile records `rightsReviewRequired=true` and `exportable=false`; D053 adds a separate corrected review, publisher approval and local source export. These original extraction flags do not describe the corrected draft status. It preserves safe emphasis, superscripts, explicit line breaks, source footnotes and local anchor links. External/ambiguous links remain inert and are reported. Single-cell layout tables preserve paragraph order as poetry; general tables/advanced media are rejected. A normalized-text mismatch rejects the entire import. No uploaded HTML is rendered and no external resource is fetched. Local retention/orphan cleanup, storage quotas and hosted isolation remain operating gaps.


D053 adds [publisher correction and approval](resource-preparation.md#organization-publisher-review-d053) with additive migrations 006–007. The 2 MiB correction JSON limit is separate from the 16 KiB management limit. Review snapshots and package bytes remain private local artifacts; role/revision checks serialize with membership writes. Member publishing/downloads, signed grants and app integration remain planned.


D054 adds explicit [public Fontes package intake](maintainer-review.md) after publisher approval/generation. That deliberate action shares a frozen package with platform maintainers; it does not grant access to the organization’s other drafts or implement private member publication. Source changes supersede the frozen submission.
