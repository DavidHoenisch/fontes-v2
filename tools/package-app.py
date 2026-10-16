"""Package already-built native applications; stdlib-only CI tooling, not app runtime."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, choices=[
        "windows-x86_64", "linux-x86_64", "macos-aarch64", "macos-x86_64", "android-aarch64",
    ])
    parser.add_argument("--output", type=Path, default=ROOT / "dist/app")
    parser.add_argument("--binary-dir", type=Path, default=ROOT / "target/release")
    parser.add_argument("--apk", type=Path, default=ROOT / "target/debug/apk/fontes-android.apk")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    commit = run("git", "rev-parse", "HEAD")
    if run("git", "status", "--porcelain", "--untracked-files=no"):
        raise SystemExit("Refuse to package a dirty tracked checkout; commit the tested source first.")
    name = f"fontes-{args.platform}"
    manifest = {
        "manifest_version": 1, "package": name, "version": "0.1.0", "commit": commit,
        "run_id": os.environ.get("GITHUB_RUN_ID"), "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "toolchain": run("rustc", "--version"), "cargo_lock_sha256": sha(ROOT / "Cargo.lock"),
        "distribution": "development-test", "includes_bible_data": False,
        "platform": args.platform,
    }
    if args.platform == "android-aarch64":
        target = args.output / f"{name}-debug.apk"
        if target.exists():
            raise SystemExit(f"Output exists: {target}")
        shutil.copy2(args.apk, target)
        manifest["signing"] = "ephemeral Android debug key; not a production update identity"
        manifest["asset"] = {"name": target.name, "bytes": target.stat().st_size, "sha256": sha(target)}
        (args.output / f"{name}.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print(target)
        return

    expected = {"windows-x86_64": "x86_64-pc-windows-msvc", "linux-x86_64": "x86_64-unknown-linux-gnu",
                "macos-aarch64": "aarch64-apple-darwin", "macos-x86_64": "x86_64-apple-darwin"}[args.platform]
    host = next(line.split(": ", 1)[1] for line in run("rustc", "-vV").splitlines() if line.startswith("host: "))
    if host != expected:
        raise SystemExit(f"Expected native host {expected}; found {host}")
    suffix = ".exe" if args.platform.startswith("windows") else ""
    binaries = {binary: args.binary_dir / (binary + suffix) for binary in ["fontes", "fontes-cli"]}
    for path in binaries.values():
        subprocess.run([str(path.resolve()), "--help"], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    catalog = json.loads(subprocess.check_output([str(binaries["fontes-cli"].resolve()), "commands"], text=True))
    if catalog != json.loads((ROOT / "specs/app/commands-v1.json").read_text()):
        raise SystemExit("Built CLI command contract differs from the checked-in contract")
    with tempfile.TemporaryDirectory() as temporary:
        staging = Path(temporary) / name
        staging.mkdir()
        if args.platform.startswith("macos"):
            contents = staging / "Fontes.app/Contents"
            (contents / "MacOS").mkdir(parents=True)
            shutil.copy2(binaries["fontes"], contents / "MacOS/fontes")
            (contents / "Info.plist").write_bytes(plistlib.dumps({
                "CFBundleIdentifier": "org.fontes.app", "CFBundleName": "Fontes", "CFBundleDisplayName": "Fontes",
                "CFBundleExecutable": "fontes", "CFBundlePackageType": "APPL", "CFBundleShortVersionString": "0.1.0",
                "CFBundleVersion": "0.1.0", "NSHighResolutionCapable": True, "LSMinimumSystemVersion": "15.0",
            }))
            notices = contents / "Resources"
        else:
            shutil.copy2(binaries["fontes"], staging / ("fontes" + suffix))
            notices = staging
        shutil.copy2(binaries["fontes-cli"], staging / ("fontes-cli" + suffix))
        notices.mkdir(parents=True, exist_ok=True)
        shutil.copytree(ROOT / "assets/ui/licenses", notices / "ui-licenses")
        shutil.copy2(ROOT / "Cargo.lock", notices / "Cargo.lock")
        shutil.copy2(ROOT / "docs/app/testing-builds.md", staging / "README.md")
        if args.platform.startswith("macos"):
            # Seal the complete bundle only after its Resources have been staged.
            subprocess.run(["codesign", "--force", "--sign", "-", str(contents.parent)], check=True)
            subprocess.run(["codesign", "--verify", "--deep", "--strict", str(contents.parent)], check=True)
        manifest["target"] = expected
        manifest["files"] = [{"path": str(p.relative_to(staging)).replace("\\", "/"), "bytes": p.stat().st_size, "sha256": sha(p)}
                             for p in sorted(staging.rglob("*")) if p.is_file()]
        (staging / "build.json").write_text(json.dumps(manifest, indent=2) + "\n")
        extension = ".tar.gz" if args.platform.startswith("linux") else ".zip"
        target = args.output / (name + extension)
        if target.exists():
            raise SystemExit(f"Output exists: {target}")
        if extension == ".zip":
            with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
                for p in sorted(staging.rglob("*")):
                    archive.write(p, p.relative_to(staging.parent))
            with zipfile.ZipFile(target) as archive:
                if archive.testzip() is not None:
                    raise SystemExit("Archive CRC validation failed")
            if args.platform.startswith("macos"):
                # Use the OS archive extractor to verify real permission/signature preservation.
                extracted = Path(temporary) / "extracted"
                subprocess.run(["ditto", "-x", "-k", str(target.resolve()), str(extracted)], check=True)
                subprocess.run(["codesign", "--verify", "--deep", "--strict", str(extracted / name / "Fontes.app")], check=True)
        else:
            with tarfile.open(target, "w:gz") as archive:
                archive.add(staging, arcname=name)
            with tarfile.open(target) as archive:
                if not archive.getmembers():
                    raise SystemExit("Empty archive")
        manifest["asset"] = {"name": target.name, "bytes": target.stat().st_size, "sha256": sha(target)}
        (args.output / f"{name}.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print(target)


if __name__ == "__main__":
    main()
