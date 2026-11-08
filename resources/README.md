# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
# Bundled handwriting model

`handwriting-model.lock.json` pins the Zinnia model and its license text by
URL, size, and SHA-256. The model itself is not committed; `scripts/fetch-model.sh`
downloads and verifies it, and the Arch package takes it from the same pinned
URL.

The model is LGPL-2.1-or-later. Its license text is
`LICENSES/LGPL-2.1-or-later.txt` and is installed beside the model.
