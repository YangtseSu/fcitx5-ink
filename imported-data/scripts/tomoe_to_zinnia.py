#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Convert a Tomoe XML corpus to Zinnia S-expression training data."""

import argparse
import xml.etree.ElementTree as ET
from pathlib import Path


def characters(path):
    root = ET.parse(path).getroot()
    if root.tag == "character-collection":
        for group in root.findall("set"):
            yield from group.findall("character")
    else:
        yield from root.findall("character")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--xml", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    count = 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as output:
        for node in characters(args.xml):
            character = (node.findtext("utf8") or "").strip()
            if not character:
                continue
            strokes = []
            for stroke in node.findall("./strokes/stroke"):
                points = [
                    (round(float(point.get("x", "0"))), round(float(point.get("y", "0"))))
                    for point in stroke.findall("point")
                ]
                if len(points) >= 2:
                    strokes.append(points)
            if not strokes:
                continue
            width = round(float(node.findtext("width") or 1000))
            height = round(float(node.findtext("height") or 1000))
            encoded = " ".join(
                f"({' '.join(f'({x} {y})' for x, y in stroke)})"
                for stroke in strokes
            )
            output.write(
                f"(character (value {character[0]}) (width {width}) "
                f"(height {height}) (strokes {encoded}))\n"
            )
            count += 1
    print(f"wrote {count} characters to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
