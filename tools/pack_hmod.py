#!/usr/bin/env python3
"""Pack rust/ into a native .hmod for Horizon.

A native .hmod is Rust source plus its manifest; the launcher compiles it on the
player's machine against the prebuilt game, so nothing is built here. The
archive root is the manifest, exactly as Horizon's package reader expects.

    python3 tools/pack_hmod.py [--out dist]
"""

from __future__ import annotations

import argparse
import re
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "rust"

# Not part of the package: the fetched SDK, build output, and the packer's own
# destination if it lives under the project.
SKIP_DIRS = {".horizon", "target", "dist", "__pycache__"}
SKIP_FILES = {".gitignore", "README.md"}


def manifest_version(manifest: str) -> str:
    match = re.search(r'^version\s*=\s*"([^"]+)"', manifest, re.MULTILINE)
    if not match:
        raise SystemExit("horizon-mod.toml has no version")
    return match.group(1)


def files() -> list[Path]:
    out = []
    for path in sorted(SOURCE.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(SOURCE)
        if any(part in SKIP_DIRS for part in relative.parts):
            continue
        if relative.name in SKIP_FILES:
            continue
        out.append(path)
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="dist")
    args = parser.parse_args()

    manifest = (SOURCE / "horizon-mod.toml").read_text(encoding="utf-8")
    version = manifest_version(manifest)
    output = ROOT / args.out
    output.mkdir(parents=True, exist_ok=True)
    target = output / f"aynthor_secondscreen-{version}.hmod"

    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in files():
            archive.write(path, path.relative_to(SOURCE).as_posix())

    print(f"{target.relative_to(ROOT)}  ({target.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
