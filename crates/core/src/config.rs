// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The persisted `config.toml` shared by the addon and the panel.
//!
//! The addon only reads `hotkey`; the panel reads and writes the whole file.
//! Parsing is strict: a malformed file or an out-of-range panel value is an
//! error instead of a silent partial fallback.

use crate::model::Language;
use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const DEFAULT_HOTKEY: &str = "Control+Alt+H";
pub const DEFAULT_STROKE_WIDTH: u8 = 3;
pub const DEFAULT_MAX_CANDIDATES: usize = 9;
pub const MIN_STROKE_WIDTH: u8 = 1;
pub const MAX_STROKE_WIDTH: u8 = 8;
pub const MIN_CANDIDATES: usize = 1;
pub const MAX_CANDIDATES: usize = 12;

const MODIFIERS: [&str; 5] = ["Shift", "Control", "Alt", "Super", "Hyper"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelConfig {
    #[serde(default)]
    pub language: Language,
    #[serde(default = "default_stroke_width")]
    pub stroke_width: u8,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            language: Language::default(),
            stroke_width: default_stroke_width(),
            max_candidates: default_max_candidates(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default)]
    pub panel: PanelConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            panel: PanelConfig::default(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        validate_hotkey(self.hotkey.trim())?;
        if !(MIN_STROKE_WIDTH..=MAX_STROKE_WIDTH).contains(&self.panel.stroke_width) {
            return Err(format!(
                "笔画粗细必须在 {MIN_STROKE_WIDTH} 到 {MAX_STROKE_WIDTH} 之间"
            ));
        }
        if !(MIN_CANDIDATES..=MAX_CANDIDATES).contains(&self.panel.max_candidates) {
            return Err(format!(
                "候选数量必须在 {MIN_CANDIDATES} 到 {MAX_CANDIDATES} 之间"
            ));
        }
        Ok(())
    }
}

fn default_hotkey() -> String {
    DEFAULT_HOTKEY.to_owned()
}

fn default_stroke_width() -> u8 {
    DEFAULT_STROKE_WIDTH
}

fn default_max_candidates() -> usize {
    DEFAULT_MAX_CANDIDATES
}

/// Parse and validate a whole `config.toml`. Missing keys keep their defaults.
pub fn parse(text: &str) -> Result<Config, String> {
    let mut config: Config =
        toml::from_str(text).map_err(|error| format!("配置文件无效：{error}"))?;
    config.hotkey = config.hotkey.trim().to_owned();
    config.validate()?;
    Ok(config)
}

/// Load the user's config. An absent or unreadable file means defaults; the
/// caller decides whether a malformed file is worth reporting.
pub fn load() -> Config {
    config_path()
        .and_then(|path| load_from(&path).ok())
        .unwrap_or_default()
}

pub fn load_from(path: &Path) -> Result<Config, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
    parse(&text)
}

/// Write the config atomically next to its final path. The file may contain a
/// hotkey and nothing secret, but 0600 keeps it private.
pub fn save_to(path: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let mut text = toml::to_string(config).map_err(|error| format!("生成配置失败：{error}"))?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} 没有父目录", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("创建 {} 失败：{error}", parent.display()))?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("{} 没有文件名", path.display()))?;
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let write = || -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| format!("创建 {} 失败：{error}", temporary.display()))?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("写入 {} 失败：{error}", temporary.display()))?;
        file.sync_all()
            .map_err(|error| format!("同步 {} 失败：{error}", temporary.display()))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("重命名 {} 失败：{error}", temporary.display()))?;
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    };
    write().inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

pub fn config_path() -> Option<PathBuf> {
    path_from(
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    )
}

/// Pure path resolution used by tests and `config_path`.
pub fn path_from(xdg_config_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    fn absolute(value: &OsStr) -> Option<PathBuf> {
        let path = PathBuf::from(value);
        path.is_absolute().then_some(path)
    }
    if let Some(base) = xdg_config_home.and_then(absolute) {
        return Some(base.join("fcitx5-ink/config.toml"));
    }
    home.and_then(absolute)
        .map(|base| base.join(".config/fcitx5-ink/config.toml"))
}

/// Validate a hotkey in the fcitx5 key syntax the addon consumes. A hotkey
/// must contain a modifier so a single letter cannot swallow ordinary typing.
pub fn validate_hotkey(value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("快捷键不能为空".to_owned());
    }
    let parts: Vec<&str> = value.split('+').collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err("快捷键包含空项".to_owned());
    }
    let Some((key, modifiers)) = parts.split_last() else {
        return Err("快捷键不能为空".to_owned());
    };
    if modifiers.is_empty() {
        return Err("快捷键必须包含 Shift、Control、Alt、Super 或 Hyper".to_owned());
    }
    let mut seen = Vec::new();
    for modifier in modifiers {
        if !MODIFIERS.contains(modifier) {
            return Err(format!("不认识的修饰键：{modifier}"));
        }
        if seen.contains(modifier) {
            return Err(format!("重复的修饰键：{modifier}"));
        }
        seen.push(*modifier);
    }
    if MODIFIERS.contains(key) {
        return Err("快捷键缺少主键".to_owned());
    }
    if !valid_key_name(key) {
        return Err(format!("无效的按键名：{key}"));
    }
    Ok(())
}

fn valid_key_name(key: &str) -> bool {
    let mut characters = key.chars();
    if let (Some(character), None) = (characters.next(), characters.next()) {
        return character.is_ascii_graphic();
    }
    if let Some(number) = key.strip_prefix('F') {
        if !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()) {
            return number
                .parse::<u32>()
                .is_ok_and(|number| (1..=35).contains(&number));
        }
    }
    const NAMED: &[&str] = &[
        "space",
        "Return",
        "Escape",
        "Tab",
        "BackSpace",
        "Delete",
        "Insert",
        "Home",
        "End",
        "Page_Up",
        "Page_Down",
        "Up",
        "Down",
        "Left",
        "Right",
        "Print",
        "Pause",
        "Scroll_Lock",
        "Num_Lock",
        "Caps_Lock",
        "Menu",
        "Multi_key",
        "KP_Enter",
        "KP_Add",
        "KP_Subtract",
        "KP_Multiply",
        "KP_Divide",
        "KP_Decimal",
    ];
    if NAMED.contains(&key) {
        return true;
    }
    let identifier = key
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    identifier
        && key
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && (key.starts_with("ISO_")
            || key.starts_with("KP_")
            || key.starts_with("dead_")
            || key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn defaults_apply_to_missing_keys() {
        let config = parse("").unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.hotkey, "Control+Alt+H");
        assert_eq!(config.panel.language, Language::SimplifiedChinese);
        assert_eq!(config.panel.stroke_width, 3);
        assert_eq!(config.panel.max_candidates, 9);
    }

    #[test]
    fn parses_a_full_config_and_trims_the_hotkey() {
        let config = parse(
            r#"
hotkey = "  Control+Alt+J  "

[panel]
language = "zh_TW"
stroke_width = 5
max_candidates = 12
"#,
        )
        .unwrap();
        assert_eq!(config.hotkey, "Control+Alt+J");
        assert_eq!(config.panel.language, Language::TraditionalChinese);
        assert_eq!(config.panel.stroke_width, 5);
        assert_eq!(config.panel.max_candidates, 12);
    }

    #[test]
    fn rejects_unknown_fields_only_when_their_values_are_invalid() {
        let config = parse("hotkey = \"Super+i\"\nfuture = true\n").unwrap();
        assert_eq!(config.hotkey, "Super+i");
    }

    #[test]
    fn rejects_invalid_hotkeys() {
        for value in [
            "",
            "Control",
            "Ctrl+Alt+H",
            "Control+Alt+",
            "+H",
            "Control+Control+H",
            "Meta+H",
            "Foo",
            "Control+Alt+F36",
        ] {
            assert!(
                validate_hotkey(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn accepts_fcitx5_hotkey_shapes() {
        for value in [
            "Control+Alt+H",
            "Control+Alt+h",
            "Shift+Control+Z",
            "Super+i",
            "Hyper+space",
            "Control+Alt+[",
            "Control+Alt+F12",
            "Control+dead_grave",
            "Control+Alt+Page_Up",
        ] {
            assert!(
                validate_hotkey(value).is_ok(),
                "{value:?} should be accepted"
            );
        }
    }

    #[test]
    fn rejects_out_of_range_panel_values() {
        for text in [
            "[panel]\nstroke_width = 0\n",
            "[panel]\nstroke_width = 9\n",
            "[panel]\nlanguage = \"en\"\n",
            "[panel]\nstroke_width = nan\n",
            "[panel]\nstroke_width = 3.5\n",
            "[panel]\nmax_candidates = 0\n",
            "[panel]\nmax_candidates = 13\n",
        ] {
            assert!(parse(text).is_err(), "{text:?} should be rejected");
        }
    }

    #[test]
    fn resolves_the_config_path() {
        assert_eq!(
            path_from(
                Some(OsStr::new("/tmp/config")),
                Some(OsStr::new("/home/user"))
            ),
            Some(PathBuf::from("/tmp/config/fcitx5-ink/config.toml"))
        );
        assert_eq!(
            path_from(Some(OsStr::new("relative")), Some(OsStr::new("/home/user"))),
            Some(PathBuf::from("/home/user/.config/fcitx5-ink/config.toml"))
        );
        assert_eq!(path_from(Some(OsStr::new("relative")), None), None);
    }

    #[test]
    fn saves_atomically_and_loads_back() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fcitx5-ink/config.toml");
        let config = Config {
            hotkey: "Control+Alt+K".to_owned(),
            panel: PanelConfig {
                language: Language::Japanese,
                stroke_width: 4,
                max_candidates: 6,
            },
        };
        save_to(&path, &config).unwrap();
        assert_eq!(load_from(&path).unwrap(), config);
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert!(!directory
            .path()
            .join("fcitx5-ink")
            .read_dir()
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp")));
    }

    #[test]
    fn invalid_configs_do_not_overwrite_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let mut config = Config::default();
        config.panel.max_candidates = 0;
        assert!(save_to(&path, &config).is_err());
        assert!(!path.exists());
    }
}
