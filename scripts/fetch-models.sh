#!/bin/sh
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
set -eu
exec python3 "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/fetch_models.py" "$@"
