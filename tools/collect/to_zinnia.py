#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Convert a collector export to Zinnia S-expression training data.

The input is the per-writer JSONL written by admin.py export (a directory with
``<language>/<writer>.jsonl`` files) or one JSONL file. The output is the
line-based ``.s`` format consumed by tools/ink-train and the Tomoe converters.

Keep a writer-level split: convert a training set and a held-out set from
different writers.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def iter_records(input_path: Path, language: str, writer: str | None):
    if input_path.is_dir():
        files = sorted(input_path.glob(f"{language}/*.jsonl"))
    else:
        files = [input_path]
    for path in files:
        for line_no, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if not line.strip():
                continue
            try:
                item = json.loads(line)
            except json.JSONDecodeError as exc:
                raise SystemExit(f"{path}:{line_no}: invalid JSON: {exc}") from exc
            if item.get("language") != language:
                continue
            if writer is not None and item.get("writer") != writer:
                continue
            yield path, line_no, item


def zinnia_line(item: dict) -> str:
    character = item.get("character")
    if not isinstance(character, str) or not character:
        raise ValueError("missing character")
    strokes = item.get("strokes")
    if not isinstance(strokes, list) or not strokes:
        raise ValueError("missing strokes")
    width = round(float(item["width"]))
    height = round(float(item["height"]))
    encoded_strokes = []
    for stroke in strokes:
        if not isinstance(stroke, list) or len(stroke) < 2:
            continue
        points = [
            f"({round(float(point[0]))} {round(float(point[1]))})"
            for point in stroke
            if isinstance(point, list) and len(point) == 2
        ]
        if len(points) >= 2:
            encoded_strokes.append("(" + " ".join(points) + ")")
    if not encoded_strokes:
        raise ValueError("no usable stroke")
    return (
        f"(character (value {character[0]}) (width {width}) "
        f"(height {height}) (strokes {' '.join(encoded_strokes)}))"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input", type=Path, required=True, help="export directory or JSONL file"
    )
    parser.add_argument("--language", choices=("zh_CN", "zh_TW", "ja"), required=True)
    parser.add_argument("--writer", help="only this writer")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)

    count = 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as output:
        for path, line_no, item in iter_records(args.input, args.language, args.writer):
            try:
                line = zinnia_line(item)
            except (KeyError, TypeError, ValueError) as exc:
                raise SystemExit(f"{path}:{line_no}: {exc}") from exc
            output.write(line + "\n")
            count += 1
    if count == 0:
        args.out.unlink(missing_ok=True)
        raise SystemExit("no matching samples")
    print(f"wrote {count} samples to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
