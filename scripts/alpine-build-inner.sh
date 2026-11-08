#!/bin/sh

set -eu

: "${TARGET:?TARGET is not set}"
: "${ARCH:?ARCH is not set}"
: "${PKGVER:?PKGVER is not set}"

# The Rust target triple. Alpine's Rust ships the standard library for its
# own triple — x86_64-alpine-linux-musl — and not for the generic musl
# one, so this is what the application is built for.
case "$ARCH" in
amd64) ALPINE_TARGET="x86_64-alpine-linux-musl" ;;
arm64) ALPINE_TARGET="aarch64-alpine-linux-musl" ;;
armv7) ALPINE_TARGET="armv7-alpine-linux-musleabihf" ;;
*) echo "unknown architecture: $ARCH" >&2; exit 1 ;;
esac

say() { printf '\n=== %s ===\n' "$*"; }

# The whole build happens in a copy of the tree under /tmp. The mounted
# tree is only read: a container running as root would otherwise leave
# root-owned files in it, which break the next build on the host.
WORK=/tmp/work
rm -rf "$WORK"
mkdir -p "$WORK"
cp -r /build "$WORK/ha-panel"
rm -rf "$WORK/ha-panel/ui/node_modules" "$WORK/ha-panel/ui/dist"
rm -rf "$WORK/ha-panel/src-tauri/target"
rm -rf "$WORK/ha-panel/.git"

say "the build packages"
apk add --no-cache \
	abuild \
	bash \
	build-base \
	ca-certificates \
	curl \
	tar \
	nodejs \
	npm \
	pkgconf \
	linux-headers \
	webkit2gtk-4.1-dev \
	gtk+3.0-dev

# The Go toolchain is the official one rather than Alpine's package.
#
# Alpine's Go is a release behind, and the standard library advisories that
# Trivy reports are fixed in the newer one. The official tarball is a
# static-ish build that runs on musl, so it works on Alpine unchanged.
#
# The version is pinned, so a build of the same commit produces the same
# binary.
GO_VERSION="1.26.10"
say "the Go toolchain"
curl -fsSL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" -o /tmp/go.tar.gz
tar xzf /tmp/go.tar.gz -C /tmp
export PATH="/tmp/go/bin:$PATH"
go version

say "the Rust toolchain"
# Alpine's own Rust is 1.91.1, which meets Tauri's 1.90 minimum. The
# `rustup` package is not used: on Alpine it ships only `rustup-init`,
# not `rustup`, so there is no rustup to configure.
apk add --no-cache cargo rust
rustc --version
cargo --version

say "the frontend"
cd "$WORK/ha-panel/ui"
npm ci
npm run build

say "the Go sidecar"
cd "$WORK/ha-panel/sidecar"

export CGO_ENABLED=0
export GOOS=linux
case "$ARCH" in
amd64) export GOARCH=amd64 ;;
arm64) export GOARCH=arm64 ;;
armv7) export GOARCH=arm; export GOARM=7 ;;
esac
go build -trimpath -ldflags="-s -w" \
	-o "$WORK/ha-panel/src-tauri/binaries/ha-panel-sidecar-$ALPINE_TARGET" .

say "the Rust application"
cd "$WORK/ha-panel/src-tauri"
# Alpine's Rust ships the standard library for its own triple only, so the
# build targets that rather than the generic musl one.
cargo build --release --target "$ALPINE_TARGET"
cp "target/$ALPINE_TARGET/release/ha-panel" /tmp/ha-panel

say "the source tree"
# abuild wants its source as a tarball, and it wants the checksum of that
# tarball in the APKBUILD. The tree is staged here rather than in the
# repository because it holds the build output, which is not source.
STAGE="/tmp/stage/ha-panel-$PKGVER"
rm -rf /tmp/stage
mkdir -p "$STAGE"

cp /tmp/ha-panel "$STAGE/ha-panel"
cp "$WORK/ha-panel/src-tauri/binaries/ha-panel-sidecar-$ALPINE_TARGET" \
	"$STAGE/ha-panel-sidecar"

mkdir -p "$STAGE/packaging" "$STAGE/icons" "$STAGE/docs"
cp "$WORK/ha-panel/packaging/ha-panel.desktop" "$STAGE/packaging/"
cp "$WORK/ha-panel/packaging/alpine/ha-panel.initd" "$STAGE/packaging/"
cp "$WORK/ha-panel/packaging/alpine/ha-panel.pre-install" "$STAGE/packaging/"
cp "$WORK/ha-panel/LICENSE" "$STAGE/"
cp "$WORK/ha-panel/README.md" "$STAGE/"
for size in 32 128 256 512; do
	cp "$WORK/ha-panel/src-tauri/icons/${size}x${size}.png" "$STAGE/icons/"
done
for doc in architecture configuration installation; do
	cp "$WORK/ha-panel/docs/$doc.md" "$STAGE/docs/"
done

# The default configuration. It is a separate file so that the package
# can own it, and so that an upgrade does not overwrite the settings on
# the device.
cat >"$STAGE/etc-ha-panel-config.json" <<'EOF'
{
  "ha": {
    "url": "http://homeassistant.local:8123",
    "dashboard_path": "/lovelace",
    "fullscreen": true,
    "reload_on_reconnect": true
  },
  "kiosk": {
    "handle_width": 12,
    "drawer_width": 380,
    "show_handle": true,
    "on_screen_keyboard": true,
    "screen_off_timeout_secs": 0,
    "screen_on_timeout_secs": 0
  },
  "audio": {
    "backend": "auto",
    "volume": 50,
    "muted": false
  },
  "display": {
    "screen_power": "auto",
    "brightness": null
  },
  "network": {
    "wifi_enabled": true
  },
  "echomuse": {
    "enabled": false
  }
}
EOF

say "the package"
# abuild refuses to run as root, so the packaging runs as an ordinary
# user. The build itself already ran; this is only the packaging step.
#
# The builder has to be in the abuild group, because abuild creates the
# package's users and groups as that user.
adduser -D -u 1000 builder 2>/dev/null || true
addgroup builder abuild 2>/dev/null || true

export HOME=/tmp/build-home
mkdir -p "$HOME"
chown -R builder:builder "$HOME"

# abuild signs the package, and it needs a key. A throwaway key is
# generated for the build: the package is signed so that it installs, not
# so that it is trusted by anybody. A release is signed with the
# project's own key instead.
say "the packaging key"
su builder -c "abuild-keygen -n -a" >/dev/null 2>&1 || true
su builder -c "abuild-keygen -n -i" >/dev/null 2>&1 || true
cp /home/builder/.abuild/*.rsa.pub /etc/apk/keys/ 2>/dev/null || true

PKGDIR="/tmp/pkg"
rm -rf "$PKGDIR"
mkdir -p "$PKGDIR"
cp "$WORK/ha-panel/packaging/alpine/APKBUILD" "$PKGDIR/"

# The install script has to sit next to the APKBUILD: abuild looks for
# it at the path the APKBUILD names, relative to the directory the
# APKBUILD is in.
cp "$WORK/ha-panel/packaging/alpine/ha-panel.pre-install" "$PKGDIR/"

# The source tarball, named the way abuild expects: pkgname-pkgver.
tar czf "$PKGDIR/ha-panel-$PKGVER.tar.gz" -C /tmp/stage "ha-panel-$PKGVER"

# The checksum of that tarball, which abuild verifies before it builds.
HASH="$(sha512sum "$PKGDIR/ha-panel-$PKGVER.tar.gz" | cut -d' ' -f1)"
printf 'sha512sums="%s  ha-panel-%s.tar.gz"\n' "$HASH" "$PKGVER" \
	>>"$PKGDIR/APKBUILD"

chown -R builder:builder "$PKGDIR"

cd "$PKGDIR"
su builder -c "abuild -r -d" 2>&1 | sed 's/^/  /'

find /home/builder/packages -name '*.apk' -exec cp {} /out/ \;

say "the result"
ls -lh /out
