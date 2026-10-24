#!/bin/bash
# Build, package, and launch the owned app through Launch Services.
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
PROFILE=debug
if [[ "${1:-}" == --release ]]; then
    PROFILE=release
    shift
fi
if (( $# > 0 )); then
    if [[ "$1" != -- ]]; then
        printf 'Usage: %s [--release] [-- search words or YouTube URL]\n' "$0" >&2
        exit 2
    fi
    shift
fi
if [[ "$PROFILE" == release ]]; then
    "$ROOT/scripts/build-app.sh" --release
else
    "$ROOT/scripts/build-app.sh"
fi
# Reuse the existing application rather than forcing another player instance.
# Launch Services forwards startup arguments only when starting a new process.
if (( $# > 0 )); then
    exec /usr/bin/open "$ROOT/target/$PROFILE/DooTube.app" --args "$@"
fi
exec /usr/bin/open "$ROOT/target/$PROFILE/DooTube.app"
