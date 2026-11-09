# fcitx5-ink (墨笔)

A standalone handwriting input tool for fcitx5. The addon is C++20, the
panel is GTK4 via `gtk4-rs`, and recognition is the vendored msime handwriting
engine. It does not depend on the msime input method, its host code, or its
platform directory.

- **Name:** `fcitx5-ink`; Chinese UI name **墨笔**.
- **Default hotkey:** `Control+Alt+H` to open or close the panel without
  switching input methods. Change it in the panel's 设置 window; fcitx5 key
  syntax such as `Super+i` is accepted.
- **Settings:** stored in `$XDG_CONFIG_HOME/fcitx5-ink/config.toml`; the addon
  reloads the hotkey through its socket when the panel saves.
- **Languages:** Japanese, Simplified Chinese and Traditional Chinese
  handwriting models are bundled. Choose the recognition language in 设置.
- **Usage:** press the hotkey; no input method entry or switch is involved.
  Ink does not appear in `fcitx5-configtool` and cannot be added to a group.

## Layout: crates/handwriting

`crates/handwriting/vendor/` is the `handwriting` module from
`msime/crates/engine/src/handwriting/`, including `features.rs`, `mod.rs`,
`model.rs`, `recognizer.rs`, and `testdata/`. It is pinned from
`metasequoiaime/msime` commit `a2365adbad` (`develop`), with one additive
change: `recognizer.rs` also exports `handwriting_recognize_ranked`, which
returns labels in model score order so Japanese kana are not moved behind every
Han candidate, and `mod.rs` exports `extract_feature_nodes` for the offline
trainer. The existing `handwriting_recognize` still applies msime's Chinese
candidate policy.

`crates/handwriting/src/lib.rs` is the crate root that wraps the vendor module:
it provides the `handwriting` module path, the `EngineError`/`Result` re-export
and the `paths::open_file_no_follow` seam that the upstream module expects.

To re-vendor, copy the upstream directory over `crates/handwriting/vendor/`,
re-add the ranked function from the current `recognizer.rs` and its `mod.rs`
export, then verify that no other difference remains:

```sh
diff -r /path/to/msime/crates/engine/src/handwriting crates/handwriting/vendor
```

Project code is at the root and in `crates/`:

```text
fcitx5-ink/
├── addon/       fcitx5 C++20 addon
├── crates/
│   ├── handwriting/  recognizer crate; vendor/ is upstream plus ranked output
│   ├── core/         config, model discovery, recognition wrapper, panel IPC
│   └── panel/        GTK4 panel, stroke history, settings
├── scripts/
├── tests/
├── PKGBUILD
└── README.md
```

## Architecture

```text
GTK4 panel (crates/panel)
  ├─ draws ink, edits strokes and shows candidates
  ├─ calls the vendored recognizer through core
  ├─ reads/writes config.toml
  └─ sends {"op":"text"} and {"op":"reload"} over a Unix socket
                 │
fcitx5 addon (addon) ── commitString ──> focused application
```

The addon is a fcitx5 module, not an input method. It owns the focused
`InputContext` and the socket at
`$XDG_RUNTIME_DIR/fcitx5-ink/panel-input.sock` (directory `0700`, socket
`0600`, same-uid peer check). It watches the configured hotkey globally through
fcitx5 key events. The panel hides itself before asking to commit so focus can
return to the editor; the addon keeps a request pending for up to 700 ms while
that focus returns.

No `PanelInputChannel.h`, IBus code, Tauri code, or msime input-method code is
compiled here. The only vendored source is `crates/handwriting/vendor/`.

## Build

Requirements: fcitx5 development headers, GTK4 development files, CMake,
Rust 1.89+, `nlohmann-json`, and the header-only `tomlplusplus` package.

```sh
./scripts/fetch-model.sh
./scripts/build.sh            # debug; pass --release for a release panel
```

The models are Tegaki 0.3 Zinnia files: `handwriting-ja.model` (26.1 MB),
`handwriting-zh_CN.model` (26.8 MB) and `handwriting-zh_TW.model` (52.2 MB).
They are not committed; `fetch-model.sh` downloads the pinned release archives,
extracts each model, and verifies the extracted bytes against
`resources/handwriting-model.lock.json`. The Arch packages install them from
the separate `fcitx5-ink-data` package.

## Install

```sh
sudo cmake --install build --prefix /usr
```

Installed files are split between the code package and `fcitx5-ink-data`:

```text
fcitx5-ink:
/usr/bin/fcitx5-ink-panel
/usr/lib/fcitx5/libfcitx5-ink.so
/usr/share/fcitx5/addon/ink.conf
/usr/share/licenses/fcitx5-ink/

fcitx5-ink-data:
/usr/share/fcitx5-ink/handwriting-ja.model
/usr/share/fcitx5-ink/handwriting-zh_CN.model
/usr/share/fcitx5-ink/handwriting-zh_TW.model
/usr/share/licenses/fcitx5-ink-data/HandwritingModel-LICENSE.txt
```

Restart fcitx5 after installing (`fcitx5 -d --replace` or your service
manager), then press `Control+Alt+H`.

The panel is split into a rice-grid canvas on the left and the recognition
results on the right. The canvas has undo, redo, delete-last-stroke and clear
buttons below it. When the candidate limit allows, the first ten candidates are
numbered `1`–`9` and `0`; the default four-column grid puts `1`–`4` in the first
row, and candidates beyond ten are click-only. Clicking a candidate inputs it,
and 复制首选 copies the first candidate to the clipboard.

Keyboard shortcuts while the panel is focused:

| Shortcut | Action |
|---|---|
| `1`–`9` | input the corresponding numbered candidate |
| `0` | input the tenth candidate |
| `Ctrl+Z` | undo |
| `Ctrl+Shift+Z`, `Ctrl+Y` | redo |
| `BackSpace` | delete the last stroke |
| `Delete` | clear all strokes |
| `Escape` | close the panel |

## Configure

Open 设置 in the panel header. The window edits the hotkey, recognition
language, stroke width (1–8 px) and the number of visible candidates (1–12).
Save validates the hotkey with fcitx5, writes the file atomically, and asks the
addon to reload it. `config.toml` has this shape:

```toml
hotkey = "Control+Alt+H"

[panel]
language = "zh_CN"
stroke_width = 3
max_candidates = 9
```

The language is one of:

| Code | Language | Model |
|---|---|---|
| `zh_CN` | 简体中文 | `handwriting-zh_CN.model` |
| `zh_TW` | 繁體中文 | `handwriting-zh_TW.model` |
| `ja` | 日本語 | `handwriting-ja.model` |

Japanese recognition keeps the model's own score order, so kana and kanji are
ranked together. Changing the language re-runs recognition on the current
strokes. `FCITX5_INK_MODEL` overrides the selected model path for testing.

Modifier names are case-sensitive: `Shift`, `Control`, `Alt`, `Super` and
`Hyper`. A hotkey must contain at least one modifier. `FCITX5_INK_HOTKEY`
remains an override for testing and wins over the file; an invalid override is
ignored. Saving from 设置 applies the hotkey immediately. If you edit the file
by hand, restart fcitx5 so the addon re-reads it; panel options are read when
the panel starts.

## Test

```sh
./scripts/test.sh
```

The test suite covers:

- the vendored engine's unit tests, including the Zinnia parity test when the
  model is available;
- config parsing, hotkey validation, override precedence and atomic saves;
- stroke history and recognition-generation tests for the panel;
- the panel socket client's success and error paths;
- real recognition self-tests for all three models (`中`, `馬` and `あ` come
  first for their languages);
- isolated `dbus-run-session` + `Xvfb` fcitx5 runs that verify a configured
  hotkey and socket reload, that selecting `ink` never opens the panel, and
  that the test client receives `TEXT:你` through the hotkey path.

The integration test needs `dbus-run-session`, `Xvfb`, `xdotool`, and GTK3
development files.

## Arch packages

The code and models are separate packages. Build and install the data package
first; both code packages depend on it:

```sh
cd packaging/arch/fcitx5-ink-data
makepkg -si
```

The root `PKGBUILD` builds this checkout as the release code package. For a
local `-git` test package:

```sh
cd packaging/arch/fcitx5-ink-git
makepkg -si
```

The `-git` package clones the parent repository, so commit the working tree
first. For an AUR upload, publish `fcitx5-ink-data` and the chosen code
package, and replace the `git+file://$startdir/../../..` source with
`git+https://github.com/YangtseSu/fcitx5-ink.git`. The root `PKGBUILD` is a
local-source package; an AUR release package should use a release tarball
source and checksum.

## Limits

- One character per recognition. Multi-character cell segmentation is not
  part of the vendored module.
- The Tegaki Japanese model covers hiragana and kanji; its label set includes
  only a few katakana (イ, エ, オ, ジ, ラ, ン).
- The panel owns focus and candidates; it does not use the fcitx5 candidate
  window.
- Wayland window placement is controlled by the compositor; the panel does not
  try to follow the text cursor.
- `handwriting-core` is GUI-independent. A future GPUI Kit frontend can reuse
  it and the same socket protocol without touching the addon.

## Acknowledgements

The project builds on two upstream layers:

- [msime](https://github.com/metasequoiaime/msime) and
  [msime-engine](https://github.com/metasequoiaime/msime-engine) — the
  handwriting module under `crates/handwriting/vendor/` is their Rust port of
  Zinnia, including the Chinese candidate policy and the original model
  packaging.
- [Tegaki](https://github.com/tegaki/tegaki) — the Japanese, Simplified Chinese
  and Traditional Chinese Zinnia models are the Tegaki 0.3 release files
  (published 2016).
- [Zinnia](https://github.com/taku910/zinnia) by Taku Kudo — the original
  handwriting recognizer and model reader, under BSD-3-Clause. Tegaki's models
  use its format.

Thanks to the Tegaki and Zinnia contributors whose recognizer and handwriting
data made these models possible.

## License

New code is GPL-3.0-or-later. The vendored handwriting module retains the
msime-engine licenses: MPL-2.0 for the engine code, BSD-3-Clause for the
Zinnia port, and GPL-3.0-only for the upstream test fixtures. The handwriting
models are LGPL-2.1-or-later; their license ships with `fcitx5-ink-data` as
`HandwritingModel-LICENSE.txt` and is recorded in the data repository.
