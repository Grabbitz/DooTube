//! Panes and sections.

use super::*;

impl AppDelegate {
    pub(crate) fn pane(&self, mode: Mode) -> Option<&Pane> {
        self.ivars().panes.get().map(|panes| &panes[mode.index()])
    }

    pub(crate) fn active(&self) -> Option<&Pane> {
        self.pane(self.ivars().mode.get())
    }

    pub(crate) fn mode_of(&self, web: &WKWebView) -> Option<Mode> {
        Mode::ALL.into_iter().find(|mode| {
            self.pane(*mode)
                .is_some_and(|pane| std::ptr::eq(&*pane.web, web))
        })
    }

    pub(crate) fn pane_of(&self, web: &WKWebView) -> Option<&Pane> {
        self.pane(self.mode_of(web)?)
    }

    pub(crate) fn is_active(&self, web: &WKWebView) -> bool {
        self.mode_of(web) == Some(self.ivars().mode.get())
    }

    /// Shows a section, opening its home page the first time it is shown.
    pub(crate) fn switch_mode(&self, mode: Mode) {
        if mode != self.ivars().mode.get() && self.ivars().fullscreen_mode.get().is_some() {
            self.exit_video_fullscreen(false);
        }
        let ivars = self.ivars();
        ivars.mode.set(mode);
        self.set_queue_visible(false);
        if let Some(bar) = ivars.bar.get() {
            bar.set_mode(mode);
            let field = &bar.search;
            let query = self
                .current_url()
                .and_then(|url| mode.search_query(&url))
                .unwrap_or_default();
            field.setStringValue(&ns(&query));
        }
        let never_opened = self
            .pane(mode)
            .is_some_and(|pane| pane.last_requested.borrow().is_none());
        if never_opened {
            self.load_in(mode, mode.home());
        } else if self
            .pane(mode)
            .is_some_and(|pane| pane.reload_when_shown.get())
        {
            self.reload_in(mode);
        }
        self.show_active_pane();
        self.reload_sidebar();
        self.update_window_title();
        self.update_backdrop();
        self.update_now_playing();
    }

    /// YouTube Music's player bar changed: keep its track, fetch new artwork, refresh the card.
    pub(crate) fn on_now_playing(&self, track: web::Track) {
        let ivars = self.ivars();
        let artwork = track.artwork_url();
        let stale = ivars
            .artwork
            .borrow()
            .as_ref()
            .is_some_and(|(url, _)| Some(url) != artwork.as_ref());
        if stale {
            *ivars.artwork.borrow_mut() = None;
        }
        if let Some(url) = artwork
            && ivars.artwork.borrow().is_none()
        {
            load_image(url, AppDelegate::on_artwork);
        }
        *ivars.track.borrow_mut() = track;
        self.update_now_playing();
    }

    /// Shows downloaded artwork if it still belongs to the current track.
    pub(crate) fn on_artwork(&self, url: &str, bytes: Vec<u8>) {
        let ivars = self.ivars();
        if ivars.track.borrow().artwork_url().as_deref() != Some(url) {
            return;
        }
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&bytes))
        else {
            return;
        };
        *ivars.artwork.borrow_mut() = Some((url.to_owned(), image));
        self.update_now_playing();
    }

    /// The sidebar card shows what Music is playing while the YouTube section is shown; in the
    /// Music section YouTube Music's own player bar is on screen.
    pub(crate) fn update_now_playing(&self) {
        let ivars = self.ivars();
        let Some(card) = ivars.now_playing.get() else {
            return;
        };
        let track = ivars.track.borrow();
        if track.title.is_empty() || ivars.mode.get() == Mode::Music {
            card.hide();
            return;
        }
        let artwork = ivars.artwork.borrow();
        card.show(
            &track.title,
            &track.artist,
            track.playing,
            artwork.as_ref().map(|(_, image)| &**image),
        );
    }

    /// The Dock icon's menu: what YouTube Music is on, with its controls. `None` (only the
    /// standard Dock items) until Music has a track.
    pub(crate) fn music_dock_menu(&self) -> Option<Retained<NSMenu>> {
        let mtm = self.mtm();
        let track = self.ivars().track.borrow();
        if track.title.is_empty() {
            return None;
        }
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("DooTube"));
        // Dock menus don't validate their items, so titles and states are set here.
        menu.setAutoenablesItems(false);
        let heading = if track.artist.is_empty() {
            track.title.clone()
        } else {
            format!("{} – {}", track.title, track.artist)
        };
        let now_playing = ui::menu_item(mtm, &heading, None, "", None);
        now_playing.setEnabled(false);
        menu.addItem(&now_playing);
        for (title, action) in [
            (
                if track.playing { "Pause" } else { "Play" },
                sel!(musicPlayPause:),
            ),
            ("Next", sel!(musicNext:)),
            ("Previous", sel!(musicPrevious:)),
        ] {
            menu.addItem(&ui::menu_item(mtm, title, Some(action), "", Some(self)));
        }
        Some(menu)
    }

    /// Presses YouTube Music's own player-bar button; the Music section keeps its page and
    /// player while the YouTube section is shown.
    pub(crate) fn music_control(&self, action: &str) {
        if let Some(pane) = self.pane(Mode::Music) {
            run_js(&pane.web, &web::music_control_script(action));
        }
    }

    /// Matches the content background, and with it the strip behind the toolbar, to the active page.
    pub(crate) fn update_backdrop(&self) {
        if let Some(backdrop) = self.ivars().backdrop.get() {
            let color = self
                .active()
                .map(|pane| unsafe { pane.web.underPageBackgroundColor() })
                .unwrap_or_else(cinema_bar::background_color);
            backdrop.setFillColor(&color);
        }
    }

    /// Shows the active pane's page, or its loading/failure state, and hides the other pane.
    pub(crate) fn show_active_pane(&self) {
        let active_mode = self.ivars().mode.get();
        let ivars = self.ivars();
        if let Some(root) = ivars.window.get().and_then(|window| window.contentView()) {
            root.layoutSubtreeIfNeeded();
        }
        for mode in Mode::ALL {
            if let Some(pane) = self.pane(mode) {
                if mode == active_mode
                    && let Some(container) = ivars.container.get()
                    && unsafe { pane.web.superview() }
                        .is_some_and(|parent| std::ptr::eq(&*parent, &**container))
                {
                    // Hidden provider panes can retain their initial zero-size viewport.
                    // The fullscreen stage owns its own geometry while the pane is detached.
                    let bounds = container.bounds();
                    if pane.web.frame() != bounds {
                        pane.web.setFrame(bounds);
                    }
                }
                pane.web
                    .setHidden(mode != active_mode || !pane.loaded.get());
            }
        }
        let (Some(pane), Some(placeholder)) = (self.active(), self.ivars().placeholder.get())
        else {
            return;
        };
        if pane.loaded.get() {
            placeholder.hide();
        } else if pane.failed.get() && pane.stopped.get() {
            placeholder.show_failure(
                "exclamationmark.triangle",
                "A Problem Repeatedly Occurred",
                &format!(
                    "{} stopped working more than once. Try again to reload it.",
                    active_mode.label()
                ),
            );
        } else if pane.failed.get() {
            placeholder.show_failure(
                "wifi.exclamationmark",
                &format!("Can't Reach {}", active_mode.label()),
                "Check your internet connection, then try again.",
            );
        } else {
            placeholder.show_loading(&format!("Loading {}", active_mode.label()));
        }
    }

    /// Pauses the other section when one starts making sound, so only one plays at a time.
    pub(crate) fn on_audible_playback(&self, web: &WKWebView, title: String) {
        let Some(mode) = self.mode_of(web) else {
            return;
        };
        if let Some(pane) = self.pane(mode) {
            *pane.now_playing.borrow_mut() = Some(title).filter(|title| !title.trim().is_empty());
        }
        for other in Mode::ALL.into_iter().filter(|other| *other != mode) {
            if let Some(pane) = self.pane(other) {
                unsafe { pane.web.pauseAllMediaPlaybackWithCompletionHandler(None) };
            }
        }
        if mode == self.ivars().mode.get() {
            self.update_window_title();
        }
    }

    /// Leaves Picture in Picture wherever it is active, otherwise floats the active section's video.
    pub(crate) fn toggle_picture_in_picture(&self) {
        let pane = Mode::ALL
            .into_iter()
            .filter_map(|mode| self.pane(mode))
            .find(|pane| pane.pip_active.get())
            .or_else(|| self.active());
        let Some(pane) = pane else {
            return;
        };
        let this = self.retain();
        let block = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
            if let Some(error) = unsafe { error.as_ref() } {
                this.show_notice(&format!(
                    "Picture in Picture didn't start: {}",
                    error.localizedDescription()
                ));
                return;
            }
            let status = unsafe { result.as_ref() }
                .and_then(|result| result.downcast_ref::<NSString>())
                .map(|status| status.to_string());
            match status.as_deref() {
                Some("none") => this.show_notice("Open a video to use Picture in Picture."),
                Some("notstarted") => {
                    this.show_notice("Start the video, then use Picture in Picture.")
                }
                Some("unsupported") => {
                    this.show_notice("This video can't play in Picture in Picture.")
                }
                // "enter"/"exit": the outcome arrives as a `pip` or `pipfailed` page message.
                _ => {}
            }
        });
        unsafe {
            pane.web
                .evaluateJavaScript_completionHandler(&ns(&web::toggle_pip_script()), Some(&block));
        }
    }

    /// Changes the active section's video speed and reports the new rate.
    pub(crate) fn change_speed(&self, delta: f64) {
        let Some(pane) = self.active() else {
            return;
        };
        let this = self.retain();
        let block = RcBlock::new(move |result: *mut AnyObject, _error: *mut NSError| {
            let rate = unsafe { result.as_ref() }
                .and_then(|result| result.downcast_ref::<NSNumber>())
                .map_or(0., |rate| rate.doubleValue());
            if rate > 0. {
                this.show_notice(&format!("Speed {rate}×"));
            } else {
                this.show_notice("Open a video to change its speed.");
            }
        });
        unsafe {
            pane.web.evaluateJavaScript_completionHandler(
                &ns(&web::change_speed_script(delta)),
                Some(&block),
            );
        }
    }
}
