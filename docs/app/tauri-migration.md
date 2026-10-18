# Tauri presentation migration — 2026-10-17

The approved React/HeroUI frontend in `apps/fontes-web` now has a Tauri 2 shell in `src-tauri`, using the same components/CSS as the browser preview. `native.tsx` is the typed adapter to Rust commands; it never opens SQL. `isTauri()` selects live data in packaged apps, while ordinary browser preview keeps isolated sample data. Browser sample notes are never imported into the user database.

## Implemented

- Bundled offline React, fonts, SVG artwork and all 66 Henry introductions; development preview chrome is omitted in native mode.
- A bounded serial engine worker shared with the older frontend; Tauri exposes `engine_dispatch`, `library_open` and explicit `library_install` transports. The engine validates commands and writes.
- Existing signed-library verification and first-run download service. Download starts only from the explicit setup button. Existing desktop data directory and Android `org.fontes.app/files` are reused, including `user.sqlite`, `library-path.txt` and `snapshots-v1`.
- Complete active-edition chapters, canonical navigation, linked-word study, actual dictionary definitions, source-token choices, occurrence chart/counts, book filters and paged occurrences.
- Installed translations as parent resources; installed study sources grouped via additive `library.catalog`, with 50-entry article pages. Full-text search uses the engine, 30 hits per page.
- Reading preferences and favorites, saved note documents, pin/edit/delete, source-verified scrapbook clips, existing notes/highlights, styled selection Apply/Remove. Native browser selection captures Unicode scalar offsets and ordered spans across verse boundaries. The engine is the final validation authority.
- Source/third-party notices are reachable through Appearance, apart from study content. `tools/frontend-notices.py` gathers installed package and bundled font notices during builds.

## Build and run

```sh
npm ci --prefix apps/fontes-web
npm run build --prefix apps/fontes-web
cd apps/fontes-web
npm run tauri -- dev
npm run tauri -- build --debug --bundles app
npm run tauri -- android build --debug --target aarch64 --apk
```

Android requires the configured Java/SDK/NDK environment. Generated Gradle project is checked in; outputs, machine paths, JNI symlinks and caches are ignored. Desktop supports `FONTES_DATA_DIR` and `FONTES_LIBRARY` for explicit local/test libraries. Android uses its private existing files directory; it does not expose arbitrary filesystem paths to JavaScript.

Application identifier remains `org.fontes.app`. Android versionCode is explicitly above the previous cargo-apk code so in-place updates work. Development APKs use the matching Android debug certificate. Production signing/distribution is separate; this is not a signed public release or tag. Before switching the S25, its user database/path preference and previous APK were retained outside the repo under the user's local backup directory. No uninstall or data clearing was used.

## Verification and limits

TypeScript production build, Rust Tauri check, engine tests (including parent catalogue paging and existing notes/highlights validation), and legacy UI regression tests pass. A macOS `.app` and arm64 Android APK build; the S25 launched the new frontend against its retained library and rendered an existing styled highlight. The remaining physical WebView selection/IME, accessibility, scroll-performance and exhaustive interaction checks are not inferred from engine tests. Initial JS chunk splitting and debug APK size remain optimization work.

The browser preview is still fixtures. No browser-local notes are authoritative native records. The prior egui frontend and packaging tools remain available for rollback, not deleted in this migration. Existing SQL schema versions, note/highlight extension formats and permanent IDs are unchanged. `library.catalog` is an additive command/view model documented in `specs/app/README.md`.

Final local evidence: 19 engine integration tests and 12 bootstrap tests passed on macOS; 18 engine integration tests and 12 bootstrap tests passed on the S25 (before the additional large-catalog pagination test was added). Legacy egui regression run passed its six enabled tests, with 17 interaction tests explicitly ignored by that target. Tauri/engine clippy with warnings denied passed. The final APK updated in place and cold launch returned success. A read-only comparison of the pre-migration backup and post-install user DB retained every original annotation ID, and SQLite integrity_check returned `ok`. Final screen capture encountered the phone's sleeping display; the retained screenshot records the earlier successful Tauri launch, not a claim of exhaustive final-build UI testing. CI was extended with frontend build/Tauri adapter checking but has not run remotely in this session.

## TSK and cross-reference preview pass

New Reading builds include locked TSK data and a cross-reference capability; Study already contained it. Existing installed Study snapshots need only the new UI to expose verse-number browsing. Older installed Reading snapshots remain valid but need a newly published signed data release to gain TSK; no signed installed snapshot was patched in place. Local `dist/reading-with-tsk` and `dist/reading-with-tsk-repeat` builds contain 31,153 TSK resources and compare byte-for-byte equal. The known 60 unresolved source citations remain in the pre-existing reviewed audit; no invented targets replace them. These local development outputs are not a published release.

The cross-reference sheet reads actual context targets and uses the new `reading.preview` command for target text. It never dispatches navigation. Engine tests assert unchanged session/history and merged-segment deduplication. Browser review opened Proverbs 16:1 → Exodus 4:11 in place, and retained Proverbs in the underlying reader. Source/engine/contract/pipeline tests and clippy pass. The bottom bar is 56 px plus platform safe area, with 48 px controls. Updated native builds are prepared; no phone was connected for installing this pass.


## Unified default bundle and startup refinement

New Tauri installs expose one complete **Free resources** bundle. The internal `study` artifact remains the compatibility identity; legacy Reading packages are still recognized rather than mutated. Pipeline and CLI defaults now select the complete bundle. Startup distinguishes checking, missing, ready, explicit installation, and genuine error. The pending state contains no text/card or install action; desktop visibility is deferred until the ready/setup/error state commits, with a narrowly scoped window-show permission. Frontend regression tests cover this sequence. Real download progress and real failures remain visible. Verse-number hover/focus is round at 44×44 px.

## Add-to-note latency correction

The previous handler waited for `note.passage`, `study.context`, and up to eight sequential `study.resource` commands before opening its sheet. It now opens immediately. `NativeInsertChoices` derives the exact active passage from the existing workspace, loads context summaries progressively, and fetches full content only for the selected resource. A bounded in-memory cache avoids repeated identical context reads; changing content/edition/verse invalidates the key. Deferred-response tests cover immediately available actions, lazy full-content retrieval/provenance, repeat opens/new snapshots, and ignoring late responses after unmount. Source validation remains in the engine's existing note transaction. Android wall-clock timings are not inferred from these frontend tests.

## Additional default dictionaries (2026-10-17)

Library presentation recognizes Abbott-Smith and Easton. Study exposes Dictionaries for cited articles and a source chooser for multiple word definitions. Strong’s remains the initial dictionary where present; Abbott-Smith is independently attributed and linked only through supplied standard G codes. Note excerpts and Scripture previews retain source/reference identity. These controls consume installed data, not browser fixtures; see the [source integration and rollout boundary](../research/abbott-smith-easton.md).

Study section navigation refinement (2026-10-17): Words, Commentary, Topics and Dictionaries share one compact row of content-sized labels with guaranteed gaps and an animated underline instead of a two-row group of filled action pills. Meaning/Occurrences retains its inner segmented control, distinguishing section choice from word detail. Buttons retain 48px touch targets, visible focus, pressed-state semantics and arrow/Home/End support; reduced motion removes transitions and narrow layouts scroll horizontally instead of squeezing or overlapping labels; the underline follows the measured active button. The browser and native app share the same component. This is presentation only, with no source/query/annotation migration.

Occurrence browsing refinement (2026-10-17): book counts now expand reference headings inside the same book; each heading expands to a complete active-edition `reading.preview` in place without navigation. Native pages append twenty references at a time under the expanded book, with loading/retry states and cached local disclosure state. Additional books can be revealed without losing the initial overview. Changing the word, edition or content generation resets disclosure caches. The chart now sorts its top five by distinct-verse count; the book browser retains its supplied order. Browser fixtures include the first eight real KJV references/texts per book for H3068, tied to their source content ID; they are isolated samples, not installed user data. Existing engine contracts, canonical IDs and user records are unchanged.
