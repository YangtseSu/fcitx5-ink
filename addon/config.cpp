// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#include "config.h"

#include <cctype>
#include <cstdlib>
#include <exception>
#include <vector>

#include <toml++/toml.hpp>

namespace fcitx5_ink {

std::string trim(std::string_view value) {
  size_t begin = 0;
  while (begin < value.size() &&
         std::isspace(static_cast<unsigned char>(value[begin])))
    ++begin;
  size_t end = value.size();
  while (end > begin &&
         std::isspace(static_cast<unsigned char>(value[end - 1])))
    --end;
  return std::string(value.substr(begin, end - begin));
}

std::filesystem::path config_path_from(const char *xdg_config_home,
                                       const char *home) {
  if (xdg_config_home && *xdg_config_home) {
    const std::filesystem::path base(xdg_config_home);
    if (base.is_absolute())
      return base / "fcitx5-ink/config.toml";
  }
  if (home && *home) {
    const std::filesystem::path base(home);
    if (base.is_absolute())
      return base / ".config/fcitx5-ink/config.toml";
  }
  return {};
}

std::filesystem::path config_path() {
  return config_path_from(std::getenv("XDG_CONFIG_HOME"), std::getenv("HOME"));
}

bool validate_hotkey(const std::string &value, fcitx::Key *key) {
  const std::string trimmed = trim(value);
  if (trimmed.empty())
    return false;
  std::vector<std::string_view> parts;
  std::string_view remaining(trimmed);
  while (true) {
    const size_t separator = remaining.find('+');
    const std::string_view part = remaining.substr(0, separator);
    if (part.empty())
      return false;
    parts.push_back(part);
    if (separator == std::string_view::npos)
      break;
    remaining.remove_prefix(separator + 1);
  }
  if (parts.size() < 2)
    return false;
  const auto modifier = [](std::string_view part) {
    return part == "Shift" || part == "Control" || part == "Alt" ||
           part == "Super" || part == "Hyper";
  };
  for (size_t index = 0; index + 1 < parts.size(); ++index) {
    if (!modifier(parts[index]))
      return false;
    for (size_t previous = 0; previous < index; ++previous) {
      if (parts[previous] == parts[index])
        return false;
    }
  }
  if (modifier(parts.back()))
    return false;
  fcitx::Key parsed(trimmed);
  if (!parsed.isValid())
    return false;
  fcitx::KeyStates modifiers{fcitx::KeyState::Shift};
  modifiers |= fcitx::KeyState::Ctrl;
  modifiers |= fcitx::KeyState::Alt;
  modifiers |= fcitx::KeyState::Super;
  modifiers |= fcitx::KeyState::Hyper;
  if (!parsed.states().testAny(modifiers))
    return false;
  if (key)
    *key = parsed;
  return true;
}

bool read_hotkey(const std::filesystem::path &path,
                 std::optional<std::string> *hotkey, std::string *error) {
  hotkey->reset();
  if (path.empty())
    return true;
  try {
    if (!std::filesystem::is_regular_file(path))
      return true;
    const toml::table table = toml::parse_file(path.string());
    const toml::node_view<const toml::node> node = table["hotkey"];
    if (!node)
      return true;
    if (!node.is_string()) {
      *error = "hotkey in " + path.string() + " must be a string";
      return false;
    }
    hotkey->emplace(trim(*node.value<std::string>()));
    return true;
  } catch (const std::exception &exception) {
    *error = "read " + path.string() + ": " + exception.what();
    return false;
  }
}

bool resolve_hotkey(const std::optional<std::string> &environment,
                    const std::optional<std::string> &configured,
                    fcitx::Key *key, std::string *error) {
  if (environment) {
    const std::string value = trim(*environment);
    fcitx::Key parsed;
    if (!value.empty() && validate_hotkey(value, &parsed)) {
      *key = parsed;
      return true;
    }
  }
  if (configured) {
    fcitx::Key parsed;
    if (!validate_hotkey(*configured, &parsed)) {
      *error = "invalid hotkey in config: " + trim(*configured);
      return false;
    }
    *key = parsed;
    return true;
  }
  *key = fcitx::Key(kDefaultHotkey);
  return true;
}

bool load_hotkey(fcitx::Key *key, std::string *error) {
  std::optional<std::string> environment;
  if (const char *value = std::getenv("FCITX5_INK_HOTKEY");
      value && *value) {
    environment.emplace(value);
  }
  std::optional<std::string> configured;
  if (!read_hotkey(config_path(), &configured, error))
    return false;
  return resolve_hotkey(environment, configured, key, error);
}

} // namespace fcitx5_ink
