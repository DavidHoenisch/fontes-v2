# Fontes book-cover art studies

Visual prototypes only, not installed application behavior. Serve the repository and open `design/book-covers/index.html`.

Three original Genesis SVG directions: Covenant Forms (landscape), Living Emblems (botanical seal), and Story Windows (narrative panels). Covenant Forms also includes Exodus, Ruth and John to evaluate consistency across books. Art is native vector paths/circles, solid fills, no embedded raster artwork, filters or external resources. Download links expose the standalone SVGs. SVG titles/descriptions describe the intended symbolism.

Genesis introduction text in `introduction.json` is extracted verbatim from the installed public-domain MHC resource `resource:matthew-henry:Gen.1.1`, specifically the paragraph beginning “We have before us the first and longest”. The opening excerpt is verbatim within that paragraph. Editorial art-theme captions and “A history of originals” display heading are prototype editorial choices, not separate Henry source metadata. The collection plan now locates book-opening MHC resources for all 66 books, with raw inspection excerpts and explicit review notes; no additional clean introduction extraction is claimed. Historical claims remain attributed to Henry. Full-canon introduction extraction and resource boundaries need review before production.

Proposed style: 600×640 art board; cream #F6EAD4, ink #253D43, rust #AD543C, gold #D9AC58, sage #8F9C83, water #6A939A. Recurring arch, shared horizon, two or three thematic motifs, flat geometry and generous negative space. The owner approved Covenant Forms as the house style on 2026-10-17. Individual compositions remain editorial proposals; the complete 66-book theme map is in `collection/plan.md` and the searchable review gallery is `collection/index.html`.

Preview typography reuses repository Lora/IBM Plex Sans plus Cormorant Garamond Italic, with existing SIL OFL files under assets/ui/licenses. CSS embeds these for in-app browser compatibility. Navigation in the phone frames is visual context; the introduction button/dialog and SVG download links work. Mobile comparison scrolls horizontally. No app or content migration.

## Complete SVG collection (2026-10-17)

All 66 first-edition book covers are now drawn. `collection/index.html` is the searchable gallery; each book opens its artwork, composition, Scripture anchors and source-review notes. `collection/fontes-covenant-forms-66.zip` contains 66 standalone SVGs, a SHA-256/byte-count manifest, editorial notes and generator source. The four original studies are preserved; `build_collection.py` deterministically composes the other 62 using explicit book-specific scenes and shared flat motif primitives. `render_collection.cjs` renders four complete contact sheets using Resvg (set FONTES_RESVG_MODULE to the installed module path).

Validation: 66 unique book mappings and distinct SVG hashes; every link resolves; every SVG parses and renders at a 600×640 viewBox; no raster image, script, external resource, filter, gradient or font dependency in the artwork. All 66 were visually reviewed as contact sheets. The gallery was checked in the in-app browser. This supersedes earlier notes describing only four illustrated books. Artwork remains a reviewable first edition; cover navigation and verified book-introduction extraction are not implemented by this art collection.
