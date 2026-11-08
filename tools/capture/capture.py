#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Minimal local handwriting capture tool for evaluation samples.

The tool records online strokes (not images) in the same JSONL shape as
`scripts/tomoe_to_jsonl.py`. It never uploads anything.
"""

import argparse
import datetime as dt
import json
import math
import os
import sys
import tkinter as tk
from pathlib import Path

LANGUAGES = ("zh_CN", "zh_TW", "ja")
CANVAS = 420


def read_targets(path):
    targets = []
    seen = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        character = line[0]
        if character not in seen:
            seen.add(character)
            targets.append(character)
    return targets


class Capture:
    def __init__(self, root, language, writer, targets, output, target_entry):
        self.root = root
        self.language = language
        self.writer = writer
        self.targets = targets
        self.output = output
        self.index = 0
        self.saved = 0
        self.strokes = []
        self.current = []
        self.last = None
        self.target_entry = target_entry

        root.title(f"fcitx5-ink capture — {language} / {writer}")
        root.resizable(False, False)

        top = tk.Frame(root)
        top.pack(fill="x", padx=8, pady=(8, 4))
        self.target_label = tk.Label(
            top, text="", font=("Sans", 44), width=3, anchor="center"
        )
        self.target_label.pack(side="left", padx=(0, 12))
        self.status = tk.Label(top, text="", anchor="w", justify="left")
        self.status.pack(side="left", fill="x", expand=True)

        self.canvas = tk.Canvas(
            root,
            width=CANVAS,
            height=CANVAS,
            background="white",
            cursor="pencil",
            highlightthickness=1,
            highlightbackground="#888",
        )
        self.canvas.pack(padx=8, pady=4)
        self.canvas.bind("<ButtonPress-1>", self.press)
        self.canvas.bind("<B1-Motion>", self.move)
        self.canvas.bind("<ButtonRelease-1>", self.release)

        controls = tk.Frame(root)
        controls.pack(fill="x", padx=8, pady=(4, 8))
        tk.Button(controls, text="清空", command=self.clear).pack(side="left")
        tk.Button(controls, text="撤销", command=self.undo).pack(side="left", padx=4)
        tk.Button(controls, text="跳过", command=self.next).pack(side="left", padx=4)
        tk.Button(controls, text="保存并下一个", command=self.save).pack(
            side="right"
        )

        root.bind("<Control-s>", lambda _event: self.save())
        root.bind("<Control-z>", lambda _event: self.undo())
        root.bind("<Escape>", lambda _event: self.clear())
        self.redraw_guides()
        self.show_target()

    def redraw_guides(self):
        self.canvas.delete("guide")
        self.canvas.create_line(
            CANVAS / 2, 0, CANVAS / 2, CANVAS, fill="#eee", tags="guide"
        )
        self.canvas.create_line(
            0, CANVAS / 2, CANVAS, CANVAS / 2, fill="#eee", tags="guide"
        )
        self.canvas.create_rectangle(
            20, 20, CANVAS - 20, CANVAS - 20, outline="#f0f0f0", tags="guide"
        )

    def redraw(self):
        self.canvas.delete("ink")
        for stroke in self.strokes + ([self.current] if self.current else []):
            if len(stroke) == 1:
                x, y = stroke[0]
                self.canvas.create_oval(
                    x - 2, y - 2, x + 2, y + 2, fill="black", outline="", tags="ink"
                )
                continue
            flat = [coordinate for point in stroke for coordinate in point]
            self.canvas.create_line(
                *flat, fill="black", width=3, capstyle=tk.ROUND,
                joinstyle=tk.ROUND, tags="ink"
            )

    def show_target(self):
        if self.targets:
            self.target_entry.delete(0, "end")
            self.target_entry.insert(0, self.targets[self.index])
            progress = f"{self.index + 1}/{len(self.targets)}"
        else:
            progress = "manual"
        target = self.target_entry.get().strip() or "?"
        self.target_label.config(text=target[:1])
        self.status.config(
            text=f"目标 {progress}  |  已保存 {self.saved} 条  |  "
            f"书写者 {self.writer}"
        )

    def press(self, event):
        self.current = [(float(event.x), float(event.y))]
        self.last = (event.x, event.y)
        self.redraw()

    def move(self, event):
        if self.last is None:
            return
        if math.dist(self.last, (event.x, event.y)) < 0.8:
            return
        self.current.append((float(event.x), float(event.y)))
        self.last = (event.x, event.y)
        self.redraw()

    def release(self, _event):
        if len(self.current) >= 2:
            self.strokes.append(self.current)
            self.current = []
        else:
            self.current = []
        self.last = None
        self.redraw()

    def clear(self):
        self.strokes = []
        self.current = []
        self.last = None
        self.redraw()

    def undo(self):
        if self.current:
            self.current = []
            self.last = None
        elif self.strokes:
            self.strokes.pop()
        self.redraw()

    def next(self):
        if self.targets:
            self.index = min(self.index + 1, len(self.targets) - 1)
        self.clear()
        self.show_target()

    def save(self):
        character = self.target_entry.get().strip()
        if not character:
            self.status.config(text="请先填写目标字")
            return
        if not self.strokes:
            self.status.config(text="还没有笔画")
            return
        record = {
            "id": f"capture-{self.language}-{self.writer}-{self.saved:06d}",
            "source": "capture",
            "language": self.language,
            "writer": self.writer,
            "character": character[0],
            "width": CANVAS,
            "height": CANVAS,
            "strokes": self.strokes,
            "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        }
        self.output.parent.mkdir(parents=True, exist_ok=True)
        with self.output.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        self.saved += 1
        self.index = min(self.index + 1, max(len(self.targets) - 1, 0))
        self.clear()
        self.show_target()
        self.status.config(text=f"已保存到 {self.output}  |  共 {self.saved} 条")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--language", choices=LANGUAGES, required=True)
    parser.add_argument("--writer", required=True, help="anonymous writer id")
    parser.add_argument(
        "--characters",
        type=Path,
        help="one target character per line; otherwise type the target manually",
    )
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    targets = read_targets(args.characters) if args.characters else []
    output = args.out or (
        Path(__file__).resolve().parent.parent.parent
        / "data"
        / "collected"
        / f"{args.language}-{args.writer}.jsonl"
    )
    root = tk.Tk()
    target_entry = tk.Entry(root, width=3, font=("Sans", 16))
    target_entry.pack(padx=8, pady=(0, 4))
    capture = Capture(root, args.language, args.writer, targets, output, target_entry)
    if targets:
        target_entry.delete(0, "end")
        target_entry.insert(0, targets[0])
        capture.show_target()
    root.mainloop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
