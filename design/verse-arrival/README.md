# Verse-arrival motion studies

## Text arrival studies — selected direction

Open [text-arrivals.html](text-arrivals.html), or use [the local preview](http://127.0.0.1:8765/design/verse-arrival/text-arrivals.html). The owner rejected the wash, bar and underline treatments below. The owner also rejected Ink and requested a combination of Settle and Resolve. **Settle + resolve** is now the default: the target starts 6 px lower with slight softness, settles into place, then finishes sharpening and rests over 1.6 seconds total. Its original color is retained throughout. The separate Settle and Resolve treatments remain for comparison. Adjacent verses stay unchanged throughout. The owner subsequently approved this combination with a subtle haptic, then requested a longer, more apparent arrival and a decelerating tactile finish. Android now plays seven soft ticks that spread out and fade on supported hardware. The React/Tauri app now implements it; the standalone page remains an isolated visual preview without native haptics. See [implementation and verification](../../docs/app/tauri-migration.md#verse-arrival-and-soft-haptics-2026-10-31).

A single reader shows one treatment at a time. Choose a treatment or press Preview arrival; use Slow motion to inspect timing at quarter speed. Destination selection, dark appearance and reduced-motion previews remain available. Reduced motion substitutes a temporary static text-weight cue with no color change, and system reduced-motion settings take precedence. On narrow screens, playing a cue brings the reader into view. Each cue ends with the exact ordinary text rendering.

The standalone [HTML](text-arrivals.html), [CSS](text-arrivals.css) and [JavaScript](text-arrivals.js) use bundled fonts and no application state or remote dependencies.

Browser checks at 1280 px and 320 px cover each treatment, slow motion, another destination, cleanup, dark appearance and reduced motion. During playback, computed styles confirm that only the target text animates; neighbours retain full opacity and no animation. The combined cue was additionally verified to animate position and blur together while retaining the same text color as neighbouring verses. Reduced motion uses a static weight cue with no transform, blur or color change. There is no horizontal overflow at 320 px. JavaScript syntax and diff checks pass. This is prototype verification, not native app/device verification.

## Target-only animation wireframes

Open [target-cues.html](target-cues.html), or visit [the local preview](http://127.0.0.1:8765/design/verse-arrival/target-cues.html) with the repository preview server running. This new comparison contains **Soft arrival wash**, **Margin beacon**, and **Reading-start sweep**. These are rejected proposals retained for comparison; the app now uses the approved Settle + resolve combination above.

The three reading wireframes share the same passage and destination. Use each Replay button or Play all three; choose Proverbs 16:2–5 to try different verse lengths. Dark appearance and reduced-motion previews are available; system reduced-motion preferences always take precedence. Cues last three seconds, affect only the target verse, and clear completely. Phones appear side by side on wide screens and stack on narrow screens.

Animations and responsive layout live in [target-cues.css](target-cues.css); [target-cues.js](target-cues.js) handles replay, destination, appearance and temporary cue cleanup. All fonts are bundled repository assets. No build, network dependency or application data is required. Open the HTML directly, or run `python3 -m http.server 8765 --bind 127.0.0.1` from the repository root.

Verification: browser review at 1440 px and 320 px; replay/all-three controls, destination change, three-second cleanup, dark/reduced-motion controls, and no horizontal overflow at 320 px. JavaScript syntax check passes. These are browser design wireframes, not installed Android verification or an adopted app design.

## Earlier four-direction study

Interactive, isolated design study for picker navigation. Open `/design/verse-arrival/index.html` on the repository preview server (port 8765). Uses existing bundled fonts, no remote assets or production app state.

Four directions: ink sweep, quiet spotlight, margin trace, paper halo. Click a direction to replay, use Play all four for a sequential comparison, or Try from picker to choose verses 2–5. Light/dark and reduced-motion previews are available; system reduced motion takes precedence. Cues last 2.4 seconds and leave no permanent mark. These are alternatives for review, not shipped preferences or changes to Android.

Browser checked: each direction, picker destination change (verse 4), reader scroll/centering and replay. Mobile-first responsive layout; physical device assessment remains pending selection of a direction.

Quiet spotlight was previously selected for the app (superseded by Settle + resolve above) and refined to use contrast alone: neighbouring verses dim while the target has no border or wash. Reduced motion applies the dimming without transitions.
