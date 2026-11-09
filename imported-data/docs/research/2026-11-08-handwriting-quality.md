# Improving handwriting recognition quality

Date: 2026-11-08
Status: research notes / baseline

## 1. Current pipeline

`fcitx5-ink` uses one Zinnia model per language:

| Language | Model | Classes | Training samples | Model size |
|---|---|---:|---:|---:|
| Simplified Chinese | `handwriting-zh_CN.model` | 6,763 | 6,763 | 26.8 MB |
| Traditional Chinese | `handwriting-zh_TW.model` | 11,853 | 11,853 | 52.2 MB |
| Japanese | `handwriting-ja.model` | 6,421 | 6,646 | 26.1 MB |

The sample counts come from the Tegaki 0.3 XML corpora. Simplified and
Traditional Chinese have **one sample per character**. Japanese has one sample
for most characters and two to four for 217 characters. This is the dominant
quality limit, not the inference code.

Zinnia uses online stroke features and a linear one-vs-rest SVM per class. The
official project states that it provides a training module and that more data
yields better quality:
<https://taku910.github.io/zinnia/>.

The current `fcitx5-ink` recognizer exposes candidates in model score order for
Japanese and applies the upstream Chinese candidate policy for Chinese.

## 2. What "train the model" can mean

There are three different targets, and they require different work:

1. **Retrain the existing Zinnia `.model` files.** Only online stroke samples
   in Zinnia's S-expression format can be used. Image models cannot produce
   Zinnia weights directly; they can only help produce or clean stroke data.
2. **Train a new online neural model.** Use the same stroke input, but a compact
   CNN/Transformer, exported to ONNX. This can use the same online corpora plus
   synthetic stroke data. It is a new runtime and model format.
3. **Train an offline image model.** Render strokes to images, or train on
   scanned handwriting. This is a different input distribution and is only
   appropriate if the product moves to image input.

The realistic options are below.

## 3. Options

### A. Stroke augmentation + Zinnia retraining

Apply affine transforms, scaling, rotation, stroke resampling, jitter and
elastic distortions to the existing online samples, then retrain Zinnia.

- Cost: low (days).
- Expected gain: robustness to position/size/small distortions; probably a few
  points at most.
- Risk: augmentation does not add new writing styles; model size can grow with
  support vectors.
- Verdict: do this first as a baseline and to validate the evaluation harness.

### B. Large model as data cleaner / reranker

Render each stroke sample to an image; use a local OCR/VLM to verify the label
or to rerank Zinnia's candidates. Keep only high-confidence corrections.

- Cost: medium (1–2 weeks).
- Expected gain: depends on label noise; likely small for a clean corpus.
- Risk: general VLMs are weak on Chinese handwriting. The GPT-4V OCR benchmark
  reports page-level character recognition rate 36.54% and line-level CER
  11.85% on CASIA-HWDB:
  <https://github.com/SCUT-DLVCLab/GPT-4V_OCR/blob/main/results/htr/results_htr.md>.
  A VLM must not be treated as ground truth.
- Verdict: useful as a weak signal, not a primary teacher.

### C. Offline-to-online derendering

Use an image-to-stroke model to convert offline handwriting images into online
trajectories, then train an online recognizer.

- Cost: high (weeks).
- Potential: offline corpora are much larger; InkSight reports 87% valid
  tracing and 67% human-like pen trajectories in human evaluation:
  <https://arxiv.org/abs/2402.05804>.
- Risk: stroke order and direction are not guaranteed. Zinnia features are
  sensitive to both; noisy trajectories can hurt. Offline corpora with usable
  redistribution licenses are rare.
- Verdict: a data-expansion research project, only after a neural online model
  exists and the licensing is settled.

### D. Compact online neural recognizer

Train a small model on online stroke trajectories and run it with ONNX Runtime.
A reference implementation is `mcyph/cnn_chinese_hw`, which renders strokes into
a 48×48×3 directMap (start, end, stroke order) and classifies with a compact
SE-ResNet (~2.9M backbone parameters; 6.7M total parameters with the
15,146-class head; ONNX export available).

- Cost: high (4–8 weeks including data and evaluation).
- Potential: higher accuracy ceiling than a linear SVM; compact int8 model
  (~7 MB) and low CPU latency.
- Risk: data licenses, per-language class filtering, new runtime dependency,
  and the reference project is a single-author alpha in active refactoring.
- Verdict: the only path likely to produce a large, durable accuracy gain.

### E. Runtime VLM / OCR reranker

Render the ink to an image and ask a local VLM to choose among candidates.

- Cost: medium.
- Risk: 7B models are too heavy for an input method; cloud models break offline
  use and privacy; gains are unproven.
- Verdict: optional “high accuracy” mode at most, not the default.

### F. User data collection

Collect real online strokes with explicit consent, local storage and deletion.

- Cost: medium ongoing.
- Potential: highest long-term value; covers actual users' styles.
- Risk: privacy, consent, review, and distribution terms.
- Verdict: the best long-term data source if the project can support it.

## 4. Recommended order

1. Build a reproducible evaluation set and measure the current Zinnia models
   (top-1/top-5/top-12, per language, latency).
2. Run A (augmentation) and measure; stop if the gain is below 2%.
3. Run B on a small sample; use it only for label cleaning and candidate
   reranking experiments.
4. If a large gain is required, build D (compact online neural model) with
   Tomoe + Make Me a Hanzi/AnimCJK and export ONNX.
5. Consider C only as a later data-expansion step, after D and after licenses
   are resolved.

## 5. Evaluation protocol

The current Chinese corpora cannot provide a within-class holdout because each
character has only one sample. A separate evaluation set is mandatory.

Minimum protocol:

- one split per language;
- top-1, top-5, top-12;
- per-script/per-language reporting;
- CPU latency per character and model size;
- same stroke preprocessing for every recognizer;
- record the exact model hash and dataset hash.

Candidate evaluation sources:

- **Tomoe/Japanese**: 217 Japanese characters have multiple samples; usable for
  a small within-corpus check.
- **CASIA-OLHWDB**: academic-only; do not redistribute models trained on it
  without permission.
- **Self-collected**: a few writers × a few hundred characters, with consent.
- **Synthetic**: Make Me a Hanzi/AnimCJK for character coverage, not writer
  style.

## 6. Non-goals

- Training on CASIA/ETL/Datatang mirrors without a commercial license.
- Mixing CC BY-SA and GPL-only training data in one model without a license
  review.
- Adding a cloud VLM to the default input path.
- Replacing Zinnia before an evaluation harness exists.

## 7. References

- Zinnia: <https://taku910.github.io/zinnia/>
- Tegaki: <https://tegaki.github.io/>
- `cnn_chinese_hw`: <https://github.com/mcyph/cnn_chinese_hw>
- InkSight: <https://arxiv.org/abs/2402.05804>
- GPT-4V OCR benchmark: <https://github.com/SCUT-DLVCLab/GPT-4V_OCR>
- CASIA online/offline handwriting databases:
  <https://nlpr.ia.ac.cn/databases/handwriting/Home.html>
