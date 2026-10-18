# Vine’s dictionary: requested integration and source review

Review date: 2026-10-17. Status: **pending source and redistribution-rights review; not imported, bundled, installed or enabled**.

The owner requested Vine’s in the complete default resources and study tools. The original *Expository Dictionary of New Testament Words* must be distinguished from the later *Complete Expository Dictionary of Old and New Testament Words*, which includes additional authors, OT material, revisions and indexes. No edition has been selected for distribution.

## Evidence and unresolved decision

- [StudyLight’s Vine entry](https://www.studylight.org/dictionaries/eng/ved/e/express.html) identifies a 1940 NT dictionary and labels its files public domain.
- [Publisher copyright page reproduced in the licensed Biblia resource](https://biblia.com/api/plugins/embeddedpreview?historybuttons=false&layout=minimal&navigationbox=false&resourceName=LLS%3A46.10.8&sharebutton=false) identifies the later compilation and states that the NT dictionary copyright was restored; it reserves reproduction rights. This is contrary evidence, not a conclusion about every historical edition in every jurisdiction.
- [F. F. Bruce resource-site account](https://www.ffbruce.com/w-e-vines-dictionary/) discusses restoration on behalf of the Vine estate.

A web reader being free to access is not an identified grant to redistribute its full text in our downloadable database. Resolve the exact edition, applicable territories and permission covering the supplied electronic transcription before ingestion. No bulk scrape, source checksum, rights-approved lock entry or fabricated module has been added.

## Implementation path after source approval

1. Pin the authorized source archive/transcription with checksum, revision, attribution, rights evidence and adapter ID in the existing source lock. Add a reproducible importer and source-specific validation report.
2. Preserve original entry text and headings as dictionary resources with stable source-scoped IDs. Preserve explicit Strong’s associations as definition links and explicit Scripture citations as canonical links. Do not infer original-language alignment from English entry names.
3. Test actual entry counts, Unicode, citations, cross-entry references, supplied Strong’s links and exact-excerpt collection. Report unresolved references without fabricating targets.
4. Include the approved source in the complete default build with notices and required-source checks. Preserve old published profile/manifest compatibility; do not silently redefine historical releases. Build and verify a new signed snapshot before making an update available.
5. Surface its own Library cover/title and entries, linked definitions in word study, cited passages in resource previews, resource search and attributed note collection. Verify on Android against the new installed snapshot. Retain Strong’s as a separately attributed dictionary; multiple definitions must remain selectable.

The generic dictionary/resource and citation facilities already exist. This document is a plan, not an importer, release or claim that Vine’s is available in the app. `data/profiles.json` records the pending source outside `required_sources`.
