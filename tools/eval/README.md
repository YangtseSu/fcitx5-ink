# Zinnia evaluation adapter

Reads `reports/tomoe.jsonl` and calls the same recognizer crate used by
`fcitx5-ink` on the normalized strokes.

The path dependency expects the sibling checkout:

```text
<parent>/fcitx5-ink
<parent>/fcitx5-ink-data
```

Clone `fcitx5-ink` next to this repository; the parent directory name does not
matter.

Run from the data repository root:

```sh
python3 scripts/tomoe_to_jsonl.py
cargo run --release --manifest-path tools/eval/Cargo.toml -- \
  --data reports/tomoe.jsonl \
  --models models/zinnia
```

**This is not a generalization benchmark.** The Chinese Tomoe corpora have one
sample per character, so these scores mostly confirm that the adapter and model
paths work. A held-out set is still required for accuracy claims.

For captured samples (same JSONL shape, with `source: "capture"` and `writer`):

```sh
cargo run --release --manifest-path tools/eval/Cargo.toml -- \
  --data data/collected/zh_CN-w1.jsonl \
  --models models/zinnia \
  --language zh_CN \
  --writer w1 \
  --latency
```

Keep evaluation writers out of any training split.
