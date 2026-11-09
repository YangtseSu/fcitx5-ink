# ink-train

A Rust port of Zinnia's offline `zinnia_learn` training loop:

- reads the same S-expression character format;
- uses the same sparse feature extraction as the fcitx5-ink recognizer;
- trains one linear SVM per class with Zinnia's coordinate descent
  (C=1, epsilon=0.1, at most 2000 iterations by default);
- writes the binary Zinnia model format directly;
- keeps only weights with `|w| >= 0.001`, as the reference trainer does;
- trains classes in parallel and keeps only sparse weights in memory.

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

`--jobs N` sets the worker count; the default uses all logical CPUs. The model output
is independent of the worker count. `--limit-classes N` limits the
classes written to the model but keeps every example as a negative, so it is a
representative smoke run rather than a tiny-corpus benchmark.

The 6,763-class Tomoe baseline completes in well under a minute on a 12-core
machine; the 9,574-class zh_CN coverage model takes about one minute and is
41 MB. Measure the target machine before adding training to CI.

The implementation is validated against the model format used by
`fcitx5-ink-handwriting`; exact parity with upstream `zinnia_learn` requires a
seeded/random-order comparison and is a follow-up test.

## License

The trainer is GPL-3.0-or-later; the SVM training loop retains Zinnia's
BSD-3-Clause terms. The loop is ported from Zinnia's `svm.cpp`, Copyright (C)
2008 Taku Kudo; see `LICENSES/BSD-3-Clause.txt`.
