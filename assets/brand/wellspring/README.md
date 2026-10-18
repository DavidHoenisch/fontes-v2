# Wellspring — Fontes identity

Selected by the owner from five concepts on 2026-10-17. The raster concept is a visual direction; these SVG masters are the production geometry. An ivory arch surrounds three broad streams above a clay basin, on deep teal. Symbol proportions and spaces are shared across all exports.

- `symbol.svg`: transparent full-color emblem; `symbol-ink.svg`: one-color `currentColor` emblem; `symbol-reversed.svg`: ivory emblem.
- `logo-horizontal.svg` and `logo-horizontal-ink.svg`: primary and one-color horizontal logos.
- `wordmark.svg`: standalone outlined wordmark. Lettering derives from bundled Cormorant Garamond Italic; retain [its OFL notice](../../ui/licenses/CORMORANT-GARAMOND-OFL.txt). No external font is needed to display the exported SVGs.
- `app-icon.svg`: square background; `app-icon-rounded.svg`: transparent outside the rounded tile, for desktop/browser.
- Android background/foreground/monochrome SVGs and `tauri-icons.json`: adaptive launcher layers. The foreground geometry is centered at 70% scale inside its layer, leaving the important silhouette inside common launcher masks. Themed monochrome uses one silhouette, without color-dependent distinctions.

Regenerate SVG masters from the repository root:

```sh
uv run --with fonttools==4.67.0 python tools/brand-assets.py
npm --prefix apps/fontes-web run tauri -- icon ../../assets/brand/wellspring/tauri-icons.json
```

The Tauri CLI writes desktop PNG/ICO/ICNS exports and the generated Android resource directory, including `mipmap-anydpi-v26/ic_launcher.xml`. Generated artwork is app identity, not a resource snapshot: no content/schema/user-data migration. `design/brand-wellspring/index.html` shows light/dark identity, small-size and mask previews. Changes to source geometry should regenerate all exports, not independently edit individual raster sizes.

Palette: deep teal `#233F47`, ivory `#F7F0E0`, clay `#AC563D`. Use sufficient clear space around the symbol. Do not stretch, add shadows/gradients, or put fine details into the fountain. The current user-facing reader header remains unchanged; the wordmark is not reintroduced there.
