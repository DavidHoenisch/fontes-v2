# Fontes UI assets

Local assets for implementing the approved dark study desk UI. Core UI assets require no runtime downloads.

## Contents

- `icons/phosphor/regular/`: 18 Phosphor Regular SVGs with 24 × 24 dimensions and white fills for tinting.
- `icons/png/24/` and `icons/png/48/`: matching white transparent PNGs for 1× and 2× display at 24 logical pixels.
- `fonts/`: IBM Plex Sans Regular/Medium for UI text, Lora Regular for Scripture, Noto Serif for Greek fallback, and Noto Serif Hebrew for Hebrew fallback.
- `tokens.json`: palette, highlights, typography, spacing, radii, desktop dimensions, and mobile touch dimensions.
- `assets.rs`: dependency-free Rust constants and `include_bytes!` tables for fonts and SVGs. A font's role is recorded separately from its family so UI body and medium labels can be configured independently.
- `manifest.json`: icon role mapping, upstream sources, licenses, file sizes, and SHA-256 hashes.
- `licenses/`: Phosphor MIT license and each font's SIL Open Font License.
- `previews/`: icon contact sheet and typography specimen.

## Approved sidebar density

Use the compact desktop values in `tokens.json`: 36 px navigation rows, 4 px sidebar gaps, and 6 px top/bottom padding for saved passages. Navigation section spacing is 16 px. These supersede the original roomier Figma sidebar. Mobile tap targets remain at least 44 px; primary mobile buttons are 48 px high.

## Native integration

Load the font bytes into the application's font system and retain the Noto fallbacks for original-language text. Greek and Hebrew fallback files provide glyph coverage; Hebrew reading also needs correct right-to-left layout and shaping in the UI text pipeline. Font coverage alone does not implement that behavior.

SVGs and PNGs use white artwork with transparent backgrounds. Tint them with the semantic icon color: muted `#ABB3AA` normally, accent `#DCBC83` for the selected destination, or inverse `#1C211B` on an amber button. Use the full Phosphor filenames in `manifest.json`; short semantic roles such as `book` and `notes` are also mapped there. The icon catalog and font tables in `assets.rs` can be referenced from an existing Rust module without a build step.

## Design references

- Mobile SVGs and 2× previews: `../../design/mobile/screens/`
- Mobile overview: `../../design/mobile/mobile-mockups.png`
- Approved sidebar comparison: `../../design/sidebar-spacing/sidebar-comparison.svg`
- [Desktop Figma file](https://www.figma.com/design/fdGlrRfs8AGQfpmCDhU3Bp)

The desktop frames remain in Figma. Their export, the compact-sidebar update, and prototype interactions remain pending because of the account's Figma Starter tool limit. The local mobile SVGs are static design references, not native auto-layout Figma components or a running app.

UI icons and fonts are included; Bible corpora, lexicons, commentaries, genealogy data, and a local AI model are separate application content imports.
