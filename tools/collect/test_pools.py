#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Checks for the committed training and evaluation pools."""

from __future__ import annotations

import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent
TARGETS = ROOT / "targets"
TOMOE = ROOT.parents[1] / "data" / "tomoe"
FILES = {
    "zh_CN": "handwriting-zh_CN.xml",
    "zh_TW": "handwriting-zh_TW.xml",
    "ja": "handwriting-ja.xml",
}
LANGUAGES = tuple(FILES)
TRAIN_COUNTS = {"zh_CN": 3498, "zh_TW": 4756, "ja": 2184}
EVAL_COUNT = 300


def read_chars(path: Path) -> list[str]:
    chars: list[str] = []
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        char = line[0]
        if char not in chars:
            chars.append(char)
    return chars


def model_classes(path: Path) -> set[str]:
    root = ET.parse(path).getroot()
    labels: list[str] = []
    if root.tag == "character-collection":
        for group in root.findall("set"):
            labels.extend(
                (node.findtext("utf8") or "").strip()
                for node in group.findall("character")
            )
    else:
        labels.extend(
            (node.findtext("utf8") or "").strip()
            for node in root.findall("character")
        )
    return {label[0] for label in labels if label}


class PoolTests(unittest.TestCase):
    def test_train_pool_counts(self) -> None:
        counts = {
            language: len(read_chars(TARGETS / "train" / f"{language}.txt"))
            for language in LANGUAGES
        }
        self.assertEqual(counts, TRAIN_COUNTS)

    def test_eval_pool_counts(self) -> None:
        for language in LANGUAGES:
            chars = read_chars(TARGETS / "eval" / f"{language}.txt")
            self.assertEqual(len(chars), EVAL_COUNT, language)

    def test_pools_have_no_duplicates(self) -> None:
        for split in ("train", "eval"):
            for language in LANGUAGES:
                path = TARGETS / split / f"{language}.txt"
                chars = read_chars(path)
                self.assertEqual(len(chars), len(set(chars)), path)

    def test_train_pool_is_inside_model_classes(self) -> None:
        for language in LANGUAGES:
            chars = set(read_chars(TARGETS / "train" / f"{language}.txt"))
            classes = model_classes(TOMOE / FILES[language])
            self.assertLessEqual(chars, classes, language)

    def test_frequency_order_starts_with_common_characters(self) -> None:
        first = {
            language: read_chars(TARGETS / "train" / f"{language}.txt")[0]
            for language in LANGUAGES
        }
        self.assertEqual(first["zh_CN"], "一")
        self.assertEqual(first["zh_TW"], "一")
        self.assertIn(first["ja"], "あいうえお")


if __name__ == "__main__":
    unittest.main()
