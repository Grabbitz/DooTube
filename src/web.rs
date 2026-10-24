//! The two web panes (YouTube and YouTube Music) and the script injected into them.
use objc2::rc::Retained;
use objc2_web_kit::WKWebView;
use serde::Deserialize;
use std::{
    cell::{Cell, RefCell},
    time::Instant,
};
use url::Url;

use crate::{
    core::is_music_url,
    google_domains::{GOOGLE_DOMAINS, GOOGLE_EXACT_HOSTS},
};

/// Name of the script message handler that reports audible playback.
pub const MESSAGE_HANDLER: &str = "dootube";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    YouTube,
    Music,
}

/// A fixed sidebar entry for one section.
pub struct Destination {
    pub title: &'static str,
    pub symbol: &'static str,
    pub url: &'static str,
    /// Path prefix that marks this destination as current; `/` matches only the root.
    pub path: &'static str,
}

const YOUTUBE_DESTINATIONS: [Destination; 4] = [
    Destination {
        title: "Home",
        symbol: "house",
        url: "https://www.youtube.com/",
        path: "/",
    },
    Destination {
        title: "Shorts",
        symbol: "play.square.stack",
        url: "https://www.youtube.com/shorts",
        path: "/shorts",
    },
    Destination {
        title: "Subscriptions",
        symbol: "rectangle.stack",
        url: "https://www.youtube.com/feed/subscriptions",
        path: "/feed/subscriptions",
    },
    Destination {
        title: "History",
        symbol: "clock.arrow.circlepath",
        url: "https://www.youtube.com/feed/history",
        path: "/feed/history",
    },
];

const MUSIC_DESTINATIONS: [Destination; 3] = [
    Destination {
        title: "Home",
        symbol: "house",
        url: "https://music.youtube.com/",
        path: "/",
    },
    Destination {
        title: "Explore",
        symbol: "safari",
        url: "https://music.youtube.com/explore",
        path: "/explore",
    },
    Destination {
        title: "Library",
        symbol: "music.note.list",
        url: "https://music.youtube.com/library",
        path: "/library",
    },
];

impl Mode {
    pub const ALL: [Mode; 2] = [Mode::YouTube, Mode::Music];

    pub fn index(self) -> usize {
        match self {
            Mode::YouTube => 0,
            Mode::Music => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::YouTube => "YouTube",
            Mode::Music => "YouTube Music",
        }
    }

    /// The pane a YouTube URL belongs in.
    pub fn for_url(url: &Url) -> Mode {
        if is_music_url(url) {
            Mode::Music
        } else {
            Mode::YouTube
        }
    }

    pub fn destinations(self) -> &'static [Destination] {
        match self {
            Mode::YouTube => &YOUTUBE_DESTINATIONS,
            Mode::Music => &MUSIC_DESTINATIONS,
        }
    }

    pub fn home(self) -> &'static str {
        self.destinations()[0].url
    }

    pub fn search_placeholder(self) -> &'static str {
        match self {
            Mode::YouTube => "Search or paste a YouTube link",
            Mode::Music => "Search songs, albums, artists",
        }
    }

    pub fn search_url(self, query: &str) -> String {
        let query = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
        match self {
            Mode::YouTube => format!("https://www.youtube.com/results?search_query={query}"),
            Mode::Music => format!("https://music.youtube.com/search?q={query}"),
        }
    }

    /// The search text of a results page in this section.
    pub fn search_query(self, url: &Url) -> Option<String> {
        let (path, key) = match self {
            Mode::YouTube => ("/results", "search_query"),
            Mode::Music => ("/search", "q"),
        };
        (url.path() == path)
            .then(|| {
                url.query_pairs()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.into_owned())
            })
            .flatten()
    }

    /// The official Google sign-in page for YouTube, returning to this section afterwards.
    pub fn sign_in_url(self) -> String {
        let encode =
            |text: &str| url::form_urlencoded::byte_serialize(text.as_bytes()).collect::<String>();
        let next = encode(self.home());
        let continue_to = encode(&format!(
            "https://www.youtube.com/signin?action_handle_signin=true&app=desktop&next={next}"
        ));
        format!(
            "https://accounts.google.com/ServiceLogin?service=youtube&uilel=3&passive=true&continue={continue_to}"
        )
    }
}

/// One persistent web view per section, so each keeps its page and history while the other is shown.
pub struct Pane {
    pub web: Retained<WKWebView>,
    /// A page has committed at least once (and the web process has not ended since).
    pub loaded: Cell<bool>,
    /// The first load failed, or the web process ended again soon after a reload; shown instead
    /// of the spinner until a retry commits.
    pub failed: Cell<bool>,
    /// `failed` because the web process ended repeatedly, not because the network failed.
    pub stopped: Cell<bool>,
    /// When WebKit last ended this pane's web process, to tell a one-off from a reload loop.
    pub ended_at: Cell<Option<Instant>>,
    /// The web process ended while this section was in the background; reload when it is shown.
    pub reload_when_shown: Cell<bool>,
    /// Last page DooTube requested here; used to retry and to know whether the pane was ever opened.
    pub last_requested: RefCell<Option<String>>,
    /// Video ID of the page when the document title last changed, so a title is
    /// never attributed to a video that YouTube has not finished switching to.
    pub title_video: RefCell<Option<String>>,
    /// Title of the media most recently heard in this pane, from the page's Media Session metadata.
    pub now_playing: RefCell<Option<String>>,
    /// A video from this pane is in Picture in Picture, so it stays playing while the window is away.
    pub pip_active: Cell<bool>,
}

impl Pane {
    pub fn new(web: Retained<WKWebView>) -> Self {
        Self {
            web,
            loaded: Cell::new(false),
            failed: Cell::new(false),
            stopped: Cell::new(false),
            ended_at: Cell::new(None),
            reload_when_shown: Cell::new(false),
            last_requested: RefCell::new(None),
            title_video: RefCell::new(None),
            now_playing: RefCell::new(None),
            pip_active: Cell::new(false),
        }
    }
}

/// A rect in page viewport points, top-left origin.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct PageRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Messages posted by [`page_script`].
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PageMessage {
    /// Media became audible; `title` is the page's Media Session title, if any.
    Audible {
        #[serde(default)]
        title: String,
    },
    /// The page requested (with the element's viewport rect) or exited video fullscreen.
    /// `id` names the request; a later request gets a new one.
    Fullscreen {
        active: bool,
        #[serde(default)]
        rect: Option<PageRect>,
        #[serde(default)]
        id: u64,
    },
    /// A video entered or left Picture in Picture.
    Pip { active: bool },
    /// Entering or leaving Picture in Picture was rejected; `reason` is the DOMException name.
    #[serde(rename = "pipfailed")]
    PipFailed {
        #[serde(default)]
        reason: String,
    },
    /// The account switcher response returned by YouTube's same-origin endpoint.
    #[serde(rename = "accountidentity")]
    AccountIdentity {
        #[serde(default)]
        response: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountIdentity {
    pub name: String,
    pub email: Option<String>,
    pub handle: Option<String>,
    /// Largest account photo YouTube offers, limited to Google's image hosts.
    pub photo: Option<String>,
}

/// Accepts only HTTPS photo URLs on Google's avatar hosts.
fn account_photo_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    (parsed.scheme() == "https"
        && [".ggpht.com", ".googleusercontent.com"]
            .iter()
            .any(|suffix| host.ends_with(suffix)))
    .then(|| url.to_owned())
}

/// Parses the active identity from a `/getAccountSwitcherEndpoint` response.
pub fn parse_account_identity(response: &str) -> Option<AccountIdentity> {
    fn run_text(value: Option<&serde_json::Value>) -> Option<&str> {
        let value = value?;
        value
            .get("simpleText")
            .and_then(|text| text.as_str())
            .or_else(|| {
                value
                    .get("runs")?
                    .as_array()?
                    .first()?
                    .get("text")?
                    .as_str()
            })
            .filter(|text| !text.trim().is_empty())
    }

    /// Name, handle, section email, and photo of the selected account.
    type Selected = (String, Option<String>, Option<String>, Option<String>);

    fn visit<'a>(
        value: &'a serde_json::Value,
        inherited_email: Option<&'a str>,
        selected: &mut Option<Selected>,
    ) {
        // A section's header applies to its contents, but never to sibling sections.
        let section_email = value
            .get("header")
            .and_then(|header| header.get("googleAccountHeaderRenderer"))
            .and_then(|header| run_text(header.get("email")))
            .or_else(|| {
                value
                    .get("googleAccountHeaderRenderer")
                    .and_then(|header| run_text(header.get("email")))
            })
            .or(inherited_email);
        if let Some(item) = value.get("accountItem") {
            if item.get("isSelected").and_then(|value| value.as_bool()) == Some(true) {
                if let Some(name) = run_text(item.get("accountName")) {
                    let handle = run_text(item.get("channelHandle")).map(str::to_owned);
                    let photo = item
                        .pointer("/accountPhoto/thumbnails")
                        .and_then(|thumbnails| thumbnails.as_array()?.last()?.get("url")?.as_str())
                        .and_then(account_photo_url);
                    *selected = Some((
                        name.to_owned(),
                        handle,
                        section_email.map(str::to_owned),
                        photo,
                    ));
                }
            }
        }
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    visit(value, section_email, selected);
                }
            }
            serde_json::Value::Object(values) => {
                for value in values.values() {
                    visit(value, section_email, selected);
                }
            }
            _ => {}
        }
    }

    let response = response.trim_start_matches(")]}'");
    let value: serde_json::Value = serde_json::from_str(response).ok()?;
    let mut selected = None;
    visit(&value, None, &mut selected);
    // Even a lone item is not necessarily the active account unless YouTube marks it selected.
    let (name, handle, email, photo) = selected?;
    Some(AccountIdentity {
        name,
        email,
        handle,
        photo,
    })
}

/// Fetches YouTube's active account from the page's own same-origin session.
pub fn account_identity_script() -> String {
    format!(
        r#"(() => {{
  const post = (response) => window.webkit.messageHandlers.{MESSAGE_HANDLER}.postMessage(
    JSON.stringify({{ type: "accountidentity", response }}));
  (async () => {{
    try {{
      if (location.origin !== "https://www.youtube.com") return post("");
      const response = await fetch("/getAccountSwitcherEndpoint", {{ credentials: "same-origin" }});
      if (!response.ok) return post("");
      post(await response.text());
    }} catch (_) {{ post(""); }}
  }})();
}})()"#
    )
}

/// Notice text for a rejected Picture-in-Picture request.
pub fn pip_failure_notice(reason: &str) -> String {
    match reason {
        "NotAllowedError" => {
            "macOS didn't allow Picture in Picture. Click the video to play it, then try again."
                .to_owned()
        }
        "NotSupportedError" | "InvalidStateError" => {
            "This video can't play in Picture in Picture.".to_owned()
        }
        "" => "Picture in Picture didn't start.".to_owned(),
        reason => format!("Picture in Picture didn't start ({reason})."),
    }
}

/// Toggles Picture in Picture for the playing (or first loaded) video; returns a status word.
/// The request itself settles later: a rejection is posted as a `pipfailed` page message.
/// Run only from a user's click or menu command; the Picture-in-Picture API expects a user gesture.
pub fn toggle_pip_script() -> String {
    format!(
        r#"(() => {{
  const failed = (error) => window.webkit.messageHandlers.{MESSAGE_HANDLER}.postMessage(
    JSON.stringify({{ type: "pipfailed", reason: (error && error.name) || "" }}));
  if (document.pictureInPictureElement) {{
    document.exitPictureInPicture().catch(failed);
    return "exit";
  }}
  const all = [...document.querySelectorAll("video")];
  const videos = all.filter((video) => video.readyState > 0);
  const video = videos.find((video) => !video.paused) || videos[0];
  if (!video) return all.length ? "notstarted" : "none";
  if (!document.pictureInPictureEnabled || video.disablePictureInPicture) return "unsupported";
  video.requestPictureInPicture().catch(failed);
  return "enter";
}})()"#
    )
}

/// Document-start script: hides the site chrome DooTube replaces and reports audible playback.
pub fn page_script() -> String {
    let youtube_css = serde_json::to_string(include_str!("youtube_chrome.css")).expect("string");
    let music_css = serde_json::to_string(include_str!("music_chrome.css")).expect("string");
    let mut script = format!(
        r#"(() => {{
  const host = location.hostname;
  const css = host === "music.youtube.com" ? {music_css}
    : ["www.youtube.com", "youtube.com", "m.youtube.com"].includes(host) ? {youtube_css} : null;
  if (css) {{
    const style = document.createElement("style");
    style.textContent = css;
    document.documentElement.appendChild(style);
  }}
  const post = (message) => window.webkit.messageHandlers.{MESSAGE_HANDLER}.postMessage(JSON.stringify(message));
  const report = (event) => {{
    const media = event.target;
    if (!(media instanceof HTMLMediaElement) || media.paused || media.muted || media.volume === 0) return;
    const meta = navigator.mediaSession && navigator.mediaSession.metadata;
    post({{ type: "audible", title: meta ? meta.title : "" }});
  }};
  document.addEventListener("playing", report, true);
  document.addEventListener("volumechange", report, true);
  document.addEventListener("enterpictureinpicture", () => post({{ type: "pip", active: true }}), true);
  document.addEventListener("leavepictureinpicture", () => post({{ type: "pip", active: false }}), true);
}})();"#
    );
    script.push_str(&fullscreen_shim_script());
    script
}

/// Keep page fullscreen events in the main app window instead of asking WebKit to create its
/// separate element-fullscreen window.
fn fullscreen_shim_script() -> String {
    r##"(() => {
  if (!["www.youtube.com", "youtube.com", "m.youtube.com", "music.youtube.com"].includes(location.hostname)) return;
  const post = (active, rect) => window.webkit.messageHandlers.__DOOTUBE_HANDLER__.postMessage(JSON.stringify({ type: "fullscreen", active, rect, id: session }));
  const style = document.createElement("style");
  style.textContent = `
    .dootube-fullscreen-element {
      position: fixed !important;
      inset: 0 !important;
      width: 100vw !important;
      height: 100vh !important;
      max-width: none !important;
      max-height: none !important;
      z-index: 2147483647 !important;
      background: #000 !important;
    }
    :root.dootube-fullscreen, :root.dootube-fullscreen body {
      overflow: hidden !important;
      scrollbar-width: none !important;
    }
    :root.dootube-fullscreen::-webkit-scrollbar,
    :root.dootube-fullscreen body::-webkit-scrollbar { display: none !important; }
  `;
  document.documentElement.appendChild(style);
  let activeElement = null;
  // Each request is a session; the app tags its replies with it so late ones are ignored.
  let session = 0;
  let pending = null;
  let exiting = null;
  const dispatch = (target, name) => {
    const event = new Event(name, { bubbles: true });
    target.dispatchEvent(event);
  };
  const notify = (element) => {
    dispatch(element, "fullscreenchange");
    dispatch(element, "webkitfullscreenchange");
    dispatch(document, "fullscreenchange");
    dispatch(document, "webkitfullscreenchange");
  };
  // The app zooms the video out from this rect, then calls __dootubeApplyFullscreen.
  const enter = (element) => {
    if (!(element instanceof Element)) return Promise.reject(new TypeError("Fullscreen target must be an Element"));
    if (activeElement || pending || exiting) return Promise.resolve();
    // YouTube requests fullscreen on <html>; the zoom starts from the player inside it.
    const page = element === document.documentElement || element === document.body || element.localName === "ytd-app";
    const r = ((page && document.querySelector("#movie_player")) || element).getBoundingClientRect();
    // The page reads as full screen at once, so a quick second toggle becomes an exit that
    // cancels this request instead of being lost while the app prepares the zoom.
    activeElement = element;
    session += 1;
    return new Promise((resolve) => {
      pending = { element, resolve, id: session };
      post(true, { x: r.left, y: r.top, width: r.width, height: r.height });
    });
  };
  window.__dootubeApplyFullscreen = (ok, id) => {
    const request = pending;
    if (!request || request.id !== id) return;
    pending = null;
    if (!ok && activeElement === request.element) activeElement = null;
    if (ok && activeElement === request.element) {
      activeElement.classList.add("dootube-fullscreen-element");
      activeElement.setAttribute("data-dootube-fullscreen", "");
      document.documentElement.classList.add("dootube-fullscreen");
      document.documentElement.setAttribute("data-dootube-fullscreen", "");
      notify(activeElement);
    }
    request.resolve();
  };
  // The app zooms back first, then calls __dootubeExitFullscreen to restore the page.
  const exit = () => {
    if (!activeElement) return Promise.resolve();
    // Toggled off before the zoom began: drop the request; the app stops preparing.
    if (pending) {
      pending.resolve();
      pending = null;
      activeElement = null;
      post(false);
      return Promise.resolve();
    }
    if (exiting) return exiting.promise;
    let resolve;
    const promise = new Promise((r) => { resolve = r; });
    exiting = { promise, resolve };
    post(false);
    return promise;
  };
  window.__dootubeExitFullscreen = (id) => {
    if (id !== session) return;
    const done = exiting;
    exiting = null;
    const element = activeElement;
    activeElement = null;
    if (element) {
      element.classList.remove("dootube-fullscreen-element");
      element.removeAttribute("data-dootube-fullscreen");
      document.documentElement.classList.remove("dootube-fullscreen");
      document.documentElement.removeAttribute("data-dootube-fullscreen");
      notify(element);
    }
    if (done) done.resolve();
  };
  const elementProto = Element.prototype;
  elementProto.requestFullscreen = function() { return enter(this); };
  elementProto.webkitRequestFullscreen = elementProto.requestFullscreen;
  elementProto.webkitRequestFullScreen = elementProto.requestFullscreen;
  const docProto = Document.prototype;
  docProto.exitFullscreen = exit;
  docProto.webkitExitFullscreen = exit;
  docProto.webkitCancelFullScreen = exit;
  for (const key of ["fullscreenElement", "webkitFullscreenElement", "webkitCurrentFullScreenElement"]) {
    Object.defineProperty(docProto, key, { configurable: true, get: () => activeElement });
  }
  // With WebKit's own fullscreen disabled these handler properties are gone, and YouTube only
  // listens for fullscreenchange when `"onfullscreenchange" in document`.
  for (const proto of [docProto, elementProto]) {
    for (const name of ["onfullscreenchange", "onwebkitfullscreenchange", "onfullscreenerror", "onwebkitfullscreenerror"]) {
      if (name in proto) continue;
      const handlers = new WeakMap();
      Object.defineProperty(proto, name, {
        configurable: true,
        get() { return handlers.get(this) || null; },
        set(handler) {
          const old = handlers.get(this);
          if (old) this.removeEventListener(name.slice(2), old);
          if (typeof handler === "function") {
            handlers.set(this, handler);
            this.addEventListener(name.slice(2), handler);
          } else {
            handlers.delete(this);
          }
        },
      });
    }
  }
  for (const key of ["webkitIsFullScreen"]) {
    Object.defineProperty(docProto, key, { configurable: true, get: () => activeElement !== null });
  }
  for (const key of ["fullscreenEnabled", "webkitFullscreenEnabled"]) {
    Object.defineProperty(docProto, key, { configurable: true, get: () => true });
  }
  // F toggles from DooTube's own state rather than YouTube's, which only catches up after the
  // zoom, so quick repeated presses reverse the zoom instead of being dropped.
  const toggle = () => {
    if (exiting) {
      exiting.resolve();
      exiting = null;
      post(true);
    } else if (activeElement) {
      exit();
    } else {
      enter(document.documentElement);
    }
  };
  window.addEventListener("keydown", (event) => {
    if (event.key !== "f" && event.key !== "F") return;
    if (event.metaKey || event.ctrlKey || event.altKey || event.repeat) return;
    const target = event.composedPath()[0];
    if (target instanceof Element && target.closest("input, textarea, select, [contenteditable]:not([contenteditable=false])")) return;
    if (!document.querySelector("#movie_player video")) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    toggle();
  }, true);
  document.addEventListener("keydown", (event) => {
    if (!activeElement || (event.key !== "Escape" && event.key !== "Esc")) return;
    setTimeout(() => {
      if (event.defaultPrevented || !activeElement) return;
      const menu = Array.from(document.querySelectorAll(".ytp-contextmenu, .ytp-popup, [role=dialog]"))
        .some((element) => element.getClientRects().length > 0);
      if (!menu) exit();
    }, 0);
  }, true);
})();"##
        .replace("__DOOTUBE_HANDLER__", MESSAGE_HANDLER)
}

/// Keep its exact host rule aligned with `core::is_google_sign_in_url`; both are generated from
/// the shared Google domain and exact-host constants.
/// Passkey ceremonies in a `WKWebView` are only authorized for apps holding Apple's restricted
/// web-browser public-key-credential entitlement (or the site's associated domains), so every
/// request is rejected. Google's page still detects `PublicKeyCredential`, offers a passkey, and
/// dead-ends on a "check Bluetooth" error. Without the API, Google goes straight to password and
/// its other challenges. Scoped to Google sign-in hosts; YouTube pages keep WebKit's defaults.
pub fn sign_in_passkey_script() -> String {
    let domains = serde_json::to_string(GOOGLE_DOMAINS).expect("Google domains serialize to JSON");
    let exact_hosts =
        serde_json::to_string(GOOGLE_EXACT_HOSTS).expect("Google hosts serialize to JSON");
    format!(
        r#"(() => {{
  const host = location.hostname;
  const googleDomains = {domains};
  const exactHosts = {exact_hosts};
  const regionalAccount = googleDomains.some(domain => host === `accounts.${{domain}}`);
  const regionalCookieEndpoint = location.pathname.startsWith("/accounts/") &&
    googleDomains.some(domain => host === domain || host === `www.${{domain}}`);
  if (!exactHosts.includes(host) && !regionalAccount && !regionalCookieEndpoint) return;
  try {{
    delete window.PublicKeyCredential;
    Object.defineProperty(window, "PublicKeyCredential", {{ value: undefined, configurable: false }});
  }} catch (error) {{}}
  try {{
    const credentials = navigator.credentials;
    if (!credentials) return;
    const prototype = Object.getPrototypeOf(credentials);
    const wrap = (original) => typeof original !== "function" ? original : function (options) {{
      if (options && options.publicKey) {{
        return Promise.reject(new DOMException("Passkeys aren't available in DooTube.", "NotAllowedError"));
      }}
      return original.apply(this, arguments);
    }};
    prototype.get = wrap(prototype.get);
    prototype.create = wrap(prototype.create);
  }} catch (error) {{}}
}})();"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_script_message_payloads_parse() {
        let parse = |json: &str| serde_json::from_str::<PageMessage>(json).unwrap();
        assert!(matches!(
            parse(r#"{"type":"pipfailed","reason":"NotAllowedError"}"#),
            PageMessage::PipFailed { reason } if reason == "NotAllowedError"
        ));
        assert!(matches!(
            parse(r#"{"type":"pip","active":true}"#),
            PageMessage::Pip { active: true }
        ));
        assert!(matches!(
            parse(
                r#"{"type":"fullscreen","active":true,"rect":{"x":24,"y":80.5,"width":854,"height":480},"id":3}"#
            ),
            PageMessage::Fullscreen {
                active: true,
                rect: Some(PageRect {
                    x: 24.,
                    y: 80.5,
                    width: 854.,
                    height: 480.
                }),
                id: 3
            }
        ));
        assert!(matches!(
            parse(r#"{"type":"fullscreen","active":false}"#),
            PageMessage::Fullscreen {
                active: false,
                rect: None,
                id: 0
            }
        ));
        assert!(
            matches!(parse(r#"{"type":"audible","title":"Song"}"#), PageMessage::Audible { title } if title == "Song")
        );
        assert!(pip_failure_notice("NotAllowedError").contains("Click the video"));
    }

    #[test]
    fn account_menu_identity_parses_selected_account() {
        let response = r#")]}'
{"actions":[{"openPopupAction":{"popup":{"multiPageMenuRenderer":{"sections":[
  {"accountSectionListRenderer":{"header":{"googleAccountHeaderRenderer":{"email":{"runs":[{"text":"owner@example.com"}]}}},"contents":[{"accountItemSectionRenderer":{"contents":[
    {"accountItem":{"accountName":{"simpleText":"Personal"},"channelHandle":{"simpleText":"@personal"},"isSelected":false}},
    {"accountItem":{"accountName":{"simpleText":"Video Channel"},"channelHandle":{"simpleText":"@videos"},"isSelected":true}}
  ]}}]}}
]}}}}]}"#;
        assert_eq!(
            parse_account_identity(response),
            Some(AccountIdentity {
                name: "Video Channel".to_owned(),
                email: Some("owner@example.com".to_owned()),
                handle: Some("@videos".to_owned()),
                photo: None,
            })
        );
    }

    #[test]
    fn account_photo_uses_largest_google_hosted_thumbnail() {
        let with_photo = |url: &str| {
            format!(
                r#"{{"accountItem":{{"accountName":{{"simpleText":"Me"}},"isSelected":true,"accountPhoto":{{"thumbnails":[{{"url":"https://yt3.ggpht.com/small=s48"}},{{"url":"{url}"}}]}}}}}}"#
            )
        };
        let photo = |url: &str| parse_account_identity(&with_photo(url)).and_then(|id| id.photo);
        assert_eq!(
            photo("https://yt3.ggpht.com/big=s88").as_deref(),
            Some("https://yt3.ggpht.com/big=s88")
        );
        assert_eq!(photo("https://evil.example.com/a.png"), None);
        assert_eq!(photo("http://yt3.ggpht.com/big=s88"), None);
    }

    #[test]
    fn account_menu_identity_uses_selected_section_email() {
        let response = r#")]}'
{"actions":[{"openPopupAction":{"popup":{"multiPageMenuRenderer":{"sections":[
  {"accountSectionListRenderer":{"header":{"googleAccountHeaderRenderer":{"email":{"simpleText":"a@example.com"}}},"contents":[{"accountItemSectionRenderer":{"contents":[
    {"accountItem":{"accountName":{"simpleText":"First Channel"},"channelHandle":{"simpleText":"@first"},"isSelected":false}}
  ]}}]}},
  {"accountSectionListRenderer":{"header":{"googleAccountHeaderRenderer":{"email":{"runs":[{"text":"b@example.com"}]}}},"contents":[{"accountItemSectionRenderer":{"contents":[
    {"accountItem":{"accountName":{"runs":[{"text":"Second Channel"}]},"channelHandle":{"runs":[{"text":"@second"}]},"isSelected":true}}
  ]}}]}}
]}}}}]}"#;
        assert_eq!(
            parse_account_identity(response),
            Some(AccountIdentity {
                name: "Second Channel".to_owned(),
                email: Some("b@example.com".to_owned()),
                handle: Some("@second".to_owned()),
                photo: None,
            })
        );
    }

    #[test]
    fn account_menu_identity_requires_selected_account() {
        let response = r#")]}'
{"actions":[{"openPopupAction":{"popup":{"multiPageMenuRenderer":{"sections":[
  {"accountSectionListRenderer":{"header":{"googleAccountHeaderRenderer":{"email":{"simpleText":"owner@example.com"}}},"contents":[{"accountItemSectionRenderer":{"contents":[
    {"accountItem":{"accountName":{"simpleText":"Channel"},"channelHandle":{"simpleText":"@channel"},"isSelected":false}}
  ]}}]}}
]}}}}]}"#;
        assert_eq!(parse_account_identity(response), None);
    }

    #[test]
    fn account_menu_identity_allows_handle_or_name_only() {
        let handle_only = r#")]}'
{"data":{"actions":[{"getMultiPageMenuAction":{"menu":{"multiPageMenuRenderer":{"sections":[
  {"accountSectionListRenderer":{"contents":[{"accountItemSectionRenderer":{"contents":[
    {"accountItem":{"accountName":{"simpleText":"Creator"},"channelHandle":{"simpleText":"@creator"},"isSelected":true}}
  ]}}]}}
]}}}}]}}"#;
        assert_eq!(
            parse_account_identity(handle_only),
            Some(AccountIdentity {
                name: "Creator".to_owned(),
                email: None,
                handle: Some("@creator".to_owned()),
                photo: None,
            })
        );

        let name_only = r#"{"data":{"actions":[{"getMultiPageMenuAction":{"menu":{"multiPageMenuRenderer":{"sections":[
  {"accountSectionListRenderer":{"contents":[{"accountItemSectionRenderer":{"contents":[
    {"accountItem":{"accountName":{"simpleText":"Viewer"},"isSelected":true}}
  ]}}]}}
]}}}}]}}"#;
        assert_eq!(
            parse_account_identity(name_only),
            Some(AccountIdentity {
                name: "Viewer".to_owned(),
                email: None,
                handle: None,
                photo: None,
            })
        );
    }
}
