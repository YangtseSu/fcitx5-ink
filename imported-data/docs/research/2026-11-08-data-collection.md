# Web capture and synthetic training data

Date: 2026-11-08
Status: feasibility notes

## 1. Can we run a public handwriting collection website?

Yes. The technical part is straightforward; the hard parts are quality,
consent, privacy and keeping contributors.

### Precedent

Google's Quick, Draw! collected 50 million drawings as timestamped vectors and
released them under CC BY 4.0. It proves that browser-based stroke collection
works at scale. Our case is smaller and needs labelled characters, not
free-form sketches.

### Minimal architecture

```text
static page (HTML + Canvas + Pointer Events)
  -> one target character at a time
  -> anonymous writer/session id
  -> consent checkbox + license notice
  -> POST /samples (JSONL)
       -> small backend (FastAPI/Worker)
       -> append-only storage + rate limit
       -> periodic review and promotion
```

Do not use a static-only host for collection: GitHub Pages cannot store
submissions. A static page can still be useful as a prototype that downloads a
local JSONL file for manual review.

### Quality controls

- show the target character and ask the writer to copy it;
- validate stroke count against a canonical source such as Make Me a Hanzi;
- reject empty/single-point strokes and duplicate submissions;
- allow several samples per character and writer;
- rate-limit by session/IP without retaining IPs long-term;
- use a few hidden gold prompts or manual spot checks to detect bots;
- record device/pointer type only if needed, not personal data.

### Consent and licenses

- explicit consent before submission, with a clear license (CC-BY-4.0 or CC0);
- anonymous writer IDs, no names or emails;
- a privacy note: what is stored, why, retention, deletion contact;
- do not publish samples from writers who did not consent;
- keep evaluation and training consent separate if the intended use differs.

The existing `promote_capture.py` workflow already supports consent-gated
promotion into `data/evaluation` with a manifest.

### Practical volume

A public page needs promotion and a reason for people to contribute. A small
invited group (3–5 writers × 300 characters) is more useful for a first
benchmark than an unfocused public launch.

## 2. Can we generate data from fonts + generic stroke order?

Partially. The useful input is not a font outline plus an abstract stroke
order; it is a **per-stroke trajectory**. Existing projects already combine
open fonts, stroke order and geometry:

| Source | Coverage | License | What it gives |
|---|---|---|---|
| Make Me a Hanzi `graphics.txt` | ~9.5k simplified/traditional characters | Arphic Public License | stroke medians / trajectories |
| AnimCJK | Japanese and Traditional Chinese glyphs | Arphic Public License | stroke order, outlines, medians |
| KanjiVG | Japanese kanji | CC BY-SA 3.0 | stroke-order SVG paths |
| Tomoe / Tegaki XML | zh_CN/zh_TW/ja online samples | LGPL-2.1-or-later | real online strokes |

A generic stroke-order table plus an open font is theoretically enough, but
mapping each ordered stroke to the corresponding part of the font outline is
the hard part. Make Me a Hanzi and AnimCJK already solve that mapping for
their coverage; re-implementing it is not a good first step.

### What synthetic data is good for

- character coverage for rare characters;
- pretraining a neural model;
- data augmentation (affine, elastic, resampling, stroke width);
- regression tests for the rendering/training pipeline.

### What it is not good for

- measuring real-user accuracy;
- representing writer style, pen pressure, speed or correction behaviour;
- replacing a held-out set of real writers.

Synthetic samples are close to canonical; training only on them overfits the
canonical shape. Use them as auxiliary pretraining or augmentation, then
fine-tune and evaluate on real online strokes.

### License groups

Keep share-alike data separate:

- Make Me a Hanzi / AnimCJK: Arphic Public License;
- KanjiVG: CC BY-SA 3.0;
- Tomoe: LGPL-2.1-or-later;
- contributor samples: the license agreed on the site (CC-BY-4.0 or CC0).

Do not mix CC BY-SA data into a GPL-only model without a license review.

## 3. Recommendation

1. Build a small **invite-only** web capture page first, with local JSONL
   download and the existing promotion script. This avoids backend/abuse work
   while validating the prompt and consent flow.
2. If the response is good, add a minimal append-only backend with rate limits
   and a privacy policy.
3. For synthetic data, use Make Me a Hanzi/AnimCJK medians, not a new
   font-to-stroke mapper. Treat it as pretraining/augmentation only.
4. Keep a real, writer-separated evaluation set as the source of truth.
