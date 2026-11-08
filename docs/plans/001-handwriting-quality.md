# 001: Handwriting quality program

Status: active
Date: 2026-11-08

## Goal

Raise recognition accuracy for Japanese, Simplified Chinese and Traditional
Chinese handwriting in `fcitx5-ink` without breaking the existing offline,
privacy-preserving panel.

## Non-goals

- Shipping a cloud recognizer or sending user strokes to a network service.
- Training on academic-only corpora (CASIA/ETL/ICDAR mirrors) for
  redistributable models.
- Replacing Zinnia before a reproducible evaluation set exists.

## Phase 0: data repository and inventory (started)

Deliverables:

- [x] New `fcitx5-ink-data` repository.
- [x] Commit Tomoe XML corpora (`ja`, `zh_CN`, `zh_TW`) with LGPL notices.
- [x] Pin current models and Wagomu releases in manifests.
- [x] Document the Hugging Face/ModelScope license survey.
- [x] Add a machine-readable class/sample inventory and generated report.
- [x] Source XMLs stay in Git; compiled models are fetched and checksum-verified.

Acceptance:

- `scripts/verify-data.sh` verifies every committed data file.
- `scripts/dataset-stats.py` prints classes, samples, stroke counts and script
  coverage for each language.

## Phase 1: evaluation harness

Deliverables:

- [x] A common normalization and JSONL conversion for online strokes.
- [x] Zinnia adapter using the same core crate as the panel.
- [ ] Wagomu adapter (offline evaluation only).
- [ ] Compact CNN adapter (experimental).
- [ ] A held-out set with a documented split; no within-character leakage.
- [x] Baseline report on the Tomoe training samples (sanity check only).
- [x] Latency and model-size probe for the Zinnia models.
- [x] Minimal local capture tool and target-list generator.
- [ ] A held-out report with top-1/top-5/top-12, latency and model size.

Acceptance:

- The same sample produces a comparable result from each adapter.
- Zinnia baseline numbers are reproducible from a clean checkout.
- The report records model and dataset hashes.
- Tomoe training-sample results are labelled as a sanity check, not accuracy.

## Phase 2: stroke augmentation + Zinnia retraining

Deliverables:

- Augmentation pipeline: affine, scale, rotate, resample, jitter, elastic.
- `tegaki-build`/`zinnia_learn` pipeline and model size report.
- A/B comparison against the baseline.

Acceptance:

- Top-1 improvement is measured on the held-out set.
- Model size and per-character latency are recorded.
- If improvement is below 2%, document the negative result and stop.

## Phase 3: compact online neural recognizer (ONNX)

Deliverables:

- Training pipeline using Tomoe + Make Me a Hanzi/AnimCJK.
- Per-language class filtering or separate checkpoints.
- ONNX export, int8 quantization, and a Rust inference prototype.
- License review of code, weights and training data.

Acceptance:

- ≥5 point top-1 improvement over Zinnia on the held-out set, or a documented
  decision not to ship.
- Model ≤30 MB (ideally ≤10 MB int8), CPU single-character latency ≤50 ms.
- No regression for existing candidate selection/commit flows.

## Phase 4: optional ensemble/reranker

Only if Phase 1 shows complementary errors:

- Combine Zinnia + Wagomu or Zinnia + neural model scores.
- Keep the default path unchanged; make the enhancer optional.

Acceptance:

- Top-5 gain at least 2–3 points over the best single recognizer.
- Latency and model size budget are respected.

## Risks

- **Data licensing**: CASIA/ETL/Datatang cannot be used for redistributable
  models without permission; KanjiVG is share-alike.
- **Data scarcity**: one sample per Chinese character; augmentation cannot
  invent writer style.
- **Metric leakage**: random splits of the same character/writer overstate
  accuracy.
- **Runtime complexity**: a new ONNX Runtime dependency and model package must
  be tested on Arch; keep Zinnia as fallback.
- **Reference project maturity**: `cnn_chinese_hw` is a single-author alpha in
  active refactoring; use it as a reference/training tool, not a runtime
  dependency.

## Immediate next actions

1. [x] Commit the initial repository and data.
2. [x] Implement `verify-data.sh`, `fetch-models.sh`, `dataset-stats.py`.
3. [x] Build the JSONL converter and a Zinnia evaluation adapter.
4. [x] Add a minimal capture tool and a consent-gated promotion script.
5. [ ] Collect a small self-written evaluation set (with consent) for Chinese,
   because the current Chinese corpora have no within-class holdout.
6. [ ] Add Wagomu and CNN adapters after the held-out samples exist.
