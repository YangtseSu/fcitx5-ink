#!/bin/sh
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
model_dir="$root/target/handwriting-model"
model="$model_dir/handwriting-zh_CN.model"
panel="$root/target/debug/fcitx5-ink-panel"

for name in handwriting-zh_CN.model handwriting-zh_TW.model handwriting-ja.model; do
  if [ ! -f "$model_dir/$name" ]; then
    echo "missing $name; run scripts/fetch-model.sh first" >&2
    exit 2
  fi
done

"$root/scripts/build.sh"

MSIME_HANDWRITING_MODEL="$model" \
  FCITX5_INK_ZH_CN_MODEL="$model_dir/handwriting-zh_CN.model" \
  FCITX5_INK_ZH_TW_MODEL="$model_dir/handwriting-zh_TW.model" \
  FCITX5_INK_JA_MODEL="$model_dir/handwriting-ja.model" \
  CARGO_TARGET_DIR="$root/target" \
  cargo test --workspace

for language in zh_CN zh_TW ja; do
  "$panel" --self-test --language "$language"
done
ctest --test-dir "$root/build" --output-on-failure
"$root/tests/isolated-commit.sh"
"$root/tests/group-no-panel.sh"
"$root/tests/hotkey.sh"
