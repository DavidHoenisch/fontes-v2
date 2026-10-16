# Initial application verification

Observed locally on 2026-10-15. These are development acceptance checks, not evidence of completed support on all target platforms. Commands and reproduction steps are in the [app guide](README.md#verification).

## Completed observations

- Native macOS application compiled and launched with the real study profile, egui/eframe 0.35.0, wgpu, and bundled design assets.
- Engine integration coverage: actual canonical bounds/aliases/ambiguous inputs; cross-book navigation; merged source segments and absent verse rows; notes surviving translation changes and reopening; edits/deletion; rejected writes; favorite idempotence; supplied alternative token analyses; typed TSK origin/target links; literal search/translation filtering/paging; CLI-compatible JSON/MCP dispatch and malformed/bounded frames; unrelated content/user-file refusal.
- Normal UI test: recoverable first-run Open Library behavior at 320×640, 390×844 and 1440×960. Real-data egui interaction/wgpu render test additionally opens the picker, saves a note, searches and opens study context at all three sizes. PNGs are generated under the caller's `FONTES_CAPTURE_DIR` for inspection, not checked-in golden screenshots.
- 54 regular tests pass across the existing data/resource workspace and application, with Clippy/format checks; ordinary CI skips only the explicitly ignored real-corpus/GPU test.
- Android aarch64 native library and debug APK built using NDK 28.2.13676358, platform 35, JDK from Android Studio, cargo-apk 0.10.0. APK alignment/signature and NativeActivity manifest checked. API minimum 28, target 35, private-storage bootstrap, arm64 ABI, no internet/external-storage permissions. Native debug symbols are stripped from the packaged development APK (approximately 52 MB in this local build).
- New GitHub Actions define Windows/macOS/Linux application checks and Android APK validation. They have not been run remotely as part of this local implementation; actual runner/device results must be recorded after execution.

The Mac was locked, so computer-use could not inspect or operate the live desktop window. Rendering evidence comes from the actual egui/wgpu test harness; it is not claimed as a completed manual OS-window test. The real-data render flow was also run on the reading profile at all three sizes. No Android device or configured AVD was available. Android compilation does not verify keyboard, safe-area, lifecycle, accessibility or physical GPU behavior.

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

Validate Windows/Linux behavior on actual runners; test keyboard/focus, screen readers, text scaling, RTL/Hebrew shaping, narrow/large viewports, and mouse/touch flows on real devices. Exercise Android IME, pause/resume/process death, GPU behavior and private import. Resolve Android NativeActivity accessibility or validate a GameActivity bridge. Add verified first-run content acquisition and safe installation, app signing/distribution, note export/backup/recovery, unsaved-draft close protection, cancellation and large-annotation-library budgets. Complete optional catalog/installer, curated overviews/genealogies, package-native links and local AI separately. iOS remains deferred.

## Cross-platform Actions follow-up

The first pushed application run (`38863847754`, commit `31ee150`) caught Windows checkout CRLF conversion changing hashed UI assets. A repository `.gitattributes` policy preserves LF for text and automatic binary detection so every target sees the committed source/manifest bytes. This also stabilizes command-catalog comparisons and package manifests. Subsequent run results are recorded below once verified.
