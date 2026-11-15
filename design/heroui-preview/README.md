# Live HeroUI / Fontes exploration

The live implementation is now `apps/fontes-web`, not a separate static HTML mockup. Run its Vite dev server on http://127.0.0.1:5173. UI changes are made directly in the React/TypeScript source intended for Tauri. Screenshots in this directory are review evidence only, not the implementation authority. See the frontend README for working interactions, fixture scope and unimplemented native integration.

`verse-resource-sheet-dark.jpg` (2026-11-04) captures the actual production resource-sheet React component/CSS/fonts at 390×760 in a temporary isolated visual-review entry. Reference targets come from existing preview fixtures; the two commentary/four dictionary/six topic entries are explicitly synthetic layout data, never installed user/library records. The review entry was removed after checking category/article/Back/Close behavior and light/narrow layouts. This capture demonstrates browser styling, not an installed-phone check. See the [implementation and limits](../../docs/app/tauri-migration.md#verse-resource-visual-fidelity-correction-2026-11-04).

`search-workspace.jpg` (2026-11-04) captures the actual App and production search workspace at 390×852 after an Enter-submitted `LORD` search in the explicitly labeled browser sample Scripture. It demonstrates the selected layout and outer-only field focus treatment; live installed-library scopes and reference parsing are verified separately through engine/command integration tests. See [behavior and native installation evidence](../../docs/app/search-workspace.md).

`share-composer.jpg` and `share-card-export.png` record the Scripture share-card implementation (2026-11-04). The first is the actual production composer at phone width; the second is a real 1080×1080 PNG downloaded through its browser action, with full passage/edition attribution. See [scope, provenance and verification](../../docs/app/share-cards.md).

`fontes-share-credit.jpg` captures the current italic Fontes credit after the owner corrected the initial Wellspring wording. Earlier share-card captures retain the previous credit as historical export evidence.

`selection-dock.jpg` records the mobile selection controls in their own layout track, outside the scrolling passage (390×820). Geometry checks confirm zero overlap and a 44×44 Close target; this is browser layout evidence, not physical Android selection verification.

`psalms-picker.jpg` records the responsive chapter grid using the actual App with an isolated synthetic native-shaped Psalms fixture. All 150 chapters and Psalm 119's 176 verses were reviewed at 390/320 px widths without horizontal cell overflow; see [scope and measurements](../../docs/app/tauri-migration.md#passage-picker-number-grid-2026-11-04).
