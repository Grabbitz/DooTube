# DooTube — resume checkpoint

Updated 2026-10-24. Earlier checkpoint (2026-10-23) is in Git history.

## Confirmed direction

- Simple, native Mac UI (“เรียบแบบ mac”), with advanced tools tucked away.
- Make YouTube Watch smooth first, then focus on Music depth. Smoothness versus Kaset is a goal, not a measured result.
- Rust + AppKit native chrome around the official YouTube and YouTube Music websites in `WKWebView`; no Electron, unofficial extractor, or custom player.

## Where things are

- Released: 0.2.3 (see [CHANGELOG.md](CHANGELOG.md)):
  - Now Playing card for YouTube Music in the sidebar and a Controls menu (⌘P, ⌘→, ⌘←).
  - Up Next thumbnails and drag-to-reorder (drag out gives the video link).
  - Saved videos resume where they were left (`t=` on open; position reported by the page script).
  - Dock menu with Music track and controls.
- Shipped behavior and data boundaries: [docs/implementation.md](docs/implementation.md). Recorded test evidence: [docs/verification.md](docs/verification.md).

## Not yet verified on a Mac

The 0.2.3 items were checked by hand on a Mac on 2026-10-24 (Up Next thumbnails and dragging, resume, Dock menu, media keys and Control Center).

Still unverified from earlier: real playback progression and audio, seeking, smoothness versus Kaset, network recovery.

## Possible next steps

- Optional auto-play of the next Up Next item (off by default, in a Settings window).
- Menu bar mini player for Music.
- Now Playing card for a YouTube video while the Music section is shown.

## Build and run

- `./scripts/build-app.sh [--release]`, `./scripts/run.sh [--release]`, `./scripts/install.sh --open`, `./scripts/make-dmg.sh`.
- Saved queue: `~/Library/Application Support/DooTube/state.json`. Don't overwrite the user's saved queue during testing.
