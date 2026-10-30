//! Page state.

use super::*;

impl AppDelegate {
    pub(crate) fn current_url(&self) -> Option<Url> {
        let text = unsafe { self.active()?.web.URL() }?
            .absoluteString()?
            .to_string();
        Url::parse(&text).ok()
    }

    pub(crate) fn current_video_url(&self) -> Option<Url> {
        match parse_input(self.current_url()?.as_str()) {
            Ok(Navigation::Video(url)) => Some(url),
            _ => None,
        }
    }

    pub(crate) fn on_page_url_changed(&self) {
        let mode = self.ivars().mode.get();
        // Show the open results page's query; clear a stale one elsewhere unless it's being typed.
        if let Some(bar) = self.ivars().bar.get() {
            let field = &bar.search;
            if field.currentEditor().is_none() {
                let query = self
                    .current_url()
                    .and_then(|url| mode.search_query(&url))
                    .unwrap_or_default();
                field.setStringValue(&ns(&query));
            }
        }
        self.sync_sidebar_selection();
        self.update_toolbar();
        self.update_window_title();
    }

    pub(crate) fn on_page_title_changed(&self, web: &WKWebView) {
        let Some(pane) = self.pane_of(web) else {
            return;
        };
        let url = unsafe { web.URL() }
            .and_then(|url| url.absoluteString())
            .and_then(|text| Url::parse(&text.to_string()).ok());
        *pane.title_video.borrow_mut() = url.as_ref().and_then(video_id);
        if !self.is_active(web) {
            return;
        }
        self.update_window_title();
        // Fill in titles for videos saved before their title was known.
        if let Some(url) = self.current_video_url()
            && let Some(title) = self.current_title_for(&url)
            && let Some(Some(index)) = self.with_state(|state| {
                state
                    .position(&url)
                    .filter(|index| state.up_next[*index].title.is_none())
            })
        {
            self.change_queue(|state| {
                state.up_next[index].title = Some(title);
                true
            });
        }
    }

    pub(crate) fn update_window_title(&self) {
        let (Some(window), Some(pane)) = (self.ivars().window.get(), self.active()) else {
            return;
        };
        let mode = self.ivars().mode.get();
        let url = self.current_url();
        let page_title =
            unsafe { pane.web.title() }.and_then(|title| clean_page_title(&title.to_string()));
        let playing = url
            .as_ref()
            .filter(|url| is_music_url(url) && url.path() == "/watch")
            .and_then(|_| pane.now_playing.borrow().clone());
        let destination = url
            .as_ref()
            .and_then(|url| self.destination_for(url))
            .map(|index| mode.destinations()[index].title.to_owned());
        let title = playing
            .or(page_title)
            .or(destination)
            .unwrap_or_else(|| mode.label().to_owned());
        window.setTitle(&ns(&title));
    }

    pub(crate) fn update_toolbar(&self) {
        let ivars = self.ivars();
        if let Some(bar) = ivars.bar.get() {
            let web = self.active().map(|pane| &pane.web);
            bar.set_navigation(
                web.is_some_and(|web| unsafe { web.canGoBack() }),
                web.is_some_and(|web| unsafe { web.canGoForward() }),
            );
            bar.set_saved(self.save_state_for_current());
        }
    }
}
