# Changelog

## Unreleased

- A video saved to Up Next resumes where you left it: while it plays on YouTube, DooTube notes the position (every 15 seconds, and when it pauses, ends, or you leave the page), and opening it from Up Next starts there. Near the start or the last 20 seconds it starts over. Music, Shorts, and live links open as saved.
- The Dock icon's menu shows what YouTube Music is playing with Play/Pause, Next, and Previous.
- Up Next rows show each video's thumbnail. Drag a row to reorder Up Next, or drag it out of the sidebar to use its video link elsewhere; Move Up and Move Down are still in the row's menu.
- The Now Playing card is separated from the sidebar list by a hairline, with a clearer play control and larger click targets. Switching play and pause no longer redraws the cover. The first-load state names what is loading.
- Added Now Playing for YouTube Music: while the YouTube section is shown, the sidebar shows the playing track's artwork, title, and artist with Previous, Play/Pause, and Next; click the artwork to go to Music.
- Added a Controls menu (Play/Pause ⌘P, Next ⌘→, Previous ⌘←) that controls YouTube Music from either section. Next and Previous step aside while you're typing, so ⌘←/⌘→ still move the cursor.
- YouTube Music now offers next and previous to the system's media controls by pressing its own player-bar buttons (not yet tested with the keyboard's media keys or Control Center).

## 0.2.2 — 2026-10-24

- Added a Home button next to Back/Forward and View → Home (⇧⌘H). The three now sit together at the leading edge of the toolbar, like Finder and Safari.
- The toolbar no longer repeats the page title, which crowded the controls on video pages. The window title (Window menu, Mission Control) and saved Up Next titles drop YouTube's unread-notification count, such as `(1) `.
- The strip behind the toolbar takes the page's own background color, so the toolbar no longer sits on a gray band above YouTube's dark page.
- The search field clears a previous query when you leave the results page.
- Aligned the Up Next sidebar heading with the rows below it.
- Pressing F starts the full-screen zoom in about 0.3 s instead of 0.7 s: the page snapshot that stands in for the page is now taken at standard resolution, which WebKit paints about five times faster with a 60 fps video playing.
- Leaving full screen no longer flashes the video back to the frame from when full screen began while the page lays out again; the current frame stays in place instead.
- When a page's web process crashes or macOS reclaims its memory, the page reloads on its own like in Safari instead of showing "Can't Reach YouTube". If it happens again within a minute, the section says a problem repeatedly occurred and offers Try Again.

## 0.2.1 — 2026-10-24

- Fixed F making the whole page full screen instead of the video: YouTube requests full screen on the page and only reacts when `onfullscreenchange` exists, which disabling WebKit's own full screen had removed.
- Pressing F repeatedly now reverses the zoom from wherever it is instead of dropping presses or jumping.

## 0.2.0 — 2026-10-24

- Video full screen (F or the player's button) now fills the screen with only the video instead of putting the app window in full screen, zooming smoothly out of and back into the page.
- The toolbar shows the Google account's photo, and the account menu combines photo, name, and email into one header.
- Added `scripts/make-dmg.sh` and a downloadable `DooTube.dmg` on GitHub Releases.
- Added a custom DooTube app icon, editable SVG source, and reproducible ICNS/preview renderer.
- Added `scripts/install.sh` to build and install an optimized, ad-hoc-signed DooTube in Applications, register it with Launch Services, and optionally launch it with `--open`, preserving existing sign-in and website data.
- Replaced the browser-style button row, URL field, and status line with native Mac chrome: a unified toolbar (sidebar toggle, Back/Forward, search/link field, Save to Up Next) and a source-list sidebar with Home, Shorts, and Up Next.
- YouTube's website masthead and guide are hidden because the native toolbar and sidebar replace them; the official player and page content are unchanged. YouTube now follows the system light/dark appearance.
- Up Next shows saved video titles, highlights the open video, and supports click to open plus context-menu and menu-bar Move Up/Down and Remove. The same video in another link form is no longer saved twice.
- The app opens YouTube Home at launch; first-load failures show a Try Again state, and messages appear as a dismissible bubble instead of a permanent status line.
- Removed the "Open / Search", "Reload", "Show Up Next", and pasted-URL fallback for Save; reload is Command-R and saving works on the open video page.
- Added separate YouTube and YouTube Music sections (toolbar switch, Command-1/2), each with its own web view, sidebar destinations, and search; links open in their own section. Only one section plays at a time.
- Added in-app Google sign-in through Google's official page, shared by both sections, with an account toolbar menu (Sign In, Switch Account, Sign Out). Google's sign-in pages no longer offer passkeys, which macOS rejects in embedded web views and which left the flow stuck on a "check Bluetooth" error; Google goes straight to password and other verification.
- Tightened in-app sign-in navigation to an exact allowlist of Google's published regional domains, and generated passkey suppression's host check from the same list; allowed sign-in popups remain in the current web view.
- The account menu shows the active YouTube account's display name and email or channel handle, fetched from YouTube's same-origin session and kept in memory only.
- The player's full-screen button now works, and a Picture in Picture toolbar button and View menu command (Control-Command-P) float the playing video; a video in Picture in Picture keeps playing when the window is hidden or minimized.
- Reworked both full-screen transitions: web views use an auto-resizing container, and native window full screen uses AppKit's auto-hiding menu bar and toolbar while keeping fullscreen geometry separate from the saved normal frame.
- Documented the video formats WebKit offers YouTube on the verification Mac (H.264, VP9, VP9 HDR; no AV1).

## 0.1.0 — 2026-10-23

- Added the Rust/AppKit Watch app with an official YouTube `WKWebView` surface, native search/link input, navigation controls, and saved manual Up Next URLs.
- Added collapsible queue controls, native menus, startup search/link input, input validation, visible errors, and app-owned state persistence.
- Added local macOS build/run scripts and an ad-hoc-signed `DooTube.app` bundle with a 14.2 deployment target, high-resolution rendering, and no added privacy permissions or ATS exceptions.
- Preserved the original design/architecture history and documented the implementation/data boundaries.
- Verified the frozen optimized local bundle builds without warnings, launches, loads public search/video pages, rejects a non-YouTube startup URL, passes strict code-signature verification, and matches the 14.2 binary/bundle minimum. Recorded isolated in-process queue/resize/lifecycle-method smoke separately from read-only native window observation. Actual video playback, smoothness, and user-driven native input remain unverified because the required live-control grant returned `no-ui`. See [Verification](docs/verification.md) for fingerprints and precise limits.

YouTube Music, EQ, auto-next, custom playback controls, and background audio are not included.
