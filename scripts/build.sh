#!/bin/sh
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
profile=debug
cargo_flags=
if [ "${1:-}" = "--release" ]; then
  profile=release
  cargo_flags=--release
fi

CARGO_TARGET_DIR="$root/target" cargo build $cargo_flags --workspace

panel="$root/target/$profile/fcitx5-ink-panel"
model="$root/target/handwriting-model/handwriting-zh_CN.model"

if [ -f "$model" ]; then
  cmake -S "$root" -B "$root/build" -DCMAKE_BUILD_TYPE=Release \
    -DFCITX5_INK_PANEL_BINARY="$panel" \
    -DFCITX5_INK_MODEL="$model"
else
  cmake -S "$root" -B "$root/build" -DCMAKE_BUILD_TYPE=Release \
    -DFCITX5_INK_PANEL_BINARY="$panel"
fi
cmake --build "$root/build" -j"$(nproc 2>/dev/null || echo 2)"
