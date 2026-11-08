#!/bin/sh
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
model="$root/target/handwriting-model/handwriting-zh_CN.model"
panel="$root/target/debug/fcitx5-ink-panel"

if [ ! -f "$model" ]; then
  echo "missing model; run scripts/fetch-model.sh first" >&2
  exit 2
fi

"$root/scripts/build.sh"

MSIME_HANDWRITING_MODEL="$model" CARGO_TARGET_DIR="$root/target" \
  cargo test --workspace

"$panel" --self-test
"$root/tests/isolated-commit.sh"
"$root/tests/hotkey.sh"
