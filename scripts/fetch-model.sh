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
import shutil
import sys
import tempfile
import urllib.request
import zipfile

lock_path, out_dir = sys.argv[1], sys.argv[2]
lock = json.load(open(lock_path, encoding="utf-8"))


def digest(path):
    value = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            value.update(block)
    return value.hexdigest()


def matches(path, artifact):
    return (
        os.path.isfile(path)
        and not os.path.islink(path)
        and os.path.getsize(path) == artifact["size"]
        and digest(path) == artifact["sha256"]
    )


def download(url, destination):
    with urllib.request.urlopen(url, timeout=300) as response, open(
        destination, "wb"
    ) as handle:
        shutil.copyfileobj(response, handle, 1 << 20)


for artifact in lock["artifacts"]:
    name = artifact["name"]
    destination = os.path.join(out_dir, name)
    if matches(destination, artifact):
        print(f"{name}: already at the locked digest")
        continue
    print(f"{name}: downloading {artifact['url']}")
    staged = None
    archive = None
    try:
        fd, archive = tempfile.mkstemp(dir=out_dir, prefix=".download.")
        os.close(fd)
        download(artifact["url"], archive)
        member = artifact.get("member")
        if member:
            fd, staged_name = tempfile.mkstemp(dir=out_dir, prefix=".model.")
            os.close(fd)
            with zipfile.ZipFile(archive) as package, package.open(member) as source:
                with open(staged_name, "wb") as handle:
                    shutil.copyfileobj(source, handle, 1 << 20)
            staged = staged_name
        else:
            staged = archive
            archive = None
        if not matches(staged, artifact):
            raise SystemExit(f"{name}: downloaded file does not match the lock")
        os.chmod(staged, 0o644)
        os.replace(staged, destination)
        staged = None
    finally:
        for path in (staged, archive):
            if path:
                try:
                    os.unlink(path)
                except FileNotFoundError:
                    pass
PY
