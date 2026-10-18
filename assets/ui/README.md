# Fontes UI assets

Local assets for seven bundled selectable native themes; Sea glass & amber is the default. The owner’s ebook references supersede the original dark study-desk default (SPEC D032); compact sidebar density and embedded fonts/icons are retained. Core UI assets require no runtime downloads.

## Contents

- `icons/phosphor/regular/`: 18 Phosphor Regular SVGs with 24 × 24 dimensions and white fills for tinting.
- `icons/png/24/` and `icons/png/48/`: matching white transparent PNGs for 1× and 2× display at 24 logical pixels.
- `fonts/`: IBM Plex Sans Regular/Medium for UI text, Lora Regular for Scripture, Noto Serif for Greek fallback, and Noto Serif Hebrew for Hebrew fallback.
- `themes.json`: authoritative seven-palette catalog, semantic colors and light/dark annotation backgrounds.
- `tokens.json`: default-theme colors, typography, spacing, radii, desktop dimensions, and mobile touch dimensions.
- `assets.rs`: dependency-free Rust constants and `include_bytes!` tables for fonts and SVGs. A font's role is recorded separately from its family so UI body and medium labels can be configured independently.
- `manifest.json`: icon role mapping, upstream sources, licenses, file sizes, and SHA-256 hashes.
- `licenses/`: Phosphor MIT license and each font's SIL Open Font License.
- `previews/`: icon contact sheet and typography specimen.

## Approved sidebar density

Use the compact desktop values in `tokens.json`: 36 px navigation rows, 4 px sidebar gaps, and 6 px top/bottom padding for saved passages. Navigation section spacing is 16 px. These supersede the original roomier Figma sidebar. Mobile tap targets remain at least 44 px; primary mobile buttons are 48 px high.

## Native integration

Load the font bytes into the application's font system and retain the Noto fallbacks for original-language text. Greek and Hebrew fallback files provide glyph coverage; Hebrew reading also needs correct right-to-left layout and shaping in the UI text pipeline. Font coverage alone does not implement that behavior.

SVGs and PNGs use white artwork with transparent backgrounds. Tint them using the current context's semantic palette: muted normally, accent ink when selected, or on-accent on a primary button. The icon catalog and font tables in `assets.rs` require no runtime downloads.

## Palette and surfaces

`themes.json` contains Sea glass & amber, Ink & saffron, Charcoal & raspberry, Cobalt & citron, Terracotta & linen, Aubergine & apricot and Midnight & mint. Sea glass is the legacy/default choice. Each palette distinguishes page, chrome, sheet, raised, selection and feature surfaces, plus ink, secondary text, actions and inverse foregrounds. Persistent annotation names remain amber/sage/blue/rose, with separate readable dark backgrounds. The native UI reads this catalog at compile time, including separate glow, feature-light, action-light and selection-mark roles for the approved material treatment. The asset generator synchronizes the legacy default constants and inventory with it.

The older static SVG/PNG and Figma references retain their historical appearance. HTML color studies in `design/color-studies/` are design references; native captures in `dist/selectable-themes/` document implementation. Local captures are not proof of a published release or physical-device rendering.

## Design references

- Mobile SVGs and 2× previews: `../../design/mobile/screens/`
- Mobile overview: `../../design/mobile/mobile-mockups.png`
- Approved sidebar comparison: `../../design/sidebar-spacing/sidebar-comparison.svg`
- [Desktop Figma file](https://www.figma.com/design/fdGlrRfs8AGQfpmCDhU3Bp)

The desktop frames remain in Figma. Their export, the compact-sidebar update, and prototype interactions remain pending because of the account's Figma Starter tool limit. The local mobile SVGs are static design references, not native auto-layout Figma components or a running app.

UI icons and fonts are included; Bible corpora, lexicons, commentaries, genealogy data, and a local AI model are separate application content imports.

## App identity

The selected [Wellspring brand kit](../brand/wellspring/README.md) holds the SVG symbol, outlined wordmark, color/monochrome logos, palette/hash manifest and adaptive app-icon masters. It is separate from the UI action-icon catalog above. Tauri desktop/Android launcher assets and the web favicon are generated from this kit; user navigation icons, saved data and reader typography are unchanged. [Identity preview](../../design/brand-wellspring/index.html).
