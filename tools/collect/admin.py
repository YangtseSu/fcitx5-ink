#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Admin commands for the fcitx5-ink handwriting collector.

Examples:
    admin.py stats
    admin.py export --out /tmp/ink-export
    admin.py backup --dir /srv/ink/backup --keep 14
    admin.py recent --limit 20
    admin.py delete-writer --writer w-0123456789ab --yes
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sqlite3
import sys
import time
from pathlib import Path

from server import (
    LICENSE,
    REPO,
    ConfigError,
    Store,
    iso,
    load_config,
    load_targets,
    utcnow,
)


def record_from_row(row: sqlite3.Row) -> dict:
    return {
        "id": row["id"],
        "source": "web",
        "language": row["language"],
        "writer": row["writer"],
        "character": row["character"],
        "width": row["width"],
        "height": row["height"],
        "strokes": json.loads(row["strokes"]),
        "created_at": row["created_at"],
    }


def export(store: Store, out: Path, *, require_empty: bool = True) -> tuple[int, int]:
    if out.exists() and require_empty and any(out.iterdir()):
        raise SystemExit(f"output directory is not empty: {out}")
    out.mkdir(parents=True, exist_ok=True)

    rows = store.export_rows()
    groups: dict[tuple[str, str], list[sqlite3.Row]] = {}
    for row in rows:
        groups.setdefault((row["language"], row["writer"]), []).append(row)

    manifest: dict = {
        "generated_at": iso(utcnow()),
        "license": LICENSE,
        "source": "web",
        "writers": [],
    }
    for (language, writer), items in sorted(groups.items()):
        path = out / language / f"{writer}.jsonl"
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", encoding="utf-8") as handle:
            for row in items:
                handle.write(
                    json.dumps(record_from_row(row), ensure_ascii=False) + "\n"
                )
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        manifest["writers"].append(
            {
                "language": language,
                "writer": writer,
                "samples": len(items),
                "license": LICENSE,
                "file": f"{language}/{writer}.jsonl",
                "sha256": digest,
                "created_at": iso(utcnow()),
            }
        )

    (out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (out / "README.md").write_text(
        "# Handwriting sample export\n\n"
        f"Generated {manifest['generated_at']} from {REPO}.\n\n"
        "Every record is an online-stroke sample released under CC BY 4.0. "
        "The writer IDs are anonymous. Keep a writer-level split: do not add a "
        "training writer to the held-out evaluation set. Review a file before "
        "use, then import it into the training pipeline. Convert the export "
        "with tools/collect/to_zinnia.py before training.\n",
        encoding="utf-8",
    )
    return len(rows), len(groups)


def backup(store: Store, directory: Path, keep_days: int) -> tuple[Path, Path]:
    directory.mkdir(parents=True, exist_ok=True)
    stamp = utcnow().strftime("%Y%m%d-%H%M%S")
    db_path = directory / f"ink-{stamp}.db"
    store.backup_to(db_path)
    export_dir = directory / f"export-{stamp}"
    export(store, export_dir, require_empty=False)

    cutoff = time.time() - max(1, keep_days) * 86400
    for item in directory.iterdir():
        if item in (db_path, export_dir):
            continue
        try:
            if item.stat().st_mtime >= cutoff:
                continue
            if item.is_dir():
                shutil.rmtree(item)
            else:
                item.unlink()
        except OSError as exc:
            print(f"ink-admin: prune failed for {item}: {exc}", file=sys.stderr)
    return db_path, export_dir


def main(argv: list[str] | None = None) -> int:
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", default="/etc/ink/collect.json")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("stats", help="print sample statistics as JSON")

    p_export = commands.add_parser("export", help="write per-writer JSONL files")
    p_export.add_argument("--out", type=Path, required=True)

    p_backup = commands.add_parser("backup", help="copy the database and export JSONL")
    p_backup.add_argument("--dir", type=Path, default=Path("/srv/ink/backup"))
    p_backup.add_argument("--keep", type=int, default=14, help="days to keep")

    p_recent = commands.add_parser("recent", help="show recent samples")
    p_recent.add_argument("--limit", type=int, default=20)

    p_delete = commands.add_parser("delete-writer", help="delete one writer's samples")
    p_delete.add_argument("--writer", required=True)
    p_delete.add_argument("--yes", action="store_true")

    args = parser.parse_args(argv)

    try:
        config = load_config(args.config)
        store = Store(config["data_dir"] / "samples.db")
    except (OSError, ValueError, ConfigError) as exc:
        print(f"ink-admin: {exc}", file=sys.stderr)
        return 1

    try:
        if args.command == "stats":
            targets = load_targets(config["targets_dir"])
            print(
                json.dumps(
                    store.stats(
                        {
                            language: set(characters)
                            for language, characters in targets.items()
                        }
                    ),
                    ensure_ascii=False,
                    indent=2,
                )
            )
        elif args.command == "export":
            samples, writers = export(store, args.out)
            print(f"exported {samples} samples from {writers} writer files to {args.out}")
        elif args.command == "backup":
            db_path, export_dir = backup(store, args.dir, args.keep)
            print(f"database: {db_path}")
            print(f"export:   {export_dir}")
        elif args.command == "recent":
            for row in store.recent_rows(args.limit):
                points = sum(len(stroke) for stroke in json.loads(row["strokes"]))
                print(
                    f"{row['created_at']}  {row['language']:<5} {row['character']}  "
                    f"{row['writer']}  {points:>5} points  {row['id']}"
                )
        elif args.command == "delete-writer":
            if not args.yes:
                print("refusing to delete without --yes", file=sys.stderr)
                return 2
            samples, sessions = store.delete_writer(args.writer)
            print(f"deleted {samples} samples, {sessions} sessions for {args.writer}")
        else:
            parser.error("unknown command")
    finally:
        store.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
