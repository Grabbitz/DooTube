# DooTube — resume checkpoint

Updated 2026-10-25. Earlier checkpoints are in Git history.

## Confirmed direction

- Integrate the approved Netflix-inspired Cinema UI into the real native Mac app, not just the reference preview.
- Rust + AppKit owns the window, native dark Cinema bar, Up Next drawer, menus, and existing fullscreen stage. The official YouTube and YouTube Music players remain in their persistent `WKWebView`s; no Electron, unofficial extractor, local server, or custom player.
- Cinema Home is a third, bundled nonpersistent web view. It shows actual current-media and queue state, including an intentional empty state, not a demo catalog.
- Preserve the existing account session and saved queue. Smoothness versus Kaset remains a goal, not a measured result.

## Where things are

- Published GitHub release: **0.2.3**. No 0.3.0 public download has been published.
- Current local version: **0.3.0**, dated **2026-10-25**, installed and running from `/Applications/DooTube.app`. Cinema integration, the real-app method smoke, instrumentation cleanup, final post-cleanup checks, release installation, and strict signature verification are complete. The release and installed executable share SHA-256 `2e7044af37f38b3692b8fd9241c41ac858c24be48bef4c11290b7b67484a8afd`; the bundle identifier remains `com.dootube.app`.
- The native Cinema bar replaces the standard toolbar; the 340-point Up Next drawer replaces the persistent split sidebar. Command-1/2 opens the selected section's Cinema Home; Browse/search opens its official provider. Resume returns to the retained page without reload. Native queue operations, Now Playing, menus, fullscreen stage, and PiP commands are retained.
- The local bridge checks the exact bundled document, web-view identity, and main frame, and accepts only Ready/Open/Resume/Queue/Save. Production assets are in `resources/cinema/`; the old `design-preview/` prototype and root `tokens.css` were removed after 0.4.0 and remain in git history.
- Current behavior and data boundaries: [docs/implementation.md](docs/implementation.md). Chronological evidence and limits: [docs/verification.md](docs/verification.md). Earlier architecture/design documents remain history, not the current implementation contract.
- The earlier fresh-navigation fix is included: a new destination/search/link clears stale load/process failure state while its request is pending.

## Verification status

The 0.2.3 hand checks and the earlier 2026-10-25 Watch reliability pass are historical evidence for their recorded builds, not a Cinema release fingerprint.

After cleanup, the engineering owner reported `cargo fmt`, 22 tests, strict Clippy, and a clean debug bundle succeeding. The director confirmed the temporary probe file is absent and main callbacks/load code are restored clean. Actual native Home and drawer open/close with focus return were observed through AX and owned-window captures. Later desktop input failed (`window_not_focused`, then AX `permission_denied`); no security settings were changed or keyboard bypass used, and no trusted F/Escape/keyboard result is claimed for this Cinema run.

A temporary in-process smoke used the actual app delegate and existing default website session, not an isolated account. Two runs ended `CINEMA_SMOKE_DONE failed=false`. The public YouTube video's progress sample, taken while muted, advanced from 4.366646657 to 6.990878309 seconds with `paused=false` across Home→Resume. Earlier smoke phases preceded the explicit mute. A programmatic official-player fullscreen-button click produced a video-only 1512×982 stage and restored the 1512×799 provider viewport. The real Music site and retained session loaded. These are app-method/WebKit observations, not physical input or sound certification.

The live queue contained zero items and its SHA-256 remained unchanged after both smoke and installation. No live saving, reorder, or drag was exercised. Bundled-renderer QA covered narrow widths, long/Thai text, literal hostile markup, unsafe URL omission, saved-row focus preservation, queue-error presentation, and protected-artwork contrast; it does not certify a native whole-app mobile layout.

The final installed app was observed through a fresh read-only computer session: a 1512×947 native window showed warm-dark Cinema Home, `YOUR NEXT WATCH.`, Browse YouTube, and empty Up Next. AX reported zero items and the existing signed-in state. This resolves installed visual/AX observation, not the earlier keyboard/input failure. All six production Cinema assets, including the local font and license, are bundled.

Still unverified this turn: successful PiP entry, physical keyboard behavior after the desktop backend failed, sound output, sustained playback/network recovery, smoothness/resource use, and long-duration stability. Earlier PiP requests reported a provider rejection; no restriction was bypassed. Temporary smoke code and diagnostic traces were removed before final packaging. Full installation, signing, process-path, and unchanged-queue evidence is recorded in Verification.

## Possible next steps

- Optional auto-play of the next Up Next item (off by default, in a Settings window).
- Menu bar mini player for Music.
- Now Playing card for a YouTube video while the Music section is shown.

## Build and run

- `./scripts/build-app.sh [--release]`, `./scripts/run.sh [--release]`, `./scripts/install.sh --open`, `./scripts/make-dmg.sh`.
- Saved queue: `~/Library/Application Support/DooTube/state.json`. Don't overwrite the user's saved queue during testing.
