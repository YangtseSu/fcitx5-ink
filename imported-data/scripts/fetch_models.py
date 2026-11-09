#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Download and verify the Zinnia and Wagomu model assets."""

import argparse
import hashlib
import json
import os
import shutil
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFESTS = ROOT / "data" / "manifests"
MODELS = ROOT / "models"
DOWNLOADS = MODELS / "downloads"


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            value.update(block)
    return value.hexdigest()


def matches(path, artifact):
    return (
        path.is_file()
        and not path.is_symlink()
        and path.stat().st_size == artifact["size"]
        and digest(path) == artifact["sha256"]
    )


def download(url, destination):
    with urllib.request.urlopen(url, timeout=300) as response, destination.open(
        "wb"
    ) as handle:
        shutil.copyfileobj(response, handle, 1 << 20)


def verify(path, artifact):
    if not matches(path, artifact):
        raise SystemExit(
            f"{artifact['name']}: size or SHA-256 mismatch at {path}"
        )


def zinnia(manifest, force):
    target = MODELS / "zinnia"
    target.mkdir(parents=True, exist_ok=True)
    DOWNLOADS.mkdir(parents=True, exist_ok=True)
    for artifact in manifest["artifacts"]:
        destination = target / artifact["name"]
        if not force and matches(destination, artifact):
            print(f"{artifact['name']}: already verified")
            continue
        print(f"{artifact['name']}: downloading {artifact['url']}")
        with tempfile.TemporaryDirectory(dir=DOWNLOADS) as temporary:
            archive = Path(temporary) / "download"
            download(artifact["url"], archive)
            if artifact.get("member"):
                with zipfile.ZipFile(archive) as package, package.open(
                    artifact["member"]
                ) as source, destination.open("wb") as output:
                    shutil.copyfileobj(source, output, 1 << 20)
            else:
                shutil.copyfile(archive, destination)
        verify(destination, artifact)
        print(f"{artifact['name']}: verified -> {destination}")


def wagomu(manifest, force):
    target = MODELS / "wagomu"
    target.mkdir(parents=True, exist_ok=True)
    DOWNLOADS.mkdir(parents=True, exist_ok=True)
    for artifact in manifest["assets"]:
        name = artifact["name"]
        if name.endswith(".tar.gz"):
            destination = target / "engine"
            marker = destination / "wagomu.cpp"
            if not force and marker.is_file():
                print(f"{name}: engine already extracted")
                continue
            print(f"{name}: downloading {artifact['url']}")
            with tempfile.TemporaryDirectory(dir=DOWNLOADS) as temporary:
                archive = Path(temporary) / name
                download(artifact["url"], archive)
                verify(archive, artifact)
                if destination.exists():
                    shutil.rmtree(destination)
                with tarfile.open(archive) as package:
                    package.extractall(target, filter="data")
                extracted = target / "tegaki-wagomu-0.3"
                if extracted.is_dir():
                    extracted.rename(destination)
            print(f"{name}: extracted -> {destination}")
            continue

        destination = target / name.removesuffix(".zip")
        member = artifact.get("member")
        member_path = (
            destination / member.split("/", 1)[1] if member else None
        )
        if not force and member_path and matches(member_path, {
            "size": artifact["member_size"],
            "sha256": artifact["member_sha256"],
        }):
            print(f"{name}: already verified")
            continue
        print(f"{name}: downloading {artifact['url']}")
        with tempfile.TemporaryDirectory(dir=DOWNLOADS) as temporary:
            archive = Path(temporary) / name
            download(artifact["url"], archive)
            verify(archive, artifact)
            if destination.exists():
                shutil.rmtree(destination)
            with zipfile.ZipFile(archive) as package:
                package.extractall(destination)
            nested = destination / name.removesuffix(".zip")
            if nested.is_dir():
                for child in nested.iterdir():
                    child.rename(destination / child.name)
                nested.rmdir()
        if member_path:
            verify(member_path, {
                "name": member_path.name,
                "size": artifact["member_size"],
                "sha256": artifact["member_sha256"],
            })
            print(f"{name}: verified -> {member_path}")
        else:
            print(f"{name}: extracted -> {destination}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--force", action="store_true", help="re-download everything")
    args = parser.parse_args()
    MODELS.mkdir(exist_ok=True)
    zinnia(json.loads((MANIFESTS / "tegaki-models.json").read_text()), args.force)
    wagomu(json.loads((MANIFESTS / "wagomu.json").read_text()), args.force)
    print("models are ready under models/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
