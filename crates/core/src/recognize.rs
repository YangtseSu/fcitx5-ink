// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::model::Language;
use fcitx5_ink_handwriting::{
    handwriting_recognize, handwriting_recognize_ranked, MAX_CANDIDATES, MAX_CANDIDATE_BYTES,
};
use std::collections::HashSet;
use std::path::Path;

/// A 420 by 420 virtual canvas. The vendored recognizer normalizes each
/// stroke's bounding box, so the drawable size only needs to stay stable while
/// a character is being written.
pub const CANVAS: f32 = 420.0;

pub fn recognize(
    model: &Path,
    strokes: &[Vec<(f32, f32)>],
    language: Language,
) -> Result<Vec<String>, String> {
    let model = model
        .to_str()
        .ok_or_else(|| "model path is not valid UTF-8".to_owned())?;
    if language == Language::Japanese {
        // The Chinese candidate policy moves every Han character ahead of
        // kana. The Japanese model's own ranking is correct, so keep it.
        let ranked = handwriting_recognize_ranked(model, strokes, CANVAS, CANVAS)
            .map_err(|error| error.to_string())?;
        Ok(ranked_candidates(ranked))
    } else {
        handwriting_recognize(model, strokes, CANVAS, CANVAS).map_err(|error| error.to_string())
    }
}

fn ranked_candidates(ranked: Vec<(String, f32)>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for (label, _) in ranked {
        if label.is_empty() || label.len() > MAX_CANDIDATE_BYTES {
            continue;
        }
        if seen.insert(label.clone()) {
            candidates.push(label);
        }
        if candidates.len() == MAX_CANDIDATES {
            break;
        }
    }
    candidates
}

/// A deterministic trajectory for the language's reference character:
/// 中, 馬 and あ. The Tegaki sample for あ and 馬 is scaled from its 1000-unit
/// canvas into [`CANVAS`].
pub fn fixture(language: Language) -> Vec<Vec<(f32, f32)>> {
    match language {
        Language::SimplifiedChinese => vec![
            vec![(35.0, 40.0), (35.0, 105.0)],
            vec![(35.0, 40.0), (125.0, 40.0), (125.0, 105.0)],
            vec![(35.0, 105.0), (125.0, 105.0)],
            vec![(80.0, 15.0), (80.0, 140.0)],
        ],
        Language::TraditionalChinese => scale(&[
            &[(286.0, 160.0), (280.0, 610.0)],
            &[(373.0, 160.0), (703.0, 160.0)],
            &[(536.0, 200.0), (516.0, 593.0)],
            &[(343.0, 323.0), (630.0, 300.0)],
            &[(373.0, 443.0), (610.0, 416.0)],
            &[
                (330.0, 643.0),
                (786.0, 616.0),
                (766.0, 880.0),
                (693.0, 916.0),
            ],
            &[(243.0, 760.0), (153.0, 860.0)],
            &[(343.0, 760.0), (410.0, 853.0)],
            &[(500.0, 753.0), (553.0, 836.0)],
            &[(630.0, 753.0), (700.0, 836.0)],
        ]),
        Language::Japanese => scale(&[
            &[(180.0, 193.0), (830.0, 226.0)],
            &[(490.0, 33.0), (483.0, 670.0), (606.0, 840.0)],
            &[
                (746.0, 343.0),
                (496.0, 766.0),
                (273.0, 800.0),
                (176.0, 680.0),
                (286.0, 496.0),
                (606.0, 463.0),
                (800.0, 573.0),
                (826.0, 746.0),
                (760.0, 833.0),
            ],
        ]),
    }
}

pub fn fixture_character(language: Language) -> &'static str {
    match language {
        Language::SimplifiedChinese => "中",
        Language::TraditionalChinese => "馬",
        Language::Japanese => "あ",
    }
}

fn scale(strokes: &[&[(f32, f32)]]) -> Vec<Vec<(f32, f32)>> {
    let factor = CANVAS / 1000.0;
    strokes
        .iter()
        .map(|stroke| {
            stroke
                .iter()
                .map(|(x, y)| (x * factor, y * factor))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranked_candidates_keep_model_order_and_drop_duplicates() {
        let ranked = vec![
            ("あ".to_owned(), 0.9),
            ("日".to_owned(), 0.8),
            ("あ".to_owned(), 0.7),
            (String::new(), 0.6),
            ("x".repeat(MAX_CANDIDATE_BYTES + 1), 0.5),
            ("本".to_owned(), 0.4),
        ];
        assert_eq!(ranked_candidates(ranked), ["あ", "日", "本"]);
    }

    #[test]
    fn ranked_candidates_are_capped() {
        let ranked = (0..20).map(|index| (format!("c{index}"), 1.0)).collect();
        assert_eq!(ranked_candidates(ranked).len(), MAX_CANDIDATES);
    }

    #[test]
    fn fixtures_name_their_expected_character() {
        for language in Language::ALL {
            assert!(!fixture(language).is_empty());
            assert!(!fixture_character(language).is_empty());
            assert!(fixture(language)
                .iter()
                .flatten()
                .all(|(x, y)| (0.0..=CANVAS).contains(x) && (0.0..=CANVAS).contains(y)));
        }
    }
}
