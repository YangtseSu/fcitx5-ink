#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Validate a consented capture file and add it to the evaluation set."""

import argparse
import datetime as dt
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def record(line, language, writer):
    item = json.loads(line)
    if item.get("language") != language:
        raise ValueError("language mismatch")
    if item.get("writer") != writer:
        raise ValueError("writer mismatch")
    character = item.get("character")
    if not isinstance(character, str) or len(character) < 1:
        raise ValueError("invalid character")
    strokes = item.get("strokes")
    if not isinstance(strokes, list) or not strokes:
        raise ValueError("no strokes")
    for stroke in strokes:
        if len(stroke) < 2:
            raise ValueError("stroke with fewer than two points")
    return {
        "id": item["id"],
        "source": "capture",
        "language": language,
        "writer": writer,
        "character": character[0],
        "width": float(item["width"]),
        "height": float(item["height"]),
        "strokes": strokes,
        "created_at": item.get("created_at", ""),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--language", choices=("zh_CN", "zh_TW", "ja"), required=True)
    parser.add_argument("--writer", required=True)
    parser.add_argument("--license", default="CC-BY-4.0")
    parser.add_argument("--consent", action="store_true")
    parser.add_argument("--out-dir", type=Path, default=ROOT / "data" / "evaluation")
    args = parser.parse_args()
    if not args.consent:
        print("refusing to promote without --consent", file=sys.stderr)
        return 2
    output = args.out_dir / args.language / f"{args.writer}.jsonl"
    output.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with output.open("w", encoding="utf-8") as handle:
        for line in args.input.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            handle.write(json.dumps(record(line, args.language, args.writer), ensure_ascii=False) + "\n")
            count += 1
    if count == 0:
        output.unlink()
        print("no samples", file=sys.stderr)
        return 1
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    manifest_path = args.out_dir / "manifest.json"
    manifest = {"writers": []}
    if manifest_path.is_file():
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["writers"] = [
        item for item in manifest.get("writers", [])
        if not (item["language"] == args.language and item["writer"] == args.writer)
    ]
    manifest["writers"].append({
        "language": args.language,
        "writer": args.writer,
        "samples": count,
        "license": args.license,
        "file": f"{args.language}/{args.writer}.jsonl",
        "sha256": digest,
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
    })
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {count} samples to {output}")
    print(f"updated {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
