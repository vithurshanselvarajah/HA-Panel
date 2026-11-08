#!/bin/sh

set -eu

: "${TARGET:?TARGET is not set}"
: "${ARCH:?ARCH is not set}"

say() { printf '\n=== %s ===\n' "$*"; }

say "the build packages"
apk add --no-cache \
	bash \
	build-base \
	ca-certificates \
	cargo \
	go \
	nodejs \
	npm \
	pkgconf \
	rustup \
	linux-headers \
	webkit2gtk-4.1-dev \
	gtk+3.0-dev

say "the Rust toolchain"
RUSTUP_TOOLCHAIN="1.91.1"
export RUSTUP_HOME=/usr/local/rustup
export CARGO_HOME=/usr/local/cargo
rustup default "$RUSTUP_TOOLCHAIN" >/dev/null
rustc --version

say "the frontend"
cd /build/ui
npm ci
npm run build

say "the Go sidecar"
cd /build/sidecar

export CGO_ENABLED=0
export GOOS=linux
case "$ARCH" in
amd64) export GOARCH=amd64 ;;
arm64) export GOARCH=arm64 ;;
armv7) export GOARCH=arm; export GOARM=7 ;;
esac
mkdir -p /build/src-tauri/binaries
go build -trimpath -ldflags="-s -w" \
	-o "/build/src-tauri/binaries/ha-panel-sidecar-$TARGET" .

say "the Rust application"
cd /build/src-tauri
cargo build --release --target "$TARGET"

say "the package"
cd /build/packaging/alpine

sed -i 's|^sha512sums=.*|sha512sums="SKIP 0"|' APKBUILD

export HOME=/tmp/build-home
mkdir -p "$HOME"

rm -rf /tmp/apk-build
mkdir -p /tmp/apk-build
cp -r /build /tmp/apk-build/ha-panel

cd /tmp/apk-build/ha-panel/packaging/alpine
abuild -r -d 2>&1 | sed 's/^/  /'

find /tmp/apk-build -name '*.apk' -exec cp {} /out/ \;

say "the result"
ls -lh /out
