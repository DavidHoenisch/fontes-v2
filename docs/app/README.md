# Fontes application

This is the first working application slice, using real content-v1 reading/study snapshots. It is an application foundation, not the completed product or resource installer. Start with [SPEC.md](../../SPEC.md), the [engine architecture](architecture.md), and the [command contract](../../specs/app/README.md).

## Run on desktop

From the repository root, with the pinned Rust toolchain:

```sh
cargo run --locked -p fontes -- --library dist/reading/library.sqlite
# A study profile enables the installed dictionaries, Greek tokens and linked articles:
cargo run --locked -p fontes -- --library dist/study/library.sqlite
# Preview the touch layout in a narrow desktop window:
cargo run --locked -p fontes -- --library dist/reading/library.sqlite --mobile
```

Build the real [reading/study data](../../README.md#build-the-reading-data) first, or use the existing data-release verifier on downloaded signed assets. For a new local reading build:

```sh
cargo run --locked -p fontes-data-pipeline --bin fontes-data -- build \
  --profile reading --output dist/reading --builder-commit "$(git rev-parse HEAD)" \
  --mirror-repository DavidHoenisch/fontes-v2
```

The output directory must be new or empty. `--offline` rebuilds from an already populated verified source cache. No Scripture is embedded in the UI executable, and the app never silently downloads or substitutes content. The initial Open Library surface and Reading Preferences accept a local SQLite path. This is an **explicitly trusted local development import**: opening a file checks its schema and permanent reference registry, but does not authenticate a release signature. Do not treat it as the future verified installer.

`--data-dir <directory>` overrides the writable app-data location. Defaults come from the operating system's local application-data directory through `directories::ProjectDirs` (`org`, `Fontes`, `Fontes`); consult the explicit override when scripting rather than guessing its platform spelling. It contains:

- `user.sqlite`: notes, highlights, favorites, recent references and last reading/preferences state; never replace it with a data release.
- `library-path.txt`: the last successfully opened local content path. An optional `library-path.tmp` is only staging for this preference, not content or user-data recovery.

Once opened successfully, the app remembers the content path and reading session. To try an isolated library without changing normal notes, supply a separate `--data-dir`. Content and user databases must be different files. Back up `user.sqlite` using SQLite backup tooling while closed or through a consistent SQLite backup operation; copying an actively written database arbitrarily is not a backup protocol.

On Linux, install `libxkbcommon-dev` and `libwayland-dev` for native builds. Windows uses the Rust MSVC toolchain; macOS uses Xcode command-line tools. The application explicitly selects eframe's **wgpu** renderer; no glow renderer or webview is selected. UI fonts/icons are bundled with their [licenses](../../assets/ui/licenses/).

## Working flows

- Read KJV/WEB, select a verse, navigate across chapter/book boundaries, and switch translations without moving canonical selection or losing annotations/study details. Missing source text stays explicit; merged source segments render once.
- Open the book/chapter/verse picker from the passage button or Cmd/Ctrl+K. Type `John 3:16`, `j3 16`, `Rom 1:3`, or `Psalm 23`. Invalid/ambiguous input keeps the existing location and gives an error. Escape closes the picker/editor. Bookmark passages and reopen recent/favorite locations.
- Search literal words in the active translation and installed articles, with 30-result pages. This is keyword search, not generated AI prose.
- Use Notes to add/edit notes or highlights and bookmark a verse. Verse context menus also offer note/highlight/copy/study actions. Notes use permanent inclusive canonical ranges; the GUI currently creates single-verse notes, while commands also accept ranges.
- Use Study for typed verse links, attributed articles, TSK targets, supplied Greek token analyses, Strong's definitions and up to 100 distinct canonical occurrence jumps. Both Greek streams and alternative analyses remain visible. English words are not assumed to align to Greek tokens. Hebrew definitions exist; the supplied corpus has no Hebrew morphology/occurrence layer. Lemmas/glosses are only shown when supplied.
- Reading Preferences controls Lora/IBM Plex Sans, 16–36 px text, 1.2–2.2 line spacing and verse numbers. Dark study desk is the first implemented theme; weight selection and other themes remain planned.

At widths below 760 logical pixels the sidebar becomes Read/Search/Notes/Study bottom navigation. Targets are at least 44 px; primary actions are 48 px. The picker Open action and note Save/Cancel actions stay at the bottom while their contents scroll. At the narrowest width the toolbar bookmark moves to Notes. At wider widths the 208 px compact sidebar returns; a 360 px companion panel appears when the remaining reading width exceeds 1,050 px. These are initial implementation breakpoints, not a completed accessibility/device evaluation. Mobile word study currently uses the Study destination; the designed contextual bottom sheet remains future work.

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

CLI, JSON and MCP use the same typed command dispatch as the UI. JSON and MCP are line-delimited stdin/stdout, with no HTTP server/listener. Logs/errors outside protocol replies go to stderr. Configure the MCP client to launch `fontes-cli --library <absolute-path> --user-db <absolute-path> mcp`; MCP tools include durable local writes and state replies include private annotations. Only connect a client you intend to grant access to that user database. This is a separate headless session, not IPC remote control of an already-running GUI; sessions share stored annotations but do not broadcast navigation events between processes. See [wire details](../../specs/app/README.md).

## Android development

The native arm64 library and debug APK have been built locally. Hardware/emulator execution, soft keyboard/IME, lifecycle recovery, safe areas, screen-reader support and Android storage import still require device validation. iOS is deferred. This first APK requires development sideloading of a local library; it has no first-run network installer or Storage Access Framework picker.

Install Android SDK platform **35**, build-tools **35.0.0** (36.0.0 also used successfully locally), NDK **28.2.13676358**, a JDK, and the Rust target. Set `ANDROID_HOME`, `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, and `JAVA_HOME` to the actual installations. Both NDK variables must name the pinned NDK: cargo-apk gives an existing `ANDROID_NDK_ROOT` precedence. NDK r28's `cargo-apk` platform discovery supports up to API 35; having only SDK platforms 36/37 produces the misleading “SDK has no platforms installed” error. The app has min API 28 and target API 35.

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk --version 0.10.0 --locked
cargo fetch --locked
CARGO_NET_OFFLINE=true cargo apk build -p fontes-android --lib
```

`cargo-apk` 0.10.0 does not accept `--locked`. Fetch the full locked workspace graph first (cargo-apk also resolves host/workspace metadata), package offline, and verify Cargo.lock has not changed (CI checks its digest). The APK is `target/debug/apk/fontes-android.apk`, signed with the local Android **debug** key and with native debug symbols stripped from the package. This is not a production signing/release process.

The following device setup recipe is provided for development; it has not been executed here because no device/AVD was available:

```sh
adb install -r target/debug/apk/fontes-android.apk
adb shell am force-stop org.fontes.app
adb shell run-as org.fontes.app mkdir -p files
adb shell -T "run-as org.fontes.app sh -c 'cat > files/library.sqlite'" < dist/reading/library.sqlite
adb shell am start -n org.fontes.app/android.app.NativeActivity
```

The library lives in the app's private `internal_data_path()/library.sqlite`, separate from `user.sqlite`. `run-as` is only a debug-build development path. Uninstalling the app can remove its private notes; backups/export and production import are still required before user distribution. NativeActivity avoids an app-authored Java/Kotlin layer, but eframe 0.35's Android AccessKit backend requires GameActivity. Desktop AccessKit is enabled; Android accessibility is an explicit open gate, not silently claimed from desktop tests.

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

[Application CI](../../.github/workflows/application.yml) builds/tests Windows x64, Linux x64, macOS Apple Silicon/Intel and an Android arm64 APK. It retains downloadable platform archives for 14 days and publishes a versioned development prerelease only after all jobs pass. See [testing build setup](testing-builds.md) and [verification results](verification.md) for actual run status. The data publisher continues testing only the three data/publisher crates, so UI/GPU dependencies do not become content-compilation dependencies.

See [verification notes](verification.md) for observed results and remaining release gates. The source-package compiler/catalog installer, resource-native v2 graph, automatic updates, AI runtime, curated genealogies/overviews, production signing/installers and live GUI automation channel remain separate milestones.
