// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include <fcitx-utils/key.h>

#include <filesystem>
#include <optional>
#include <string>
#include <string_view>

namespace fcitx5_ink {

inline constexpr const char *kDefaultHotkey = "Control+Alt+H";

std::string trim(std::string_view value);

std::filesystem::path config_path_from(const char *xdg_config_home,
                                       const char *home);
std::filesystem::path config_path();

/// Parse a hotkey and require at least one modifier, so a plain key cannot
/// capture ordinary typing.
bool validate_hotkey(const std::string &value, fcitx::Key *key);

/// Read `hotkey` from a TOML file. A missing file or key yields nullopt.
bool read_hotkey(const std::filesystem::path &path,
                 std::optional<std::string> *hotkey, std::string *error);

/// Resolve an environment override, then the config value, then the default.
/// An invalid test override is ignored; an invalid config value is an error.
bool resolve_hotkey(const std::optional<std::string> &environment,
                    const std::optional<std::string> &configured,
                    fcitx::Key *key, std::string *error);

bool load_hotkey(fcitx::Key *key, std::string *error);

} // namespace fcitx5_ink
