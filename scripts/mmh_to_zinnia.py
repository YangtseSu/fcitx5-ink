#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Convert Make Me a Hanzi median trajectories to Zinnia S-expressions.

The source uses font coordinates with y-up; Zinnia/Tomoe use screen
coordinates. Apply Make Me a Hanzi's own transform y_screen = 900 - y_font.
"""

import argparse
import json
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MMH = ROOT / "models" / "makemeahanzi" / "graphics.txt"
CANVAS = 1024


def classes_from(xml_paths):
    classes = set()
    for path in xml_paths:
        root = ET.parse(path).getroot()
        nodes = []
        if root.tag == "character-collection":
            for group in root.findall("set"):
                nodes.extend(group.findall("character"))
        else:
            nodes = root.findall("character")
        for node in nodes:
            label = (node.findtext("utf8") or "").strip()
            if label:
                classes.add(label[0])
    return classes


def zinnia(character, strokes):
    parts = []
    for stroke in strokes:
        points = " ".join(
            f"({max(0, min(CANVAS, round(x)))} {max(0, min(CANVAS, round(y)))})"
            for x, y in stroke
        )
        parts.append(f"({points})")
    return (
        f"(character (value {character}) (width {CANVAS}) (height {CANVAS}) "
        f"(strokes ({' '.join(parts)})))"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--graphics", type=Path, default=DEFAULT_MMH)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument(
        "--exclude-xml", type=Path, action="append", default=[],
        help="Tomoe XML whose classes should not be emitted; repeatable",
    )
    parser.add_argument(
        "--only-xml", type=Path, action="append", default=[],
        help="emit only classes present in these Tomoe XML files; repeatable",
    )
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    skip = classes_from(args.exclude_xml)
    include = classes_from(args.only_xml)
    count = 0
    skipped = 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.graphics.open(encoding="utf-8") as source, args.out.open(
        "w", encoding="utf-8"
    ) as output:
        for line in source:
            line = line.strip()
            if not line:
                continue
            item = json.loads(line)
            character = item.get("character")
            medians = item.get("medians") or []
            if not character or len(character) != 1 or not medians:
                continue
            if include and character not in include:
                continue
            if character in skip:
                skipped += 1
                continue
            strokes = [
                [(x, 900.0 - y) for x, y in stroke]
                for stroke in medians
                if len(stroke) >= 2
            ]
            if not strokes:
                continue
            output.write(zinnia(character, strokes) + "\n")
            count += 1
            if args.limit and count >= args.limit:
                break
    print(f"wrote {count} characters to {args.out} (skipped {skipped} existing)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
