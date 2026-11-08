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

Coverage:

| Model | Current | New from MMH | Current not in MMH |
|---|---:|---:|---:|
| zh_CN | 6,763 | 2,811 | 0 |
| zh_TW | 11,853 | 2,883 | 5,162 |
| ja | 6,421 | 4,545 Unicode overlap | 1,392 |

## Missing prerequisites

- **Zinnia trainer**: `zinnia_learn` is not installed and Arch does not provide
  a `zinnia` package. Build zinnia 0.07 from source in a clean environment.
- **Training compute**: a full 9.5k-class SVM is CPU-intensive; measure time and
  model size before committing to all classes.
- **Model packaging**: the generated `.s` files and the trained model are
  derived from Arphic-licensed data; include the Arphic notice in the model data
  package.
- **Evaluation**: there is no real held-out set; a coverage smoke test can prove
  the new classes load and are emitted, but not that recognition is useful for
  real writers.

## Work

1. Build zinnia 0.07 and verify `zinnia_learn` on a tiny model.
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
4. Train a model that combines `zh_CN.s` + `zh_CN-new.s`.
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

Zinnia's last upstream commit is from 2016 (about ten years old at the time of
writing). The runtime used by fcitx5-ink is already a Rust port; only the batch
trainer is old.

Do not rewrite the trainer before the coverage experiment. First build and pin
upstream zinnia 0.07 (`zinnia_learn`) in a clean environment. Rewrite only if:

- it no longer builds on a supported toolchain;
- we need sample weights, parallel training or reproducible CI training;
- we need to generate the same model format without a C++ build dependency.

A future Rust trainer can reuse the vendored feature extraction and emit the
same Zinnia model format, but it must be validated against `zinnia_learn` on a
small corpus. That is a separate project, not part of this coverage change.

## Risks

- Canonical median trajectories do not represent writer style; accuracy on real
  writers may be poor.
- Adding 2,811 classes to zh_CN is a 42% class increase; model size and latency
  will grow.
- Rare characters/radicals may not be worth the model cost; filter by demand if
  necessary.
- This does not replace the need for a real writer-separated evaluation set.
