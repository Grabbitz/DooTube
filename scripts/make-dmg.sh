#!/bin/bash
# Build the optimized app and package it as a drag-to-Applications disk image.
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
"$ROOT/scripts/build-app.sh" --release

DMG="$ROOT/target/DooTube.dmg"
STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT
/usr/bin/ditto "$ROOT/target/release/DooTube.app" "$STAGING/DooTube.app"
ln -s /Applications "$STAGING/Applications"
rm -f "$DMG"
/usr/bin/hdiutil create -volname DooTube -srcfolder "$STAGING" -fs HFS+ -format UDZO -ov "$DMG"
/usr/bin/hdiutil verify "$DMG"
echo "$DMG"
