#!/bin/sh

set -eu

SCRIPT_DIR=$(cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

TARGET=""
PROFILE="release"
MUSL="no"

while [ $# -gt 0 ]; do
	case "$1" in
	--target)
		TARGET="$2"
		shift 2
		;;
	--debug)
		PROFILE="debug"
		shift
		;;
	--musl)
		MUSL="yes"
		shift
		;;
	*)
		echo "unknown argument: $1" >&2
		exit 1
		;;
	esac
done

say() { printf '\n=== %s ===\n' "$*"; }

case "$TARGET" in
"" )
	RUST_TARGET=""
	GOARCH="amd64"
	SUFFIX="x86_64-unknown-linux-gnu"
	;;
*x86_64*)
	RUST_TARGET="$TARGET"
	GOARCH="amd64"
	SUFFIX="$TARGET"
	;;
*aarch64*)
	RUST_TARGET="$TARGET"
	GOARCH="arm64"
	SUFFIX="$TARGET"
	;;
*armv7*)
	RUST_TARGET="$TARGET"
	GOARCH="arm"
	SUFFIX="$TARGET"
	;;
*)
	echo "unknown target: $TARGET" >&2
	exit 1
	;;
esac

if [ -z "$RUST_TARGET" ]; then
	RUST_TARGET=$(rustc -vV | sed -n 's/^host: //p')
fi
case "$RUST_TARGET" in
*musl*) SUFFIX="$RUST_TARGET" ;;
esac

say "the frontend"
cd "$ROOT/ui"
if [ ! -d node_modules ]; then
	npm ci
fi
npm run build

say "the Go sidecar"
cd "$ROOT/sidecar"

export GOOS=linux
export GOARCH
GOARM=""
if [ "$GOARCH" = "arm" ]; then
	GOARM="7"
	export GOARM
fi
export CGO_ENABLED=0
mkdir -p "$ROOT/src-tauri/binaries"
go build -trimpath -ldflags="-s -w" \
	-o "$ROOT/src-tauri/binaries/ha-panel-sidecar-$SUFFIX" .

say "the Rust application"
cd "$ROOT/src-tauri"
if [ -n "$TARGET" ]; then
	cargo build --profile "$PROFILE" --target "$TARGET"
	BINARY="$ROOT/src-tauri/target/$TARGET/$PROFILE/ha-panel"
else
	cargo build --profile "$PROFILE"
	BINARY="$ROOT/src-tauri/target/$PROFILE/ha-panel"
fi

say "done"
echo "the application: $BINARY"
echo "the sidecar:    $ROOT/src-tauri/binaries/ha-panel-sidecar-$SUFFIX"
