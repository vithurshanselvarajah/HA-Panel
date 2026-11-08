#!/bin/sh

set -eu

SCRIPT_DIR=$(cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

ALPINE_VERSION="3.23"

# The image is pinned by digest, but a digest names one platform's
# manifest, not the index that holds all of them. The digests are
# therefore listed per architecture: pulling the amd64 digest on an
# arm64 host fails with "does not provide the specified platform".
#
# These are the digests of the manifests that alpine:3.23 resolves to.
ALPINE_AMD64_DIGEST="sha256:644c1ad8fc45cc581ab1b5ea7ec1bebc9b037fd4801ad697ca124deb3f9c9332"
ALPINE_ARM64_DIGEST="sha256:5dda816445facf5f7282dc7689a333278834c74265ba25db2e2eb90509e3a9dd"
ALPINE_ARMV7_DIGEST="sha256:a69938a920926e3ddb659f68a1432d1bcda694b411e4352b1e7a16aff20f5f05"

ARCH="amd64"
PLATFORM="linux/amd64"
TARGET="x86_64-alpine-linux-musl"
ALPINE_DIGEST="$ALPINE_AMD64_DIGEST"

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
        ALPINE_DIGEST="$ALPINE_AMD64_DIGEST"
        ;;
arm64)
        PLATFORM="linux/arm64"
        TARGET="aarch64-alpine-linux-musl"
        ALPINE_DIGEST="$ALPINE_ARM64_DIGEST"
        ;;
armv7)
        PLATFORM="linux/arm/v7"
        TARGET="armv7-alpine-linux-musleabihf"
        ALPINE_DIGEST="$ALPINE_ARMV7_DIGEST"
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
