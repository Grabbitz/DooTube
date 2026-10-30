//! Navigation.

use super::*;

impl AppDelegate {
    pub(crate) fn navigation_policy(
        &self,
        web_view: &WKWebView,
        action: &WKNavigationAction,
    ) -> WKNavigationActionPolicy {
        let request = unsafe { action.request() };
        let target = request.URL();
        let frame = unsafe { action.targetFrame() };
        let main_frame = frame
            .as_ref()
            .is_some_and(|info| unsafe { info.isMainFrame() });
        let subframe = frame
            .as_ref()
            .is_some_and(|info| !unsafe { info.isMainFrame() });
        let activated = unsafe { action.navigationType() } == WKNavigationType::LinkActivated;
        let parsed = target
            .as_ref()
            .and_then(|url| url.absoluteString())
            .and_then(|text| Url::parse(&text.to_string()).ok());
        // Let WKUIDelegate decide whether a target=_blank request is handled in-pane,
        // opened in the browser, or rejected.
        if frame.is_none() {
            return WKNavigationActionPolicy::Allow;
        }
        if subframe
            && parsed
                .as_ref()
                .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
        {
            return WKNavigationActionPolicy::Allow;
        }
        if main_frame && let Some(url) = parsed.as_ref() {
            if is_google_sign_in_url(url) {
                return WKNavigationActionPolicy::Allow;
            }
            if is_supported_youtube_url(url) {
                // A link into the other section opens there, keeping this section's page.
                let pane_mode = self.mode_of(web_view);
                let target_mode = Mode::for_url(url);
                let reroute = match pane_mode {
                    Some(Mode::YouTube) => target_mode == Mode::Music,
                    Some(Mode::Music) => activated && target_mode == Mode::YouTube,
                    None => false,
                };
                if reroute {
                    self.load_url(url.as_str());
                    return WKNavigationActionPolicy::Cancel;
                }
                return WKNavigationActionPolicy::Allow;
            }
        }
        let browser_eligible = activated
            && parsed.as_ref().is_some_and(|url| {
                matches!(url.scheme(), "http" | "https")
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.port().is_none()
            });
        let current_is_sign_in = main_frame
            && !browser_eligible
            && unsafe { web_view.URL() }
                .and_then(|url| url.absoluteString())
                .and_then(|text| Url::parse(&text.to_string()).ok())
                .is_some_and(|url| is_google_sign_in_url(&url));
        if main_frame
            && !browser_eligible
            && current_is_sign_in
            && let Some(blocked) = parsed.as_ref()
        {
            let target = format!(
                "{}{}",
                blocked.host_str().unwrap_or("unknown host"),
                blocked.path()
            );
            self.show_sticky_notice(&format!(
                "Sign-in tried to open {target}, which DooTube blocked. Use DooTube → Sign In… again or Open in Browser."
            ));
            return WKNavigationActionPolicy::Cancel;
        }
        if frame.is_some() && main_frame && activated {
            if let (Some(url), Some(parsed_url)) = (target.as_ref(), parsed.as_ref()) {
                if matches!(parsed_url.scheme(), "http" | "https")
                    && parsed_url.username().is_empty()
                    && parsed_url.password().is_none()
                    && parsed_url.port().is_none()
                {
                    if NSWorkspace::sharedWorkspace().openURL(url) {
                        self.show_notice("Opened in your browser.");
                    } else {
                        self.show_notice("Couldn't open that link in your browser.");
                    }
                } else {
                    self.show_notice("That link can't be opened from DooTube.");
                }
            }
        } else if main_frame {
            self.show_notice("Blocked a redirect away from YouTube. Your page is unchanged.");
        }
        WKNavigationActionPolicy::Cancel
    }

    /// Opens search text (in the current section) or a YouTube link (in its section);
    /// returns whether a load started.
    pub(crate) fn navigate(&self, text: &str) -> bool {
        match parse_input(text) {
            Ok(Navigation::Search(query)) => {
                let mode = self.ivars().mode.get();
                self.load_in(mode, &mode.search_url(&query));
                true
            }
            Ok(Navigation::Page(url) | Navigation::Video(url)) => {
                self.load_url(url.as_str());
                true
            }
            Err(message) => {
                self.show_notice(message);
                false
            }
        }
    }

    /// Loads a YouTube page in the section it belongs to.
    pub(crate) fn load_url(&self, url: &str) {
        let mode = Url::parse(url).map_or(Mode::YouTube, |url| Mode::for_url(&url));
        self.load_in(mode, url);
    }

    pub(crate) fn load_in(&self, mode: Mode, url: &str) {
        self.set_queue_visible(false);
        let Some(pane) = self.pane(mode) else {
            return;
        };
        let Some(nsurl) = NSURL::URLWithString(&ns(url)) else {
            self.show_notice("Couldn't open that YouTube link.");
            return;
        };
        *pane.last_requested.borrow_mut() = Some(url.to_owned());
        // A new navigation replaces the previous failure while WebKit tries the new request.
        pane.failed.set(false);
        pane.stopped.set(false);
        // This load replaces any pending reload of a page whose web process ended.
        pane.reload_when_shown.set(false);
        if self.ivars().mode.get() != mode {
            self.switch_mode(mode);
        }
        unsafe { pane.web.loadRequest(&NSURLRequest::requestWithURL(&nsurl)) };
        self.show_active_pane();
    }

    pub(crate) fn reload_page(&self) {
        self.reload_in(self.ivars().mode.get());
    }

    pub(crate) fn reload_in(&self, mode: Mode) {
        let Some(pane) = self.pane(mode) else {
            return;
        };
        pane.failed.set(false);
        pane.stopped.set(false);
        pane.reload_when_shown.set(false);
        // WebKit reloads the current page, also after a failed load or an ended web process.
        if unsafe { pane.web.reload() }.is_some() {
            self.show_active_pane();
            return;
        }
        // Nothing loaded yet: request the page again.
        let url = unsafe { pane.web.URL() }
            .and_then(|url| url.absoluteString())
            .map(|url| url.to_string())
            .or_else(|| pane.last_requested.borrow().clone())
            .unwrap_or_else(|| mode.home().to_owned());
        self.load_in(mode, &url);
    }

    /// WebKit ended a page's web process: it crashed, or macOS reclaimed its memory. Like Safari,
    /// the page reloads on its own (a section in the background when it is next shown). If it
    /// ends again within a minute, the section shows a Try Again state instead of reloading in a
    /// loop.
    pub(crate) fn on_web_process_ended(&self, web: &WKWebView) {
        let Some(mode) = self.mode_of(web) else {
            return;
        };
        if self.ivars().fullscreen_mode.get() == Some(mode) {
            self.exit_video_fullscreen(false);
        }
        let Some(pane) = self.pane(mode) else {
            return;
        };
        pane.loaded.set(false);
        pane.pip_active.set(false);
        if mode == Mode::Music {
            // The player bar went with the process; the reloaded page reports again once playing.
            *self.ivars().track.borrow_mut() = web::Track::default();
            self.update_now_playing();
        }
        let now = Instant::now();
        let repeated = pane
            .ended_at
            .replace(Some(now))
            .is_some_and(|last| now.duration_since(last) < Duration::from_secs(60));
        if repeated {
            pane.failed.set(true);
            pane.stopped.set(true);
            pane.reload_when_shown.set(false);
        } else if self.is_active(web) {
            self.reload_in(mode);
        } else {
            pane.reload_when_shown.set(true);
        }
        self.show_active_pane();
    }

    pub(crate) fn on_load_error(&self, web: &WKWebView, error: &NSError) {
        let domain = error.domain().to_string();
        // Cancelled loads and loads stopped by the navigation policy are not failures.
        if error.code() == -999 || (domain == "WebKitErrorDomain" && error.code() == 102) {
            return;
        }
        let Some(pane) = self.pane_of(web) else {
            return;
        };
        if !pane.loaded.get() {
            pane.failed.set(true);
            self.show_active_pane();
        } else if self.is_active(web) {
            self.show_notice(
                "YouTube didn't finish loading. Check your connection, then press ⌘R.",
            );
        }
    }

    pub(crate) fn open_in_browser(&self) {
        let url = self
            .current_url()
            .filter(is_supported_youtube_url)
            .map(|url| url.to_string())
            .unwrap_or_else(|| self.ivars().mode.get().home().to_owned());
        if let Some(url) = NSURL::URLWithString(&ns(&url))
            && !NSWorkspace::sharedWorkspace().openURL(&url)
        {
            self.show_notice("Couldn't open your browser.");
        }
    }
}
