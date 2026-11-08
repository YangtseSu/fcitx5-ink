# 002: Expand character coverage with Make Me a Hanzi

Status: data preparation implemented; training pending
Date: 2026-11-08

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

Coverage:

| Model | Current | New from MMH | Current not in MMH |
|---|---:|---:|---:|
| zh_CN | 6,763 | 2,811 | 0 |
| zh_TW | 11,853 | 2,883 | 5,162 |
| ja | 6,421 | 4,545 Unicode overlap | 1,392 |

## Missing prerequisites

- **Zinnia trainer**: `zinnia_learn` is not installed and Arch does not provide
  a `zinnia` package. `tools/ink-train` now replaces it for the coverage
  experiment; upstream zinnia 0.07 is only needed for a parity comparison.
- **Training compute**: a full 9.5k-class SVM is CPU-intensive; measure time and
  model size before committing to all classes.
- **Model packaging**: the generated `.s` files and the trained model are
  derived from Arphic-licensed data; include the Arphic notice in the model data
  package.
- **Evaluation**: there is no real held-out set; a coverage smoke test can prove
  the new classes load and are emitted, but not that recognition is useful for
  real writers.

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
   can be emitted.
7. Record model size, latency and class count.
8. Optional: repeat for zh_TW after checking demand. Do not use MMH as the
   Japanese source; use AnimCJK/Tomoe for Japanese forms.

## Acceptance

- New model loads in the panel and the three-language self-test still passes.
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

Remaining before using it for a full model:

- benchmark a full ~9.5k-class run and add parallelism/optimizations if needed;
- keep an upstream parity test for a small corpus in CI;
- validate the resulting model in the panel and on real held-out strokes.

## Risks

- Canonical median trajectories do not represent writer style; accuracy on real
  writers may be poor.
- Adding 2,811 classes to zh_CN is a 42% class increase; model size and latency
  will grow.
- Rare characters/radicals may not be worth the model cost; filter by demand if
  necessary.
- This does not replace the need for a real writer-separated evaluation set.
