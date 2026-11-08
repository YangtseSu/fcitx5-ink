// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::path::{Path, PathBuf};

/// Find the packaged zinnia model. An explicit `--model` wins, then the
/// environment, then the development target directory, then installed paths.
pub fn discover(explicit: Option<&Path>) -> Option<PathBuf> {
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

    let mut candidates = Vec::new();
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            // target/debug/fcitx5-ink-panel -> target/handwriting-model
            if let Some(target) = directory.parent() {
                candidates.push(target.join("handwriting-model/handwriting-zh_CN.model"));
            }
            candidates.push(directory.join("handwriting-zh_CN.model"));
        }
    }

    if let Some(data_home) = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        candidates.push(data_home.join("fcitx5-ink/handwriting-zh_CN.model"));
        candidates.push(data_home.join("msime-client/handwriting/handwriting-zh_CN.model"));
    }
    if let Some(home) = env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        candidates.push(home.join(".local/share/fcitx5-ink/handwriting-zh_CN.model"));
        candidates.push(
            home.join(".config/msime-client/resource-packs/handwriting/handwriting-zh_CN.model"),
        );
    }
    candidates.push(PathBuf::from(
        "/usr/local/share/fcitx5-ink/handwriting-zh_CN.model",
    ));
    candidates.push(PathBuf::from(
        "/usr/share/fcitx5-ink/handwriting-zh_CN.model",
    ));
    candidates.push(PathBuf::from(
        "/usr/local/share/msime-client/handwriting/handwriting-zh_CN.model",
    ));
    candidates.push(PathBuf::from(
        "/usr/share/msime-client/handwriting/handwriting-zh_CN.model",
    ));

    candidates.into_iter().find(|path| path.is_file())
}
