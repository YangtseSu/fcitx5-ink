# 003: Compact online neural recognizer

Status: proposed; needs data and a new work session
Date: 2026-11-08

## Goal

Replace or augment the linear Zinnia classifier with a compact online neural
recognizer that has a higher accuracy ceiling, exports to ONNX, and runs on CPU
inside the panel.

## Why this is a separate project

It changes the model format, training stack and runtime dependency. It should
not be mixed with the short-term Zinnia coverage work.

## Missing prerequisites

- **Real held-out data**: writer-separated samples for ja, zh_CN and zh_TW. The
  Tomoe Chinese corpora have one sample per character.
- **GPU**: the reference CNN trains from scratch; CPU training is impractical
  for a full run. A single CUDA GPU is the target.
- **Data license review**: Tomoe LGPL, Make Me a Hanzi/AnimCJK Arphic, KanjiVG
  CC BY-SA, user samples CC-BY-4.0. Keep license groups separate.
- **Runtime**: Arch has `onnxruntime-cpu`; the panel integration must be
  optional and keep Zinnia as fallback.
- **Reference code**: `mcyph/cnn_chinese_hw` is a useful reference but is a
  single-author alpha in refactoring; do not depend on it at runtime.

## Work

### Phase 3a: data pipeline

- Convert Tomoe, MMH and AnimCJK to the common JSONL schema.
- Render online strokes to the directMap representation (start, end, order).
- Split by writer; keep an evaluation set that never enters training.
- Record manifest hashes and license groups.

### Phase 3b: baseline model

- Train a compact CNN (reference: 48×48×3 directMap, SE-ResNet, ~3M backbone
  parameters).
- Compare with Zinnia on the same held-out samples: top-1/5/12, latency, size.
- Stop if there is no clear improvement.

### Phase 3c: real-data fine-tuning

- Fine-tune on real online samples, per language or with language-conditioned
  heads.
- Evaluate write style variation and out-of-order strokes.
- Confirm no regression in the panel's candidate flow.

### Phase 3d: ONNX integration

- Export fp32 and int8 ONNX; verify parity with PyTorch.
- Implement Rust preprocessing + ONNX Runtime inference.
- Add an optional model data package; keep Zinnia fallback.
- Measure cold start, memory, CPU latency.

## Acceptance

- ≥5 points top-1 improvement over Zinnia on the held-out set, or a documented
  decision not to ship.
- Model ≤30 MB (ideally ≤10 MB int8); single-character CPU latency ≤50 ms.
- License review passes for code, weights and training data.
- Panel still works with the neural model absent.

## Effort

Roughly 4–8 weeks, dominated by data collection, training experiments and
license review, not by the model code.

## Entry criteria for a new session

- a held-out writer-separated dataset is available;
- a GPU training environment is available;
- the license matrix is agreed;
- the short-term coverage work has a baseline model and evaluation report.
