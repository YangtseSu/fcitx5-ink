// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

/// A handwriting model language. Tegaki ships one Zinnia model per language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    #[serde(rename = "zh_CN")]
    SimplifiedChinese,
    #[serde(rename = "zh_TW")]
    TraditionalChinese,
    #[serde(rename = "ja")]
    Japanese,
}

impl Language {
    pub const ALL: [Language; 3] = [
        Language::SimplifiedChinese,
        Language::TraditionalChinese,
        Language::Japanese,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Language::SimplifiedChinese => "zh_CN",
            Language::TraditionalChinese => "zh_TW",
            Language::Japanese => "ja",
        }
    }

    pub fn model_name(self) -> &'static str {
        match self {
            Language::SimplifiedChinese => "handwriting-zh_CN.model",
            Language::TraditionalChinese => "handwriting-zh_TW.model",
            Language::Japanese => "handwriting-ja.model",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Language::SimplifiedChinese => "简体中文",
            Language::TraditionalChinese => "繁體中文",
            Language::Japanese => "日本語",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
    }
}

/// Find the packaged Zinnia model for `language`. An explicit `--model` wins,
/// then the environment, then the development target directory, then installed
/// fcitx5-ink and Tegaki locations.
pub fn discover(explicit: Option<&Path>, language: Language) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return path.is_file().then(|| path.to_path_buf());
    }

    for key in ["FCITX5_INK_MODEL", "MSIME_HANDWRITING_MODEL"] {
        let Some(value) = env::var_os(key).filter(|value| !value.is_empty()) else {
            continue;
        };
        let path = PathBuf::from(value);
        if path.is_absolute() && path.is_file() {
            return Some(path);
        }
    }

    let model = language.model_name();
    let mut candidates = Vec::new();
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            // target/debug/fcitx5-ink-panel -> target/handwriting-model
            if let Some(target) = directory.parent() {
                candidates.push(target.join("handwriting-model").join(model));
            }
            candidates.push(directory.join(model));
        }
    }

    if let Some(data_home) = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        candidates.push(data_home.join("fcitx5-ink").join(model));
        candidates.push(data_home.join("tegaki/models/zinnia").join(model));
        if language == Language::SimplifiedChinese {
            candidates.push(data_home.join("msime-client/handwriting").join(model));
        }
    }
    if let Some(home) = env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        candidates.push(home.join(".local/share/fcitx5-ink").join(model));
        candidates.push(home.join(".local/share/tegaki/models/zinnia").join(model));
        if language == Language::SimplifiedChinese {
            candidates.push(
                home.join(".local/share/msime-client/handwriting")
                    .join(model),
            );
            candidates.push(
                home.join(".config/msime-client/resource-packs/handwriting")
                    .join(model),
            );
        }
    }
    for prefix in ["/usr/local", "/usr"] {
        candidates.push(PathBuf::from(prefix).join("share/fcitx5-ink").join(model));
        candidates.push(
            PathBuf::from(prefix)
                .join("share/tegaki/models/zinnia")
                .join(model),
        );
        if language == Language::SimplifiedChinese {
            candidates.push(
                PathBuf::from(prefix)
                    .join("share/msime-client/handwriting")
                    .join(model),
            );
        }
    }

    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_map_to_their_model_files() {
        assert_eq!(Language::default(), Language::SimplifiedChinese);
        assert_eq!(
            Language::SimplifiedChinese.model_name(),
            "handwriting-zh_CN.model"
        );
        assert_eq!(
            Language::TraditionalChinese.model_name(),
            "handwriting-zh_TW.model"
        );
        assert_eq!(Language::Japanese.model_name(), "handwriting-ja.model");
        assert_eq!(Language::Japanese.code(), "ja");
        assert_eq!(
            Language::from_code("zh_TW"),
            Some(Language::TraditionalChinese)
        );
        assert_eq!(Language::from_code("en"), None);
    }
}
