# DooTube

<img src="resources/icon/preview-512.png" width="96" align="right" alt="DooTube icon">

A native Mac app for YouTube and YouTube Music. Rust + AppKit chrome around the official websites. No Electron, no custom player.

## Download

Get **DooTube.dmg** from the [latest release](../../releases/latest) and drag it to Applications. Requires an Apple silicon Mac with macOS 14.2 or later.

The app isn't notarized, so macOS blocks the first launch. Open System Settings → Privacy & Security and click **Open Anyway**, or run:

```sh
xattr -dr com.apple.quarantine /Applications/DooTube.app
```

## Known limitations

- **Apple silicon only**, macOS 14.2 or later.
- **Not notarized**: macOS blocks the first launch (see above).
- **No passkeys**: they can't be used in embedded web views, so Google asks for your password or another verification method instead.
- **Depends on YouTube's page layout**: a YouTube redesign can break the hidden headers and other page tweaks until DooTube is updated.

## Features

- **YouTube and Music** in separate sections (⌘1 / ⌘2). Only one plays at a time.
- **Opens straight into YouTube** with its own feed and sidebar, like youtube.com.
- **One slim bar** in the title bar row: Back/Forward, YouTube/Music, search, Save, PiP, Up Next, and your account. The official players remain unchanged.
- **Google sign-in** in the app. Your existing WebKit session is retained.
- **Up Next**: save videos with ⌘S, then reorder or remove them in the on-demand drawer.
- **Now Playing**: the drawer offers Music controls while browsing YouTube. The Controls and Dock menus remain available.
- **Video full screen**: press F and the video zooms out to fill the screen. Press F or Esc to zoom back.
- **Picture in Picture** (⌃⌘P). Keeps playing while the app is hidden.
- **Playback speed up to 4×** (⌥⌘. / ⌥⌘,), kept for the next video. ⌥⌘0 returns to normal.
- **No Shorts shelves** in Home, Subscriptions, or search. The guide's Shorts entry still opens them.
- **Dock badge** shows how many videos are in Up Next.

| Shortcut | Action |
| --- | --- |
| ⌘L | Search |
| ⌘[ / ⌘] | Back / Forward |
| ⇧⌘H | Home |
| ⌘R | Reload |
| ⌘P | Music: Play / Pause |
| ⌘← / ⌘→ | Music: Previous / Next |
| ⌃⌘S | Toggle Up Next |
| ⌃⌘F | Window full screen |
| ⌥⌘. / ⌥⌘, | Speed up / Slow down |
| ⌥⌘0 | Normal speed |

## Build from source

Requires Rust and Xcode Command Line Tools.

```sh
./scripts/run.sh            # debug build and launch
./scripts/install.sh --open # optimized build into /Applications
./scripts/make-dmg.sh       # target/DooTube.dmg
```

## More

[Implementation](docs/implementation.md) · [Verification](docs/verification.md) · [Changelog](CHANGELOG.md) · [MIT License](LICENSE)
