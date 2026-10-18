# Fontes React + Tauri application

**Native integration added 2026-10-17.** Packaged Tauri mode now uses the Rust engine and existing installed library/user store. The ordinary browser preview below remains fixtures. See [current migration status and build instructions](../../docs/app/tauri-migration.md). The original exploration notes below describe the browser-only boundary and are superseded for native mode.

# Fontes React frontend · HeroUI v3 exploration

This is an executable React 19 / TypeScript / Vite frontend using the actual pinned HeroUI React and styles packages (3.2.6) and Tailwind CSS 4. Its components are intended to become the Tauri presentation layer; this is not an HTML reference to later reproduce in egui. It deliberately keeps HeroUI's component defaults while carrying forward Fontes artwork and typography.

Run from the repository root:

```sh
npm ci --prefix apps/fontes-web
npm run dev --prefix apps/fontes-web
npm run build --prefix apps/fontes-web
```

Preview: http://127.0.0.1:5173 . The development server binds loopback only. Phone/wide controls are development scaffolding outside the application shell. Responsive production CSS uses the same React components. No CDN runtime assets or remotely hosted fonts.

## Implemented for review

Read, Study, Notes and Library navigation; actual HeroUI cards, buttons, tabs, chips, inputs, textareas and modal dialogs; Fontes fonts and all 66 Covenant Forms SVG covers with Henry introductions. Reading includes the first four KJV verses of each book plus featured passages in Psalms 23, Proverbs 16 and John 3 (the original featured Genesis fixture contains three verses). First-chapter excerpts were copied read-only from the published Study library, not manufactured as new canonical data. The H3068 occurrence snapshot uses the engine's distinct-canonical-verse counting query; `src/data/occurrences.json` records its content ID. These are bounded preview fixtures, not a live library.

Notes support search, tabs/empty states, pinning, modal reading, a scrapbook editor, insertion of sample thought/passage/Henry/dictionary pieces, reordering and explicit saving. Preview note documents persist only under browser-local key `fontes-heroui-preview-notes-v1`, isolated from installed app annotations. Delete/discard use HeroUI confirmation dialogs. Highlights/bookmark and theme changes currently last for the mounted session. Browser text selection/right-click/hold demonstration opens a note/highlight context card; highlighting marks the selected quote rather than the full verse. The hold fallback selects a first word; full mobile-native selection ergonomics still require physical WebView testing. Inline markers currently group notes at verse level. Search is book discovery, not engine full-text search. Source modals show retained fixture excerpts.

Default HeroUI blue plus optional Clay/Sea glass accents and light/dark appearance. Artwork, Plex UI text, Lora Scripture, Cormorant italic headings and Noto Hebrew are repository assets; original font license files remain under assets/ui/licenses. Dependency licenses are governed by the installed packages and lockfile (HeroUI 3.2.6 packages identify MIT).

## Integration boundary / limitations

`src/data/preview.ts` contains preview fixtures/storage; no UI SQL and no calls to the native app's user store. Tauri shell, permissions, Rust command adapter, signed library bootstrap and installed-data compatibility are **not yet connected**. Production data must map to the existing engine command/view models, including Unicode scalar selection offsets, source verification/provenance and note IDs. Preview note types and quote/reference anchors are not a replacement for those versioned Rust contracts.

The migration should reuse this component/CSS build inside Tauri, changing the data adapter rather than translating the design again. Native package identity/signing, Android user-data path migration, selection gestures, IME/safe areas, offline launch, scrolling and visual comparisons on S25 need validation before replacing the installed frontend. No old frontend or saved app data was removed. Build succeeds; Vite reports a large initial JS chunk, so route/data splitting remains a packaging optimization task, not a claimed performance result.

Verification: TypeScript production build; browser review of reading/notebook, HeroUI modal and scrapbook insertion. Mobile-device appearance and complete accessibility conformance are not claimed by browser checks.

Feedback pass (2026-10-17): restored an anchored two-face CSS 3D word-card flip (instant change for reduced motion), added a Recharts top-five-book horizontal comparison alongside exact occurrence counts, expanded thought textareas to the editor width, removed the compact header wordmark, and aligned passage navigation left. The passage sheet now searches canonical books grouped by testament before choosing a chapter; only fixture chapters are offered until the engine bridge is connected. Browser review checked both flip faces, chart rendering, book/chapter navigation, and the full-width editor. Production build passes; physical-device behavior remains unverified.

Selection styling pass: compact reference/quote toolbar with Note, Highlight and dismiss; a separate HeroUI style sheet includes nine colors, twelve markup treatments and the existing study-symbol names. Selected controls show checkmarks/pressed state, with a sticky selection preview, Apply and Remove. CSS decorations clone across wrapped inline fragments; study motifs tile across the text as SVG backgrounds. These are Fontes preview treatments rather than exact reproductions of third-party designs. Highlights remain in-memory fixtures (one selection per verse); persistent multi-range storage and installed-library integration remain pending. No native annotations are modified.

Library hierarchy: top-level entries represent resources/editions, not the 66 canonical Bible books. KJV opens its internal Bible-book grid; Matthew Henry opens its introduction contents; Strong’s opens its sample entry. WEB is a separately identified edition placeholder, explicitly unavailable in these fixtures. The Rust registry remains the future authority for installed editions and resource availability.


TSK pass: verse numbers open cross-reference sheets. The native sheet loads typed installed cross-reference links and calls `reading.preview` to read a target without moving the reader; Back restores the reference list. The browser uses a lazily loaded, read-only TSK/KJV sample copied from the verified Study DB for four featured chapters (first four verses). It does not invent cross-reference links. Navigation is 56 px before device safe areas with 48 px buttons. Native APK built; a disconnected phone is not an on-device UI verification.


Unified startup: native mode shows a single **Install free resources** option only after confirming no installed library. While opening an existing library it emits no setup/loading card; desktop visibility is deferred until ready/setup/error. Real open errors retain Retry. `npm test` runs the cold-start regression suite. The installer always requests the complete shared default bundle; old profile identifiers remain backend compatibility details.

Floating navigation refinement: the compact/mobile bottom bar is inset 12 px, pill-shaped and softly elevated. The scroll area reserves clearance so its last content can move above the bar; selection actions, the new-note button and editor dock share the same safe-area-aware spacing. Native desktop keeps its rail and omits bottom-navigation clearance.

Note-insertion responsiveness: removed the eager passage/context/eight-resource command chain before opening the sheet. Native insertion now opens immediately, uses the loaded workspace for thought/passage actions, progressively loads related summaries, and reads only the selected full resource. Context promises are deduplicated in a 16-entry snapshot/translation/verse cache. Loading/error states stay inside the picker; late selections do not append after it closes. Regression tests use deferred responses rather than a fabricated timing threshold. This does not claim measured Android frame latency.
