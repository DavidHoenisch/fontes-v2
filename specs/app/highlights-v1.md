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

## Highlight removal correction (2026-11-04)

The initial layering UI mistakenly reused exact-selection/style matching for Remove. A slightly different touch selection, selecting a subset of a mark, or leaving the picker on its default style could silently find nothing and dismiss the picker. Removal now finds saved highlight annotations touching the selected canonical range and, for text-bound marks, overlapping scalar spans in the current edition. Adjacent spans do not count as overlaps. Whole-range and legacy highlights remain removable; notes and edition-specific marks from a different edition are excluded.

One match removes that annotation directly, regardless of the currently selected Apply style. Multiple matches open **Remove a highlight**, with the saved style/symbol, color, reference range and quote for each layer; only the explicitly chosen annotation ID is deleted. Other layers remain. No matches produces an actionable message instead of closing. Deletion errors stay visible for retry, and pending removal disables duplicate Apply/Remove actions. Notebook's individual-ID removal is unchanged. Applying a style retains the strict exact-selection match needed to preserve independent layers.

Verification: all 64 frontend tests and the TypeScript/Vite production build pass. Regression tests cover partial reselection of a blue underline while the picker defaults to Highlighter, a failed deletion followed by successful retry, explicit study-symbol choice over a normal fill, no-match feedback, cross-verse/whole-range/legacy candidates, different editions, adjacent spans and note exclusion. This changes presentation matching only; no engine command, schema, annotation migration or source update is required. These tests do not establish physical touch/selection behavior on Android.

The arm64 debug APK built and installed in place on the connected Samsung SM-S938U1 using the existing debug certificate. Installed SHA-256 matches the built artifact: `93654f9090bbcb1f4eda2cf679e3d03214403c99f3aefb1478736e608e9e6728`. The user database, library pointer and three receipt/high-water files compare byte-for-byte unchanged before reopening; the private rollback backup includes the previous APK and an integrity-checked user database. Cold launch returned success. On-phone removal/selection interaction remains separate from build/install and component-test evidence.

## Layered React/Tauri highlights (2026-11-04)

The native React reader renders every overlapping rich highlight as nested styled spans over a single copy of the Scripture text, placing normal highlighter fills underneath other markup and study-symbol patterns regardless of save order. Partial overlaps are split at selection boundaries. Exact selections render only in their saved edition while their stored quotes still match the source text.

Applying a style updates an existing record only when the complete canonical range, edition and ordered selection spans (including quotes) match, together with the markup kind and, for symbols, the symbol name. Different kinds and named symbols create independent annotation IDs. Thus a study symbol can overlay a normal highlight without replacing it; another color for the same kind on the exact selection edits that layer. The initial exact-selection Remove behavior is superseded by the removal correction above. Notebook removal still deletes its individual annotation. This replaces the previous any-shared-span update and first-overlapping-mark rendering. Browser fixture markup remains a single transient style per reference; native persisted layering is covered here.

The engine already permits independent highlights over the same text. No SQL, command, published contract or user-data migration is required. Existing records are preserved; marks previously overwritten by an older UI cannot be reconstructed. Older React builds can retain the independent records but display only their first overlap.

Verification: all 53 frontend tests and the TypeScript/Vite production build pass. Integration coverage checks normal-fill/study-pattern nesting independent of save order, unchanged Scripture text, selected-symbol removal and creating a new symbol without reusing the normal highlight ID. Selection tests cover complete-span/edition equality, canonical boundaries and independent kinds/symbol names. Physical-device visual/selection verification is separate from these frontend checks.

## Notebook highlight navigation (2026-11-04)

In React/Tauri, each notebook highlight's reference and excerpt form one keyboard-accessible Read button. Activating it dispatches the existing `reading.select` command with the annotation's `start_ref`, then opens Read and centers/cues that verse using the existing arrival behavior. Ranges open at their first verse; the active translation is retained. Navigation errors leave the list open and show the error. Remove highlight remains a separate action and never triggers navigation. Browser preview navigates only among its available sample passages.

Frontend integration tests verify the selected canonical reference, waiting for navigation before switching panes, destination centering/cueing despite a saved reader offset, failure handling, and independent deletion. All 43 frontend tests and the TypeScript/Vite production build pass. Installed-device interaction remains unverified for this change. No command, schema, annotation, preference or data migration is required.

## React/Tauri preview height correction (2026-10-31)

The current React/Tauri picker retains its sticky preview, with the Scripture text bounded to `min(128px, 20dvh)`. Longer text scrolls inside a named, keyboard-focusable preview region, with wrapping for unbroken text and contained scroll overshoot. The surrounding sheet body independently scrolls colors/style choices; Apply and Remove remain in its footer. This supersedes the earlier two-line excerpt sizing for this frontend. The complete selected quote and annotation remain unchanged; there is no command, data or preference migration.

Browser verification with a three-verse selection observed a 128 px preview for 432 px of text at 393×852, and approximately 114 px for 562 px of text at 320×568. Underline selection, independent preview scrolling, scrolling to the final Margin bar style, and reachable Apply were checked against the production bundle. The TypeScript/Vite build passes. These are browser-layout checks, not an installed Android/physical-device verification.
