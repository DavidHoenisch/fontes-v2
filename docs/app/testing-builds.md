# Fontes development testing builds

Download the build for your CPU/OS from an `app-v0.1.0-dev.<run>.<attempt>` prerelease in [GitHub Releases](https://github.com/DavidHoenisch/fontes-v2/releases). Application runs also retain the five platform artifacts for 14 days. All five build/test jobs must pass before the trusted default-branch publisher creates a prerelease. Failed builds never replace a published version.

| Download | Contents / target |
| --- | --- |
| `fontes-windows-x86_64.zip` | Windows x64 GUI (`fontes.exe`) and CLI (`fontes-cli.exe`) |
| `fontes-linux-x86_64.tar.gz` | Linux x64 GUI (`fontes`) and CLI (`fontes-cli`); built on Ubuntu 24.04, requires glibc 2.39+ and native Wayland/X11/GPU libraries |
| `fontes-macos-aarch64.zip` | macOS 15+ Apple Silicon `Fontes.app` and CLI |
| `fontes-macos-x86_64.zip` | macOS 15+ Intel `Fontes.app` and CLI |
| `fontes-android-aarch64-debug.apk` | Android arm64, minimum API 28, target API 35; debug signed |

Release assets also include per-platform JSON build manifests and `SHA256SUMS`. Manifests bind source commit, toolchain, Cargo.lock, platform and payload hashes/lengths. Desktop archives retain executable permissions and contain the matching binaries' command-contract check. Archives include their own internal file inventory; Android also carries UI/font license texts in its assets. Checksums provide download integrity, not the signed/rollback-protected content-update protocol.

These are test builds: Windows is not Authenticode signed; macOS uses ad-hoc code signing and is not notarized. Your OS may require an explicit user decision to open a downloaded development application. No production signing certificate or trust setting is installed by this workflow.

## Desktop setup

Extract the archive completely, then open `Fontes.app` on macOS or run the GUI executable on Windows/Linux. On first launch choose **Reading** or **Study**, select **Continue**, review download/storage sizes and select **Download and open**. No path or compiler is needed. Once installed, reading works offline. See the [first-run guide](https://github.com/DavidHoenisch/fontes-v2/blob/main/docs/app/first-run-setup.md).

Advanced development imports can still launch with an explicit trusted-local path:

```sh
# Linux:
./fontes --library /absolute/path/to/library.sqlite
# macOS:
open Fontes.app --args --library /absolute/path/to/library.sqlite
# Windows PowerShell:
.\fontes.exe --library C:\path\to\library.sqlite
```

Bible data remains separate from application binaries. The in-app setup verifies signed release metadata, payload hashes and SQLite before activation; it downloads only the chosen compiled profile. The manual local-file opener checks schema/registry but does not authenticate arbitrary imported files. For offline developer imports, use locally compiled data or the [signed data-release verification instructions](https://github.com/DavidHoenisch/fontes-v2/blob/main/README.md#github-releases); a `.sqlite.zst` file must be verified/decompressed before manual opening.

Notes stay in the platform's separate local app-data directory. `--data-dir <directory>` selects an isolated test workspace. Keep this directory when changing binaries. See [the application guide](https://github.com/DavidHoenisch/fontes-v2/blob/main/docs/app/README.md) for keyboard shortcuts, shared CLI/JSON/MCP commands and remaining feature gaps.

## Android setup

Install the APK using Android’s development-install flow or `adb install`, then follow the same **Reading/Study → Continue → Download and open** wizard. It uses app-private storage and needs no filesystem path, USB data transfer or storage permission. The first download requires internet; later reading does not.

Optional legacy developer sideloading remains available:

```sh
adb install -r fontes-android-aarch64-debug.apk
adb shell am force-stop org.fontes.app
adb shell run-as org.fontes.app mkdir -p files
adb shell -T "run-as org.fontes.app sh -c 'cat > files/library.sqlite'" < library.sqlite
adb shell am start -n org.fontes.app/android.app.NativeActivity
```

Each clean CI run has a fresh Android debug signing key. Installing a differently signed APK over an existing install can fail; this is not a production update channel. Export/retain private user data before any uninstall or replacement that removes app storage. Do not remove an install containing important notes just to resolve a signing mismatch. Stable production/debug-distribution signing and user-data export UI are later milestones.

Compilation, command smoke checks and packaging checks do not establish device support. Android IME, touch, lifecycle recovery, safe areas/accessibility and actual GPU behavior still require device testing; desktop native-window/accessibility tests also remain. iOS is deferred.
