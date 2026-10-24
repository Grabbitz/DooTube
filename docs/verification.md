# Verification

Date: 2026-10-23. Results apply only to the environment and actions explicitly recorded here; API availability and a successful build are not evidence of smooth playback or compatibility on an older Mac.

## Observed environment

| Item | Observed value |
| --- | --- |
| Host | Apple Silicon (`arm64`), macOS 27.0.1, build 26A434 |
| Rust | Homebrew `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Cargo | `cargo 1.99.1 (5f94df478 2026-08-27)` |
| Toolchain manager | `rustup` was not installed; discovery exited 127 at that command, after Rust/Cargo versions succeeded |
| Developer directory | `/Library/Developer/CommandLineTools` |
| macOS SDK | 27.0, `/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` |
| Compiler | Apple clang 21.0.0, target `arm64-apple-darwin27.0.0` |
| Deployment target | 14.2 is the app's chosen minimum, not a claim that runtime testing occurred on 14.2 |

## Packaging checks

- `/bin/bash -n` passed for both build and launch scripts.
- `plutil -lint` passed for the source `Info.plist`.
- Build-script unknown and extra arguments, and run-script unknown arguments and a missing `--` separator, each exited with usage status 2 before invoking Cargo.
- Those argument checks were invoked through a temporary symlink named `DooTube workspace`, exercising script location resolution with spaces. The temporary directory was removed. This is not evidence of a full build from a physically copied repository path containing spaces.

- The initial app packaging attempt correctly propagated Cargo's compile failure (exit 101, an `Option<NSString>` conversion in app source). After the implementation owner corrected it, the debug bundle built, its `Info.plist` passed lint, and ad-hoc signing completed.
- 2026-10-24: Ran `./scripts/install.sh --open`, `ls -ld /Applications/DooTube.app`, `codesign --verify --strict --verbose=2 /Applications/DooTube.app`, `plutil -extract CFBundleIdentifier raw /Applications/DooTube.app/Contents/Info.plist`, and `pgrep -fl 'DooTube.app/Contents/MacOS/dootube'`; installation and signature verification succeeded, the bundle identifier was `com.dootube.app`, and PID 60400 was running `/Applications/DooTube.app/Contents/MacOS/dootube`.

## Observed native runtime

All screenshots below are captures of the owned DooTube window only, stored outside the repository. Public YouTube content was used without signing in.

| Action | Observed result |
| --- | --- |
| Open `target/debug/DooTube.app` through Launch Services | A real `DooTube — Watch` AppKit window appeared. AX exposed the native input, navigation, queue, and browser-handoff controls. No website/media was loaded automatically on a no-argument launch. |
| Launch with `NASA space videos` | Official search page loaded. AX reported `NASA space videos - YouTube`, the search query, public NASA result links, and native status `YouTube page ready.` |
| Launch with `https://www.youtube.com/watch?v=jNQXAC9IVRw` | Official `Me at the zoo - YouTube` page and YouTube player appeared. Screenshot showed a paused poster, Play button, and `0:00 / 0:19`. This proves page/player rendering, not video playback. |
| Launch with `https://example.com/not-youtube` | Native status reported `Only YouTube links can be opened here.` No external website loaded. |
| End each owned app process between startup-input cases | Only the observed DooTube PID received SIGTERM. This is process termination/relaunch, not a test of the native close/minimize media lifecycle. |
For the signed-in account identity path, a temporary boolean-only check in the bundled app reported that the same-origin account switcher response yielded a parsed identity and that an email was found (`true`). It emitted no account values or query-bearing URLs. The account menu itself was not opened because native input/accessibility was unavailable for this check.


Initial runtime screenshots: `/tmp/dootube-startup.png`, `/tmp/dootube-search.png`, `/tmp/dootube-video.png`, `/tmp/dootube-invalid-input.png`. These are local verification artifacts, not portable repository assets.

## Native automation boundary

The exposed native computer backend reported Quartz capture, accessibility, background-window input, applications, and menus support; capture/input/accessibility permissions were granted and the session was unlocked. However, the required task-scoped live control request returned exactly `{ active: false, reason: "no-ui" }` in the worker session. The parent did not have an interactive grant surface either. No synthetic input or alternate accessibility/AppleScript route was used to bypass that denial.

Consequently, read-only observation and explicitly requested build/run launches are allowed, but native button interaction, resize, queue interaction, and media lifecycle input cannot be claimed as exercised through automation in this session. Core unit tests do not substitute for that UI evidence. No user account, cookies, browser history, Kaset settings, global volume, or security configuration was inspected or changed for this task.

## Captured corrected bundle snapshot

After the initial observations, the corrected app was rebuilt and launched through `scripts/run.sh` from a temporary `DooTube workspace` symlink containing spaces. The script completed successfully, including `-- "NASA space videos"` forwarding. This exercised the complete build/package/launch path through a symlink with spaces, not a physically copied checkout. The temporary symlink was removed after the owned app processes exited.

The captured bundle had a native dark empty state with a real two-line instruction, a collapsed `Show Up Next` control, and no unsolicited website load. CLI search and video input populated the native field. The search settled at `NASA space videos - YouTube`; the video settled at `Me at the zoo - YouTube` with native status `YouTube page ready.` The player remained visibly paused at `0:00 / 0:19`. The invalid external URL again showed `Only YouTube links can be opened here.`

Snapshot screenshots (owned window only):

- `/tmp/dootube-final-startup.png`
- `/tmp/dootube-final-search.png`
- `/tmp/dootube-final-video.png`
- `/tmp/dootube-final-invalid-input.png`

Despite the filenames, these identify a historical debug snapshot, **not verification of later source edits**. The implementation owner subsequently revised resizing, queue layout, lifecycle, and navigation. The frozen optimized release evidence below supersedes this debug snapshot for the delivered bundle.

Tested bundle executable SHA-256: `e1395fb329b399d1eeff6e491a071ec63aea3dc0ccdab097bb49e5977ae2c198`.
Bundle executable modification time: `2026-10-23T01:22:20+0700`.

`codesign --verify --strict --verbose=2 target/debug/DooTube.app` reported valid on disk and satisfaction of its designated requirement. `xcrun vtool -show-build` reported `minos 14.2`, SDK 27.0; `plutil` read `LSMinimumSystemVersion` as 14.2. This historical debug build succeeded with 10 unnecessary-unsafe warnings; the later frozen release build below was warning-free.

## Core tests reported by the implementation owner

The implementation owner reported `cargo test`: **5 passed**, covering:

- Supported search, YouTube pages/video links, and query preservation.
- Unsafe schemes, host boundaries, credentials/custom ports, natural-language colon queries, and HTTP upgrade.
- Bounded queue add/duplicate/remove/reorder behavior.
- Saved queue round-trip in an isolated temporary directory, without touching the user's home.
- Reporting malformed saved queue data while leaving its bytes untouched.

These are reported unit-test results, not native UI interaction or real playback evidence. This worker did not alter the user's saved queue. No playback duration/progression, smoothness, dropped frames, sound output, player quality/captions interaction, native queue interaction, resize, network-failure recovery, or close/minimize lifecycle was verified through native input.

## In-process native behavior smoke

Before source freeze, the implementation owner temporarily called the app's own methods inside its process, with isolated HOME `/tmp/dootube-native-smoke-home`, rather than sending OS input. The reported result was:

`NATIVE_SMOKE_OK: resize 940x480/1280x900, queue persistence/reorder/remove, webview identity, and lifecycle guards`

This checks programmatic app-method behavior, not accessibility actions, real keyboard/mouse interaction, or playing-media lifecycle. The queue used synthetic saved-URL test data; it is not evidence that those test URLs play. Read-only observation of that exact owned process confirmed a 940×512 outer window (940×480 content), expanded Up Next, one saved test entry, Open selected/Remove controls, disabled boundary-move buttons, and a complete two-line sign-in note. The owned-window capture is `/tmp/dootube-native-smoke-minimum.png`.

The implementation owner confirmed the smoke process was stopped, temporary HOME removed, instrumentation/env hook removed, and source frozen. Final `cargo fmt && cargo test && cargo build` was reported successful with five tests and no warnings. No smoke hook is part of the release bundle.

## Frozen optimized release

After that explicit source freeze, this worker ran:

```sh
./scripts/build-app.sh --release
open "$PWD/target/release/DooTube.app"
open "$PWD/target/release/DooTube.app" --args "NASA space videos"
open "$PWD/target/release/DooTube.app" --args "https://www.youtube.com/watch?v=jNQXAC9IVRw"
open "$PWD/target/release/DooTube.app" --args "https://example.com/not-youtube"
```

Each launch case began after terminating only the preceding owned app PID. The optimized build succeeded without warnings, source plist lint passed, and the local bundle was ad-hoc signed. Read-only AX and owned-window screenshots confirmed:

- Native startup instructions and initially collapsed Up Next, with no website loaded automatically.
- Populated CLI search field and official `NASA space videos - YouTube` page with native `YouTube page ready.` status.
- Populated CLI video URL and official `Me at the zoo - YouTube` page. AX separately reported native `YouTube page ready.` and the video title heading. The captured pixels, including a later capture, still showed native `Loading YouTube…` and partially rendered/skeleton page chrome, so **no screenshot is claimed to show a fully settled page**. The visible player showed a poster/Play button at `0:00 / 0:19`; this is page/player rendering evidence, not actual playback. The AX-ready observation and loading-state image are distinct evidence, not proof of the same settled visual frame.
- Invalid external URL rejected with `Only YouTube links can be opened here.` and the native empty view retained.

Release screenshots: `/tmp/dootube-release-startup.png`, `/tmp/dootube-release-search.png`, `/tmp/dootube-release-video.png`, `/tmp/dootube-release-invalid-input.png`. A distinct later video capture is `/tmp/dootube-release-video-later.png`; it also visibly retains loading/skeleton chrome and is not settled-page evidence. The original video pathname was overwritten during capture attempts, so use the distinct later path to avoid cached-image ambiguity.

Final bundle: `target/release/DooTube.app`.
Executable SHA-256: `9982d372fb010a0fadc9bcb28e8ad230059810c41be5be63a656bcffd30a30d9`.
Executable modification time: `2026-10-23T01:35:54+0700`.

Frozen input SHA-256 values:

- `src/main.rs`: `95916e746424efa58e17ce7bceb18a5f94e35de7d885545a838f480094b49763`
- `src/core.rs`: `c1f53c76ae368f9a3b669b276b5912ee3e68839487762e8073f1a2c041277d10`
- `Cargo.lock`: `7d322415f788ed45b6f8e6dcab005972cf3f73e3880b24e4bcdd001100397efa`

`codesign --verify --strict --verbose=2 target/release/DooTube.app` passed. `xcrun vtool -show-build` reported binary `minos 14.2` and SDK 27.0; `plutil` read bundle `LSMinimumSystemVersion` as 14.2. This confirms packaging consistency, not runtime compatibility testing on macOS 14.2.

All owned verification processes were stopped. Temporary path-with-spaces symlinks and isolated smoke HOME were removed; no persisted user queue was changed. Remaining limits are real playback/audio/smoothness, user-driven native controls/menus, network failure recovery, and lifecycle behavior with media actually playing. No login or account data was used, and no native input approval was bypassed.

## Native chrome redesign smoke (debug build, unfrozen)

The toolbar/sidebar redesign was built with `./scripts/run.sh` (debug, no warnings) after `cargo test` passed 7 core tests. Screen capture was permitted; native input and accessibility were not (`inputPermission: denied`, `axPermission: denied`), so only launch arguments and owned-window screenshots were used. Observed:

- No argument: YouTube Home in YouTube's dark theme matching the system, no website masthead or guide, window title `Home`, Home selected in the sidebar, Back/Forward/Save disabled, and the empty Up Next hint `Save videos with ⌘S`.
- `lofi`: official search results, window title `lofi`, the query shown in the toolbar search field.
- A temporary queue with one untitled and one titled entry, launched on the untitled `watch?v=jNQXAC9IVRw`: the entry was selected in the sidebar, the window title read `Me at the zoo`, and the saved file gained `"title": "Me at the zoo"`. Launched on `https://youtu.be/dQw4w9WgXcQ`, the matching `watch?v=` entry was selected and the long title truncated at the sidebar edge. The player showed its poster and controls at `0:00`; this is rendering evidence, not playback.

The temporary queue file and its folder were removed afterwards; no user queue existed before the test. Not exercised: typing into search, clicks, the context menu, Command-S/Delete/Option-arrow commands, sidebar collapse, the Try Again failure state, notices, and the optimized release build. The frozen release fingerprints above describe the previous source, not this one.

## Sections, sign-in, and media capability smoke (debug build, unfrozen)

After adding the YouTube/Music sections, sign-in, full screen, and Picture in Picture, `cargo test` passed 8 core tests and `cargo clippy` reported nothing. Native input was still not permitted, so checks used launch arguments, owned-window screenshots, and temporary in-app traces (removed afterwards). The user was also using the app during these runs, so some navigations in the traces were theirs.

- `https://music.youtube.com/` opened in the Music section: Music Home rendered without YouTube Music's nav bar or guide, the sidebar showed Home/Explore/Library, and the search field read `Search songs, albums, artists`. A trace confirmed the Music pane loaded `music.youtube.com/` and that switching to YouTube loaded YouTube Home in the other pane.
- `https://www.youtube.com/account` (signed out) redirected inside the app to Google's `Sign in — to continue to YouTube` page with an editable email field; no credentials were entered.
- A playing ad in the YouTube section posted the audible-playback message the one-at-a-time rule relies on.
- A temporary capability probe inside the YouTube pane reported: user agent ending `Version/27.0 Safari/605.1.15`; screen 1512×982 at 2×; `dynamic-range: high` true; MSE and ManagedMediaSource both accept H.264 (`avc1.640033`), VP9, VP9 Profile 2 10-bit (`vp09.02.51.10.01.09.16.09.00`), and Opus, and reject AV1 (`av01.0.16M.08`, `av01.0.16M.10…`); `pictureInPictureEnabled`, `fullscreenEnabled`, and `webkitFullscreenEnabled` true.
- The Picture in Picture command on Music Home (no video) showed `Open a video to use Picture in Picture.`.

Not exercised: completing sign-in or sign-out, actually entering Picture in Picture or element full screen, HDR/8K playback, and the pause-the-other-section behavior with audible media in both sections. An automated Picture in Picture attempt on an unplayed video produced no enter event; it is inconclusive, because the video had not started and the user was navigating at the time.

## Sign-in passkey suppression smoke (debug build, unfrozen)

The unbundled debug binary was run twice, signed out with its own empty website data, using temporary env-gated traces that logged timing and navigations and opened the in-app sign-in (traces and logs deleted afterwards; no credentials entered). Each run went `ServiceLogin` → `accounts.google.com/v3/signin/identifier`. Time from sign-in start to the identifier page finishing: 1.65 s with suppression and 0.83 s without. One run each on a live network does not show a difference caused by the script. On the finished identifier page, `typeof window.PublicKeyCredential` was `undefined` with the suppression script and `function` without it.

Not exercised: the password, second-factor, and post-sign-in redirect steps, whether Google skips its passkey challenge for a passkey-enabled account, and `PublicKeyCredential` on YouTube pages (the script's host check excludes them).

## Full-screen runtime probe (unbundled debug binary)

The unbundled `./target/debug/dootube` was run on the public video used for the probe with a temporary `DOOTUBE_FS_TEST=1` hook. The hook invoked AppKit and WebKit APIs directly, logged only frame/state information, and self-terminated after the checks. The hook and its test-only dependency features were removed afterwards.

- Native window fullscreen entered and exited: `windowDidEnterFullScreen` and `windowDidExitFullScreen` fired, and the fullscreen style-mask check was true during entry. The window frame was 1512×949 points both times because the probe's saved starting frame was already screen-sized. The container/web-view frames matched at 1292×949 in fullscreen and 1292×897 after exit. The defaults value for `NSWindow Frame DooTubeMainWindow` was identical before and after the fullscreen transition. This did not verify restoring a smaller normal window frame.
- After exiting element fullscreen, the active web view's superview was still the container. Programmatically resizing the window to 1632×949 changed the container and web-view frames together to 1412×897, confirming autoresizing after exit.
- A `requestFullscreen()` attempt on `#movie_player` activated fullscreen and emitted `fullscreenchange`; one earlier attempt returned `TypeError` without an event, while the final attempt succeeded. The later `webkitRequestFullscreen()` call was made while fullscreen was still active, so it was not an isolated test of that API. After the exit request, `document.fullscreenElement` and `document.webkitFullscreenElement` were both false.
- Twelve `screencapture -x` samples completed, but each was uniformly black, so they provided no usable visual evidence about flashes or black frames during transitions.

These programmatic checks confirm frame/layout behavior and DOM fullscreen state, not a visual smoothness result. Native keyboard/mouse input, hover-revealed toolbar usability, the player controls during element fullscreen, and a separate `webkitRequestFullscreen()` entry/exit were not exercised.
## Element-fullscreen runtime comparison (installed binary)

- With `DOOTUBE_FS_RECOVERY=0` and `=1`, ran two cycles on `https://www.youtube.com/watch?v=jNQXAC9IVRw` using a muted `video.play()`, a JavaScript `.ytp-fullscreen-button.click()`, and a `#movie_player.requestFullscreen()` fallback. The fake button click did not enter or exit full screen; the fallback entered and `document.exitFullscreen()` exited.
- Before entry and after API exit in both runs: window frame `1512×949`, content bounds `1512×949`, container bounds `1292×897`; the active web view's superview was the container, and its frame/bounds were `1292×897` with autoresizing mask `NSAutoresizingMaskOptions(18)`. `window.innerWidth/innerHeight` were `1292×897`; video client size was `889×667`.
- During API element full screen in both runs: window/content remained `1512×949`, container remained `1292×897`; the active web view's superview was no longer the container, and its parent bounds and web frame/bounds were `1512×949` (mask `NSAutoresizingMaskOptions(18)`). `innerWidth/innerHeight` were `1512×949`; video client size was `1048×786`; `document.fullscreenElement` and `document.webkitFullscreenElement` were true, while `.ytp-fullscreen` and `ytd-watch-flexy[fullscreen]` were false.
- A synthetic player-button exit attempt left both fullscreen-element checks true. After `document.exitFullscreen()`, the fullscreen-change message reported `active=false`; the web view returned to the container at `1292×897` without recovery enabled. The recovery-enabled measurements matched the disabled measurements.
- Screenshots after all four API exits showed the normal YouTube watch page, native toolbar and sidebar, and video controls. They did not show the reported full-screen quick-action row, bottom chevron, or clipped player. The synthetic button attempt displayed YouTube's “Full screen is unavailable” notice. Thus these captures do not reproduce the user's reported state.
- A trusted native player-button click and Escape were not exercised: desktop input capability was denied. The JavaScript click was not equivalent to a trusted click, and the `requestFullscreen()` fallback did not activate YouTube's own `.ytp-fullscreen` class or `ytd-watch-flexy[fullscreen]` attribute. These measurements do not establish the cause of the reported trusted-button exit bug.

## Regional sign-in redirect handling

For the exact Google-domain allowlist change, `cargo fmt`, `cargo clippy --all-targets` (no warnings), `cargo test` (10 passed), and `./scripts/build-app.sh` completed successfully. No sign-in runtime flow was exercised. The proposed regional CheckCookie/SetSID redirect chain remains unconfirmed until the user retries sign-in.
