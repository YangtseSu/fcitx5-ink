# Maintainer: Yangtse Su <yangtsesu@gmail.com>
# Local-source PKGBUILD: run makepkg in this checkout. For an AUR upload,
# replace the build from $startdir with a release tarball source and pin its
# checksum. The model archives are pinned by resources/handwriting-model.lock.json.

pkgname=fcitx5-ink
pkgver=0.1.0
pkgrel=1
pkgdesc="Handwriting input tool for fcitx5 with a GTK4 panel"
arch=('x86_64' 'aarch64')
url="https://github.com/YangtseSu/fcitx5-ink"
license=('GPL-3.0-or-later' 'MPL-2.0' 'BSD-3-Clause' 'LGPL-2.1-or-later')
depends=('fcitx5' 'gtk4' 'cairo' 'pango' 'glib2')
makedepends=('cmake' 'rust' 'nlohmann-json' 'tomlplusplus' 'pkgconf' 'python')
source=(
  "tegaki-zinnia-simplified-chinese-0.3.zip::https://github.com/tegaki/tegaki/releases/download/v0.3/tegaki-zinnia-simplified-chinese-0.3.zip"
  "tegaki-zinnia-traditional-chinese-0.3.zip::https://github.com/tegaki/tegaki/releases/download/v0.3/tegaki-zinnia-traditional-chinese-0.3.zip"
  "tegaki-zinnia-japanese-0.3.zip::https://github.com/tegaki/tegaki/releases/download/v0.3/tegaki-zinnia-japanese-0.3.zip"
  "HandwritingModel-LICENSE.txt::https://raw.githubusercontent.com/tegaki/tegaki/7a74e442c4130cccc226a7e7c2b683ac94c0cccb/tegaki-models/LGPL"
)
sha256sums=(
  '45bb61a9a5e1ff571de9ac0711c316f29a064260feea6f5db6979ecd980498a3'
  'f41032e67a4eff056813d243eabc32ea07eea404c714bdb5882a5b0fcda51690'
  '07861bbb5440d74e705864721f12a5ce1c7f21a869f9c95ff21433d8994aafda'
  'a190dc9c8043755d90f8b0a75fa66b9e42d4af4c980bf5ddc633f0124db3cee7'
)

prepare() {
  mkdir -p "$srcdir/model"
  bsdtar -xOf "$srcdir/tegaki-zinnia-simplified-chinese-0.3.zip" \
    "tegaki-zinnia-simplified-chinese-0.3/handwriting-zh_CN.model" \
    > "$srcdir/model/handwriting-zh_CN.model"
  bsdtar -xOf "$srcdir/tegaki-zinnia-traditional-chinese-0.3.zip" \
    "tegaki-zinnia-traditional-chinese-0.3/handwriting-zh_TW.model" \
    > "$srcdir/model/handwriting-zh_TW.model"
  bsdtar -xOf "$srcdir/tegaki-zinnia-japanese-0.3.zip" \
    "tegaki-zinnia-japanese-0.3/handwriting-ja.model" \
    > "$srcdir/model/handwriting-ja.model"
  cp "$srcdir/HandwritingModel-LICENSE.txt" "$srcdir/model/"
}

build() {
  cd "$startdir"

  CARGO_TARGET_DIR="$srcdir/target" cargo build --release --workspace

  cmake -S . -B "$srcdir/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DFCITX5_INK_PANEL_BINARY="$srcdir/target/release/fcitx5-ink-panel" \
    -DFCITX5_INK_MODEL_DIR="$srcdir/model"
  cmake --build "$srcdir/build" -j"$(nproc)"
}

check() {
  cd "$startdir"
  MSIME_HANDWRITING_MODEL="$srcdir/model/handwriting-zh_CN.model" \
    FCITX5_INK_ZH_CN_MODEL="$srcdir/model/handwriting-zh_CN.model" \
    FCITX5_INK_ZH_TW_MODEL="$srcdir/model/handwriting-zh_TW.model" \
    FCITX5_INK_JA_MODEL="$srcdir/model/handwriting-ja.model" \
    CARGO_TARGET_DIR="$srcdir/target" \
    cargo test --workspace
  ctest --test-dir "$srcdir/build" --output-on-failure
  for language in zh_CN zh_TW ja; do
    case "$language" in
      zh_CN) model=handwriting-zh_CN.model ;;
      zh_TW) model=handwriting-zh_TW.model ;;
      ja) model=handwriting-ja.model ;;
    esac
    FCITX5_INK_MODEL="$srcdir/model/$model" \
      "$srcdir/target/release/fcitx5-ink-panel" --self-test --language "$language"
  done
}

package() {
  DESTDIR="$pkgdir" cmake --install "$srcdir/build"
}
