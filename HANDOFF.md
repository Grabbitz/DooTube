# DooTube — resume checkpoint

Saved 2026-10-23 at the user's request to pause work and continue later. This is a file checkpoint, not a Git commit; no `.git` directory was found at the repository root.

## Confirmed direction

- Simple, native Mac UI (“เรียบแบบ mac”), with advanced tools tucked away.
- Make YouTube Watch smooth first, then focus on Music depth and customization. Smoothness versus Kaset is a goal, not a measured result.
- Current app: Rust + AppKit native chrome around the visible official YouTube website in `WKWebView`; no Electron, unofficial extractor, or custom player.

## Implemented today

- Native window: unified toolbar (sidebar toggle, Back/Forward, YouTube | Music switch, search, Picture in Picture, Save to Up Next, account menu) and source-list sidebar with per-section destinations and titled Up Next. The sites' own headers/guides are hidden by injected CSS; players are untouched. Browser handoff and native menus/shortcuts.
- YouTube and YouTube Music are separate web views (official websites); one plays at a time. In-app Google sign-in via Google's own page, shared by both. Element full screen and Picture in Picture enabled.
- Up Next with saved URLs and titles, click to open, context-menu/menu Move Up/Down and Remove. No auto-next or restored playback/autoplay.
- Queue persistence at `~/Library/Application Support/DooTube/state.json`; unreadable state/save errors are surfaced. Do not overwrite the user's saved queue during testing.
- YouTube-host navigation restrictions (plus Google sign-in hosts) and pause/suspend lifecycle guards, except for Picture in Picture. Implementation is not proof of user-driven playback/lifecycle behavior.
- Native Music catalog, lyrics, credits, and EQ are **not implemented**; YouTube Music is its official website. Earlier design documents are proposals, not shipped-feature claims.

## Build/run and current app

- Build/package: `./scripts/build-app.sh` or `./scripts/build-app.sh --release`.
- Build and launch: `./scripts/run.sh` or `./scripts/run.sh --release`; startup input follows `--`.
- Existing optimized bundle: `target/release/DooTube.app` (host-architecture, ad-hoc signed local development app, not notarized).
- The app was launched at the last request. Leave it running for this pause: do not quit, kill, rebuild, change account/global settings, or alter permissions. This checkpoint does not independently assert live process status. Launch Services reuses the running app; startup arguments do not replace its current page.

## Saved verification — not rerun for this checkpoint

Source of results and exact evidence: [docs/verification.md](docs/verification.md).

- Five behavioral/core tests reported passing: URL parsing/boundaries, bounded queue operations, isolated persistence round-trip, and malformed saved-state preservation.
- Frozen optimized release built without warnings; strict codesign verification passed. Packaging recorded a macOS 14.2 minimum, not runtime compatibility testing on 14.2.
- Actual native launch/page-render smoke observed official YouTube search and a video page/player. The player was paused at `0:00 / 0:19`; release screenshots retained loading/skeleton chrome, so do not describe them as fully settled-page or playback evidence.
- In-process geometry/queue smoke checked resize, queue persistence/reorder/remove, webview identity, and lifecycle guards. It is not real keyboard/mouse interaction or media proof.
- **Still unverified:** actual video progression, audio, seeking, smoothness versus Kaset, native user-driven controls, network recovery, and lifecycle behavior while media plays. Prior task-scoped live UI control returned `{ active: false, reason: "no-ui" }`; no bypass was used. Page rendering is not playback.

## Resume order

1. Obtain permitted live control with an explicit interactive grant, or have the user perform a genuine playback gesture. Respect provider restrictions; do not bypass denied input or sign-in requirements.
2. Verify and record real playback progression, audible audio, seeking, and media lifecycle/native controls. Inspect any observed loading/status/rendering or interaction issues and fix demonstrated problems.
3. Measure Watch smoothness against Kaset on the same hardware, network, media, and access conditions, including WebKit helper costs. Do not claim a performance win before measurement.
4. Only after proving the Watch daily core, move the delivery focus to Music; lyrics/credits/EQ need feasibility and permitted implementation, not placeholder controls.

## Read next

- [README.md](README.md): usage and build commands.
- [docs/implementation.md](docs/implementation.md): shipped behavior and data boundaries.
- [.impeccable.md](.impeccable.md): confirmed design direction.
- [docs/design-brief.md](docs/design-brief.md) and [docs/architecture.md](docs/architecture.md): design history/future options.
- `src/main.rs`: native app; `src/core.rs`: input/queue/persistence logic.

No feature code, existing design documents, or user state were changed for this checkpoint. No build, tests, codesign, playback checks, or Git commit were run.
