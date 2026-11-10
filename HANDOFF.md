# DooTube — resume checkpoint

Updated 2026-11-11. Earlier checkpoints (including the Cinema UI of 0.3.0) are in Git history.

## Confirmed direction

- A YouTube-like native Mac app: Rust + AppKit own the window, the slim title-bar row, the Up Next drawer, menus, and the fullscreen stage. The official YouTube and YouTube Music sites run in their own persistent `WKWebView`s; no Electron, unofficial extractor, local server, or custom player.
- Page tweaks stay small and easy to drop: hidden site headers, hidden Shorts shelves, player keys, playback speed.
- Preserve the existing account session and saved queue.

## Where things are

- Published GitHub release: **0.4.3** (2026-11-11), [release page](https://github.com/Grabbitz/DooTube/releases/tag/v0.4.3). `main` matches it.
- 0.4.1 added: sign-out detection by the `SAPISID` cookie (YouTube keeps `LOGIN_INFO` after sign-out), playback speed up to 4× (⌥⌘. / ⌥⌘, / ⌥⌘0) carried over to the next video, hidden Shorts shelves, the Up Next count on the Dock icon, and no system beep for player keys the page handles (`QuietWindow` in `src/main.rs`).
- Code is one module per app area under `src/` (window, panes, sidebar, queue, page state, account, navigation, notices, lifecycle, status_item). Page scripts and CSS: `src/web.rs`, `src/youtube_chrome.css`, `src/music_chrome.css`, `src/music_player.js`.
- Current behavior and data boundaries: [docs/implementation.md](docs/implementation.md). Earlier evidence: [docs/verification.md](docs/verification.md).
- CI (GitHub Actions, macOS): format, Clippy, and tests on every push and pull request.

## Verification status

Current combined tree: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (21 passed), and `./scripts/build-app.sh` passed (exit 0). The Music menu-bar item was seen running with a Music track, and its View toggle was checked and unchecked. In the debug app, a manually clicked Up Next row opened paused at 0:00. After manually starting the first of two short queued videos, the native five-second countdown appeared and the next video opened playing with YouTube Autoplay both on and off. With Autoplay on and the second entry saved at 20 seconds, its seek position was observed at 28 seconds and the player offered Pause; with Autoplay off, it advanced from 3 to 19 seconds with Pause shown. A temporary probe forced the SPA app-element lookup to fail: the normal load-request fallback opened the next entry paused; the probe was removed before the final build. These observations cover those two videos and do not establish behavior for every link, ad, or provider change. The original state file was restored byte-identically, the initially unset auto-next preference was removed, and YouTube Autoplay was restored on.

After adding the three-second fallback check, the debug app was run again with YouTube Autoplay on: the five-second notice appeared and the next video showed Pause while its seek position advanced from 3 to 18 seconds. The saved state checksum and initially unset preference were restored; YouTube Autoplay remained on. The four checks above were rerun successfully.

0.4.1: `cargo build`, `cargo clippy`, and `cargo test` (19 pass) are clean, CI passed on `main`, and the speed-step logic was checked in Node. The DMG reports version 0.4.1. The new features were installed to `/Applications` but have not been tried by hand yet: sign-out updating the account button, speed carrying over to the next video, Shorts shelves hidden, the Dock badge, and no beep on T/F.

Known ceilings: the Shorts CSS and speed carry-over depend on YouTube's page (the carry-over keys on the video's `currentSrc`; switch to `yt-navigate-finish` if YouTube starts reusing it). Auto-next playback depends on YouTube's internal `yt-navigate` event on `ytd-app` and its watch-endpoint shape (`commandMetadata.webCommandMetadata` and `watchEndpoint`). If the app element is missing or the script fails, it falls back to a full load, which was observed opening paused. If the event is silently ignored and the source URL is still unchanged after three seconds, it also falls back to a full load; that timeout path has not been exercised at runtime.

## Possible next steps

- SponsorBlock or Return YouTube Dislike (both call an outside API; make them opt-in).

## Build and run

- `./scripts/build-app.sh [--release]`, `./scripts/run.sh [--release]`, `./scripts/install.sh --open`, `./scripts/make-dmg.sh`.
- Saved queue: `~/Library/Application Support/DooTube/state.json`. Don't overwrite the user's saved queue during testing.
- Push and release as the Grabbitz GitHub account (`gh auth switch -u Grabbitz`).
