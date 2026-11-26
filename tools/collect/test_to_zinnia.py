#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Tests for the collector export to Zinnia conversion."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import to_zinnia


class ZinniaTests(unittest.TestCase):
    def test_line_format(self) -> None:
        item = {
            "language": "zh_CN",
            "writer": "w-000000000001",
            "character": "永",
            "width": 420,
            "height": 420,
            "strokes": [[[10.2, 20.7], [30.4, 40.1]], [[5, 5], [6, 6], [7, 7]]],
        }
        line = to_zinnia.zinnia_line(item)
        self.assertEqual(
            line,
            "(character (value 永) (width 420) (height 420) "
            "(strokes ((10 21) (30 40)) ((5 5) (6 6) (7 7))))",
        )

    def test_converts_export_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / "zh_CN").mkdir()
            (root / "zh_TW").mkdir()
            record = {
                "id": "web-1",
                "language": "zh_CN",
                "writer": "w-000000000001",
                "character": "永",
                "width": 420,
                "height": 420,
                "strokes": [[[1, 2], [3, 4]]],
            }
            (root / "zh_CN" / "w-000000000001.jsonl").write_text(
                json.dumps(record, ensure_ascii=False) + "\n", encoding="utf-8"
            )
            other = dict(record, language="zh_TW", character="永")
            (root / "zh_TW" / "w-000000000002.jsonl").write_text(
                json.dumps(other, ensure_ascii=False) + "\n", encoding="utf-8"
            )
            out = root / "out.s"
            self.assertEqual(
                to_zinnia.main(
                    [
                        "--input",
                        str(root),
                        "--language",
                        "zh_CN",
                        "--out",
                        str(out),
                    ]
                ),
                0,
            )
            self.assertEqual(
                out.read_text(encoding="utf-8").strip(),
                "(character (value 永) (width 420) (height 420) "
                "(strokes ((1 2) (3 4))))",
            )

    def test_writer_filter(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            path = root / "zh_CN.jsonl"
            records = [
                {
                    "language": "zh_CN",
                    "writer": "w-1",
                    "character": "永",
                    "width": 420,
                    "height": 420,
                    "strokes": [[[1, 2], [3, 4]]],
                },
                {
                    "language": "zh_CN",
                    "writer": "w-2",
                    "character": "字",
                    "width": 420,
                    "height": 420,
                    "strokes": [[[1, 2], [3, 4]]],
                },
            ]
            path.write_text(
                "\n".join(json.dumps(item, ensure_ascii=False) for item in records)
                + "\n",
                encoding="utf-8",
            )
            out = root / "out.s"
            self.assertEqual(
                to_zinnia.main(
                    [
                        "--input",
                        str(path),
                        "--language",
                        "zh_CN",
                        "--writer",
                        "w-2",
                        "--out",
                        str(out),
                    ]
                ),
                0,
            )
            self.assertIn("value 字", out.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
