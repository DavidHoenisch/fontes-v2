# Calvin / Beveridge EPUB evaluation

Date: 2026-10-17. Status: **local evaluation and organization publishing-team preview; not an approved source package, member release or app installation**. The user supplied `Institutes_John_Calvin_Beveridge.epub` after the [Monergism download](https://www.monergism.com/thethreshold/sdg/calvin/Institutes_John_Calvin_Beveridge.epub) returned an automated-access challenge. The audit uses the supplied bytes, not a substituted edition.

## Exact input and findings

| Item | Observed value |
| --- | --- |
| SHA-256 | `b5b3f101794bf03696780c0b74aedafd1ce4c8a809c0b789370030bc24bc2382` |
| Archive | 2,354,927 bytes; 115 entries; 6,017,697 expanded bytes |
| Package | `OEBPS/package.opf`; 108 reading-order XHTML sections |
| Declared metadata | Institutes of the Christian Religion; John Calvin; contributor/translator Henry Beveridge; Monergism Books; English; edition date 2026 |
| Extracted review | 4,260 text/image blocks; 3,919,798 retained text characters including intra-block whitespace |
| Footnotes | 658 footnote bodies and 658 note references |
| Reading-body links | 6,379 source anchors with hrefs; all 6,379 resolve in the retained reading-order model |
| Artwork/layout | One decoded raster cover; three one-cell tables containing poetry, retained as ordered poetry paragraphs |
| Scripture detector | 4,918 **unapproved occurrence candidates**, including index/footnote repetitions; not a count of distinct verses or proof that every citation was detected |

The older personal prose-to-source converter rejects the first cover image. Its 2-million-character/3 MB saved-state profile also cannot hold this complete book. That converter remains unchanged; this test does not claim that it can now export this EPUB as a validated source ZIP.

A separate `epub-review-v1` evaluation parser respects well-formed XHTML, including self-closing non-HTML-void anchors/spans. Treating this XHTML as error-corrected HTML can incorrectly nest/duplicate elements and merge paragraph boundaries. Regression fixtures cover that case. The review parser uses the OPF spine for order, EPUB navigation for descriptive/grouped section labels, explicit local anchors for links, original `epub:type` footnote/noteref relationships, safe emphasis/superscripts and source artwork. It never embeds uploaded HTML, runs scripts or fetches external resources.

Text preservation is checked by concatenating reading-body text in spine order, removing whitespace, and comparing its SHA-256 with the independently traversed review blocks. Both equal `a962ac1a5e98961fd1a2a0c4ae18a7d513478e4aba7803798918ecc18736774b`. A separate Python ElementTree traversal of the original ZIP produced the same digest. This demonstrates non-whitespace text/order preservation for these bytes; it does not establish visual equivalence, correct theological interpretation, accurate citation detection or approved rights.

Inline styling is limited to emphasis, strong text, superscript and explicit line breaks. Source CSS, page geometry and arbitrary layout are not reproduced. General multi-cell/nested tables and advanced media are rejected instead of flattened. Duplicate source anchor targets remain unresolved rather than pointing arbitrarily at one paragraph. This book's reading-body links have no unresolved targets.

## Rights finding

The OPF and `OEBPS/text/copyright.xhtml` distinguish the original public-domain work from this Monergism edition's editorial material, modernization, annotations and original artwork. The supplied notice allows some personal/church/classroom sharing but expressly restricts incorporation into another website, digital library, database, retrieval/AI system or application. Preserve and review the actual notice; the age of Calvin's original work is not evidence that these exact edition bytes may be published by Fontes.

No rights authorization is inferred or attested on the user's behalf. The current result is retained for local evaluation by the organization's publishing team, with `rightsReviewRequired=true` and `exportable=false`. Source files and book prose remain in ignored private local storage; Postgres retains inventory, bibliographic/rights-report metadata, hashes and import history. There is no member release, signing, storefront publication or AI processing. Further distribution needs explicit rights clearance or an independently acquired suitable source edition, with source/cover provenance reviewed separately.

## Implemented organization review and remaining work

A publishing team can upload/replace an EPUB on its organization draft and open section/block pages, follow source footnote/index links, inspect the conversion/rights report and see separately labeled detector candidates. Reads require a fresh owner/admin/publisher membership; ordinary members and Fontes reviewers without that organization role cannot access drafts. Imports recheck organization/resource revisions and role under a row lock before committing inventory/history. A stale edit/removal cannot commit an already parsed draft under old authorization.

Evaluation is deliberately a separate model from the existing source XML/XSD and personal source-package preparer. Its image/link/footnote mapping has not yet been translated into a strict, rights-reviewed source export. Publisher correction/adoption controls, exact-byte approval, approved member previews, private publication authorization, hosted isolated workers/object storage and native delivery remain future work. Do not relabel an evaluated book as published.

Reproduce the real-file audit from `apps/fontes-site`:

```sh
FONTES_EPUB_FIXTURE=/path/to/Institutes_John_Calvin_Beveridge.epub npm test -- tests/epub-review.test.ts
```

The real fixture test is optional and skipped when that environment variable is absent. It writes only an ignored local audit report. The original book/cover are not committed as test fixtures. `npm run test:epub-review` exercises the live organization endpoints using original synthetic prose; it removes only the exact organization created by that invocation. Importer/schema tests and HTTP tests do not substitute for browser or native-device verification.

Verification for this change: 62 unit/real-file checks, 138 live HTTP/Postgres checks across the dashboard suites, TypeScript, production build and production-auth probes passed. Browser review observed the real chapter list, poetry, source footnotes and paginated link jumps. Decision: D052.
