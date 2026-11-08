// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! UI-independent core for the fcitx5-ink handwriting input tool:
//! model discovery, the recognition wrapper, and the panel socket client.
//! Recognition lives in the `fcitx5-ink-handwriting` crate, which wraps the
//! unmodified upstream module under `crates/handwriting/vendor`.

pub mod config;
pub mod model;
pub mod recognize;
pub mod socket;
