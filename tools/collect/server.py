#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Handwriting sample collection API for fcitx5-ink.

The service records online strokes from the browser, keeps them in SQLite, and
reports anonymous aggregate statistics. It binds to localhost; nginx terminates
TLS and serves the static page. See README.md for the deployment.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import hmac
import ipaddress
import json
import math
import re
import secrets
import sqlite3
import sys
import threading
import time
import uuid
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

LANGUAGES = ("zh_CN", "zh_TW", "ja")
LICENSE = "CC-BY-4.0"
REPO = "https://github.com/YangtseSu/fcitx5-ink"

MAX_BODY = 64 * 1024
MAX_STROKES = 64
MAX_POINTS_PER_STROKE = 2000
MAX_TOTAL_POINTS = 8000
MIN_STROKE_POINTS = 2
MIN_SIZE = 100.0
MAX_SIZE = 2000.0

WRITER_RE = re.compile(r"w-[0-9a-f]{12}")


class ConfigError(Exception):
    """Invalid service configuration."""


class ApiError(Exception):
    """An error that maps to a JSON API response."""

    def __init__(self, status: int, code: str, message: str = "") -> None:
        super().__init__(message or code)
        self.status = int(status)
        self.code = code
        self.message = message or code


def utcnow() -> dt.datetime:
    return dt.datetime.now(dt.timezone.utc)


def iso(ts: dt.datetime) -> str:
    return ts.isoformat(timespec="microseconds")


def parse_iso(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value)


def load_config(path: str | Path) -> dict:
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        raise ConfigError("config must be a JSON object")

    secret = raw.get("secret")
    if not isinstance(secret, str) or len(secret) < 32:
        raise ConfigError("secret must be a string of at least 32 characters")

    config = dict(raw)
    config["secret"] = secret
    config["listen"] = str(raw.get("listen", "127.0.0.1"))
    config["port"] = int(raw.get("port", 8081))
    config["domain"] = str(raw.get("domain", ""))
    config["origin"] = f"https://{config['domain']}" if config["domain"] else ""
    config["data_dir"] = Path(raw.get("data_dir", "/srv/ink/data"))
    config["targets_dir"] = Path(raw.get("targets_dir", "/srv/ink/targets"))
    config["eval_token"] = str(raw.get("eval_token", ""))
    config["cookie_prefix"] = str(raw.get("cookie_prefix", "ink_"))
    config["difficulty"] = int(raw.get("difficulty", 16))
    config["session_ttl"] = int(raw.get("session_ttl", 7200))
    config["challenge_ttl"] = int(raw.get("challenge_ttl", 600))
    limits = raw.get("limits", {})
    if not isinstance(limits, dict):
        raise ConfigError("limits must be an object")
    config["limits"] = {
        "samples_per_hour": int(limits.get("samples_per_hour", 300)),
        "samples_per_day": int(limits.get("samples_per_day", 1000)),
        "sessions_per_hour": int(limits.get("sessions_per_hour", 30)),
        "global_per_day": int(limits.get("global_per_day", 5000)),
    }

    if not 8 <= config["difficulty"] <= 24 or config["difficulty"] % 4:
        raise ConfigError("difficulty must be a multiple of 4 between 8 and 24")
    if config["port"] < 1 or config["port"] > 65535:
        raise ConfigError("port out of range")
    if config["session_ttl"] < 60 or config["challenge_ttl"] < 30:
        raise ConfigError("session and challenge TTLs are too small")
    return config


def load_targets(directory: Path) -> dict[str, list[str]]:
    targets: dict[str, list[str]] = {}
    for language in LANGUAGES:
        path = directory / f"{language}.txt"
        seen: set[str] = set()
        characters: list[str] = []
        for line in path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            character = line[0]
            if character not in seen:
                seen.add(character)
                characters.append(character)
        if not characters:
            raise ConfigError(f"target list is empty: {path}")
        targets[language] = characters
    return targets


def leading_zero_bits(data: bytes) -> int:
    bits = 0
    for byte in data:
        if byte == 0:
            bits += 8
            continue
        bits += 8 - byte.bit_length()
        break
    return bits


def hash_token(token: str) -> str:
    return hashlib.sha256(token.encode("utf-8")).hexdigest()


SCHEMA = """
CREATE TABLE IF NOT EXISTS writers (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    consent INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS samples (
    id TEXT PRIMARY KEY,
    writer TEXT NOT NULL,
    language TEXT NOT NULL,
    character TEXT NOT NULL,
    width REAL NOT NULL,
    height REAL NOT NULL,
    strokes TEXT NOT NULL,
    created_at TEXT NOT NULL,
    consent INTEGER NOT NULL,
    license TEXT NOT NULL,
    ip_hash TEXT NOT NULL,
    dedup TEXT NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS samples_lang_char ON samples(language, character);
CREATE INDEX IF NOT EXISTS samples_created ON samples(created_at);
CREATE INDEX IF NOT EXISTS samples_writer ON samples(writer);
CREATE INDEX IF NOT EXISTS samples_ip_created ON samples(ip_hash, created_at);
CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    writer TEXT NOT NULL,
    ip_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_created ON sessions(created_at);
CREATE INDEX IF NOT EXISTS sessions_expires ON sessions(expires_at);
"""


class Store:
    """Small SQLite store shared by the service and the admin CLI."""

    def __init__(self, db_path: Path) -> None:
        db_path.parent.mkdir(parents=True, exist_ok=True)
        self.lock = threading.Lock()
        self.conn = sqlite3.connect(
            str(db_path), timeout=10.0, check_same_thread=False
        )
        self.conn.row_factory = sqlite3.Row
        self.conn.execute("PRAGMA journal_mode=WAL")
        self.conn.execute("PRAGMA synchronous=NORMAL")
        self.conn.execute("PRAGMA busy_timeout=5000")
        self.conn.executescript(SCHEMA)
        self.conn.commit()

    def close(self) -> None:
        with self.lock:
            self.conn.close()

    def ensure_writer(self, writer: str, consent: bool) -> None:
        with self.lock:
            self.conn.execute(
                "INSERT INTO writers(id, created_at, consent) VALUES(?,?,?) "
                "ON CONFLICT(id) DO UPDATE SET consent=MAX(consent,excluded.consent)",
                (writer, iso(utcnow()), 1 if consent else 0),
            )
            self.conn.commit()

    def add_sample(self, record: dict) -> bool:
        """Insert a sample; return False when the dedup hash already exists."""
        with self.lock:
            try:
                self.conn.execute(
                    "INSERT INTO samples("
                    "id, writer, language, character, width, height, strokes, "
                    "created_at, consent, license, ip_hash, dedup"
                    ") VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
                    (
                        record["id"],
                        record["writer"],
                        record["language"],
                        record["character"],
                        record["width"],
                        record["height"],
                        record["strokes"],
                        record["created_at"],
                        record["consent"],
                        record["license"],
                        record["ip_hash"],
                        record["dedup"],
                    ),
                )
                self.conn.execute(
                    "INSERT INTO writers(id, created_at, consent) VALUES(?,?,1) "
                    "ON CONFLICT(id) DO UPDATE SET consent=1",
                    (record["writer"], record["created_at"]),
                )
                self.conn.commit()
                return True
            except sqlite3.IntegrityError as exc:
                self.conn.rollback()
                if "samples.dedup" in str(exc):
                    return False
                raise

    def count_samples(self, ip_hash: str | None, since: str) -> int:
        with self.lock:
            if ip_hash is None:
                row = self.conn.execute(
                    "SELECT COUNT(*) FROM samples WHERE created_at>=?", (since,)
                ).fetchone()
            else:
                row = self.conn.execute(
                    "SELECT COUNT(*) FROM samples "
                    "WHERE ip_hash=? AND created_at>=?",
                    (ip_hash, since),
                ).fetchone()
        return int(row[0])

    def create_session(
        self, writer: str, ip_hash: str, ttl: int
    ) -> tuple[str, dt.datetime]:
        token = secrets.token_urlsafe(32)
        now = utcnow()
        expires = now + dt.timedelta(seconds=ttl)
        with self.lock:
            self.conn.execute(
                "DELETE FROM sessions WHERE expires_at<?", (iso(now),)
            )
            self.conn.execute(
                "INSERT INTO sessions(token_hash, writer, ip_hash, created_at, "
                "expires_at) VALUES(?,?,?,?,?)",
                (hash_token(token), writer, ip_hash, iso(now), iso(expires)),
            )
            self.conn.commit()
        return token, expires

    def get_session(self, token: str) -> str | None:
        with self.lock:
            row = self.conn.execute(
                "SELECT writer FROM sessions "
                "WHERE token_hash=? AND expires_at>?",
                (hash_token(token), iso(utcnow())),
            ).fetchone()
        return str(row["writer"]) if row else None

    def count_sessions(self, ip_hash: str, since: str) -> int:
        with self.lock:
            row = self.conn.execute(
                "SELECT COUNT(*) FROM sessions "
                "WHERE ip_hash=? AND created_at>=?",
                (ip_hash, since),
            ).fetchone()
        return int(row[0])

    def stats(self, targets: dict[str, set[str]] | None = None) -> dict:
        with self.lock:
            rows = self.conn.execute(
                "SELECT language, COUNT(*) AS samples, "
                "COUNT(DISTINCT writer) AS writers, "
                "COUNT(DISTINCT character) AS characters "
                "FROM samples WHERE consent=1 GROUP BY language"
            ).fetchall()
            covered: dict[str, set[str]] = {}
            if targets is not None:
                for row in self.conn.execute(
                    "SELECT DISTINCT language, character FROM samples "
                    "WHERE consent=1"
                ):
                    pool = targets.get(row["language"])
                    if pool is not None and row["character"] in pool:
                        covered.setdefault(row["language"], set()).add(
                            row["character"]
                        )
        languages = {
            language: {"samples": 0, "writers": 0, "characters": 0}
            for language in LANGUAGES
        }
        for row in rows:
            characters = int(row["characters"])
            if targets is not None:
                characters = len(covered.get(row["language"], set()))
            languages[row["language"]] = {
                "samples": int(row["samples"]),
                "writers": int(row["writers"]),
                "characters": characters,
            }
        return {
            "languages": languages,
            "samples": sum(item["samples"] for item in languages.values()),
            "writers": sum(item["writers"] for item in languages.values()),
        }

    def export_rows(self) -> list[sqlite3.Row]:
        with self.lock:
            return self.conn.execute(
                "SELECT id, writer, language, character, width, height, strokes, "
                "created_at FROM samples WHERE consent=1 "
                "ORDER BY language, writer, created_at, id"
            ).fetchall()

    def recent_rows(self, limit: int) -> list[sqlite3.Row]:
        with self.lock:
            return self.conn.execute(
                "SELECT id, writer, language, character, width, height, strokes, "
                "created_at, ip_hash FROM samples "
                "ORDER BY created_at DESC LIMIT ?",
                (max(1, min(limit, 500)),),
            ).fetchall()

    def delete_writer(self, writer: str) -> tuple[int, int]:
        with self.lock:
            samples = self.conn.execute(
                "DELETE FROM samples WHERE writer=?", (writer,)
            ).rowcount
            sessions = self.conn.execute(
                "DELETE FROM sessions WHERE writer=?", (writer,)
            ).rowcount
            self.conn.execute("DELETE FROM writers WHERE id=?", (writer,))
            self.conn.commit()
        return int(samples), int(sessions)

    def backup_to(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        with self.lock:
            dest = sqlite3.connect(str(path))
            try:
                self.conn.backup(dest)
            finally:
                dest.close()


def validate_sample(
    payload: dict, targets: dict[str, list[str]]
) -> tuple[str, str, float, float, list[list[list[float]]]]:
    language = payload.get("language")
    if language not in LANGUAGES:
        raise ApiError(HTTPStatus.BAD_REQUEST, "language", "unknown language")

    character = payload.get("character")
    if not isinstance(character, str) or len(character) != 1:
        raise ApiError(HTTPStatus.BAD_REQUEST, "character", "invalid character")
    if character not in targets[language]:
        raise ApiError(HTTPStatus.BAD_REQUEST, "character", "character is not a target")

    width = payload.get("width")
    height = payload.get("height")
    if (
        not isinstance(width, (int, float))
        or isinstance(width, bool)
        or not isinstance(height, (int, float))
        or isinstance(height, bool)
        or not math.isfinite(width)
        or not math.isfinite(height)
        or not MIN_SIZE <= width <= MAX_SIZE
        or not MIN_SIZE <= height <= MAX_SIZE
    ):
        raise ApiError(HTTPStatus.BAD_REQUEST, "size", "invalid canvas size")

    strokes = payload.get("strokes")
    if not isinstance(strokes, list) or not 1 <= len(strokes) <= MAX_STROKES:
        raise ApiError(HTTPStatus.BAD_REQUEST, "strokes", "invalid stroke count")

    clean: list[list[list[float]]] = []
    total = 0
    for stroke in strokes:
        if (
            not isinstance(stroke, list)
            or not MIN_STROKE_POINTS <= len(stroke) <= MAX_POINTS_PER_STROKE
        ):
            raise ApiError(HTTPStatus.BAD_REQUEST, "strokes", "invalid stroke")
        points: list[list[float]] = []
        for point in stroke:
            if not isinstance(point, list) or len(point) != 2:
                raise ApiError(HTTPStatus.BAD_REQUEST, "point", "invalid point")
            x, y = point
            if (
                not isinstance(x, (int, float))
                or isinstance(x, bool)
                or not isinstance(y, (int, float))
                or isinstance(y, bool)
                or not math.isfinite(float(x))
                or not math.isfinite(float(y))
                or not 0.0 <= float(x) <= width
                or not 0.0 <= float(y) <= height
            ):
                raise ApiError(HTTPStatus.BAD_REQUEST, "point", "point out of range")
            points.append([float(x), float(y)])
        total += len(points)
        if total > MAX_TOTAL_POINTS:
            raise ApiError(HTTPStatus.BAD_REQUEST, "strokes", "too many points")
        clean.append(points)

    if payload.get("consent") is not True:
        raise ApiError(HTTPStatus.BAD_REQUEST, "consent", "consent is required")
    return language, character, float(width), float(height), clean


def dedup_hash(language: str, character: str, strokes: list) -> str:
    body = json.dumps(strokes, ensure_ascii=False, separators=(",", ":"))
    raw = f"{language}\x00{character}\x00{body}".encode("utf-8")
    return hashlib.sha256(raw).hexdigest()


class App:
    def __init__(self, config: dict, targets: dict[str, list[str]], store: Store):
        self.config = config
        self.targets = targets
        self.target_sets = {
            language: set(characters) for language, characters in targets.items()
        }
        self.store = store
        self.secret = config["secret"].encode("utf-8")
        self.eval_token = str(config.get("eval_token", ""))
        self.cookie_prefix = str(config.get("cookie_prefix", "ink_"))
        self.eval_attempts: dict[str, list[float]] = {}
        self.eval_lock = threading.Lock()
        self.difficulty = int(config["difficulty"])
        self.origin = config["origin"]
        self.challenges: dict[str, float] = {}
        self.challenge_lock = threading.Lock()

    def ip_hash(self, ip: str) -> str:
        try:
            normalized = ipaddress.ip_address(ip).compressed
        except ValueError:
            normalized = ip
        return hmac.new(self.secret, normalized.encode("utf-8"), hashlib.sha256).hexdigest()

    def eval_cookie_value(self) -> str:
        expires = int(time.time()) + 86400
        signature = hmac.new(
            self.secret, f"eval:{expires}".encode("utf-8"), hashlib.sha256
        ).hexdigest()
        return f"{expires}.{signature}"

    def eval_cookie_ok(self, value: str | None) -> bool:
        if not value:
            return False
        expires, _, signature = value.partition(".")
        if not expires.isdigit() or int(expires) < time.time():
            return False
        expected = hmac.new(
            self.secret, f"eval:{expires}".encode("utf-8"), hashlib.sha256
        ).hexdigest()
        return hmac.compare_digest(signature, expected)

    def eval_attempt_allowed(self, ip_hash: str) -> bool:
        with self.eval_lock:
            now = time.monotonic()
            attempts = [
                stamp
                for stamp in self.eval_attempts.get(ip_hash, [])
                if now - stamp < 3600
            ]
            if len(attempts) >= 10:
                return False
            attempts.append(now)
            self.eval_attempts[ip_hash] = attempts
            if len(self.eval_attempts) > 10000:
                self.eval_attempts.clear()
            return True

    def issue_challenge(self) -> dict:
        challenge = secrets.token_urlsafe(16)
        ttl = int(self.config["challenge_ttl"])
        now = time.monotonic()
        with self.challenge_lock:
            for key, expires in list(self.challenges.items()):
                if expires < now:
                    self.challenges.pop(key, None)
            if len(self.challenges) > 20000:
                self.challenges.clear()
            self.challenges[challenge] = now + ttl
        return {
            "challenge": challenge,
            "difficulty": self.difficulty,
            "expires_in": ttl,
        }

    def verify_challenge(self, challenge: str, nonce: str) -> None:
        if not isinstance(challenge, str) or not isinstance(nonce, str):
            raise ApiError(HTTPStatus.BAD_REQUEST, "challenge", "invalid challenge")
        if not nonce.isdigit() or not 1 <= len(nonce) <= 20:
            raise ApiError(HTTPStatus.BAD_REQUEST, "challenge", "invalid nonce")
        with self.challenge_lock:
            expires = self.challenges.pop(challenge, None)
        if expires is None:
            raise ApiError(
                HTTPStatus.BAD_REQUEST, "challenge_unknown", "unknown challenge"
            )
        if time.monotonic() > expires:
            raise ApiError(
                HTTPStatus.BAD_REQUEST, "challenge_expired", "expired challenge"
            )
        digest = hashlib.sha256(f"{challenge}:{nonce}".encode("utf-8")).digest()
        if leading_zero_bits(digest) < self.difficulty:
            raise ApiError(
                HTTPStatus.BAD_REQUEST, "challenge_pow", "proof of work failed"
            )

    def new_writer(self) -> str:
        return "w-" + secrets.token_hex(6)

    def session_rate_limited(self, ip_hash: str) -> bool:
        since = iso(utcnow() - dt.timedelta(hours=1))
        limit = int(self.config["limits"]["sessions_per_hour"])
        return self.store.count_sessions(ip_hash, since) >= limit

    def sample_rate_limited(self, ip_hash: str) -> str | None:
        now = utcnow()
        hour_since = iso(now - dt.timedelta(hours=1))
        day_since = iso(now - dt.timedelta(days=1))
        limits = self.config["limits"]
        if self.store.count_samples(ip_hash, hour_since) >= int(limits["samples_per_hour"]):
            return "hour"
        if self.store.count_samples(ip_hash, day_since) >= int(limits["samples_per_day"]):
            return "day"
        if self.store.count_samples(None, day_since) >= int(limits["global_per_day"]):
            return "global"
        return None


def make_handler(app: App):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"
        server_version = "ink"
        sys_version = ""

        def log_message(self, fmt: str, *args) -> None:  # noqa: A003
            return

        def client_ip(self) -> str:
            value = self.headers.get("X-Real-IP") or self.client_address[0]
            return value.strip()

        def client_ip_hash(self) -> str:
            return app.ip_hash(self.client_ip())

        def origin_ok(self) -> bool:
            origin = self.headers.get("Origin")
            if origin is not None:
                if not app.origin or origin != app.origin:
                    return False
            site = self.headers.get("Sec-Fetch-Site")
            if site is not None and site not in ("same-origin", "none"):
                return False
            return True

        def read_json(self) -> dict:
            if not self.origin_ok():
                raise ApiError(HTTPStatus.FORBIDDEN, "origin", "origin rejected")
            ctype = self.headers.get("Content-Type", "").split(";")[0].strip().lower()
            if ctype != "application/json":
                raise ApiError(
                    HTTPStatus.UNSUPPORTED_MEDIA_TYPE,
                    "content_type",
                    "application/json required",
                )
            raw_length = self.headers.get("Content-Length")
            if raw_length is None:
                raise ApiError(HTTPStatus.LENGTH_REQUIRED, "length", "length required")
            try:
                length = int(raw_length)
            except ValueError as exc:
                raise ApiError(HTTPStatus.BAD_REQUEST, "length", "invalid length") from exc
            if length < 0 or length > MAX_BODY:
                raise ApiError(HTTPStatus.REQUEST_ENTITY_TOO_LARGE, "too_large", "body too large")
            raw = self.rfile.read(length)
            if len(raw) != length:
                raise ApiError(HTTPStatus.BAD_REQUEST, "body", "truncated body")
            try:
                payload = json.loads(raw.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise ApiError(HTTPStatus.BAD_REQUEST, "json", "invalid JSON") from exc
            if not isinstance(payload, dict):
                raise ApiError(HTTPStatus.BAD_REQUEST, "json", "object required")
            return payload

        def cookie(self, name: str) -> str | None:
            full = app.cookie_prefix + name
            for part in self.headers.get("Cookie", "").split(";"):
                key, _, value = part.strip().partition("=")
                if key == full:
                    return value
            return None

        def eval_authed(self) -> bool:
            if not app.eval_token:
                return True
            return app.eval_cookie_ok(self.cookie("auth"))

        def send_json(
            self, status: int, payload: dict, cookies: list[str] | None = None,
            cache: str = "no-store",
        ) -> None:
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", cache)
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Referrer-Policy", "no-referrer")
            for cookie in cookies or []:
                self.send_header("Set-Cookie", cookie)
            self.end_headers()
            self.wfile.write(body)

        def cookie_header(self, name: str, value: str, max_age: int) -> str:
            return (
                f"{app.cookie_prefix}{name}={value}; Path=/; Max-Age={max_age}; "
                "HttpOnly; Secure; SameSite=Lax"
            )

        def do_GET(self) -> None:  # noqa: N802
            try:
                parsed = urlsplit(self.path)
                path = parsed.path
                query = parse_qs(parsed.query, keep_blank_values=True)
                if path not in ("/api/health", "/api/eval/status"):
                    if not self.eval_authed():
                        raise ApiError(
                            HTTPStatus.UNAUTHORIZED,
                            "eval_auth",
                            "evaluation login required",
                        )
                if path == "/api/eval/status":
                    self.send_json(HTTPStatus.OK, {"ok": self.eval_authed()})
                elif path == "/api/health":
                    with app.store.lock:
                        app.store.conn.execute("SELECT 1").fetchone()
                    self.send_json(HTTPStatus.OK, {"ok": True})
                elif path == "/api/config":
                    self.send_json(
                        HTTPStatus.OK,
                        {
                            "languages": list(LANGUAGES),
                            "license": LICENSE,
                            "repo": REPO,
                            "difficulty": app.difficulty,
                        },
                        cache="public, max-age=600",
                    )
                elif path == "/api/targets":
                    language = query.get("lang", [""])[0]
                    if language not in app.targets:
                        raise ApiError(HTTPStatus.BAD_REQUEST, "language", "unknown language")
                    characters = app.targets[language]
                    self.send_json(
                        HTTPStatus.OK,
                        {
                            "language": language,
                            "count": len(characters),
                            "characters": characters,
                        },
                        cache="public, max-age=600",
                    )
                elif path == "/api/stats":
                    self.send_json(
                        HTTPStatus.OK,
                        app.store.stats(app.target_sets),
                        cache="public, max-age=10",
                    )
                elif path == "/api/challenge":
                    self.send_json(HTTPStatus.OK, app.issue_challenge())
                elif path == "/api/session":
                    token = self.cookie("session")
                    writer = app.store.get_session(token) if token else None
                    payload = {"ok": bool(writer)}
                    if writer:
                        payload["writer"] = writer
                    self.send_json(HTTPStatus.OK, payload)
                else:
                    raise ApiError(HTTPStatus.NOT_FOUND, "not_found", "unknown endpoint")
            except ApiError as exc:
                print(
                    f"ink-collect: {self.command} {urlsplit(self.path).path} "
                    f"-> {exc.status} {exc.code}",
                    flush=True,
                )
                self.send_json(exc.status, {"error": exc.code, "message": exc.message})
            except Exception as exc:  # noqa: BLE001
                print(f"ink-collect: GET error: {exc.__class__.__name__}: {exc}", file=sys.stderr, flush=True)
                self.send_json(HTTPStatus.INTERNAL_SERVER_ERROR, {"error": "internal"})

        def do_POST(self) -> None:  # noqa: N802
            try:
                parsed = urlsplit(self.path)
                path = parsed.path
                payload = self.read_json()
                ip_hash = self.client_ip_hash()
                if path != "/api/eval/login" and not self.eval_authed():
                    raise ApiError(
                        HTTPStatus.UNAUTHORIZED,
                        "eval_auth",
                        "evaluation login required",
                    )

                if path == "/api/eval/login":
                    if not app.eval_token:
                        raise ApiError(
                            HTTPStatus.NOT_FOUND,
                            "eval_disabled",
                            "evaluation mode is not enabled",
                        )
                    if not app.eval_attempt_allowed(ip_hash):
                        raise ApiError(
                            HTTPStatus.TOO_MANY_REQUESTS,
                            "rate",
                            "too many evaluation login attempts",
                        )
                    token = payload.get("token")
                    if not isinstance(token, str) or not hmac.compare_digest(
                        token, app.eval_token
                    ):
                        raise ApiError(
                            HTTPStatus.FORBIDDEN,
                            "eval_token",
                            "invalid evaluation token",
                        )
                    cookies = [
                        self.cookie_header("auth", app.eval_cookie_value(), 86400)
                    ]
                    self.send_json(
                        HTTPStatus.OK, {"ok": True}, cookies=cookies
                    )
                elif path == "/api/eval/reset":
                    cookies = [
                        self.cookie_header("writer", "", 0),
                        self.cookie_header("session", "", 0),
                    ]
                    self.send_json(HTTPStatus.OK, {"ok": True}, cookies=cookies)
                elif path == "/api/session":
                    if app.session_rate_limited(ip_hash):
                        raise ApiError(HTTPStatus.TOO_MANY_REQUESTS, "rate", "too many sessions")
                    challenge = payload.get("challenge")
                    nonce = payload.get("nonce")
                    if not isinstance(challenge, str) or not isinstance(nonce, str):
                        raise ApiError(HTTPStatus.BAD_REQUEST, "challenge", "challenge required")
                    app.verify_challenge(challenge, nonce)
                    writer = self.cookie("writer")
                    if writer is None or not isinstance(writer, str) or not WRITER_RE.fullmatch(writer):
                        writer = app.new_writer()
                    app.store.ensure_writer(writer, consent=False)
                    token, expires = app.store.create_session(
                        writer, ip_hash, int(app.config["session_ttl"])
                    )
                    cookies = [
                        self.cookie_header("writer", writer, 31536000),
                        self.cookie_header("session", token, int(app.config["session_ttl"])),
                    ]
                    self.send_json(
                        HTTPStatus.OK,
                        {"ok": True, "writer": writer, "expires_at": iso(expires)},
                        cookies=cookies,
                    )
                elif path == "/api/samples":
                    token = self.cookie("session")
                    if not token:
                        raise ApiError(HTTPStatus.UNAUTHORIZED, "session", "session required")
                    writer = app.store.get_session(token)
                    if not writer:
                        raise ApiError(HTTPStatus.UNAUTHORIZED, "session", "session expired")
                    language, character, width, height, strokes = validate_sample(
                        payload, app.targets
                    )
                    limited = app.sample_rate_limited(ip_hash)
                    if limited:
                        raise ApiError(HTTPStatus.TOO_MANY_REQUESTS, "rate", f"rate limit: {limited}")
                    now = utcnow()
                    record = {
                        "id": "web-" + uuid.uuid4().hex,
                        "writer": writer,
                        "language": language,
                        "character": character,
                        "width": width,
                        "height": height,
                        "strokes": json.dumps(strokes, ensure_ascii=False, separators=(",", ":")),
                        "created_at": iso(now),
                        "consent": 1,
                        "license": LICENSE,
                        "ip_hash": ip_hash,
                        "dedup": dedup_hash(language, character, strokes),
                    }
                    inserted = app.store.add_sample(record)
                    status = HTTPStatus.OK
                    self.send_json(
                        status,
                        {"ok": True, "id": record["id"], "duplicate": not inserted},
                    )
                    if inserted:
                        print(
                            f"ink-collect: accepted {record['id']} {language} "
                            f"{character} {writer}",
                            flush=True,
                        )
                else:
                    raise ApiError(HTTPStatus.NOT_FOUND, "not_found", "unknown endpoint")
            except ApiError as exc:
                print(
                    f"ink-collect: {self.command} {urlsplit(self.path).path} "
                    f"-> {exc.status} {exc.code}",
                    flush=True,
                )
                self.send_json(exc.status, {"error": exc.code, "message": exc.message})
            except Exception as exc:  # noqa: BLE001
                print(f"ink-collect: POST error: {exc.__class__.__name__}: {exc}", file=sys.stderr, flush=True)
                self.send_json(HTTPStatus.INTERNAL_SERVER_ERROR, {"error": "internal"})

        def do_OPTIONS(self) -> None:  # noqa: N802
            self.send_json(HTTPStatus.METHOD_NOT_ALLOWED, {"error": "method"})

    return Handler


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="fcitx5-ink handwriting collector")
    parser.add_argument("--config", default="/etc/ink/collect.json")
    args = parser.parse_args(argv)

    try:
        config = load_config(args.config)
        targets = load_targets(config["targets_dir"])
        store = Store(config["data_dir"] / "samples.db")
    except (OSError, ValueError, ConfigError) as exc:
        print(f"ink-collect: {exc}", file=sys.stderr)
        return 1

    app = App(config, targets, store)
    handler = make_handler(app)
    server = ThreadingHTTPServer((config["listen"], config["port"]), handler)
    server.daemon_threads = True
    print(
        f"ink-collect: listening on {config['listen']}:{config['port']} "
        f"targets={ {k: len(v) for k, v in targets.items()} }",
        flush=True,
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        store.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
