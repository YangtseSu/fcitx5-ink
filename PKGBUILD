# Maintainer: Yangtse Su <yangtsesu@gmail.com>
# Local-source PKGBUILD: run makepkg in this checkout. For an AUR upload,
# replace the build from $startdir with a release tarball source and pin its
# checksum. The separate fcitx5-ink-data package pins the handwriting models.

pkgname=fcitx5-ink
pkgver=0.1.0
pkgrel=2
pkgdesc="Handwriting input tool for fcitx5 with a GTK4 panel"
arch=('x86_64' 'aarch64')
url="https://github.com/YangtseSu/fcitx5-ink"
license=('GPL-3.0-or-later' 'MPL-2.0' 'BSD-3-Clause')
depends=('fcitx5' 'gtk4' 'cairo' 'pango' 'glib2' 'fcitx5-ink-data')
makedepends=('cmake' 'rust' 'nlohmann-json' 'tomlplusplus' 'pkgconf')
source=()
sha256sums=()

build() {
  cd "$startdir"

  CARGO_TARGET_DIR="$srcdir/target" cargo build --release --workspace

  cmake -S . -B "$srcdir/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DFCITX5_INK_PANEL_BINARY="$srcdir/target/release/fcitx5-ink-panel" \
    -DFCITX5_INK_MODEL_DIR=
  cmake --build "$srcdir/build" -j"$(nproc)"
}

check() {
  cd "$startdir"
  model_dir=/usr/share/fcitx5-ink
  if [ -f "$model_dir/handwriting-zh_CN.model" ] \
    && [ -f "$model_dir/handwriting-zh_TW.model" ] \
    && [ -f "$model_dir/handwriting-ja.model" ]; then
    export MSIME_HANDWRITING_MODEL="$model_dir/handwriting-zh_CN.model"
    export FCITX5_INK_ZH_CN_MODEL="$model_dir/handwriting-zh_CN.model"
    export FCITX5_INK_ZH_TW_MODEL="$model_dir/handwriting-zh_TW.model"
    export FCITX5_INK_JA_MODEL="$model_dir/handwriting-ja.model"
  fi
  CARGO_TARGET_DIR="$srcdir/target" cargo test --workspace
  ctest --test-dir "$srcdir/build" --output-on-failure
  if [ -f "$model_dir/handwriting-zh_CN.model" ] \
    && [ -f "$model_dir/handwriting-zh_TW.model" ] \
    && [ -f "$model_dir/handwriting-ja.model" ]; then
    for language in zh_CN zh_TW ja; do
      case "$language" in
        zh_CN) model=handwriting-zh_CN.model ;;
        zh_TW) model=handwriting-zh_TW.model ;;
        ja) model=handwriting-ja.model ;;
      esac
      FCITX5_INK_MODEL="$model_dir/$model" \
        "$srcdir/target/release/fcitx5-ink-panel" --self-test --language "$language"
    done
  fi
}

package() {
  DESTDIR="$pkgdir" cmake --install "$srcdir/build"
}
