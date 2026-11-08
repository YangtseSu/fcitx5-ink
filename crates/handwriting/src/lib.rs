// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Handwriting recognition: the upstream `handwriting` module from
//! `msime-engine`, wrapped with the small error and file-opening seam that its
//! module code expects. The upstream files under `vendor/` are unmodified.

mod error;
pub(crate) mod paths;

#[path = "../vendor/mod.rs"]
mod handwriting;

pub use error::{EngineError, Result};
pub use handwriting::{
    contains_cjk, extract_feature_nodes, handwriting_recognize, handwriting_recognize_ranked,
    order_handwriting_candidates, MAX_CANDIDATES, MAX_CANDIDATE_BYTES,
};
