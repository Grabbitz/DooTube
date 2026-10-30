# DooTube — resume checkpoint

Updated 2026-10-31. Earlier checkpoints (including the Cinema UI of 0.3.0) are in Git history.

## Confirmed direction

- A YouTube-like native Mac app: Rust + AppKit own the window, the slim title-bar row, the Up Next drawer, menus, and the fullscreen stage. The official YouTube and YouTube Music sites run in their own persistent `WKWebView`s; no Electron, unofficial extractor, local server, or custom player.
- Page tweaks stay small and easy to drop: hidden site headers, hidden Shorts shelves, player keys, playback speed.
- Preserve the existing account session and saved queue.

## Where things are

- Published GitHub release: **0.4.1** (2026-10-31), [release page](https://github.com/Grabbitz/DooTube/releases/tag/v0.4.1). `main` matches it; nothing unreleased.
- 0.4.1 added: sign-out detection by the `SAPISID` cookie (YouTube keeps `LOGIN_INFO` after sign-out), playback speed up to 4× (⌥⌘. / ⌥⌘, / ⌥⌘0) carried over to the next video, hidden Shorts shelves, the Up Next count on the Dock icon, and no system beep for player keys the page handles (`QuietWindow` in `src/main.rs`).
- Code is one module per app area under `src/` (window, panes, sidebar, queue, page state, account, navigation, notices, lifecycle). Page scripts and CSS: `src/web.rs`, `src/youtube_chrome.css`, `src/music_chrome.css`, `src/music_player.js`.
- Current behavior and data boundaries: [docs/implementation.md](docs/implementation.md). Earlier evidence: [docs/verification.md](docs/verification.md).
- CI (GitHub Actions, macOS): format, Clippy, and tests on every push and pull request.

## Verification status

0.4.1: `cargo build`, `cargo clippy`, and `cargo test` (19 pass) are clean, CI passed on `main`, and the speed-step logic was checked in Node. The DMG reports version 0.4.1. The new features were installed to `/Applications` but have not been tried by hand yet: sign-out updating the account button, speed carrying over to the next video, Shorts shelves hidden, the Dock badge, and no beep on T/F.

Known ceilings: the Shorts CSS and speed carry-over depend on YouTube's page (the carry-over keys on the video's `currentSrc`; switch to `yt-navigate-finish` if YouTube starts reusing it).

## Possible next steps

- SponsorBlock or Return YouTube Dislike (both call an outside API; make them opt-in).
- Optional auto-play of the next Up Next item.
- Menu bar mini player for Music.

## Build and run

- `./scripts/build-app.sh [--release]`, `./scripts/run.sh [--release]`, `./scripts/install.sh --open`, `./scripts/make-dmg.sh`.
- Saved queue: `~/Library/Application Support/DooTube/state.json`. Don't overwrite the user's saved queue during testing.
- Push and release as the Grabbitz GitHub account (`gh auth switch -u Grabbitz`).
