# DooTube

A native Rust macOS app for watching YouTube, with AppKit window controls around a visible official YouTube website in `WKWebView`. Watch comes first; this is not a custom YouTube player or a shipped YouTube Music client.

## Build and run

Build requirements: Rust with Cargo, macOS, and Apple Command Line Tools (`xcode-select --install` if they are not already installed). The observed toolchain is Rust/Cargo 1.99.1; the minimum compiler for the complete resolved dependency graph has not been verified. The app's deployment target is macOS 14.2; this is not a claim of runtime testing on 14.2. See the [verification environment](docs/verification.md) for the Mac actually used. Cargo downloads the dependencies listed in `Cargo.lock` on the first build. No global JavaScript tools, web server, API key, or account is required to build.

```sh
./scripts/run.sh
```

This builds the `dootube` binary and opens `target/debug/DooTube.app`. To build a bundle without opening it:

```sh
./scripts/build-app.sh
```

For an optimized local build:

```sh
./scripts/run.sh --release
# Bundle: target/release/DooTube.app
```

The optimized bundle at `target/release/DooTube.app` was built and launched in the recorded verification environment. See [Verification](docs/verification.md) for the frozen binary fingerprint, observed page-loading results, and playback/input limitations.

To start with a public YouTube URL or search:

```sh
./scripts/run.sh -- "https://www.youtube.com/watch?v=jNQXAC9IVRw"
./scripts/run.sh --release -- "NASA space videos"
```

The `--` separates build options from the app's startup input. Input arguments are joined as search text or parsed as a YouTube link. Quit DooTube before using a startup argument or rebuilding: Launch Services reuses a running app instead of creating a second player, and startup arguments do not replace the current page of an already-running instance.

Scripts resolve paths relative to themselves and support repository paths containing spaces. Both the compiler deployment target and bundle minimum are macOS 14.2. Builds use the host architecture; this is not a universal binary. Bundles use the provisional identifier `com.dootube.app`, are ad-hoc signed for local development, and are not Developer ID signed or notarized. The scripts do not change Gatekeeper, quarantine, permissions, or global audio settings. Quit a running app before rebuilding the bundle you are using.

### Download

Get `DooTube.dmg` from the [latest release](../../releases/latest), open it, and drag DooTube to Applications. It is an Apple-silicon build signed ad hoc, not notarized by Apple, so the first launch is blocked: open System Settings → Privacy & Security and click **Open Anyway** (or run `xattr -dr com.apple.quarantine /Applications/DooTube.app`).

To build the disk image yourself: `./scripts/make-dmg.sh` (writes `target/DooTube.dmg`).

### Install from source

```sh
./scripts/install.sh
```

Add `--open` to launch after installing. This builds an optimized, ad-hoc-signed local build in `/Applications/DooTube.app`, not a Developer ID-signed or notarized distribution. Afterwards, open DooTube from Applications, Launchpad, or Spotlight, or keep it in the Dock. Re-run the script to update; it quits a running DooTube before replacing the installed app. Sign-in and website data carry over through the unchanged `com.dootube.app` bundle identifier; the installer does not touch `~/Library` data.

The icon source is `resources/icon/DooTube.svg`; regenerate with `scripts/make-icon.sh`.

## Using DooTube

DooTube opens to YouTube Home in a native window: a sidebar on the left, a unified toolbar on top, and the official page filling the rest. The toolbar's YouTube | Music switch (Command-1 / Command-2) flips between two separate sections, YouTube and YouTube Music, each keeping its own page and history. Only one section plays at a time: when one starts making sound, the other pauses. Music keeps playing while you browse YouTube until you play something there. The websites' own headers and side menus are hidden because the toolbar and sidebar replace them; the official players, their controls, ads, captions, and page content are unchanged.

Search in the toolbar searches the current section; pasting a YouTube or YouTube Music link opens it in the section it belongs to. The YouTube sidebar has Home, Shorts, Subscriptions, and History; the Music sidebar has Home, Explore, and Library. Up Next lists videos and songs you saved with the toolbar's Save button or Up Next → Save Current Video (Command-S), shown by title. Click a saved item to open it; right-click it, or use the Up Next menu while the sidebar is focused, to move it up/down (Option-Command-Up/Down) or remove it (Command-Delete). Opening the next entry is always manual.

The account button in the toolbar (also DooTube → Sign In to YouTube…) opens Google's own sign-in page inside the app; both sections share the session. When YouTube provides it, the toolbar shows the account's photo and the account menu shows its name with the email (or @handle) beneath. The toolbar tooltip shows the name with the email, or the handle if no email is available. This identity is fetched from YouTube and kept in memory only. If the lookup is unavailable, the menu falls back to “Signed in to YouTube.” Passkeys can't be used inside DooTube (macOS allows them in embedded web views only for apps Apple approves as web browsers), so the sign-in page doesn't offer them and Google goes straight to your password, phone prompt, or other verification. If DooTube blocks a main-frame redirect during sign-in, a sticky notice shows the blocked host and path (without query or fragment) and suggests restarting sign-in or opening the page in a browser. Sign Out uses YouTube's sign-out.

Pressing F or the player's full-screen button fills the screen with the video alone, not the app window: the video zooms out from its place on the page while the background fades to black, and the Dock and menu bar hide. F, Escape, or the button again zooms it back. The official player and its controls are unchanged. Control-Command-F (or the green window button) enters native window full screen; AppKit auto-hides the menu bar and toolbar and reveals them on hover, so search and the section switch remain available. The sidebar stays in place and can still be toggled with Control-Command-S. Full-screen window geometry uses a separate autosave name from the normal window frame. The toolbar's Picture in Picture button (View → Picture in Picture, Control-Command-P) floats the playing video; it keeps playing while DooTube is hidden or minimized, unlike other media, which pauses. Video quality, HDR, and resolution are chosen in the player's settings menu: YouTube offers what this Mac's WebKit can decode. See [Implementation](docs/implementation.md#video-formats).

Command-L focuses search, Command-[ and Command-] go back and forward, Command-R reloads, and Control-Command-S shows or hides the sidebar. File → Open in Browser hands the current page to your default browser. Standard editing shortcuts are available; Control-Command-F toggles native full screen for the window, Command-W closes the window, Command-H hides the app, and Command-Q quits. These describe the implemented commands, not a claim that every keyboard/menu interaction was automated in verification.

## Scope and verification

The app's native chrome is separate from the official web media surface: YouTube owns its page, player, ads, captions, quality options, availability, and sign-in restrictions. Loading a page does not prove that video playback works. Network, region, provider changes, and account requirements can affect the embedded website.

See [Implementation](docs/implementation.md) for the implemented controls and data boundaries, and [Verification](docs/verification.md) for observed runtime behavior and remaining verification limits. The [design brief](docs/design-brief.md) and [architecture proposal](docs/architecture.md) preserve design history and future options; their full scope is not a shipped-feature claim.
