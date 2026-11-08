// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

/// A 420 by 420 virtual canvas. The vendored recognizer normalizes each
/// stroke's bounding box, so the drawable size only needs to stay stable while
/// a character is being written.
pub const CANVAS: f32 = 420.0;

pub fn recognize(model: &Path, strokes: &[Vec<(f32, f32)>]) -> Result<Vec<String>, String> {
    let model = model
        .to_str()
        .ok_or_else(|| "model path is not valid UTF-8".to_owned())?;
    fcitx5_ink_handwriting::handwriting_recognize(model, strokes, CANVAS, CANVAS)
        .map_err(|error| error.to_string())
}

/// The reference pen trajectory for 中 used by the upstream recognizer tests.
/// It gives the self-test a deterministic input without a display server.
pub fn zhong_fixture() -> Vec<Vec<(f32, f32)>> {
    vec![
        vec![(35.0, 40.0), (35.0, 105.0)],
        vec![(35.0, 40.0), (125.0, 40.0), (125.0, 105.0)],
        vec![(35.0, 105.0), (125.0, 105.0)],
        vec![(80.0, 15.0), (80.0, 140.0)],
    ]
}
