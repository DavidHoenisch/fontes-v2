# Fontes color and material studies

Interactive HTML explorations for owner review. All seven directions are now also implemented as selectable native themes (SPEC D036); these HTML pages remain design references rather than the app runtime. Open `index.html` or one of the seven named direction files. Keep this directory inside the repository so relative bundled font/icon paths resolve. No external runtime dependencies or network font requests.

- `sea-glass.html`: warm paper, deep teal, amber selections.
- `ink-saffron.html`: cream, blue-black ink, saffron selections.
- `charcoal-raspberry.html`: layered charcoal, plum and raspberry.
- `cobalt-citron.html`: saturated blue, porcelain and a fresh citron selection.
- `terracotta-linen.html`: linen, sandstone and burnished red clay.
- `aubergine-apricot.html`: rich plum, rose-white paper and warm apricot.
- `midnight-mint.html`: dark navy layers, a blue study card and luminous mint actions.

Each direction presents Reading, Word study, and Selection actions. Switch themes to compare identical compositions. Tap LORD to open its action sheet, open the Hebrew dictionary card, change study tabs, bookmark, copy, preview a note, or adjust reading size. Prototype note/highlight state is temporary. Reading text is KJV Proverbs 16:1–3; the Hebrew form is explicitly a dictionary headword, not an asserted aligned verse token. The definition is an abridged dictionary preview.

Local preview from repository root: `python3 -m http.server 8765 --bind 127.0.0.1`, then visit `http://127.0.0.1:8765/design/color-studies/`. Fonts/icons retain the licenses recorded in `assets/ui/manifest.json` and `assets/ui/licenses/`.

The four additional directions retain identical layout and interaction so color can be compared directly. Midnight/mint separates the text color on the dark study card from the text color on bright mint buttons. Rebuild the portable ZIP with `python3 design/color-studies/package_mockups.py`.
