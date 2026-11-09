#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Convert Tomoe XML corpora to one JSONL record per character."""

import argparse
import json
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "tomoe"
DEFAULT_OUT = ROOT / "reports" / "tomoe.jsonl"

LANGUAGES = {
    "handwriting-zh_CN.xml": "zh_CN",
    "handwriting-zh_TW.xml": "zh_TW",
    "handwriting-ja.xml": "ja",
}


def characters(path):
    root = ET.parse(path).getroot()
    if root.tag == "character-collection":
        for group in root.findall("set"):
            yield from group.findall("character")
    else:
        yield from root.findall("character")


def records(language, path):
    for index, node in enumerate(characters(path)):
        label = (node.findtext("utf8") or "").strip()
        if not label:
            continue
        strokes = []
        for stroke in node.findall("./strokes/stroke"):
            points = [
                [float(point.get("x", "0")), float(point.get("y", "0"))]
                for point in stroke.findall("point")
            ]
            if points:
                strokes.append(points)
        if not strokes:
            continue
        width = float(node.findtext("width") or 1000)
        height = float(node.findtext("height") or 1000)
        yield {
            "id": f"{language}-{index:06d}",
            "language": language,
            "character": label[0],
            "width": width,
            "height": height,
            "strokes": strokes,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--language", choices=sorted(LANGUAGES.values()))
    args = parser.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with args.out.open("w", encoding="utf-8") as handle:
        for filename, language in LANGUAGES.items():
            if args.language and language != args.language:
                continue
            for record in records(language, DATA / filename):
                handle.write(json.dumps(record, ensure_ascii=False) + "\n")
                count += 1
    print(f"wrote {count} records to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
