# Approved visual reference

The owner supplied these eight captures on 2026-10-16 and named the HTML mockups the gold standard. Treat the composition, proportions and material detail as an implementation target, not loose inspiration. The source remains `../styles.css` and `../app.js`.

Match logical widths before comparing native and browser screenshots. The reference grid resizes its devices; browser DPR and zoom affect exported pixel dimensions. The 43px simulated status bar, device bezel, camera and home indicator are presentation chrome, not app UI to paint over Android system bars. Preserve actual safe areas and 44px native touch targets.

Native reference-flow captures use 340×729 (340×772 minus 43 simulated status pixels) and 390×844. Reading uses 24px Lora at 1.63 line spacing by default for newly created preferences; older saved typography is retained. Chapter type is 42px/1.16 with −1.8px tracking; the reading page has 24px chapter insets and 18px verse insets. Word cards use 53px source text, small tracked labels, serif transliteration, a separate footer/code badge and restrained rings. The action sheet previews only headwords returned by the installed dictionary. Preferences starts with size and live preview; theme/typeface/spacing options stay available behind a disclosure.

The mockup's explanatory/sample prose is not corpus data. Native dictionary/commentary/reference/topic cards must show actual installed titles, excerpts and attribution, not substitute the illustrative copy or invent a gloss. The native blur uses only an app-owned frame in RAM and never captures the OS desktop or sends/saves that frame.

Verification: `real_library_reference_fidelity`, the existing real-library flows and regular theme contrast/persistence checks. Native text rasterization differs from browser rasterization; screenshots and tests establish the observed level of fidelity, not a claim of pixel identity or physical-phone verification.
