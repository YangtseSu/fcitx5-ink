# Handwriting sample collector

A small web service for collecting online handwriting strokes for
[fcitx5-ink](../../README.md). The browser records strokes, not images, in the
same JSONL shape as `tools/capture`:

```json
{"id":"web-…","source":"web","language":"zh_CN","writer":"w-…",
 "character":"永","width":420,"height":420,
 "strokes":[[[12.3,40.1],[15.0,44.2]]],"created_at":"…"}
```

The site is a real project page: it shows the target character, records the
strokes, submits them anonymously, and shows aggregate coverage. It is not a
proxy cover and does not mention the proxy.

## Data use

- Samples train and evaluate the handwriting recognizer. Keep a writer-level
  split: a writer used for training must not be added to the held-out
  evaluation set in `data/evaluation/`.
- Writers consent explicitly on the page. Samples are released under
  CC BY 4.0; writer IDs are random and anonymous.
- The service stores no name, email, raw IP or user agent. It stores a salted
  IP hash only for rate limiting, and nginx access logging is off.
- No accounts, no public gallery. Review an export before using it.

## Layout

```text
tools/collect/
  server.py            API service (Python stdlib only)
  admin.py             stats / export / backup / delete CLI
  test_server.py       unit tests
  targets/             300 target characters per language
  web/                 static page served by nginx
  deploy/              systemd, nginx and config templates
```

Targets are generated from the Tomoe corpora:

```sh
for lang in zh_CN zh_TW ja; do
  python3 scripts/make_capture_list.py --language "$lang" --limit 300 \
    --seed 1 --out "tools/collect/targets/$lang.txt"
done
```

## Character pools

`targets/` has two pools per language:

- `targets/train/<lang>.txt` — the training pool used by the site. It is the
  official common-character set, ordered by Unicode Unihan `kFrequency`:
  zh_CN 3,500 (义务教育语文课程标准 常用字表 2500 + 1000, i.e. 通用规范汉字表
  一级字, read from the HearWrite assets under GPL-3.0-or-later); zh_TW 4,808
  (臺灣《常用國字標準字體表》, zispace/hanzi-chars); ja 2,136 (日本《常用漢字表》,
  zispace/hanzi-chars) plus the kana in the Tomoe model. Characters outside the
  Tomoe model classes are dropped. The files are 3,498 / 4,756 / 2,184 chars.
- `targets/eval/<lang>.txt` — 300 random Tomoe characters (seed 1), held out
  for writer-independent evaluation. Evaluation writers must not be used for
  training; the site's training writers do not enter this set.

The site samples a random 20-character round from the training pool
(`BATCH_SIZE` in `web/app.js`); "再来一轮" draws a new batch. Coverage in the
statistics is measured against the training pool.

The frequency table used by HearWrite (`scripts/data/xiandaihanyuchangyongcibiao.txt`,
《现代汉语常用词表（草案）》) is **not** copied: its upstream
`liangqi/chinese-frequency-word-list` repository has no license. The Unihan
`kFrequency`/`kGradeLevel` data is under the Unicode License V3
(`LICENSES/Unicode-3.0.txt`). Rebuild the pools with the sources:

```sh
python3 tools/collect/make_pools.py \
  --zh-tw /path/to/臺灣常用國字表.txt \
  --ja /path/to/日本常用漢字表.txt \
  --unihan /path/to/Unihan.zip
```

The error lists in `reports/eval-*.txt` are within-corpus errors, not held-out
generalization; do not use them as the pool's error set. Add characters where a
new-writer evaluation actually fails once such a run exists.

## Local run

```sh
cp deploy/collect.example.json /tmp/ink-collect.json
sed -i "s/REPLACE_WITH_RANDOM_SECRET/$(openssl rand -hex 32)/" /tmp/ink-collect.json
mkdir -p /tmp/ink-data
python3 - <<'PY'
import json
p = "/tmp/ink-collect.json"
c = json.load(open(p))
c["data_dir"] = "/tmp/ink-data"
c["targets_dir"] = "tools/collect/targets"
json.dump(c, open(p, "w"))
PY
python3 tools/collect/server.py --config /tmp/ink-collect.json
```

The API listens on `127.0.0.1:8081`. Serve `web/` with any static server for a
local UI, or point nginx at it.

Run the tests:

```sh
cd tools/collect && python3 -m unittest -v test_server.py
```

## API

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/health` | liveness |
| GET | `/api/targets?lang=zh_CN` | target characters |
| GET | `/api/stats` | per-language samples, writers, covered characters |
| GET | `/api/challenge` | one-time proof-of-work challenge |
| GET | `/api/session` | `{"ok": true}` when the session cookie is valid |
| POST | `/api/session` | verify the proof of work, set writer and session cookies |
| POST | `/api/samples` | validate and store one sample |

`POST /api/samples` requires a session cookie and a body like:

```json
{"language":"zh_CN","character":"永","width":420,"height":420,
 "strokes":[[[1,2],[3,4]]],"consent":true}
```

The service assigns the ID, writer, timestamp and license. It rejects unknown
targets, out-of-range points, missing consent, oversized bodies, duplicates and
rate-limited clients. Limits: 300 samples/hour and 1000/day per hashed IP,
5000/day globally, 30 sessions/hour per IP.

## Admin

```sh
# numbers as JSON
python3 tools/collect/admin.py --config /etc/ink/collect.json stats

# per-writer JSONL plus a manifest, for review and training
python3 tools/collect/admin.py --config /etc/ink/collect.json \
  export --out /tmp/ink-export

# SQLite backup plus an export, pruning after 14 days
python3 tools/collect/admin.py --config /etc/ink/collect.json \
  backup --dir /srv/ink/backup --keep 14

# spot-check recent samples (no IPs printed)
python3 tools/collect/admin.py --config /etc/ink/collect.json recent --limit 20

# honor a deletion request
python3 tools/collect/admin.py --config /etc/ink/collect.json \
  delete-writer --writer w-0123456789ab --yes
```

Training records are online strokes. Convert an export to the trainer's
Zinnia format with `to_zinnia.py`, and keep writer-level splits:

```sh
python3 tools/collect/to_zinnia.py --input /tmp/ink-export \
  --language zh_CN --out /tmp/web-zh_CN.s
```

## Evaluation mode (maintainer)

`/eval` is a separate service and database for the held-out 300-character
pool; the public site stays on the training pool. The eval service uses
`eval-collect.example.json` (`cookie_prefix: ink_eval_`, its own `eval_token`
and `data_dir`), the nginx `/eval-api/` proxy, and `targets/eval`.

```sh
python3 /srv/ink/app/admin.py --config /etc/ink/eval-collect.json \
  export --out /tmp/ink-eval-export
```

The evaluation writer is kept separate from training by both the cookie prefix
and the separate database. Use a fresh browser for each evaluation writer, or
press "换一位书写者" to clear the eval writer/session.

## Deployment

See [deploy/README.md](deploy/README.md). The service runs as an unprivileged
system user, binds to localhost, and is exposed by nginx with a Let's Encrypt
certificate.

## License

GPL-3.0-or-later for the code; CC BY 4.0 for collected samples.
