// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use fcitx5_ink_core::model::Language;
use fcitx5_ink_core::recognize::{fixture, fixture_character, recognize};
use std::path::PathBuf;

fn model(variable: &str) -> Option<PathBuf> {
    let value = std::env::var_os(variable)?;
    let path = PathBuf::from(value);
    path.is_file().then_some(path)
}

fn recognizes(language: Language, variable: &str) {
    let Some(model) = model(variable) else {
        eprintln!("skipped: set {variable} to the {} model", language.code());
        return;
    };
    let candidates = recognize(&model, &fixture(language), language).unwrap();
    assert_eq!(
        candidates.first().map(String::as_str),
        Some(fixture_character(language)),
        "{} candidates: {candidates:?}",
        language.code()
    );
}

#[test]
fn simplified_chinese_model_ranks_its_fixture() {
    recognizes(Language::SimplifiedChinese, "FCITX5_INK_ZH_CN_MODEL");
}

#[test]
fn traditional_chinese_model_ranks_its_fixture() {
    recognizes(Language::TraditionalChinese, "FCITX5_INK_ZH_TW_MODEL");
}

#[test]
fn japanese_model_ranks_hiragana_first() {
    recognizes(Language::Japanese, "FCITX5_INK_JA_MODEL");
}
