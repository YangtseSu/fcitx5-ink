# Data and training

Research, training data and evaluation tooling for the handwriting recognizer
used by `fcitx5-ink`. The data lives in the same repository as the addon and
panel, but the `fcitx5-ink-data` Arch package still ships only the models:

- source handwriting corpora and manifests;
- fetchers and checksum verification for models;
- dataset statistics and evaluation harnesses;
- training experiments and their results;
- research notes and plans for improving recognition accuracy.

## Layout

```text
docs/
  data.md         this document
  research/       current findings, dataset/license survey, wagomu notes
  plans/          implementation plans
data/
  tomoe/          online stroke corpora: ja, zh_CN, zh_TW (LGPL-2.1-or-later)
  manifests/      pinned URLs, sizes and SHA-256 for data and models
  capture/        generated target-character lists; ignored
  collected/      local captured samples; ignored by Git
  evaluation/     consented writer samples and manifest
models/           fetched and trained models; ignored by Git
scripts/          fetch, verify, convert, evaluate, make capture lists
tools/
  ink-train/      offline Zinnia trainer
  eval/           Zinnia evaluation adapter
  capture/        local capture UI
reports/          generated statistics and evaluation results
experiments/      training configurations and run records
```

Run the commands from the repository root.

Capture a held-out evaluation sample locally:

```sh
python3 scripts/make_capture_list.py --language zh_CN --limit 300
python3 tools/capture/capture.py --language zh_CN --writer w1 \
  --characters data/capture/generated/zh_CN.txt
```

Collected samples stay under `data/collected/` and are ignored by Git. To add
a consented writer to the repository:

```sh
python3 scripts/promote_capture.py \
  --input data/collected/zh_CN-w1.jsonl \
  --language zh_CN --writer w1 --license CC-BY-4.0 --consent
```

This writes `data/evaluation/<language>/<writer>.jsonl` and updates
`data/evaluation/manifest.json`. Evaluation samples use CC-BY-4.0 by default;
see `data/evaluation/NOTICE.md`. Use anonymous writer IDs and keep evaluation
writers out of training. Generated target lists are not committed; collected
samples are committed only after consent.

## Quick start

```sh
./scripts/verify-data.sh
./scripts/fetch-models.sh
./scripts/dataset-stats.py

# short-term coverage work
./scripts/fetch_mmh.py
./scripts/mmh_coverage.py
./scripts/tomoe_to_zinnia.py --xml data/tomoe/handwriting-zh_CN.xml \
  --out models/tomoe/zh_CN.s
./scripts/mmh_to_zinnia.py \
  --exclude-xml data/tomoe/handwriting-zh_CN.xml \
  --out models/mmh/zh_CN-new.s
./scripts/mmh_to_zinnia.py \
  --only-xml data/tomoe/handwriting-zh_CN.xml \
  --out models/mmh/zh_CN-second.s

# offline trainer (Rust port of zinnia_learn)
cargo run --release --manifest-path tools/ink-train/Cargo.toml -- \
  --out models/zinnia/zh_CN-coverage.model \
  models/tomoe/zh_CN.s models/mmh/zh_CN-new.s
```

The trainer and evaluator are normal Cargo workspace members. The trainer uses
`crates/features`; the evaluator uses the runtime recognizer. No sibling
checkout is required.

`dataset-stats.py` parses the three Tomoe XML corpora and writes
`reports/dataset-stats.md`. No third-party packages are required.

## Data policy

Source data is committed when it is small enough and licensed for
redistribution. Large compiled models are downloaded by script into
`models/` and verified against `data/manifests/`; they are not committed.

The Tomoe XML corpora are LGPL-2.1-or-later; Make Me a Hanzi medians are
Arphic-1999. The compiled Zinnia and Wagomu
models retain their own licenses; the model license text is installed with the
model data package. Do not mix share-alike corpora (for example KanjiVG,
CC BY-SA) into a model whose license cannot carry the same terms.

## Research status

- Current recognizer: per-language Zinnia models from Tegaki 0.3.
- Data bottleneck: zh_CN and zh_TW each have one sample per character.
- Recommended path: build a held-out evaluation set, try stroke augmentation,
  then train a compact online neural recognizer and export it to ONNX.
- High-level model replacement is not a drop-in task; see
  `docs/research/2026-11-08-handwriting-quality.md`.

## Acknowledgements

- [Tegaki](https://github.com/tegaki/tegaki) for the Tomoe/Zinnia models and
  the Wagomu recognizer.
- [Zinnia](https://github.com/taku910/zinnia) by Taku Kudo for the original
  online recognizer and model format.
- [msime](https://github.com/metasequoiaime/msime) and
  [msime-engine](https://github.com/metasequoiaime/msime-engine) for the Rust
  handwriting module used by fcitx5-ink.

## License

Scripts, documentation and original files are GPL-3.0-or-later.
`tools/ink-train` and `crates/features` retain Zinnia's BSD-3-Clause terms;
see `LICENSES/BSD-3-Clause.txt`. Committed Tomoe data is LGPL-2.1-or-later;
Make Me a Hanzi medians are Arphic-1999. See `data/tomoe/COPYING`,
`data/manifests/makemeahanzi.json` and `REUSE.toml`.
