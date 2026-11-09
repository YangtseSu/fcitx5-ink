#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Download and verify the Make Me a Hanzi stroke-median corpus."""

import argparse
import hashlib
import json
import shutil
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DIR = ROOT / "models" / "makemeahanzi"


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            value.update(block)
    return value.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", type=Path, default=DEFAULT_DIR)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    manifest = json.loads(
        (ROOT / "data" / "manifests" / "makemeahanzi.json").read_text()
    )
    args.dir.mkdir(parents=True, exist_ok=True)
    for item in manifest["files"]:
        destination = args.dir / item["name"]
        if (
            not args.force
            and destination.is_file()
            and destination.stat().st_size == item["size"]
            and digest(destination) == item["sha256"]
        ):
            print(f"{item['name']}: already verified")
            continue
        print(f"{item['name']}: downloading {item['url']}")
        with tempfile.NamedTemporaryFile(dir=args.dir, delete=False) as staged:
            staged_path = Path(staged.name)
            with urllib.request.urlopen(item["url"], timeout=300) as response:
                shutil.copyfileobj(response, staged, 1 << 20)
        try:
            if (
                staged_path.stat().st_size != item["size"]
                or digest(staged_path) != item["sha256"]
            ):
                raise SystemExit(f"{item['name']}: size or SHA-256 mismatch")
            staged_path.chmod(0o644)
            staged_path.replace(destination)
        except BaseException:
            staged_path.unlink(missing_ok=True)
            raise
        print(f"{item['name']}: verified -> {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
