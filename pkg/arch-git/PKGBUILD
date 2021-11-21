# Maintainer: Nicolas Stalder <n+archlinux@stalder.io>
pkgname=solo2-cli-git
pkgver=r18.221f4d1
pkgrel=1
pkgdesc="Command line interface to SoloKeys Solo 2 devices"
url="https://github.com/solokeys/solo2-cli"
arch=(any)
license=(MIT)
depends=(rust)
provides=(solo2-cli)

source=('git+https://github.com/solokeys/solo2-cli.git#branch=main')
# add dummy entries for `make generate-checksums` to create SHA256 instead of MD5 check sums
sha256sums=('SKIP')

pkgver() {
  cd "$srcdir/solo2-cli"
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

prepare() {
  true
}

pkg=solo2-cli

build() {
  cd $srcdir/solo2-cli
  cargo build --release
}

package() {
  install -Dm755 "$srcdir/solo2-cli/target/release/solo2" -T "$pkgdir/usr/bin/solo2"

  _src=$srcdir/$pkg
  install -Dm644 "$_src/LICENSE-MIT" "$pkgdir/usr/share/licenses/$pkgname/LICENSE-MIT"

  install -Dm644 $_src/target/release/_solo2 -t $pkgdir/usr/share/zsh/site-functions
  install -Dm644 $_src/target/release/solo2.bash $pkgdir/usr/share/bash-completion/completions/solo2
}
