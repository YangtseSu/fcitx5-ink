<!-- SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com> -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# 001: Handwriting UX follow-up

Status: proposed
Date: 2026-11-08

## Problems

1. Selecting Ink in `fcitx5-configtool` causes continuous screen flicker.
   The input method entry calls `InkEngine::activate`, which launches the GTK4
   panel. The panel takes focus, the target context loses focus, and the
   activation/focus cycle may repeat. The exact trigger still needs a verbose
   fcitx5 log and a configtool reproduction before changing the code.
2. Adding Ink to the input method group is documented as a normal usage path.
   It should not be: handwriting is a hotkey tool. The hotkey path does not
   require an input method switch, and the input method entry should be
   optional or removed.
3. The panel has only drawing, recognition, candidate selection and copy. It
   lacks undo, redo, delete-last-stroke and clear-all.
4. There is no settings surface. The hotkey is only `FCITX5_INK_HOTKEY`; there
   is no UI and no persisted configuration.

## Decisions

- **Hotkey is the primary and documented usage.** `Ctrl+Alt+H` opens/closes the
  panel without switching input methods.
- **Do not auto-open the panel from the input method entry.** If the entry is
  kept, activation should do nothing except show a status hint; only the
  hotkey should open the panel. This removes the activation/focus loop even if
  the entry remains installed.
- **Evaluate removing the input method entry entirely.** Keep it only if it
  has a clear use case and no flicker. Removing `listInputMethods` and the
  `inputmethod` manifest would simplify the addon and avoid the configtool
  path altogether.
- **Give the panel direct editing controls.** Undo last stroke, redo, delete
  last stroke and clear all are panel-local operations; they do not need the
  addon.
- **Add one persisted config file** at `$XDG_CONFIG_HOME/fcitx5-ink/config.toml`
  with at least `hotkey`. The addon reads it at startup; a settings change
  should either reload the addon or tell the user to restart fcitx5.
- **Settings live in the panel for now.** Add a compact preferences popover or
  a `--settings` GTK window; do not add a second executable until the settings
  surface grows.

## Work

### 1. Reproduce and fix the flicker

- Run an isolated fcitx5 with `--verbose '*=5'`, add Ink to the group, switch to
  it, and capture repeated `activate`, `deactivate`, focus and panel-launch
  lines.
- Confirm whether the loop is caused by `activate` launching the panel while
  the panel steals focus.
- Change `activate` to not launch the panel. Keep `target_` tracking only for
  the hotkey and commit path.
- If the entry remains, disable auto-launch and verify configtool switching no
  longer flickers.
- Add an isolated regression test that selects `ink` and asserts the panel is
  not spawned; keep the existing hotkey and commit tests.

### 2. Remove the group recommendation

- README already points to the hotkey and marks the group path as a bug; keep
  it that way.
- Remove `fcitx5-configtool` from `optdepends` in both PKGBUILDs.
- Decide whether to delete `addon/ink-inputmethod.conf` and
  `InkEngine::listInputMethods`. If removed, also remove the `inputmethod`
  install rule and update the integration test that switches to `ink`.

### 3. Panel editing controls

- Add a compact toolbar: undo, redo, delete last stroke, clear.
- Keyboard shortcuts while the panel has focus:
  - `Ctrl+Z`: undo;
  - `Ctrl+Shift+Z` or `Ctrl+Y`: redo;
  - `BackSpace`: delete last stroke;
  - `Delete`: clear all;
  - `Escape`: close.
- Keep recognition/commit state consistent: clear cancels pending recognition;
  undo/redo re-runs or clears candidates as appropriate.
- Disable controls when there is nothing to act on.
- Add UI tests for stroke history, undo/redo/clear and the recognition
  generation guard.

### 4. Settings

- Define `config.toml` schema with `hotkey` and optional panel keys.
- Add a settings button to the panel header or a `--settings` window.
- Validate the hotkey with fcitx5 key syntax; reject invalid values.
- Persist atomically and reload:
  - simplest: addon reads the file at startup; settings UI tells the user to
    restart fcitx5;
  - better: addon watches the file or exposes a reload path through its socket.
- Keep `FCITX5_INK_HOTKEY` as an environment override for testing, but make
  the config file the normal source.
- Add tests for config parsing, invalid hotkeys and override precedence.

## Acceptance

- Adding Ink to the input method group does not flicker; the hotkey is the
  documented usage and needs no input method switch.
- Panel can undo, redo, delete the last stroke and clear all, with working
  keyboard shortcuts and correct candidate state.
- Hotkey can be changed in the settings UI, persists across restart and is
  actually used by the addon.
- Existing recognition, commit and hotkey integration tests still pass.
