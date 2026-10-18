# Passage selection → notes / highlights

Interactive design prototype only. Sample notes in memory; no native app or storage changes.

Hold a word in Proverbs 16:3 (500ms, movement cancels), drag native text selection on desktop, or use the explicit demo action. Adjust custom handles by dragging near the desired words; focused handles support left/right arrows. The bottom contextual toolbar exposes Add note and Highlight. Three highlight colors and removal are interactive. Add note opens an anchored editor. Saving adds an inline note marker; tapping it opens all notes together with Edit and Add another note. Scenario controls demonstrate zero/one/two notes. Modal Escape/backdrop close protects unsaved edits and keyboard focus is contained.

Implementation considerations: prototype anchors use token indices for one verse, not production Unicode ranges. Production must retain exact selection/edition identity and group notes by range, supporting overlapping ranges and cross-verse selections. Prototype groups all sample notes on the same demonstration selection; it is not the production overlapping-range query. No word-study gesture on long press. Short word taps are intentionally inert in this focused mockup. Fonts inherited from the Notes study (bundled SIL OFL). Existing Notes page unchanged.
