"""Verify same-run application artifacts, publish an immutable testing prerelease, verify downloads."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

PLATFORMS = {"windows-x86_64", "linux-x86_64", "macos-aarch64", "macos-x86_64", "android-aarch64"}


def gh(*args):
    return subprocess.check_output(["gh", *args], text=True).strip()


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    repository = os.environ["GITHUB_REPOSITORY"]
    commit = os.environ["GITHUB_SHA"]
    run_id, attempt = os.environ["GITHUB_RUN_ID"], os.environ["GITHUB_RUN_ATTEMPT"]
    tag = f"app-v0.1.0-dev.{os.environ['GITHUB_RUN_NUMBER']}.{attempt}"
    release_files = []
    platforms = set()
    for path in sorted(args.artifacts.glob("fontes-*.json")):
        record = json.loads(path.read_text())
        platform = record["platform"]
        assert platform in PLATFORMS and platform not in platforms, path
        platforms.add(platform)
        assert record["commit"] == commit and record["run_id"] == run_id and record["run_attempt"] == attempt, path
        asset = record["asset"]
        assert Path(asset["name"]).name == asset["name"], path
        payload = args.artifacts / asset["name"]
        assert payload.stat().st_size == asset["bytes"] and sha(payload) == asset["sha256"], payload
        release_files.extend([path, payload])
    assert platforms == PLATFORMS, platforms
    expected_names = {p.name for p in release_files}
    assert {p.name for p in args.artifacts.iterdir()} == expected_names, "Unexpected artifact files"
    checksums = args.artifacts / "SHA256SUMS"
    checksums.write_text("".join(f"{sha(p)}  {p.name}\n" for p in sorted(release_files)))
    release_files.append(checksums)
    notes = args.artifacts / "release-notes.md"
    notes.write_text(f"""Development testing builds from `{commit}`.

Windows x64, Linux x64, macOS Apple Silicon + Intel, and Android arm64 (API 28+, target 35).
Desktop archives contain the GUI, headless CLI, build manifest, UI licenses and launch instructions.
Bible data is separate: see [testing setup](https://github.com/{repository}/blob/{commit}/docs/app/testing-builds.md).

These are experimental test builds. macOS bundles are ad-hoc signed, not notarized; Windows executables are not Authenticode signed. Android uses a per-run debug key; it is not a production update identity. Preserve notes before changing installs.

SHA256SUMS checks transfer integrity; it is not a signed application update protocol.
Build/test evidence: https://github.com/{repository}/actions/runs/{run_id}
""")
    # An existing tag is never overwritten. A rerun has its own attempt/version.
    subprocess.run(["gh", "release", "create", tag, "--repo", repository, "--target", commit,
                    "--title", f"Fontes testing build {tag}", "--notes-file", str(notes),
                    "--prerelease", "--draft", "--latest=false", *map(str, release_files)], check=True)
    with tempfile.TemporaryDirectory() as temp:
        for stage in ["draft", "public"]:
            dest = Path(temp) / stage
            subprocess.run(["gh", "release", "download", tag, "--repo", repository, "--dir", str(dest)], check=True)
            assert {p.name for p in dest.iterdir()} == {p.name for p in release_files}
            for path in release_files:
                assert sha(dest / path.name) == sha(path), path
            if stage == "draft":
                subprocess.run(["gh", "release", "edit", tag, "--repo", repository, "--draft=false", "--latest=false"], check=True)
    print(gh("release", "view", tag, "--repo", repository, "--json", "url", "--jq", ".url"))


if __name__ == "__main__":
    main()
