#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify the committed Tomoe data against its manifest."""

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "tomoe"


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            value.update(block)
    return value.hexdigest()


def main():
    manifest = json.loads((ROOT / "data" / "manifests" / "tomoe.json").read_text())
    failed = False
    for item in manifest["files"]:
        path = DATA / item["name"]
        if not path.is_file():
            print(f"FAIL {item['name']}: missing")
            failed = True
            continue
        size = path.stat().st_size
        checksum = digest(path)
        if size != item["size"] or checksum != item["sha256"]:
            print(f"FAIL {item['name']}: size/hash mismatch")
            failed = True
        else:
            print(f"OK   {item['name']} ({size} bytes)")
    if not (DATA / "COPYING").is_file():
        print("FAIL data/tomoe/COPYING: missing")
        failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
