# App rich highlights v1

Implemented local app extension. The published `data/schema/user-v1.sql`, SQLite `user_version=1`, canonical registry and annotation IDs are unchanged. The generated `commands-v1.json` is the input authority for the additive `highlight.save` command. `annotation.save` retains its legacy four-color/range contract; `annotation.delete` removes either kind of highlight using its stable ID.

`highlight.save` accepts an optional existing highlight ID, canonical inclusive `start_ref`/`end_ref`, a typed style and an optional edition-bound text selection. A null selection means the whole canonical range. A selection contains `translation_id` and ordered `spans`; each span has the installed `verse_id`, Unicode scalar `start`/`end` (half-open, not UTF-8 bytes or graphemes), and the exact `quote`. One to 200 nonempty spans and at most 100,000 quote bytes are accepted. The engine resolves the requested canonical range in the supplied installed edition, requires the same ordered text rows, validates every scalar bound and quote, and requires complete interior spans and adjacent boundary coverage. Merged source rows are stored/rendered once. A malformed/mismatched selection rejects the transaction without changing user data. UI handles can select across verses in the loaded chapter; the engine can accept a valid contiguous range beyond it.

The UI renders exact selections only in their anchored edition, and only while each stored quote still matches its saved scalar positions. A changed or absent edition never causes a text highlight to drift onto another phrase. User records remain stored when not renderable; automatic anchor remapping is not implemented. Whole-range highlights retain canonical behavior across editions.

Styles combine nine named colors (yellow, blue, green, orange, pink, purple, red, brown, gray) with highlighter, foreground, underline, double/triple/wavy underline, box, brackets, bold, italic, strikethrough, margin bar or a named study symbol. Symbols are nonblank labels of at most 40 scalar characters and are legal only for symbol styles. Bold uses an additional clipped glyph pass; original Scripture characters and selection indices remain unchanged. Inductive symbol presets are original labeled markers, not imported third-party artwork. This does not claim every Logos typography/effect preset.

The engine adds this app-owned table only after validating the existing user database and canonical registry:

```sql
CREATE TABLE IF NOT EXISTS fontes_highlights_v1 (
  annotation_id TEXT PRIMARY KEY REFERENCES annotations(id) ON DELETE CASCADE,
  style_json TEXT NOT NULL,
  selection_json TEXT
) STRICT;
```

An annotation and its extension are committed in one transaction. Existing `annotations.translation_id`, `quoted_text` and `anchor_content_id` retain edition/quote/source-snapshot provenance. JSON is serialized from the typed models in `crates/fontes-engine/src/highlights.rs`. The workspace adds a default-empty `highlights` array with annotation ID, typed style and optional selection. Existing notes and legacy highlights require no conversion. Editing through the legacy `annotation.save` command explicitly removes the rich extension, returning to legacy range/color behavior. Removing an annotation cascades the extension and does not remove neighboring annotations or notes.

Older apps ignore the extension and may display the retained four-color whole-range fallback instead of exact selection/style; use a current app for accurate rich-highlight display. No content download/republication is needed. Recent style choices are derived from saved highlights rather than an independent history store. Platform selection-handle accessibility and full theme contrast audit remain separate acceptance work.

## Study-pattern rendering and picker feedback (2026-10-17)

`Symbol` now renders a tinted exact-text span with repeating vector border motifs on each wrapped line, replacing the earlier single margin monogram. Existing symbol strings and IDs are retained; no annotation conversion is required. The selected color controls tint and border. Preset names map to original geometric edge families (waves, chevrons, dotted edges, cross ticks, double rules); unknown/custom names use a wave border. These are Fontes designs, not imported Logos artwork. Twenty-six labeled presets are offered. Older apps continue their older rendering of the same saved style.

The picker shows an outlined selected row with a checkmark, a persistent Selected style/color label, and a two-line live excerpt preview outside the scrollable style list. The preview uses the same decorations as saved rendering, with the current selection or canonical verse text. Preview changes do not persist until Apply; cancellation preserves the saved annotation. Long selections are clipped to a preview excerpt, not changed. Picker sizing reserves room for preview, removal (when editing), and Apply on narrow screens. No command/content/user SQL changes.
