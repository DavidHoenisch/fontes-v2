# Initial application verification

Observed locally on 2026-10-15. These are development acceptance checks, not evidence of completed support on all target platforms. Commands and reproduction steps are in the [app guide](README.md#verification).

## Completed observations

- Native macOS application compiled and launched with the real study profile, egui/eframe 0.35.0, wgpu, and bundled design assets.
- Engine integration coverage: actual canonical bounds/aliases/ambiguous inputs; cross-book navigation; merged source segments and absent verse rows; notes surviving translation changes and reopening; edits/deletion; rejected writes; favorite idempotence; supplied alternative token analyses; typed TSK origin/target links; literal search/translation filtering/paging; CLI-compatible JSON/MCP dispatch and malformed/bounded frames; unrelated content/user-file refusal.
- Normal UI test: recoverable first-run Open Library behavior at 320×640, 390×844 and 1440×960. Real-data egui interaction/wgpu render test additionally opens the picker, saves a note, searches and opens study context at all three sizes. PNGs are generated under the caller's `FONTES_CAPTURE_DIR` for inspection, not checked-in golden screenshots.
- 54 regular tests pass across the existing data/resource workspace and application, with Clippy/format checks; ordinary CI skips only the explicitly ignored real-corpus/GPU test.
- Android aarch64 native library and debug APK built using NDK 28.2.13676358, platform 35, JDK from Android Studio, cargo-apk 0.10.0. APK alignment/signature and NativeActivity manifest checked. API minimum 28, target 35, private-storage bootstrap, arm64 ABI, no internet/external-storage permissions. Native debug symbols are stripped from the packaged development APK (approximately 52 MB in this local build).
- Cross-platform GitHub Actions have now passed on all four desktop targets and Android, with a verified public testing prerelease; see the run evidence below. Physical-device/native-window checks remain distinct.

The Mac was locked, so computer-use could not inspect or operate the live desktop window. Rendering evidence comes from the actual egui/wgpu test harness; it is not claimed as a completed manual OS-window test. The real-data render flow was also run on the reading profile at all three sizes. No Android device or configured AVD was available. Android compilation does not verify keyboard, safe-area, lifecycle, accessibility or physical GPU behavior.

## Desktop study correction · 2026-10-16

Local source changes after the published testing release now keep desktop Scripture visible beside a resizable study panel. These observations do not claim that the earlier GitHub binaries include this correction.

- All **56 regular workspace tests** pass. New engine checks exercise explicit edition-specific Unicode word spans, alternative-analysis code deduplication, canonical token compatibility, invalid spans, absence of inferred links in another translation, and active-edition cross-reference previews with missing text left absent. Format checks and targeted engine/UI Clippy checks pass.
- The real study and reading snapshots were exercised through the egui/wgpu harness at 1440×960, 390×844 and 320×640. Existing picker, note, search and study flows pass. Desktop study additionally selects **loved** in John 3:16 using native text selection, opens its right-click menu, follows the explicit missing-alignment fallback, chooses source Greek G0025 and displays its real definition with Scripture still visible. Selection remains highlighted when study/menu controls own focus.
- A **synthetic** alignment added only to the test view model exercises the direct **Look up Strong’s G0025** menu branch against the real dictionary command. The real corpus still contains no English alignments. This fixture is not an acquired/reviewed aligned edition and does not satisfy the complete FR4 data requirement.
- The same flow changes to Mark 1:4, confirms old selection/dictionary context clears, renders narrower-coverage-first commentary and named cross-references with actual verse previews, then switches to WEB and verifies native/view-model text selection is invalidated. Both source readings/alternative analyses remain available through the original-word chooser.
- Captures are regenerated under `dist/app-captures`, including `desktop-study.png`, `desktop-word-menu.png`, `desktop-original-words.png`, `desktop-word-definition.png`, `desktop-mark-study.png` and `desktop-mark-references.png`. Reading-only captures are under `dist/app-captures-reading`. These are actual wgpu renders, not new manual OS-window or Android-device evidence. The local desktop binary was rebuilt.

The remaining mandatory English selection → exact Strong’s data work is tracked as **O018** in SPEC.md. No data compiler/source lock/schema migration or content release was changed for this UI correction. Mobile retains the initial Study destination; its future contextual sheet and real-device selection/accessibility checks remain open.

## Matthew Henry paragraph and entity correction · 2026-10-16

The locked MHC source record `Mark.1.1` contains real OSIS paragraph milestones and nested numeric encodings (`Roma &amp;#224; fratribus`). The older SQLite retains paragraph newlines but displays the residual `&#224;` literally. The article view now renders separate spaced paragraphs and a readable line height instead of one label for the whole article. Width is bounded to the viewport and 640 px. Each article opens at its beginning.

All **58 regular workspace tests** pass, together with format and targeted Clippy checks. Pipeline regression checks verify decoded MHC text reaches both resource content and search documents while aliases and links survive. A shared-helper test covers Latin/Greek decimal/hex characters, malformed references, one-layer decoding and literal angle brackets. An engine test verifies legacy MHC display repair leaves the stored text unchanged and does not transform another source. Real-study wgpu captures at 1440×960, 390×844 and 320×640 open the actual `Mark.1.1` article, check `Roma à fratribus`, paragraph spacing and viewport wrapping, and save `desktop-matthew-henry.png`, `mobile-matthew-henry.png` and `small-mobile-matthew-henry.png` under `dist/app-captures`.

This remains plain-paragraph display; original inline italics, typed headings and inline scripture links need structured source display metadata. No source prose or paragraph boundaries are inferred from punctuation. Existing release databases are repaired only in view models; no published data assets are rewritten. This local change is not a new GitHub release or a manual native-window/device test.

## Initial real-study timings

One debug-build stdio smoke run, macOS 27.0.1, MacBookPro18,3, 584,884,224-byte study SQLite, content ID `ec35deccbcbcad0eb0d4b2cc07ef0f92ca4fa4f0e79266fe3ea7363dc43d92c9`. Measured from writing a JSON command to receiving its complete reply, including workspace serialization; startup also includes process/library/user-store initialization. Other compilation/test work was running on the machine.

| Operation | Observed elapsed |
| --- | ---: |
| First workspace/startup | 292 ms |
| Navigate Romans 1:3 | 24 ms |
| Context Romans 1:3 | 609 ms |
| Strong's G25 definitions/occurrences | 29 ms |
| Strong's H7225 definitions/no Hebrew token matches | 25 ms |
| Keyword forgiveness, first 30 results | 75 ms |
| Note save / WEB switch / delete | 24 / 33 / 24 ms |

The smoke test required multiple Romans resources and real Greek tokens, real dictionary definitions and Greek occurrences, search hits, and preservation of a newly created note after switching to WEB, then removed that test note. These timings are neither cold/warm distributions nor modest-hardware/mobile benchmarks; NFR2 remains only partially evaluated. The ignored `dist/app-verification.json` captures the local sample rather than becoming a production benchmark fixture.

## Before a user release

Exercise actual Windows/Linux/macOS native windows and test keyboard/focus, screen readers, text scaling, RTL/Hebrew shaping, narrow/large viewports, and mouse/touch flows on real devices. Exercise Android IME, pause/resume/process death, GPU behavior and private import. Resolve Android NativeActivity accessibility or validate a GameActivity bridge. Add verified first-run content acquisition and safe installation, app signing/distribution, note export/backup/recovery, unsaved-draft close protection, cancellation and large-annotation-library budgets. Complete optional catalog/installer, curated overviews/genealogies, package-native links and local AI separately. iOS remains deferred.

## Cross-platform Actions follow-up

The first pushed application run (`38863847754`, commit `31ee150`) caught Windows checkout CRLF conversion changing hashed UI assets. A repository `.gitattributes` policy preserves LF for text and automatic binary detection so every target sees the committed source/manifest bytes. This also stabilizes command-catalog comparisons and package manifests. The corrected run results are recorded below.

Run `38863947932` verified the Windows asset fix and caught Android offline metadata resolving host crates outside a target-only fetch. CI now fetches the complete locked graph and sets both NDK environment variables to r28. The local archive round trip also caught resources staged after macOS signing; packaging now signs last and verifies the extracted bundle using ditto/codesign.

## Verified GitHub testing release — 2026-10-16

[Application run 38864239152](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38864239152), source commit `f548885ea0ff0004fe7375d99349ad81efc82c27`, completed successfully. [Release `app-v0.1.0-dev.3.1`](https://github.com/DavidHoenisch/fontes-v2/releases/tag/app-v0.1.0-dev.3.1) is public and marked prerelease. All five platform jobs and the separate publisher passed.

| Target | Build artifact | CI validation |
| --- | --- | --- |
| Windows x64 / windows-2025 | `fontes-windows-x86_64.zip` | Format, engine/UI tests, Clippy, optimized GUI/CLI, command-contract/layer checks, executable smoke checks and archive CRC |
| Linux x64 / ubuntu-24.04 | `fontes-linux-x86_64.tar.gz` | Same native desktop checks and tar packaging |
| macOS Apple Silicon / macos-15 | `fontes-macos-aarch64.zip` | Same desktop checks; full app-bundle ad-hoc signing and verification after archive extraction |
| macOS Intel / macos-15-intel | `fontes-macos-x86_64.zip` | Same desktop/signature/extraction checks on an Intel runner |
| Android arm64 / ubuntu-24.04 + NDK r28 | `fontes-android-aarch64-debug.apk` | Native build/package, unchanged Cargo.lock, APK alignment/signature and manifest verification; not on-device execution |

Each desktop job executed the resulting GUI/CLI `--help` and CLI command-catalog smoke checks before packaging. This tests native executable startup without opening a GUI window. The four desktop jobs also ran the toolkit-independent engine tests and recoverable first-run egui interaction test. The real-corpus wgpu render tests remain the separately recorded local checks, not a hidden CI/device claim.

The publisher required the exact five platform manifests to match this source commit, run ID and attempt, verified every payload size/hash, created an immutable draft test release and downloaded/compared all eleven assets both before and after public publication. Release assets are four desktop archives, one APK, five build manifests and SHA256SUMS. Same-run Actions artifacts are also retained for 14 days.

Independent local downloads of the GitHub-built artifacts confirmed their outer and internal hashes. Both extracted macOS bundles passed `codesign --verify --deep --strict`; the Apple Silicon packaged CLI navigated actual study data to John 3:16. Linux ELF headers/executable modes and Windows x64 PE headers were checked. The Android download's signature, arm64 shared library and included UI/font license assets were checked. Windows import inspection identified the v14 runtime dependency; the application guide now links the official x64 redistributable prerequisite. No Windows/Android GUI or physical-device result is inferred from these checks.

The original data-release run `38863847726` completed reproducible builds but its stale workflow target received HTTP 403 during publication after workflow revisions advanced. [A fresh current-commit run, 38864673059](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38864673059), passed both build and signed publication and produced [data-v1-stable-4](https://github.com/DavidHoenisch/fontes-v2/releases/tag/data-v1-stable-4). No broader token/secret permission was added. The initial [data-contract run](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38863847784) and [resource-source publishing run](https://github.com/DavidHoenisch/fontes-v2/actions/runs/38863847809) also passed. Superseded application runs were cancelled after their diagnosed failures to avoid redundant builds.

The final documentation-only commit records these results; published binaries remain bound to the tested source commit above. Production signing, stable Android update identity, automatic verified content setup, installers and native-window/device acceptance remain future gates.
