# Maintainer: Yangtse Su <yangtsesu@gmail.com>
# Local-source PKGBUILD: run makepkg in this checkout. For an AUR upload,
# replace the build from $startdir with a release tarball source and pin its
# checksum. The model URL is already pinned by resources/handwriting-model.lock.json.

pkgname=fcitx5-ink
pkgver=0.1.0
pkgrel=1
pkgdesc="Handwriting input method for fcitx5 with a GTK4 panel"
arch=('x86_64' 'aarch64')
url="https://github.com/YangtseSu/fcitx5-ink"
license=('GPL-3.0-or-later' 'MPL-2.0' 'BSD-3-Clause' 'LGPL-2.1-or-later')
depends=('fcitx5' 'gtk4' 'cairo' 'pango' 'glib2')
makedepends=('cmake' 'rust' 'nlohmann-json' 'pkgconf' 'python')
optdepends=('fcitx5-configtool: add Ink to the input method group')
source=("handwriting-zh_CN.model::https://raw.githubusercontent.com/metasequoiaime/msime-engine/a9b9f092219505166c927843762b650d1e0e501d/handwriting/models/handwriting-zh_CN.model")
sha256sums=('e16153d1ff267cd479aea260d6f71a3edda8b4ba06db2d121513adda65a4449e')

prepare() {
  mkdir -p "$srcdir/model"
  cp -L "$srcdir/handwriting-zh_CN.model" "$srcdir/model/handwriting-zh_CN.model"
}

build() {
  cd "$startdir"

  CARGO_TARGET_DIR="$srcdir/target" cargo build --release --workspace

  cmake -S . -B "$srcdir/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DFCITX5_INK_PANEL_BINARY="$srcdir/target/release/fcitx5-ink-panel" \
    -DFCITX5_INK_MODEL="$srcdir/model/handwriting-zh_CN.model"
  cmake --build "$srcdir/build" -j"$(nproc)"
}

check() {
  cd "$startdir"
  MSIME_HANDWRITING_MODEL="$srcdir/model/handwriting-zh_CN.model" \
    CARGO_TARGET_DIR="$srcdir/target" \
    cargo test --workspace
}

package() {
  DESTDIR="$pkgdir" cmake --install "$srcdir/build"
}
