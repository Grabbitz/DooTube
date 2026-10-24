#!/bin/bash
# Build and install a local, ad-hoc-signed app in Applications.
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OPEN=false
if (( $# > 0 )); then
    if (( $# != 1 )) || [[ "$1" != --open ]]; then
        printf 'Usage: %s [--open]\n' "$0" >&2
        exit 2
    fi
    OPEN=true
fi

"$ROOT/scripts/build-app.sh" --release

if /usr/bin/pgrep -f 'DooTube.app/Contents/MacOS/dootube' >/dev/null; then
    if ! /usr/bin/osascript -e 'quit app "DooTube"'; then
        printf 'Could not quit DooTube. Please quit it and run this installer again.\n' >&2
        exit 1
    fi
    for (( attempt=0; attempt<100; attempt++ )); do
        if ! /usr/bin/pgrep -f 'DooTube.app/Contents/MacOS/dootube' >/dev/null; then
            break
        fi
        /bin/sleep 0.1
    done
    if /usr/bin/pgrep -f 'DooTube.app/Contents/MacOS/dootube' >/dev/null; then
        printf 'DooTube is still running. Please quit it and run this installer again.\n' >&2
        exit 1
    fi
fi

APP=/Applications/DooTube.app
STAGING="$(/usr/bin/mktemp -d /Applications/.DooTube.install.XXXXXX)"
trap '/bin/rm -rf "$STAGING"' EXIT
/usr/bin/ditto "$ROOT/target/release/DooTube.app" "$STAGING/DooTube.app"
/bin/rm -rf "$APP"
/bin/mv "$STAGING/DooTube.app" "$APP"
/usr/bin/codesign --verify --strict "$APP"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP"
printf 'Installed %s\n' "$APP"

if [[ "$OPEN" == true ]]; then
    /usr/bin/open "$APP"
fi
