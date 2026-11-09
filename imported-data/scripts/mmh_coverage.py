#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Compare Make Me a Hanzi characters with the committed Tomoe classes."""

import json
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MMH = ROOT / "models" / "makemeahanzi" / "graphics.txt"
LANGUAGES = {
    "zh_CN": "handwriting-zh_CN.xml",
    "zh_TW": "handwriting-zh_TW.xml",
    "ja": "handwriting-ja.xml",
}


def tomoe_classes(filename):
    root = ET.parse(ROOT / "data" / "tomoe" / filename).getroot()
    classes = set()
    if root.tag == "character-collection":
        for group in root.findall("set"):
            for node in group.findall("character"):
                label = (node.findtext("utf8") or "").strip()
                if label:
                    classes.add(label[0])
    else:
        for node in root.findall("character"):
            label = (node.findtext("utf8") or "").strip()
            if label:
                classes.add(label[0])
    return classes


def mmh_classes():
    classes = set()
    with MMH.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            item = json.loads(line)
            character = item.get("character")
            if character:
                classes.add(character[0])
    return classes


def main():
    mmh = mmh_classes()
    print(f"Make Me a Hanzi: {len(mmh)} characters")
    for language, filename in LANGUAGES.items():
        current = tomoe_classes(filename)
        print(
            f"{language}: current={len(current)} "
            f"both={len(current & mmh)} "
            f"new_from_mmh={len(mmh - current)} "
            f"current_not_mmh={len(current - mmh)}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
