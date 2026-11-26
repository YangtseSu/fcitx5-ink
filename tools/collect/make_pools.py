#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Build the collector's training and evaluation character pools.

Training pools are the official common-character sets, ordered by the Unicode
Unihan ``kFrequency`` grade:

- zh_CN: 义务教育语文课程标准 常用字表 2500 + 1000 (通用规范汉字表 一级字),
  read from the HearWrite assets (GPL-3.0-or-later).
- zh_TW: 臺灣《常用國字標準字體表》 4,808, from zispace/hanzi-chars.
- ja: 日本《常用漢字表》 2,136 plus the kana present in the Tomoe model,
  from zispace/hanzi-chars and data/tomoe.

The evaluation pools are the existing 300 random targets per language from
the Tomoe corpus (``make_capture_list.py --limit 300 --seed 1``); they stay
held out for writer-independent evaluation.

The output keeps only characters that exist in the corresponding Tomoe model
classes, because a character outside the model cannot be trained into it.
"""

from __future__ import annotations

import argparse
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

LANGUAGES = ("zh_CN", "zh_TW", "ja")
TOMOE_FILES = {
    "zh_CN": "handwriting-zh_CN.xml",
    "zh_TW": "handwriting-zh_TW.xml",
    "ja": "handwriting-ja.xml",
}
EVAL_HEADER = """\
# Held-out evaluation pool for the fcitx5-ink handwriting collector.
# 300 characters sampled uniformly from the Tomoe corpus (seed 1).
# Source: data/tomoe/handwriting-{language}.xml, LGPL-2.1-or-later.
# Collect evaluation samples from different writers than the training pool.
"""
TRAIN_HEADER = {
    "zh_CN": """\
# Training pool for the fcitx5-ink handwriting collector.
# Source: 义务教育语文课程标准 常用字表 2500 + 1000 (= 通用规范汉字表 一级字),
#   via the HearWrite project, GPL-3.0-or-later;
#   cross-check: 通用规范汉字表 一级字 (shengdoushi/common-standard-chinese-characters-table).
# Order: Unicode Unihan kFrequency / kGradeLevel, Unicode-3.0.
""",
    "zh_TW": """\
# Training pool for the fcitx5-ink handwriting collector.
# Source: 臺灣《常用國字標準字體表》 4,808 (zispace/hanzi-chars, official standard).
# Order: Unicode Unihan kFrequency / kGradeLevel, Unicode-3.0.
""",
    "ja": """\
# Training pool for the fcitx5-ink handwriting collector.
# Source: 日本《常用漢字表》 2,136 (zispace/hanzi-chars, official standard)
#   plus the kana present in the Tomoe Japanese model.
# Order: Unicode Unihan kFrequency / kGradeLevel, Unicode-3.0.
""",
}


def read_chars(path: Path) -> list[str]:
    chars: list[str] = []
    seen: set[str] = set()
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        char = line[0]
        if char not in seen:
            seen.add(char)
            chars.append(char)
    return chars


def tomoe_classes(path: Path) -> set[str]:
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


def unihan_frequency(zip_path: Path) -> tuple[dict[str, int], dict[str, int]]:
    frequency: dict[str, int] = {}
    grade: dict[str, int] = {}
    with zipfile.ZipFile(zip_path) as archive:
        with archive.open("Unihan_DictionaryLikeData.txt") as handle:
            for raw in handle:
                line = raw.decode("utf-8").strip()
                if not line or line.startswith("#"):
                    continue
                code, field, value = line.split("\t")
                if field not in ("kFrequency", "kGradeLevel"):
                    continue
                char = chr(int(code[2:], 16))
                if field == "kFrequency":
                    frequency[char] = int(value)
                else:
                    grade[char] = int(value)
    return frequency, grade


def is_kana(char: str) -> bool:
    code = ord(char)
    return 0x3040 <= code <= 0x30FF


def order_chars(
    chars: list[str], frequency: dict[str, int], grade: dict[str, int]
) -> list[str]:
    return sorted(
        chars,
        key=lambda char: (frequency.get(char, 9), grade.get(char, 9), ord(char)),
    )


def write_pool(path: Path, header: str, chars: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(header.strip() + "\n" + "\n".join(chars) + "\n", encoding="utf-8")


def build_pool(
    language: str,
    sources: list[Path],
    classes: set[str],
    frequency: dict[str, int],
    grade: dict[str, int],
) -> list[str]:
    chars: list[str] = []
    seen: set[str] = set()
    for path in sources:
        for char in read_chars(path):
            if char not in seen and char in classes:
                seen.add(char)
                chars.append(char)
    if language == "ja":
        kana = [char for char in classes if is_kana(char)]
        chars = sorted(kana, key=ord) + order_chars(
            [char for char in chars if not is_kana(char)], frequency, grade
        )
    else:
        chars = order_chars(chars, frequency, grade)
    return chars


def main(argv: list[str] | None = None) -> int:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--zh-cn-2500",
        type=Path,
        default=root.parent.parent / "HearWrite/app/src/main/assets/义务教育语文课程标准/常用字表1 2500.txt",
    )
    parser.add_argument(
        "--zh-cn-1000",
        type=Path,
        default=root.parent.parent / "HearWrite/app/src/main/assets/义务教育语文课程标准/常用字表2 1000.txt",
    )
    parser.add_argument("--zh-tw", type=Path, required=True, help="臺灣常用國字表 text")
    parser.add_argument("--ja", type=Path, required=True, help="日本常用漢字表 text")
    parser.add_argument("--unihan", type=Path, required=True, help="Unihan.zip")
    parser.add_argument("--tomoe", type=Path, default=root / "data" / "tomoe")
    parser.add_argument("--eval", type=Path, default=root / "tools/collect/targets/eval")
    parser.add_argument("--out", type=Path, default=root / "tools/collect/targets")
    args = parser.parse_args(argv)

    frequency, grade = unihan_frequency(args.unihan)
    sources = {
        "zh_CN": [args.zh_cn_2500, args.zh_cn_1000],
        "zh_TW": [args.zh_tw],
        "ja": [args.ja],
    }
    classes = {
        language: tomoe_classes(args.tomoe / TOMOE_FILES[language])
        for language in LANGUAGES
    }
    for language in LANGUAGES:
        train = build_pool(
            language, sources[language], classes[language], frequency, grade
        )
        eval_path = args.eval / f"{language}.txt"
        eval_chars = read_chars(eval_path)
        write_pool(
            args.out / "train" / f"{language}.txt",
            TRAIN_HEADER[language],
            train,
        )
        write_pool(
            args.out / "eval" / f"{language}.txt",
            EVAL_HEADER.format(language=language),
            eval_chars,
        )
        print(
            f"{language}: train {len(train)} chars, eval {len(eval_chars)} chars, "
            f"model classes {len(classes[language])}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
