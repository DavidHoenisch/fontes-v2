# Fontes application

This is the first working application slice, using real content-v1 reading/study snapshots. It is an application foundation, not the completed product or resource installer. Start with [SPEC.md](../../SPEC.md), the [engine architecture](architecture.md), and the [command contract](../../specs/app/README.md).

## Run on desktop

From the repository root, with the pinned Rust toolchain:

```sh
cargo run --locked -p fontes
# Choose Reading or Study, Continue, then Download and open.
# Advanced: open a trusted locally compiled snapshot:
cargo run --locked -p fontes -- --library dist/reading/library.sqlite
# A study profile enables the installed dictionaries, Greek tokens and linked articles:
cargo run --locked -p fontes -- --library dist/study/library.sqlite
# Preview the touch layout in a narrow desktop window:
cargo run --locked -p fontes -- --library dist/reading/library.sqlite --mobile
```

First launch offers a [guided library download](first-run-setup.md) on desktop and Android; no filesystem path is required. Signed release checks, download/preparation progress, cancellation and retry run in the shared background service. Once installed, reading opens offline. The remaining instructions describe optional developer imports.

Build the real [reading/study data](../../README.md#build-the-reading-data) first, or use the existing data-release verifier on downloaded signed assets. For a new local reading build:

```sh
cargo run --locked -p fontes-data-pipeline --bin fontes-data -- build \
  --profile reading --output dist/reading --builder-commit "$(git rev-parse HEAD)" \
  --mirror-repository DavidHoenisch/fontes-v2
```

The output directory must be new or empty. `--offline` rebuilds from an already populated verified source cache. No Scripture is embedded in the UI executable, and the app downloads content only after the setup wizard is confirmed. Explicit desktop arguments and Reading Preferences still accept a trusted local SQLite path. This is an **explicitly trusted local development import**: opening a file checks its schema and permanent reference registry, but does not authenticate a release signature. Do not treat it as the future verified installer.

`--data-dir <directory>` overrides the writable app-data location. Defaults come from the operating system's local application-data directory through `directories::ProjectDirs` (`org`, `Fontes`, `Fontes`); consult the explicit override when scripting rather than guessing its platform spelling. It contains:

- `snapshots-v1/`: verified downloaded generations, activation receipt, replay high-water record and private setup staging. This is separate from the future library-v2 composed store.
- `user.sqlite`: notes, highlights, favorites, recent references and last reading/preferences state; never replace it with a data release.
- `library-path.txt`: the last successfully opened local content path. An optional `library-path.tmp` is only staging for this preference, not content or user-data recovery.

Once opened successfully, the app remembers the content path and reading session. To try an isolated library without changing normal notes, supply a separate `--data-dir`. Content and user databases must be different files. Back up `user.sqlite` using SQLite backup tooling while closed or through a consistent SQLite backup operation; copying an actively written database arbitrarily is not a backup protocol.

Downloaded Windows test binaries use the Microsoft v14 runtime (`VCRUNTIME140.dll`, confirmed in their import tables). If it is missing, install the **x64** [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170) before launching; the portable ZIP does not install prerequisites.

On Linux, install `libxkbcommon-dev` and `libwayland-dev` for native builds. Windows uses the Rust MSVC toolchain; macOS uses Xcode command-line tools. The application explicitly selects eframe's **wgpu** renderer; no glow renderer or webview is selected. UI fonts/icons are bundled with their [licenses](../../assets/ui/licenses/).

## Working flows

- Read KJV/WEB, select a verse, navigate across chapter/book boundaries, and switch translations without moving canonical selection or losing annotations/study details. Missing source text stays explicit; merged source segments render once.
- Open the book/chapter/verse picker from the passage button or Cmd/Ctrl+K. Type `John 3:16`, `j3 16`, `Rom 1:3`, or `Psalm 23`. Invalid/ambiguous input keeps the existing location and gives an error. Escape closes the picker/editor. Bookmark passages and reopen recent/favorite locations.
- Search literal words in the active translation and installed articles, with 30-result pages. This is keyword search, not generated AI prose.
- Use Notes to add/edit notes or highlights and bookmark a verse. Verse context menus also offer note/highlight/copy/study actions. Notes use permanent inclusive canonical ranges; the GUI currently creates single-verse notes, while commands also accept ranges.
- On desktop, Study opens a resizable companion panel while the passage stays visible. Choose Words, Commentary, Cross-references or Topics. Commentary/reference results default to articles covering the verse; enable **Include mentions elsewhere** for citations in other articles. Cross-references show named passage links with previews from the active translation (the first verse for a range); source articles remain available. More specific source coverage is listed first. Open articles and their Scripture links in the panel. Article prose uses spaced source paragraphs, readable line height and bounded wrapping; articles open at the beginning. Legacy Matthew Henry numeric escapes such as `&#224;` display as characters without modifying the content database.
- Drag/double-click to select Scripture text and right-click (mobile uses the grouped action sheet). The new Study build supplies KJV dictionary tags: **Look up Strong’s G…/H…** opens definitions and a prominent, labeled dictionary headword. Tags are edition-specific; 1 John 2:23 is withheld for a source-text mismatch. Selections without tags, including WEB, offer **Look up original word…**. The verse chooser now includes OSHB Hebrew/Aramaic words, source morphology and occurrence links, with source omissions preserved. Choosing a source word promotes its actual supplied form. Dictionary tags do not assert a match to a particular Hebrew/Greek verse token. Source credits/licenses are available in Reading preferences → Theme & typography → Library settings → Source credits & licenses. Older installed Study libraries still require an updated published library to gain this data.
- Reading Preferences controls Lora/IBM Plex Sans, 16–36 px text, 1.2–2.2 line spacing and verse numbers. Mobile opens a dismissible bottom sheet over the current destination with a live verse preview; Library settings opens the full settings page. The working default is paper-and-sea-glass (warm ivory, dark ink, deep teal); owner visual approval, theme selection and weight selection remain open.

The reading toolbar uses one continuous surface with flat controls, consistent heights and transient hover/press feedback. The passage uses available space and visually truncates long names while preserving its full label.

At widths below 760 logical pixels the sidebar becomes Read/Search/Notes/Study bottom navigation. Targets are at least 44 px; primary actions are 48 px. The picker shows one Book, Chapter or Verse grid at a time. Book/chapter taps advance in place and reset the next grid to its top; a verse tap opens it directly. Fixed step tabs revisit choices, natural-reference input stays visible, and pinned Open can open the current draft early. Recent/favorite shortcuts are on the Book step. Note Save/Cancel actions also stay at the bottom while content scrolls. At the narrowest width the toolbar bookmark moves to Notes. At wider widths the 208 px compact sidebar returns; a 320 px study invitation appears when the remaining reading width exceeds 1,050 px. Opening Study instead displays a resizable companion panel at desktop widths, initially 260–420 px depending on available space. These are initial implementation breakpoints, not a completed accessibility/device evaluation. Mobile word-study results currently use the Study destination; selection actions use a bottom sheet, while a sheet containing dictionary results remains future work.

## Headless application

```sh
cargo run --locked -p fontes-cli -- commands
cargo run --locked -p fontes-cli -- \
  --library dist/study/library.sqlite --user-db dist/headless-user.sqlite \
  run reading.navigate --params '{"reference":"Rom 1:3"}'
cargo run --locked -p fontes-cli -- \
  --library dist/reading/library.sqlite --user-db dist/headless-user.sqlite json
cargo run --locked -p fontes-cli -- \
  --library dist/study/library.sqlite --user-db dist/headless-user.sqlite mcp
```

`fontes-cli setup --data-dir DIRECTORY --profile reading` (or `study`) uses the same verified bootstrap service as the GUI and prints the managed library path. Setup needs no `--library` or `--user-db`. It is a bootstrap lifecycle operation, not an initialized-engine JSON/MCP tool.

CLI, JSON and MCP use the same typed command dispatch as the UI. JSON and MCP are line-delimited stdin/stdout, with no HTTP server/listener. Logs/errors outside protocol replies go to stderr. Configure the MCP client to launch `fontes-cli --library <absolute-path> --user-db <absolute-path> mcp`; MCP tools include durable local writes and state replies include private annotations. Only connect a client you intend to grant access to that user database. This is a separate headless session, not IPC remote control of an already-running GUI; sessions share stored annotations but do not broadcast navigation events between processes. See [wire details](../../specs/app/README.md).

## Android development

The native arm64 library and debug APK build locally and in CI. All 22 engine/bootstrap tests execute on an Android API 28 x64 emulator in CI and an API 36 arm64 emulator locally; a full real Study download/verification/open also passed through the shared CLI service on Android. The APK uses the same guided first-run downloader with INTERNET permission and app-private storage; no storage permission or filesystem path is required. Android uses native flock because the pinned Rust standard lock API is unsupported there. GUI emulator checks encountered graphics-driver failures, recorded in [verification](verification.md#android-setup-lock-regression--2026-10-16). Android system-bar/cutout insets are now bridged into egui and the root UI is constrained to the safe content area; see [safe-area evidence](verification.md#android-system-safe-area--2026-10-16). Physical-device networking, soft keyboard/IME, lifecycle recovery, safe areas and screen-reader support still require validation. A Storage Access Framework import picker is not implemented. iOS is deferred.

Install Android SDK platform **35**, build-tools **35.0.0** (36.0.0 also used successfully locally), NDK **28.2.13676358**, a JDK, and the Rust target. Set `ANDROID_HOME`, `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, and `JAVA_HOME` to the actual installations. Both NDK variables must name the pinned NDK: cargo-apk gives an existing `ANDROID_NDK_ROOT` precedence. NDK r28's `cargo-apk` platform discovery supports up to API 35; having only SDK platforms 36/37 produces the misleading “SDK has no platforms installed” error. The app has min API 28 and target API 35.

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk --version 0.10.0 --locked
cargo fetch --locked
CARGO_PROFILE_DEV_OPT_LEVEL=2 CARGO_NET_OFFLINE=true cargo apk build -p fontes-android --lib
```

`cargo-apk` 0.10.0 does not accept `--locked`. Fetch the full locked workspace graph first (cargo-apk also resolves host/workspace metadata), package offline, and verify Cargo.lock has not changed (CI checks its digest). The APK is `target/debug/apk/fontes-android.apk`, signed with the local Android **debug** key and with native debug symbols stripped from the package. This is not a production signing/release process.

Install the APK and use its Reading/Study setup wizard. The following optional offline sideload recipe is provided for development; it has not been executed here because no device/AVD was available:

```sh
adb install -r target/debug/apk/fontes-android.apk
adb shell am force-stop org.fontes.app
adb shell run-as org.fontes.app mkdir -p files
adb shell -T "run-as org.fontes.app sh -c 'cat > files/library.sqlite'" < dist/reading/library.sqlite
adb shell am start -n org.fontes.app/android.app.NativeActivity
```

Managed libraries live below the app’s private `internal_data_path()/snapshots-v1/`, separate from `user.sqlite`. The old `internal_data_path()/library.sqlite` location remains a legacy development import fallback. `run-as` is only a debug-build development path. Uninstalling the app can remove its private notes; backups/export and production import are still required before user distribution. NativeActivity avoids an app-authored Java/Kotlin layer, but eframe 0.35's Android AccessKit backend requires GameActivity. Desktop AccessKit is enabled; Android accessibility is an explicit open gate, not silently claimed from desktop tests.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
# Real-corpus interaction and actual wgpu render checks (adapter required):
FONTES_TEST_LIBRARY="$PWD/dist/study/library.sqlite" \
FONTES_CAPTURE_DIR="$PWD/dist/app-captures" \
cargo test --locked -p fontes-ui-egui real_library_desktop_and_mobile -- --ignored
```

The ignored test is deliberate: ordinary headless CI need not have a GPU or a corpus. It checks the real verse, picker, save-note, search and study flow at 1440×960, 390×844 and 320×640, and exports PNGs for review. Run it on a reading profile as well; the study view should explain missing optional data. The normal suite includes recoverable first-run UI tests and engine integration tests for parsing, translation/merged segments, durable annotations, typed links/analyses, search, transports and wrong-file protection.

[Application CI](../../.github/workflows/application.yml) builds/tests Windows x64, Linux x64, macOS Apple Silicon/Intel and an Android arm64 APK, and executes native engine/bootstrap tests on Android API 28. It retains downloadable platform archives for 14 days and publishes a versioned development prerelease only after all jobs pass. [The corrected Android setup test release](https://github.com/DavidHoenisch/fontes-v2/releases/tag/app-v0.1.0-dev.9.1) and [application run](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38927379496) are verified green. See [testing build setup](testing-builds.md) and [verification results](verification.md) for exact evidence. The data publisher continues testing only the three data/publisher crates, so UI/GPU dependencies do not become content-compilation dependencies.

See [verification notes](verification.md) for observed results and remaining release gates. The source-package compiler/catalog installer, resource-native v2 graph, automatic updates, AI runtime, curated genealogies/overviews, production signing/installers and live GUI automation channel remain separate milestones.

## Selectable color themes

Open **Reading preferences → Color theme** on desktop or the reading toolbar on mobile. Choose Sea glass & amber, Ink & saffron, Charcoal & raspberry, Cobalt & citron, Terracotta & linen, Aubergine & apricot, or Midnight & mint. Each swatch previews its page, feature and selection colors. Changes apply immediately across reading, study, menus and settings and survive restart. All palettes are bundled and work offline; no Study library update is needed. Sea glass remains the default for older preferences. Android requests matching light/dark system-bar foregrounds; physical-device appearance still needs verification. Published in [app-v0.1.0-dev.11.1](https://github.com/DavidHoenisch/fontes-v2/releases/tag/app-v0.1.0-dev.11.1); installation and word-study rendering were observed on a Samsung SM-S938U1.

The native layout now follows the color-study mockups as well as their palettes: tinted chapter headers, softly graded verse selection, lighter bottom navigation, a unified original-word card, separate definition cards, single-row study tabs, and a grouped action sheet with a prominent lookup action. Search results, notes and study articles share the same card treatment. These are native vector surfaces, with no browser dependency or additional downloads.

D038 treats the [approved captures](../../design/color-studies/reference/README.md) as the visual reference. Mobile Reading preferences starts with size and the selected verse preview; **Theme & typography** reveals palettes, typeface, line spacing, verse numbers and Library settings. Done stays fixed. New preferences use 24px/1.63; existing saved choices are retained. Tap a word with a supplied KJV dictionary tag or verified word link to open its actions and actual dictionary-headword preview. Untagged editions retain selection and verse-word browsing. The soft backdrop uses an app-only frame in memory, with a scrim fallback if readback is unavailable.

### Phrase highlights and styles

Hold a word (desktop: select text and right-click), then **Highlight…**. Choose **Adjust selection** to drag the start/end handles across the desired phrase or neighboring verses, then **Choose style**. Pick a color and style and select **Apply highlight**. **Whole verse** retains canonical verse/range highlighting. Tap an existing highlight, or use its Highlight action, to change its style or **Remove highlight**. Notes & highlights also offers Edit/Delete. Exact phrase marks belong to their selected translation; they are not guessed onto another edition. Existing highlights and notes are retained.

The picker includes highlighter/text colors, underlines, boxes/brackets, bold/italic/strikethrough, margin bars, labeled study symbols and recent saved styles. See [the rich-highlight contract](../../specs/app/highlights-v1.md) for supported styles, anchoring and older-app compatibility.

Word occurrences now offer a tappable distribution chart for the five most frequent books, a filter covering every matching book, and six verse previews per page. Open a passage and use “Return to word study” to resume the same filter and page.

Chapter openings use Cormorant Garamond Italic for the book name, with a separate chapter label and translation name. Font credits and the bundled SIL Open Font License are available in Source credits & licenses.

Tap the original-language word card to flip to its dictionary meaning. Long entries show an excerpt and Read full entry; Back to word restores the front. Occurrences remains accessible above the card. Reading preferences → Theme & typography → Reduce word-card motion switches the flip to a fade.

Study symbols now decorate the selected text with tinted patterned borders, including across wrapped lines. The highlight picker shows a selected-style checkmark and live preview before Apply. Existing saved symbol highlights gain the new rendering automatically.

Book artwork and introductions: tap the large book heading in Read, or tap the top passage reference and choose **About [book] · artwork & introduction**. Scroll below the artwork for Matthew Henry's introduction. **Start reading** opens 1:1; **Choose a chapter** opens that book's chapter picker. All 66 covers and introductions are bundled offline.

Notes now supports passage-linked scrapbook documents. Hold Scripture, adjust the selection, then choose **Add note** or **Highlight**. Tap the counted note marker to read/edit attached notes in a modal. In Notes, use search and the Notes/Highlights/Bookmarks tabs; tap a card to open it. **Add to note** collects an exact passage or resource excerpt, and Study articles offer **Add excerpt to note** with new/existing-note targeting. Saved source links retain excerpts and attribution. Existing plain notes remain editable.
