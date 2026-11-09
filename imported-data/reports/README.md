# Reports

- `dataset-stats.md` — generated inventory of the committed Tomoe corpora.
- `eval-zh_CN.txt`, `eval-zh_TW.txt`, `eval-ja.txt` — baseline output from
  `tools/eval` against the current Zinnia models.
- `latency.txt` — 500-sample latency/size probe from `tools/eval --latency`.

The evaluation reports use the Tomoe training samples. They verify the adapter
and model loading; they are **not** a held-out generalization benchmark because
Simplified and Traditional Chinese have one sample per character. A real
benchmark needs a separate writer/style evaluation set.

Generated JSONL and training run directories are ignored.
