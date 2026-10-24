# DooTube — resume checkpoint

Updated 2026-10-24. Earlier checkpoint (2026-10-23) is in Git history.

## Confirmed direction

- Simple, native Mac UI (“เรียบแบบ mac”), with advanced tools tucked away.
- Make YouTube Watch smooth first, then focus on Music depth. Smoothness versus Kaset is a goal, not a measured result.
- Rust + AppKit native chrome around the official YouTube and YouTube Music websites in `WKWebView`; no Electron, unofficial extractor, or custom player.

## Where things are

- Released: 0.2.2 (see [CHANGELOG.md](CHANGELOG.md)). Video full screen on F, Home button, toolbar on the page's background, web-process crash reload.
- Unreleased (Now Playing and Controls are on `main`; the rest is on branch `claude/check-latest-work-67m5zl`, not merged yet):
  - Now Playing card for YouTube Music in the sidebar and a Controls menu (⌘P, ⌘→, ⌘←).
  - Up Next thumbnails and drag-to-reorder (drag out gives the video link).
  - Saved videos resume where they were left (`t=` on open; position reported by the page script).
  - Dock menu with Music track and controls.
- Shipped behavior and data boundaries: [docs/implementation.md](docs/implementation.md). Recorded test evidence: [docs/verification.md](docs/verification.md).

## Not yet verified on a Mac

The unreleased items above were written in a Linux session: `cargo check`/`clippy --tests` for `aarch64-apple-darwin` pass with no warnings, and the `core.rs` tests pass (copied into a standalone crate, since the app crate only builds on macOS). Nothing has been built into an app, launched, or looked at. Check on a Mac:

1. `cargo test`, then `./scripts/run.sh`.
2. Up Next: thumbnails appear, row height looks right, dragging reorders and survives relaunch, dragging into Safari opens the link.
3. Resume: save a video, watch past 0:10, go elsewhere, open it from Up Next — it should start near where you left. Check that an ad doesn't overwrite the position.
4. Dock menu: right-click the Dock icon while Music plays; Play/Pause, Next, Previous work (also while the window is minimized — WebKit playback is suspended then, so they may not).
5. Media keys / Control Center next and previous for Music (never tested).

Still unverified from earlier: real playback progression and audio, seeking, smoothness versus Kaset, network recovery.

## Possible next steps

- Optional auto-play of the next Up Next item (off by default, in a Settings window).
- Menu bar mini player for Music.
- Now Playing card for a YouTube video while the Music section is shown.

## Build and run

- `./scripts/build-app.sh [--release]`, `./scripts/run.sh [--release]`, `./scripts/install.sh --open`, `./scripts/make-dmg.sh`.
- Saved queue: `~/Library/Application Support/DooTube/state.json`. Don't overwrite the user's saved queue during testing.
