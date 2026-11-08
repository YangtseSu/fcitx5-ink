// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#include "../addon/config.h"

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <string>
#include <unistd.h>

namespace {

int failures = 0;

void check(bool condition, const char *what) {
  if (!condition) {
    std::cerr << "FAIL: " << what << '\n';
    ++failures;
  }
}

void check_hotkey(const fcitx::Key &key, const char *expected,
                  const char *what) {
  if (key.toString() != expected) {
    std::cerr << "FAIL: " << what << ": got " << key.toString() << ", want "
              << expected << '\n';
    ++failures;
  }
}

void write_file(const std::filesystem::path &path, const std::string &text) {
  std::ofstream stream(path);
  stream << text;
}

} // namespace

int main() {
  using namespace fcitx5_ink;

  check(trim("  Control+Alt+H \n") == "Control+Alt+H", "trim");
  check(trim("\t ") == "", "trim whitespace");

  check(config_path_from("/tmp/config", "/home/user") ==
            std::filesystem::path("/tmp/config/fcitx5-ink/config.toml"),
        "xdg path");
  check(config_path_from("relative", "/home/user") ==
            std::filesystem::path("/home/user/.config/fcitx5-ink/config.toml"),
        "relative xdg falls back to home");
  check(config_path_from("relative", nullptr).empty(), "no base directory");

  fcitx::Key key;
  check(validate_hotkey("Control+Alt+H", &key), "valid hotkey");
  check_hotkey(key, "Control+Alt+H", "valid hotkey text");
  check(validate_hotkey("Super+i", &key), "super hotkey");
  check(validate_hotkey("Shift+Control+Z", &key), "shift hotkey");
  check_hotkey(key, "Control+Shift+Z", "normalized hotkey text");
  check(validate_hotkey("Control+Alt+[", &key), "punctuation key");
  check(validate_hotkey("Control+Alt+F12", &key), "function key");
  check(validate_hotkey("Control+Alt+Page_Up", &key), "named key");

  check(!validate_hotkey("", &key), "empty hotkey");
  check(!validate_hotkey("Control", &key), "modifier only");
  check(!validate_hotkey("Ctrl+Alt+H", &key), "Ctrl is not a modifier");
  check(!validate_hotkey("Control+Control+H", &key), "duplicate modifier");
  check(!validate_hotkey("Meta+H", &key), "Meta is not accepted");
  check(!validate_hotkey("a", &key), "hotkey needs a modifier");
  check(!validate_hotkey("Control+Foo", &key), "unknown key name");

  const auto directory =
      std::filesystem::temp_directory_path() /
      ("fcitx5-ink-config-test-" + std::to_string(::getpid()));
  std::filesystem::remove_all(directory);
  std::filesystem::create_directories(directory / "fcitx5-ink");
  const auto config = directory / "fcitx5-ink" / "config.toml";
  ::setenv("XDG_CONFIG_HOME", directory.c_str(), 1);
  ::unsetenv("FCITX5_INK_HOTKEY");

  std::optional<std::string> configured;
  std::string error;
  check(read_hotkey(config, &configured, &error),
        "missing config file is not an error");
  check(!configured, "missing config file has no hotkey");

  write_file(config, "hotkey = \"Control+Alt+J\"\n");
  check(read_hotkey(config, &configured, &error), "read config hotkey");
  check(configured == "Control+Alt+J", "config hotkey text");
  check(resolve_hotkey(std::nullopt, configured, &key, &error),
        "resolve configured hotkey");
  check_hotkey(key, "Control+Alt+J", "configured hotkey");

  check(resolve_hotkey(std::optional<std::string>("Control+Alt+K"), configured,
                       &key, &error),
        "environment override");
  check_hotkey(key, "Control+Alt+K", "environment hotkey");
  check(resolve_hotkey(std::optional<std::string>("not a key"), configured,
                       &key, &error),
        "invalid environment override is ignored");
  check_hotkey(key, "Control+Alt+J", "config used after invalid override");
  check(resolve_hotkey(std::nullopt, std::nullopt, &key, &error),
        "default hotkey");
  check_hotkey(key, kDefaultHotkey, "default hotkey text");
  check(!resolve_hotkey(std::nullopt, std::optional<std::string>("bad"), &key,
                        &error),
        "invalid configured hotkey is an error");
  check(!error.empty(), "invalid configured hotkey has a message");

  write_file(config, "hotkey = \"Control+Alt+L\"\n");
  check(load_hotkey(&key, &error), "load from the default path");
  check_hotkey(key, "Control+Alt+L", "default path hotkey");
  ::setenv("FCITX5_INK_HOTKEY", "Control+Alt+M", 1);
  check(load_hotkey(&key, &error), "load with the environment override");
  check_hotkey(key, "Control+Alt+M", "environment override hotkey");
  ::setenv("FCITX5_INK_HOTKEY", "not a key", 1);
  check(load_hotkey(&key, &error), "invalid environment override is ignored");
  check_hotkey(key, "Control+Alt+L", "config hotkey after invalid override");
  ::unsetenv("FCITX5_INK_HOTKEY");

  std::filesystem::remove_all(directory);
  if (failures != 0) {
    std::cerr << failures << " failure(s)\n";
    return 1;
  }
  std::cout << "config tests passed\n";
  return 0;
}
