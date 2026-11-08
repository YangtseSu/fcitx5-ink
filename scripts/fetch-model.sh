#!/bin/sh
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
out="$root/target/handwriting-model"
mkdir -p "$out"

python3 - "$root/resources/handwriting-model.lock.json" "$out" <<'PY'
import hashlib
import json
import os
import sys
import tempfile
import urllib.request

lock_path, out_dir = sys.argv[1], sys.argv[2]
lock = json.load(open(lock_path, encoding="utf-8"))


def digest(path):
    value = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            value.update(block)
    return value.hexdigest()


for artifact in lock["artifacts"]:
    name = artifact["name"]
    destination = os.path.join(out_dir, name)
    if (
        os.path.isfile(destination)
        and not os.path.islink(destination)
        and os.path.getsize(destination) == artifact["size"]
        and digest(destination) == artifact["sha256"]
    ):
        print(f"{name}: already at the locked digest")
        continue
    print(f"{name}: downloading {artifact['url']}")
    with tempfile.NamedTemporaryFile(dir=out_dir, delete=False) as staged:
        staged_path = staged.name
        with urllib.request.urlopen(artifact["url"], timeout=300) as response:
            while block := response.read(1 << 20):
                staged.write(block)
    try:
        if (
            os.path.getsize(staged_path) != artifact["size"]
            or digest(staged_path) != artifact["sha256"]
        ):
            raise SystemExit(f"{name}: downloaded file does not match the lock")
        os.chmod(staged_path, 0o644)
        os.replace(staged_path, destination)
    except BaseException:
        os.unlink(staged_path)
        raise
PY
