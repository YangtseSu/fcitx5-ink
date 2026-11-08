# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
# Bundled handwriting models

`handwriting-model.lock.json` pins the Tegaki 0.3 Zinnia models and their
license text by URL, archive member, size, and SHA-256. The models are not
committed; `scripts/fetch-model.sh` downloads the release archives, extracts
`handwriting-ja.model`, `handwriting-zh_CN.model` and
`handwriting-zh_TW.model`, and verifies each extracted file. The Arch packages
fetch the same pinned archives.

The source is the [Tegaki 0.3 release](https://github.com/tegaki/tegaki/releases/tag/v0.3),
published in 2016; the files inside the archives carry 2009 timestamps.
The models are LGPL-2.1-or-later. Their license text is installed beside them
as `HandwritingModel-LICENSE.txt` and kept in
`LICENSES/LGPL-2.1-or-later.txt`.
