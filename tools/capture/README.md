# Minimal capture tool

Records online strokes for a held-out evaluation set. Nothing leaves the
machine.

```sh
python3 scripts/make_capture_list.py --language zh_CN --limit 300
python3 tools/capture/capture.py \
  --language zh_CN \
  --writer w1 \
  --characters data/capture/generated/zh_CN.txt
```

Controls:

- draw on the white canvas;
- `Ctrl+S` or 保存并下一个: append the sample to
  `data/collected/<language>-<writer>.jsonl`;
- `Ctrl+Z` or 撤销: remove the last stroke;
- `Escape` or 清空: clear the canvas;
- 跳过: skip the target.

Each record uses the common JSONL shape (`id`, `language`, `character`,
`width`, `height`, `strokes`) plus `source`, `writer` and `created_at`.
Collected files are ignored by Git because they are personal writing samples.
To add a consented writer to the evaluation set, follow
`data/evaluation/README.md` and use `scripts/promote_capture.py`.
