#!/bin/sh

set -eu

SCRIPT_DIR=$(cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

ALPINE_DIGEST="sha256:644c1ad8fc45cc581ab1b5ea7ec1bebc9b037fd4801ad697ca124deb3f9c9332"
ALPINE_VERSION="3.23"

ARCH="amd64"
PLATFORM="linux/amd64"
TARGET="x86_64-alpine-linux-musl"

# The package version. It is read from the Tauri manifest, because that
# is the one version the application, the frontend and the package all
# agree on.
PKGVER="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
	"$ROOT/src-tauri/tauri.conf.json" | head -n 1)"

if [ -z "$PKGVER" ]; then
	echo "cannot read the version from src-tauri/tauri.conf.json" >&2
	exit 1
fi

while [ $# -gt 0 ]; do
	case "$1" in
	--arch)
		ARCH="$2"
		shift 2
		;;
	*)
		echo "unknown argument: $1" >&2
		exit 1
		;;
	esac
done

case "$ARCH" in
amd64)
	PLATFORM="linux/amd64"
	TARGET="x86_64-alpine-linux-musl"
	;;
arm64)
	PLATFORM="linux/arm64"
	TARGET="aarch64-alpine-linux-musl"
	;;
armv7)
	PLATFORM="linux/arm/v7"
	TARGET="armv7-alpine-linux-musleabihf"
	;;
*)
	echo "unknown architecture: $ARCH" >&2
	exit 1
	;;
esac

echo "=== building the Alpine package for $ARCH ($TARGET) ==="

mkdir -p "$ROOT/dist"

docker run --rm \
	--platform "$PLATFORM" \
	-v "$ROOT":/build:ro \
	-v "$ROOT/dist":/out \
	-w /build \
	-e "TARGET=$TARGET" \
	-e "ARCH=$ARCH" \
	-e "PKGVER=$PKGVER" \
	"alpine:$ALPINE_VERSION@$ALPINE_DIGEST" \
	sh /build/scripts/alpine-build-inner.sh

echo
echo "=== the artefacts ==="
ls -lh "$ROOT/dist" || true
