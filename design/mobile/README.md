# Fontes mobile mockups

Eight 390 × 844 mobile screens matching the desktop study desk direction:

1. Reading
2. Book selection
3. Chapter and verse selection
4. Contextual word study bottom sheet
5. Note and highlight
6. Local AI search with a separate Scripture source
7. Genealogies with relationship evidence
8. Reading preferences

Open `mobile-mockups.png` for the overview. Individual screens are in `screens/`, as vector SVGs and 2× PNG previews.

## Figma import

Drag the eight SVG files from `screens/` into the existing Fontes Figma file. Each file is 390 × 844. The SVGs contain named vector groups and text elements, with no flattened UI image. Figma's SVG importer may outline text; these are import assets, not native Figma components or auto-layout frames. Native Figma creation remains pending because the account's Starter MCP tool quota is exhausted.

Install the included font files if needed to retain the intended typography when editing the source SVGs: Lora for reading, IBM Plex Sans for controls, and Noto Serif for polytonic Greek. Font and icon licenses are included in `assets/`.

## Mobile behavior represented

- Four bottom navigation destinations: Read, Search, Notes, Study.
- Word study opens as a bottom sheet above the reading passage.
- Passage navigation progresses from book selection to chapter and optional verse.
- Chapter and verse cells have 44 × 44 targets; action buttons are 48 high.
- Full-screen note editing keeps the save action near the bottom.
- Search explains locally generated answers separately from quoted KJV Scripture.
- The family tree surfaces the cited evidence for a selected relationship.

These are static UI mockups. Search, word-study details, notes, and selection states are illustrative, with no running backend or click-through interactions.

## Validation

Text bounds checked against each screen. All screens rendered to PNG using the included font files and reviewed as a composition. Polytonic Greek checked separately after adding its font fallback.
