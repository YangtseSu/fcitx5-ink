# 002: Expand character coverage with Make Me a Hanzi

Status: full zh_CN coverage model trained and smoke-evaluated; real-writer
evaluation and packaging pending
Date: 2026-11-08; updated: 2026-11-09

## Goal

Add characters that the current Zinnia models cannot output, using Make Me a
Hanzi median trajectories as a synthetic training source. This is a coverage
project, not an accuracy project for existing common characters.

## Current progress

- [x] Pin Make Me a Hanzi `graphics.txt` and its license in
  `data/manifests/makemeahanzi.json`.
- [x] Add `scripts/fetch_mmh.py` with size and SHA-256 verification.
- [x] Add `scripts/tomoe_to_zinnia.py` and `scripts/mmh_to_zinnia.py`.
- [x] Reproduce the coverage numbers with `scripts/mmh_coverage.py`.
- [x] Generate a zh_CN new-class set (2,811 characters) locally.
- [x] Generate a second canonical sample for the existing 6,763 zh_CN classes
  with `--only-xml` (local, ignored).
- [x] Implement `tools/ink-train`, a Rust port of `zinnia_learn` (same
  features, coordinate descent and binary model format).
- [x] Train classes in parallel, keep sparse weights in memory and write the
  model without a dense per-class vector.
- [x] Train the full 6,763-class baseline and the 9,574-class zh_CN coverage
  model.
- [x] Pass the panel zh_CN self-test and check that new classes are emitted
  from their MMH samples.

Coverage:

| Model | Current | New from MMH | Current not in MMH |
|---|---:|---:|---:|
| zh_CN | 6,763 | 2,811 | 0 |
| zh_TW | 11,853 | 2,883 | 5,162 |
| ja | 6,421 | 4,545 Unicode overlap | 1,392 |

## Missing prerequisites

- **Zinnia trainer**: `zinnia_learn` is not installed and Arch does not provide
  a `zinnia` package. `tools/ink-train` replaces it for the coverage
  experiment; upstream zinnia 0.07 is only needed for a parity comparison.
- **Training compute**: resolved for zh_CN. The 9,574-class model trains in
  about one minute on 12 cores with sparse weights.
- **Model packaging**: the generated `.s` files and the trained model are
  derived from Arphic-licensed data; include the Arphic notice in the model data
  package.
- **Evaluation**: there is no real held-out set; the MMH check proves the new
  classes load and are emitted, but not that recognition is useful for real
  writers.

## Work

1. Use `tools/ink-train` on a tiny corpus; optionally build zinnia 0.07 and
   compare the upstream and Rust model outputs.
2. Generate Tomoe S-expression files:
   ```sh
   ./scripts/tomoe_to_zinnia.py --xml data/tomoe/handwriting-zh_CN.xml \
     --out models/tomoe/zh_CN.s
   ```
3. Generate only new classes for zh_CN:
   ```sh
   ./scripts/mmh_to_zinnia.py \
     --exclude-xml data/tomoe/handwriting-zh_CN.xml \
     --out models/mmh/zh_CN-new.s
   ```
4. Train a model that combines `zh_CN.s` + `zh_CN-new.s`:
   ```sh
   cargo run --release --manifest-path tools/ink-train/Cargo.toml -- \
     --out models/zinnia/zh_CN-coverage.model \
     models/tomoe/zh_CN.s models/mmh/zh_CN-new.s
   ```
5. A/B experiment for a second canonical sample per existing class:
   ```sh
   ./scripts/mmh_to_zinnia.py --only-xml data/tomoe/handwriting-zh_CN.xml \
     --out models/mmh/zh_CN-second.s
   ```
   Train Tomoe-only vs Tomoe + MMH-second and compare model size, latency and
   real-script behavior. This is not a second human writing style; it is a
   font-derived second trajectory.
6. Smoke test in `fcitx5-ink-panel`: common characters still work, new classes
   can be emitted. (Done for zh_CN.)
7. Record model size, latency and class count. (Done for zh_CN.)
8. Optional: repeat for zh_TW after checking demand. Do not use MMH as the
   Japanese source; use AnimCJK/Tomoe for Japanese forms.
9. Collect writer-separated samples and compare the coverage model against the
   baseline on those samples.

## Measured zh_CN run

On a 12-core machine with `--jobs 12 --max-iterations 2000`:

| Model | Classes | Model size | Wall time |
|---|---:|---:|---:|
| Tomoe baseline | 6,763 | 26.8 MB | 28.7 s |
| Tomoe + MMH new | 9,574 | 41.5 MB | 69.1 s |

The sparse and earlier dense implementations produced byte-identical models.
`--limit-classes 100` with all 6,763 negatives took 2.9 s on one thread and
0.5 s on 12 threads at 200 iterations. A parallel determinism test compares
one-worker and four-worker models byte for byte.

Evaluation:

| Data | Model | Top-1 | Top-5 | Top-12 | Median ms | P95 ms |
|---|---|---:|---:|---:|---:|---:|
| 2,811 MMH new classes | coverage | 98.83% | 99.54% | 99.86% | 31.03 | 39.33 |
| 2,811 MMH new classes | baseline | 0.00% | 0.00% | 0.00% | 20.19 | 24.88 |
| first 500 Tomoe zh_CN | coverage | 98.40% | 99.60% | 99.80% | 21.93 | 29.17 |
| first 500 Tomoe zh_CN | baseline | 98.80% | 99.60% | 99.80% | 14.69 | 18.32 |

The panel zh_CN self-test passes with the coverage model. The MMH check is a
coverage test, not a held-out generalization benchmark: the model was trained
on those exact trajectories. The first 500 Tomoe records are also training
samples; they only show that existing classes still work and quantify the
latency/accuracy cost of the larger model.

## Acceptance

- New model loads in the panel and the zh_CN self-test passes; the zh_TW and ja
  models are unchanged.
- New classes are emitted for their MMH canonical samples.
- Model size and latency are recorded; an increase above the agreed budget is
  documented before shipping.
- Arphic attribution and license notice are included.

## Trainer decision

Zinnia's last upstream commit is from 2016 (about ten years old). The runtime
used by fcitx5-ink is already a Rust port; only the batch trainer was old.

We now have a Rust trainer in `tools/ink-train`. It reads the official
S-expression shape, reuses `extract_feature_nodes` from the recognizer, runs
the same one-vs-rest coordinate descent (C=1, epsilon=0.1) and writes the
binary model format. A 100-class smoke run and a two-class end-to-end test pass;
a two-class model trained by upstream `zinnia_learn` and by the Rust trainer
produced the same top-1 answers.

Remaining:

- keep an upstream parity test for a small corpus in CI;
- validate the resulting model on real held-out strokes.

## Risks

- Canonical median trajectories do not represent writer style; accuracy on real
  writers may be poor.
- Adding 2,811 classes to zh_CN is a 42% class increase; model size and latency
  will grow.
- Rare characters/radicals may not be worth the model cost; filter by demand if
  necessary.
- This does not replace the need for a real writer-separated evaluation set.
