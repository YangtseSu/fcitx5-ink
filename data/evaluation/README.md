# Evaluation samples

`data/collected/` is the local capture directory and is ignored by Git.
Promote a consented writer's file here only after review:

```sh
python3 scripts/promote_capture.py \
  --input data/collected/zh_CN-w1.jsonl \
  --language zh_CN \
  --writer w1 \
  --license CC-BY-4.0 \
  --consent
```

The script validates the JSONL, writes
`data/evaluation/<language>/<writer>.jsonl`, and updates
`data/evaluation/manifest.json` with sample count, license, checksum and date.

Rules:

- use an anonymous writer ID (`w1`, `w2`, ...), never a name or email;
- obtain explicit consent before committing handwriting samples;
- evaluation samples use CC-BY-4.0 by default; record the chosen license in
  the manifest;
- keep evaluation writers separate from any training split;
- do not add samples that the writer wants to keep private.
