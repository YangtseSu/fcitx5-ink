#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Pick target characters for the capture tool from a Tomoe corpus."""

import argparse
import random
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = {
    "zh_CN": "handwriting-zh_CN.xml",
    "zh_TW": "handwriting-zh_TW.xml",
    "ja": "handwriting-ja.xml",
}


def characters(path):
    root = ET.parse(path).getroot()
    if root.tag == "character-collection":
        for group in root.findall("set"):
            yield from group.findall("character")
    else:
        yield from root.findall("character")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--language", choices=sorted(FILES), required=True)
    parser.add_argument("--limit", type=int, default=300)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    labels = []
    for node in characters(ROOT / "data" / "tomoe" / FILES[args.language]):
        label = (node.findtext("utf8") or "").strip()
        if label:
            labels.append(label[0])
    labels = sorted(set(labels))
    if args.limit < len(labels):
        labels = random.Random(args.seed).sample(labels, args.limit)
    output = args.out or (
        ROOT / "data" / "capture" / "generated" / f"{args.language}.txt"
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(labels) + "\n", encoding="utf-8")
    print(f"wrote {len(labels)} targets to {output}")


if __name__ == "__main__":
    raise SystemExit(main())
