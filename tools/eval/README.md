# Zinnia evaluation adapter

Reads `reports/tomoe.jsonl` and calls the same recognizer crate used by
`fcitx5-ink` on the normalized strokes.

The path dependency expects the sibling checkout:

```text
~/projects/handwriting/fcitx5-ink
~/projects/handwriting/fcitx5-ink-data
```

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
