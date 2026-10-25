# DooTube

<img src="resources/icon/preview-512.png" width="96" align="right" alt="DooTube icon">

A native Mac app for YouTube and YouTube Music. Rust + AppKit chrome around the official websites. No Electron, no custom player.

## Download

Get **DooTube.dmg** from the [latest release](../../releases/latest) and drag it to Applications. Requires an Apple silicon Mac with macOS 14.2 or later.

The published GitHub release is **0.2.3**. The Cinema features below describe **0.3.0**, which passed local checks and is installed locally at `/Applications/DooTube.app`; no 0.3.0 public download has been published.

The app isn't notarized, so macOS blocks the first launch. Open System Settings → Privacy & Security and click **Open Anyway**, or run:

```sh
xattr -dr com.apple.quarantine /Applications/DooTube.app
```

## Features

- **YouTube and Music** in separate sections (⌘1 / ⌘2). Only one plays at a time.
- **Cinema Home**: Netflix-inspired typography, real current-media artwork, and a shelf of your saved videos. No demo catalog or custom player.
- **Dark navigation bar** for YouTube/Music, search, your account, and Up Next; the official players remain unchanged.
- **Google sign-in** in the app. Your existing WebKit session is retained.
- **Up Next**: save videos with ⌘S, then reorder or remove them in the on-demand drawer.
- **Now Playing**: Cinema Home shows Music metadata, and the drawer offers Music controls while browsing YouTube. The Controls and Dock menus remain available.
- **Video full screen**: press F and the video zooms out to fill the screen. Press F or Esc to zoom back.
- **Picture in Picture** (⌃⌘P). Keeps playing while the app is hidden.

| Shortcut | Action |
| --- | --- |
| ⌘L | Search |
| ⌘[ / ⌘] | Back / Forward |
| ⇧⌘H | Cinema Home |
| ⌘R | Reload |
| ⌘P | Music: Play / Pause |
| ⌘← / ⌘→ | Music: Previous / Next |
| ⌃⌘S | Toggle Up Next |
| ⌃⌘F | Window full screen |

Passkeys can't be used in embedded web views, so Google asks for your password or another verification method instead.

## Build from source

Requires Rust and Xcode Command Line Tools.

```sh
./scripts/run.sh            # debug build and launch
./scripts/install.sh --open # optimized build into /Applications
./scripts/make-dmg.sh       # target/DooTube.dmg
```

## More

[Implementation](docs/implementation.md) · [Verification](docs/verification.md) · [Changelog](CHANGELOG.md)
