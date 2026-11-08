# ink-train

A Rust port of Zinnia's offline `zinnia_learn` training loop:

- reads the same S-expression character format;
- uses the same sparse feature extraction as the fcitx5-ink recognizer;
- trains one linear SVM per class with Zinnia's coordinate descent
  (C=1, epsilon=0.1, at most 2000 iterations by default);
- writes the binary Zinnia model format directly;
- keeps only weights with `|w| >= 0.001`, as the reference trainer does.

It is a training tool, not part of the panel runtime.

The trainer resolves `fcitx5-ink` through a path dependency. Clone both
repositories under the same parent:

```text
<parent>/fcitx5-ink
<parent>/fcitx5-ink-data
```

```sh
cargo run --release --manifest-path tools/ink-train/Cargo.toml -- \
  --out models/zinnia/zh_CN-coverage.model \
  models/tomoe/zh_CN.s models/mmh/zh_CN-new.s
```

Use `--limit-classes` for a smoke run. A full 9.5k-class run is CPU-heavy;
measure it before using it in CI.

The implementation is validated against the model format used by
`fcitx5-ink-handwriting`; exact parity with upstream `zinnia_learn` requires a
seeded/random-order comparison and is a follow-up test.

## License

The trainer is GPL-3.0-or-later; the SVM training loop retains Zinnia's
BSD-3-Clause terms. The loop is ported from Zinnia's `svm.cpp`, Copyright (C)
2008 Taku Kudo; see `LICENSES/BSD-3-Clause.txt`.
