#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Tests for the collector service and admin helpers."""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import server
from admin import export, record_from_row


def solve(challenge: str, difficulty: int) -> str:
    for nonce in range(1_000_000):
        digest = hashlib.sha256(f"{challenge}:{nonce}".encode()).digest()
        if server.leading_zero_bits(digest) >= difficulty:
            return str(nonce)
    raise AssertionError("no nonce found")


class ValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.targets = {"zh_CN": ["永", "字"], "zh_TW": ["永"], "ja": ["あ"]}

    def sample(self, **overrides):
        payload = {
            "language": "zh_CN",
            "character": "永",
            "width": 420,
            "height": 420,
            "strokes": [[[10, 10], [20, 20]]],
            "consent": True,
        }
        payload.update(overrides)
        return payload

    def test_accepts_valid_sample(self) -> None:
        language, character, width, height, strokes = server.validate_sample(
            self.sample(), self.targets
        )
        self.assertEqual(language, "zh_CN")
        self.assertEqual(character, "永")
        self.assertEqual((width, height), (420.0, 420.0))
        self.assertEqual(strokes, [[[10.0, 10.0], [20.0, 20.0]]])

    def test_rejects_unknown_character(self) -> None:
        with self.assertRaises(server.ApiError):
            server.validate_sample(self.sample(character="水"), self.targets)

    def test_rejects_missing_consent(self) -> None:
        with self.assertRaises(server.ApiError):
            server.validate_sample(self.sample(consent=False), self.targets)

    def test_rejects_short_stroke(self) -> None:
        with self.assertRaises(server.ApiError):
            server.validate_sample(self.sample(strokes=[[[10, 10]]]), self.targets)

    def test_rejects_out_of_range_point(self) -> None:
        with self.assertRaises(server.ApiError):
            server.validate_sample(
                self.sample(strokes=[[[10, 10], [500, 20]]]), self.targets
            )

    def test_rejects_bool_coordinate(self) -> None:
        with self.assertRaises(server.ApiError):
            server.validate_sample(
                self.sample(strokes=[[[True, 10], [20, 20]]]), self.targets
            )

    def test_rejects_bad_language(self) -> None:
        with self.assertRaises(server.ApiError):
            server.validate_sample(self.sample(language="ko"), self.targets)


class StoreTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.store = server.Store(Path(self.tempdir.name) / "samples.db")
        self.record = {
            "id": "web-1",
            "writer": "w-000000000001",
            "language": "zh_CN",
            "character": "永",
            "width": 420.0,
            "height": 420.0,
            "strokes": "[[[1,1],[2,2]]]",
            "created_at": server.iso(server.utcnow()),
            "consent": 1,
            "license": server.LICENSE,
            "ip_hash": "hash",
            "dedup": "dedup-1",
        }

    def tearDown(self) -> None:
        self.store.close()
        self.tempdir.cleanup()

    def test_insert_and_dedup(self) -> None:
        self.assertTrue(self.store.add_sample(self.record))
        self.assertFalse(self.store.add_sample(self.record))
        stats = self.store.stats()
        self.assertEqual(stats["languages"]["zh_CN"]["samples"], 1)
        self.assertEqual(stats["languages"]["zh_CN"]["writers"], 1)

    def test_session_roundtrip(self) -> None:
        token, _ = self.store.create_session("w-000000000001", "hash", 600)
        self.assertEqual(self.store.get_session(token), "w-000000000001")
        self.assertIsNone(self.store.get_session("nope"))

    def test_delete_writer(self) -> None:
        self.store.add_sample(self.record)
        samples, _ = self.store.delete_writer("w-000000000001")
        self.assertEqual(samples, 1)
        self.assertEqual(self.store.stats()["samples"], 0)

    def test_stats_counts_only_pool_characters(self) -> None:
        self.store.add_sample(self.record)
        other = dict(self.record, id="web-2", character="字", dedup="dedup-2")
        self.store.add_sample(other)
        outside = dict(self.record, id="web-3", character="水", dedup="dedup-3")
        self.store.add_sample(outside)
        stats = self.store.stats({"zh_CN": {"永", "字"}})
        self.assertEqual(stats["languages"]["zh_CN"]["samples"], 3)
        self.assertEqual(stats["languages"]["zh_CN"]["characters"], 2)

    def test_export(self) -> None:
        self.store.add_sample(self.record)
        out = Path(self.tempdir.name) / "export"
        samples, writers = export(self.store, out)
        self.assertEqual((samples, writers), (1, 1))
        line = (out / "zh_CN" / "w-000000000001.jsonl").read_text().strip()
        item = json.loads(line)
        self.assertEqual(item["character"], "永")
        self.assertEqual(item["source"], "web")
        manifest = json.loads((out / "manifest.json").read_text())
        self.assertEqual(manifest["writers"][0]["samples"], 1)
        self.assertEqual(manifest["writers"][0]["license"], "CC-BY-4.0")


class ChallengeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.store = server.Store(Path(self.tempdir.name) / "samples.db")
        self.config = {
            "secret": "x" * 32,
            "origin": "https://example.test",
            "difficulty": 12,
            "challenge_ttl": 60,
            "session_ttl": 600,
            "limits": {
                "samples_per_hour": 10,
                "samples_per_day": 10,
                "sessions_per_hour": 10,
                "global_per_day": 10,
            },
        }
        self.app = server.App(self.config, {"zh_CN": ["永"], "zh_TW": ["永"], "ja": ["あ"]}, self.store)

    def tearDown(self) -> None:
        self.store.close()
        self.tempdir.cleanup()

    def test_challenge_accepts_valid_proof(self) -> None:
        issued = self.app.issue_challenge()
        nonce = solve(issued["challenge"], issued["difficulty"])
        self.app.verify_challenge(issued["challenge"], nonce)

    def test_challenge_is_single_use(self) -> None:
        issued = self.app.issue_challenge()
        nonce = solve(issued["challenge"], issued["difficulty"])
        self.app.verify_challenge(issued["challenge"], nonce)
        with self.assertRaises(server.ApiError):
            self.app.verify_challenge(issued["challenge"], nonce)


class TargetTests(unittest.TestCase):
    def test_load_targets(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            for language in server.LANGUAGES:
                (root / f"{language}.txt").write_text("永\n字\n\n永\n", encoding="utf-8")
            targets = server.load_targets(root)
            self.assertEqual(targets["zh_CN"], ["永", "字"])

    def test_leading_zero_bits(self) -> None:
        self.assertEqual(server.leading_zero_bits(bytes.fromhex("00ff")), 8)
        self.assertEqual(server.leading_zero_bits(bytes.fromhex("0fff")), 4)
        self.assertEqual(server.leading_zero_bits(bytes.fromhex("ffff")), 0)


if __name__ == "__main__":
    unittest.main()
