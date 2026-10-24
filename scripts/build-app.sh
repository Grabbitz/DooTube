#!/bin/bash
# Build a local, ad-hoc-signed native macOS application bundle.
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
PROFILE=debug
case "${1:-}" in
    "") ;;
    --release) PROFILE=release ;;
    *) printf 'Usage: %s [--release]\n' "$0" >&2; exit 2 ;;
esac
if (( $# > 1 )); then
    printf 'Usage: %s [--release]\n' "$0" >&2
    exit 2
fi
if [[ "$(/usr/bin/uname -s)" != Darwin ]]; then
    printf 'DooTube requires macOS and the Apple Command Line Tools.\n' >&2
    exit 1
fi

# Keep the linked binary and bundle minimum in sync. The SDK can be newer.
export MACOSX_DEPLOYMENT_TARGET=14.2
TARGET_DIR="$ROOT/target"
APP="$TARGET_DIR/$PROFILE/DooTube.app"

cargo build --locked --manifest-path "$ROOT/Cargo.toml" \
    --target-dir "$TARGET_DIR" "$@"
VERSION="$(cargo metadata --locked --no-deps --format-version 1 \
    --manifest-path "$ROOT/Cargo.toml" | \
    /usr/bin/plutil -extract packages.0.version raw -o - -)"

/usr/bin/install -d "$APP/Contents/MacOS" "$APP/Contents/Resources"
/usr/bin/install -m 755 "$TARGET_DIR/$PROFILE/dootube" "$APP/Contents/MacOS/dootube"
/usr/bin/install -m 644 "$ROOT/resources/DooTube.icns" "$APP/Contents/Resources/DooTube.icns"
/usr/bin/install -m 644 "$ROOT/resources/Info.plist" "$APP/Contents/Info.plist"
/usr/bin/plutil -replace CFBundleShortVersionString -string "$VERSION" "$APP/Contents/Info.plist"
/usr/bin/plutil -lint "$APP/Contents/Info.plist"
/usr/bin/codesign --force --sign - --timestamp=none "$APP"
printf '%s\n' "$APP"
