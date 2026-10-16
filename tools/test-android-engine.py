#!/usr/bin/env python3
"""Build and execute engine tests on an already booted Android emulator/device.

Needs the Rust Android target, ANDROID_NDK_HOME and adb on PATH. Uses only a
unique /data/local/tmp directory; never installs/replaces an app or its data.
"""
import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True,
                        choices=["aarch64-linux-android", "x86_64-linux-android"])
    parser.add_argument("--serial", help="adb device serial (or set ANDROID_SERIAL)")
    args = parser.parse_args()
    ndk = os.environ.get("ANDROID_NDK_HOME") or os.environ.get("ANDROID_NDK_ROOT")
    if not ndk:
        parser.error("Set ANDROID_NDK_HOME to the installed Android NDK directory.")
    host = {"darwin": "darwin-x86_64", "linux": "linux-x86_64"}.get(sys.platform)
    if not host:
        parser.error("This test helper currently supports macOS and Linux hosts.")
    toolchain = Path(ndk) / "toolchains/llvm/prebuilt" / host / "bin"
    compiler = toolchain / f"{args.target}28-clang"
    if not compiler.is_file():
        parser.error(f"Missing NDK compiler: {compiler}")
    env = os.environ.copy()
    target_key = args.target.replace("-", "_")
    env[f"CARGO_TARGET_{target_key.upper()}_LINKER"] = str(compiler)
    env[f"CC_{target_key}"] = str(compiler)
    env[f"AR_{target_key}"] = str(toolchain / "llvm-ar")
    build = subprocess.run(
        ["cargo", "test", "--locked", "-p", "fontes-engine", "--target", args.target,
         "--no-run", "--message-format=json"],
        env=env, text=True, stdout=subprocess.PIPE,
    )
    executables = []
    for line in build.stdout.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-message":
            print(message["message"].get("rendered", ""), file=sys.stderr, end="")
        if (message.get("reason") == "compiler-artifact"
                and message.get("profile", {}).get("test") and message.get("executable")):
            executables.append(message["executable"])
    build.check_returncode()
    if not executables:
        raise RuntimeError("Cargo produced no Android engine test executables.")
    adb = ["adb"] + (["-s", args.serial] if args.serial else [])
    remote = f"/data/local/tmp/fontes-engine-tests-{uuid.uuid4().hex}"
    def shell(command):
        subprocess.run(adb + ["shell", command], check=True)

    shell(f"mkdir -p {remote}/tmp")
    try:
        for index, executable in enumerate(executables):
            binary = f"{remote}/tests-{index}"
            subprocess.run(adb + ["push", executable, binary], check=True)
            shell(f"chmod 700 {binary}")
            print(f"Running {Path(executable).name} on Android", flush=True)
            shell(f"TMPDIR={remote}/tmp {shlex.quote(binary)} --test-threads=1")
    finally:
        subprocess.run(adb + ["shell", f"rm -rf {remote}"], check=False)


if __name__ == "__main__":
    main()
