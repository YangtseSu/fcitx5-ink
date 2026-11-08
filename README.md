# fcitx5-ink (墨笔)

A standalone handwriting input method for fcitx5. The addon is C++20, the
panel is GTK4 via `gtk4-rs`, and recognition is the vendored msime handwriting
engine. It does not depend on the msime input method, its host code, or its
platform directory.

- **Name:** `fcitx5-ink`; Chinese UI name **墨笔**.
- **Default hotkey:** `Ctrl+Alt+H` to open or close the panel without switching
  input methods. Override it with `FCITX5_INK_HOTKEY`, using fcitx5 key syntax
  such as `Control+Alt+H` or `Super+i`.
- **Input method entry:** `Ink` / 墨笔, label 墨. It also appears in
  `fcitx5-configtool` if you prefer to switch by input method.

## Layout: crates/handwriting

`crates/handwriting/vendor/` is a byte-identical copy of
`msime/crates/engine/src/handwriting/`, including `features.rs`, `mod.rs`,
`model.rs`, `recognizer.rs`, and `testdata/`. It is pinned from
`metasequoiaime/msime` commit `a2365adbad` (`develop`); the model lock is
`resources/handwriting-model.lock.json`.

`crates/handwriting/src/lib.rs` is the crate root that wraps the vendor module:
it provides the `handwriting` module path, the `EngineError`/`Result` re-export
and the `paths::open_file_no_follow` seam that the upstream module expects.
There is no overlay directory: add the wrapper or a real patch only when the
upstream code cannot be used unchanged.

To re-vendor, copy the upstream directory over `crates/handwriting/vendor/`
and verify:

```sh
diff -r /path/to/msime/crates/engine/src/handwriting crates/handwriting/vendor
```

The diff must be empty.

Project code is at the root and in `crates/`:

```text
fcitx5-ink/
├── addon/       fcitx5 C++20 addon
├── crates/
│   ├── handwriting/  recognizer crate; vendor/ is upstream, byte-identical
│   ├── core/         model discovery, recognition wrapper, panel IPC
│   └── panel/        GTK4 panel
├── scripts/
├── tests/
├── PKGBUILD
└── README.md
```

## Architecture

```text
GTK4 panel (crates/panel)
  ├─ draws ink and shows candidates
  ├─ calls the vendored recognizer through core
  └─ sends {"op":"text","text":"..."} over a Unix socket
                 │
fcitx5 addon (addon) ── commitString ──> focused application
```

The addon owns the focused `InputContext` and the socket at
`$XDG_RUNTIME_DIR/fcitx5-ink/panel-input.sock` (directory `0700`, socket
`0600`, same-uid peer check). It also watches `Ctrl+Alt+H` globally through
fcitx5 key events. The panel hides itself before asking to commit so focus can
return to the editor; the addon keeps a request pending for up to 700 ms while
that focus returns.

No `PanelInputChannel.h`, IBus code, Tauri code, or msime input-method code is
compiled here. The only vendored source is `crates/handwriting/vendor/`.

## Build

Requirements: fcitx5 development headers, GTK4 development files, CMake,
Rust 1.89+, and `nlohmann-json`.

```sh
./scripts/fetch-model.sh
./scripts/build.sh            # debug; pass --release for a release panel
```

The model is the 26.8 MB zinnia `handwriting-zh_CN.model`. It is not committed;
`fetch-model.sh` verifies its size and SHA-256 against
`resources/handwriting-model.lock.json`.

## Install

```sh
sudo cmake --install build --prefix /usr
```

Installed files:

```text
/usr/bin/fcitx5-ink-panel
/usr/lib/fcitx5/libfcitx5-ink.so
/usr/share/fcitx5/addon/ink.conf
/usr/share/fcitx5/inputmethod/ink.conf
/usr/share/fcitx5-ink/handwriting-zh_CN.model
/usr/share/licenses/fcitx5-ink/
```

Restart fcitx5 after installing (`fcitx5 -d --replace` or your service
manager), then either press `Ctrl+Alt+H` or add 「墨笔」 in
`fcitx5-configtool`.

## Test

```sh
./scripts/test.sh
```

The test suite covers:

- the vendored engine's unit tests, including the Zinnia parity test when the
  model is available;
- the panel socket client's success and error paths;
- a real recognition self-test (`中` is the first candidate);
- an isolated `dbus-run-session` + `Xvfb` fcitx5 run that loads the addon,
  switches to `ink`, and verifies that the test client receives `TEXT:你`.

The integration test needs `dbus-run-session`, `Xvfb`, `xdotool`, and GTK3
development files.

## Arch packages

The root `PKGBUILD` builds this checkout as a release package. For a local
`-git` test package:

```sh
cd packaging/arch/fcitx5-ink-git
makepkg -si
```

It clones the parent repository, so commit the working tree first. For an AUR
upload, replace its `git+file://$startdir/../..` source with
`git+https://github.com/YangtseSu/fcitx5-ink.git`; the model source is already
pinned. The root `PKGBUILD` is a local-source package; an AUR release package
should use a release tarball source and checksum.

## Limits

- One character per recognition. Multi-character cell segmentation is not
  part of the vendored module.
- The panel owns focus and candidates; it does not use the fcitx5 candidate
  window.
- Wayland window placement is controlled by the compositor; the panel does not
  try to follow the text cursor.
- `handwriting-core` is GUI-independent. A future GPUI Kit frontend can reuse
  it and the same socket protocol without touching the addon.

## License

New code is GPL-3.0-or-later. The vendored handwriting module retains the
msime-engine licenses: MPL-2.0 for the engine code, BSD-3-Clause for the
Zinnia port, and GPL-3.0-only for the upstream test fixtures. The handwriting
model is LGPL-2.1-or-later; its license is installed with the model and kept in
`LICENSES/LGPL-2.1-or-later.txt`.
