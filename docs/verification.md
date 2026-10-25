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

## Watch reliability pass — 2026-10-25

### Installed release, native input

Task-scoped desktop control was approved. The installed `/Applications/DooTube.app` executable and the pre-change optimized build both had SHA-256 `a38264f953cdad718ad9a441cc1fb31f0c052e338e42acbaac85fe7402c6f5a6`. These observations describe that baseline, not the later network-state fix:

- Video frames progressed in the existing Watch page; a native click paused it. F opened the borderless video stage and F returned to the normal toolbar/sidebar layout. One completed cycle was captured; this is not a frame-time or rapid-toggle benchmark.
- The official video's seek slider reported 60 seconds after an AX increment and 55 seconds after a decrement; the captured player displayed `1:00 / 15:45` at the former position. An earlier AX value-write call returned an error and is not counted as a successful action.
- YouTube/Music switching preserved the pages. Music changed from Play to Pause when started through the native Controls shortcut; its timeline advanced from `0:11` to `0:12`, later `1:39` and `2:11`, while its Now Playing card remained available in Watch. Starting the public Watch video changed Music's card back to Play. Sound output and absence of audible overlap were not independently measured.
- Window → Minimize removed the main window from the visible-window list. Reopening the same app restored it with Music paused at `2:11`; a later AX read still reported that position and Play. This exercises pause-on-minimize and no automatic resume, not successful PiP background playback.
- PiP requests on the existing playing video and the public `jNQXAC9IVRw` probe displayed `This video can't play in Picture in Picture.` No PiP window appeared. The command's rejection feedback is verified; its underlying capability/provider cause and successful PiP entry are not. No disabled-video flag or provider restriction was overridden.

Some background input attempts were rejected with `UserIntervened` or `BackgroundUnavailable`; they are not counted as actions. Intermittent website keyboard-shortcut dialogs were dismissed through their AX button; their cause was not established. No app-owned queue edits, sign-in/out, account changes, or system network/volume changes were made. Viewing pages used the app's existing website session.

Local captures (not portable repository assets): fullscreen stage `/var/folders/7r/s5q3ccyj5vq41z9nkm_2kgkc0000gn/T/omp-computer-15b3c43266e867c6.png`; returned Watch page `omp-computer-15b3c43c5aa867c8.png`, seek result `omp-computer-15b3c51585e867d2.png`, and PiP rejection `omp-computer-15b3c4ba7da867cf.png` in the same directory. None establishes smoothness or sound quality.

### Fresh navigation after a network failure

A throwaway in-process smoke used the actual app delegate, WebKit navigation callbacks, and native placeholder. Its queue HOME was `/tmp/dootube-watch-recovery-home`; the two web views used a nonpersistent website data store. A proxy override applied only to that data store, not to the installed app or system networking:

1. A refused loopback HTTP CONNECT proxy produced a real `NSURLErrorDomain` error `-1004`; the failure state offered Try Again.
2. The smoke replaced it with a loopback proxy that accepted connections without responding, then called the app's Home-loading path. Before the fix it reported `failed=true stopped=false retry_visible=true` and exited 1: the old failure persisted during the new request.
3. With `load_in` clearing `failed` and `stopped` before requesting the new page, the same step reported `failed=false stopped=false retry_visible=false`. Native AX exposed `Loading YouTube` and a busy indicator instead of Try Again.
4. The proxy override was removed and reload was requested; a native Home press was also exercised while the request was pending. A real page commit then asserted the placeholder was hidden and failure flags were clear. The smoke exited 0 with `WATCH_SMOKE_OK: network error, new navigation loading state, online recovery`.

This is a failing-before/passing-after app-method/native-state regression smoke, not a fully native-input network-disconnection test or a test of buffered-media recovery. Background captures of the isolated window were white and do not prove settled website rendering; the loading/commit evidence is the AX state and actual delegate callbacks. No permanent native test was added because this scenario needs an AppKit main-thread application, live WebKit, and scoped proxy lifecycle rather than the ordinary isolated core-test runner.

A separate nonpersistent WebKit HTML probe reported `pictureInPictureEnabled=true` both with element fullscreen enabled and disabled; disabling element fullscreen alone did not disable the PiP capability in that probe. An unattached public-site probe timed out, including a compiled retry, so it yielded no provider-capability conclusion.

### Final clean build

After removing the temporary module, selectors, proxy overrides, and environment gates, `cargo test` passed 19 tests, `cargo clippy --all-targets -- -D warnings` succeeded, and both `cargo build` and `./scripts/build-app.sh --release` succeeded. The release plist lint and `codesign --verify --strict --verbose=2 target/release/DooTube.app` passed.

Final executable SHA-256: `0f8e2753619a89eee0f471fccd53ccd5901b7d042f1167139f2eeb62975ed0e8`. The smoke helper/source files, compiled site probe, and temporary queue HOME were removed. The rebuilt app remains at `target/release/DooTube.app`; `/Applications/DooTube.app` was not replaced.

## Cinema integration — 2026-10-25

This section records the 0.3.0 Cinema integration after the earlier Watch reliability pass. The actual local date was 2026-10-25 (+0700); a next-day screenshot expiry was not the verification date. Historical fingerprints above describe their own builds, not the Cinema release. GitHub's published release remains 0.2.3.

### Native observation and desktop-control boundary

Actual native Cinema Home and the Up Next drawer were observed through AX and owned-window screenshots. Drawer open/close and focus return were exercised before the desktop backend failed. The live queue contained zero items; no live saving, reorder, drag, or removal was exercised in this pass. Earlier hand checks remain historical.

The original desktop-control session was closed. Subsequent Orca key attempts returned `window_not_focused`, including after the user confirmed focus. A later `get-app-state` AX read returned `permission_denied` even though the permission report showed accessibility and screenshots granted. No security settings or permissions were changed, and no forced keyboard route was used. Consequently, this Cinema run provides no trusted-native F, Escape, or keyboard certification.

Native full-window Home and drawer captures are local artifacts:

- `/var/folders/7r/s5q3ccyj5vq41z9nkm_2kgkc0000gn/T/orca-computer-use/7fb51a43-9fde-45d5-8ce5-6422f48ca3c9-screenshot.png`
- `/var/folders/7r/s5q3ccyj5vq41z9nkm_2kgkc0000gn/T/orca-computer-use/ccb1d85e-a5f6-43dd-84ca-af086f386e46-screenshot.png`

### Actual app-method and provider smoke

A temporary in-process smoke used the actual `AppDelegate` and `WKWebView`s with the real existing default website session, not an isolated account. The public probe was `https://www.youtube.com/watch?v=jNQXAC9IVRw`. Two runs reported `CINEMA_SMOKE_DONE failed=false`.

- Cinema Home showed the actual `Me at the zoo` title and image, zero saved cards, an intentional real empty state, and no horizontal overflow.
- Returning Home→player retained the existing provider page without reload. In the last run the video's current time advanced from 4.366646657 to 6.990878309 seconds with `paused=false`. This progress sample was taken while the video was muted; earlier smoke phases preceded the explicit mute. The original media mute state was restored after pausing. No system volume was changed. This does not establish audible output or sustained performance.
- Programmatically clicking the official `.ytp-fullscreen-button` entered the native 1512×982 video stage. The actual WK snapshot showed video only. Exit restored the 1512×799 provider viewport and original container. This was a JavaScript player-button click, not physical trusted input or F. An earlier generic `requestFullscreen` probe showed page overlays and is not counted as visual fullscreen proof; the definitive player-button case replaced that test entry, not the player implementation.
- The actual Music provider reported host `music.youtube.com`, `ytmusic-app`, title `YouTube Music`, and viewport 1512×799. Music Home before entering the provider showed `Your next listen` and `Browse Music`, not fake tracks. The real provider retained the existing account and restored its site media state; new Music sound playback was not certified.
- The initial native layout exposed a real geometry bug: an unconstrained `NSBox` consumed the content and left Home only 97 points high. Explicit 72-point Home / 116-point provider bar heights and a pinned Home fixed the Home viewport to 843 points. Provider reveal synchronizes to its container only while attached, not while on the fullscreen stage. The observed normal window was 1512×947, with a 1512×799 provider viewport.

After the smoke, `~/Library/Application Support/DooTube/state.json` still had SHA-256 `de0dee94e846d8ad77b673acdb90741392b4009ad8001a45c5a846606ed5f20f`. The queue file was unchanged. No queue schema, default website data store, or bundle identifier (`com.dootube.app`) change was introduced. No account names, emails, or cookies are recorded here.

Actual WKWebView snapshots, stored outside the repository:

- `/tmp/dootube-cinema-final-contrast-home.png`
- `/tmp/dootube-cinema-player-button-fullscreen.png`
- `/tmp/dootube-cinema-official-player.png`
- `/tmp/dootube-cinema-official-music.png`

These snapshots contain actual existing-session website content. They are not fabricated OS/browser chrome, portable repository assets, or proof of trusted OS input.

### Bundled renderer and contrast QA

Browser-only QA of the bundled renderer covered widths 320, 375, 414, and 768: no horizontal overflow or wrapped action labels was observed. Long and Thai titles were checked. This is renderer responsiveness evidence, not native whole-app mobile certification.

Hostile markup remained literal text with no injected element or hostile image; bad URL schemes were omitted. Saved-position updates and reordering preserved DOM focus, including the label `Saved at 2:22`. An unreadable queue disabled Save and showed an unavailable state rather than falsely claiming the queue was empty. These renderer cases do not imply edits to the real user's queue.

Final protected-image worst-case computed WCAG ratios over pure-white artwork were 6.67:1 for muted body text, 11.12:1 for headlines, and 3.38:1 for the accent icon/ring. An 82%-opaque named dark scrim covers the copy width while artwork outside it stays visible. The final contrast adjustment did not alter structural sizing, and final native paint was observed afterward. No aggregate “58/58” score is claimed; native platform controls are not web marketing-page gates.

### Clean-check and installation status

Before source-freeze cleanup, the implementation owner reported 22 tests passing and `cargo clippy --all-targets -- -D warnings` succeeding. These are pre-cleanup results, not final-bundle verification.

After cleanup, the engineering owner reported `cargo fmt --all`, `cargo test --locked` (22 passed), `cargo clippy --all-targets --locked -- -D warnings`, and `./scripts/build-app.sh` succeeding. The director's source review confirmed temporary `src/cinema_smoke.rs` is absent, main callbacks/load code are restored clean, and the temporary wiring and `NAV_DIAG`/`LOAD_DIAG` traces are removed.

The authorized `./scripts/install.sh --open` then succeeded, building the release profile and installing `/Applications/DooTube.app`. The installed plist reports version `0.3.0` and identifier `com.dootube.app`. `codesign --verify --deep --strict --verbose=2 /Applications/DooTube.app` reported valid on disk and satisfaction of its designated requirement.

Both `target/release/DooTube.app/Contents/MacOS/dootube` and `/Applications/DooTube.app/Contents/MacOS/dootube` have SHA-256 `2e7044af37f38b3692b8fd9241c41ac858c24be48bef4c11290b7b67484a8afd`. `pgrep` reported PID 90237 running the installed executable; `lsof -a -p 90237 -d txt -Fn` independently confirmed `/Applications/DooTube.app/Contents/MacOS/dootube`.

The installed `Contents/Resources/cinema/` contains all six production assets: `index.html`, `cinema.js`, `cinema.css`, `tokens.css`, `BebasNeue-Regular.ttf`, and `BebasNeue-OFL.txt`. After installation, `~/Library/Application Support/DooTube/state.json` still has SHA-256 `de0dee94e846d8ad77b673acdb90741392b4009ad8001a45c5a846606ed5f20f`, unchanged from the pre-smoke baseline.

A fresh read-only computer session used only window listing and `observe()` (screenshot plus AX) on the actual installed process. PID 90237's native window was titled `DooTube — YouTube` and measured 1512×947 points. Its screenshot showed warm near-black Cinema Home, a red YouTube marker, `YOUR NEXT WATCH.`, Browse YouTube, and the empty Up Next shelf. AX exposed YouTube/Music navigation, search, Up Next 0 items, 0 queued items, and `Signed in to YouTube`. This confirms final installed native paint and the retained signed-in state without recording account identity. It does not resolve the earlier input-backend failure or establish trusted physical keyboard/fullscreen, PiP, audio, or performance behavior. No input or security settings were changed in this read-only observation.

This completes local installation evidence only. The published GitHub release remains 0.2.3; no 0.3.0 public release or download has been published.

Limits for this Cinema pass remain successful PiP entry, trusted physical keyboard behavior after the backend failed, sound output, smoothness/resource benchmarks, sustained playback, long-duration stability, and new buffered-media/network-recovery evidence. Provider restrictions were not bypassed.
