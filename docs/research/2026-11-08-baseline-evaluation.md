# Baseline evaluation: Zinnia on Tomoe training samples

Date: 2026-11-08
Status: sanity check only

## Command

From the repository root:

```sh
python3 scripts/tomoe_to_jsonl.py
cargo run --release --manifest-path tools/eval/Cargo.toml -- \
  --data reports/tomoe.jsonl \
  --models models/zinnia \
  --language zh_CN
```

The evaluator uses the same `fcitx5-ink-core` recognizer code as the panel.
Points are clamped to the record canvas, matching the panel's
`model_point` behavior.

## Results

| Language | Samples | Top-1 | Top-5 | Top-12 |
|---|---:|---:|---:|---:|
| Simplified Chinese | 6,763 | 99.35% | 99.93% | 99.97% |
| Traditional Chinese | 11,853 | 99.41% | 99.96% | 99.97% |
| Japanese | 6,646 | 99.61% | 99.94% | 99.95% |

Raw outputs: `reports/eval-zh_CN.txt`, `reports/eval-zh_TW.txt`,
`reports/eval-ja.txt`.

## Latency and model size

500 samples per language, single-threaded CPU, including the first-call model
load:

| Language | Median | P95 | Model |
|---|---:|---:|---:|
| Simplified Chinese | 14.24 ms | 18.77 ms | 26.8 MB |
| Traditional Chinese | 29.14 ms | 38.36 ms | 52.2 MB |
| Japanese | 14.13 ms | 18.63 ms | 26.1 MB |

Raw output: `reports/latency.txt`. These are Zinnia CPU numbers; a neural ONNX
model must be compared on the same machine before it can replace Zinnia.

## How to read this

**These samples are the training data.** The scores verify that the adapter,
model paths and normalization work. They are not a generalization benchmark:
Simplified and Traditional Chinese have one sample per character, so there is
no within-class holdout.

Observations:

- Even the training samples are not 100% top-1: the linear SVM confuses
  visually similar characters such as 日/曰, 丬/干, 丶/丨.
- Top-5 and top-12 are almost perfect on training data. This suggests the
  candidate list is rarely missing the right answer; top-1 ranking is the weak
  point on these samples.
- A real benchmark needs a separate writer/style test set. Candidate sources
  and license constraints are in `2026-11-08-datasets.md`.

## Reproducibility

- data: `data/manifests/tomoe.json`
- models: `data/manifests/tegaki-models.json`
- evaluator: `tools/eval`
- recognizer code: [`fcitx5-ink`](https://github.com/YangtseSu/fcitx5-ink) at
  commit `dc13b46`
