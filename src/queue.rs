//! Queue.

use super::*;
use objc2_foundation::NSUserDefaults;

pub(crate) fn auto_next_enabled() -> bool {
    NSUserDefaults::standardUserDefaults().boolForKey(ns_string!("PlayUpNextAutomatically"))
}

impl AppDelegate {
    pub(crate) fn toggle_auto_next(&self) {
        let enabled = !auto_next_enabled();
        NSUserDefaults::standardUserDefaults()
            .setBool_forKey(enabled, ns_string!("PlayUpNextAutomatically"));
        if !enabled {
            self.cancel_auto_next();
        }
    }

    pub(crate) fn on_video_ended(&self, text: &str) {
        let ivars = self.ivars();
        if !auto_next_enabled()
            || ivars.mode.get() != Mode::YouTube
            || ivars.app_hidden.get()
            || ivars.window_minimized.get()
            || ivars.window_closed.get()
            || ivars.fullscreen_mode.get().is_some()
            || ivars.auto_next_target.borrow().is_some()
        {
            return;
        }
        let Ok(url) = Url::parse(text) else { return };
        if url.path() != "/watch"
            || video_id(&url).is_none()
            || self.current_url().and_then(|current| video_id(&current)) != video_id(&url)
        {
            return;
        }
        let Some((target, title)) = self
            .with_state(|state| {
                state
                    .next_after(&url)
                    .map(|item| (item.url.clone(), item.label()))
            })
            .flatten()
        else {
            return;
        };
        *ivars.auto_next_target.borrow_mut() = Some((target, title));
        ivars.auto_next_seconds.set(5);
        self.update_auto_next_notice();
        self.schedule_auto_next_tick();
    }

    fn update_auto_next_notice(&self) {
        if let (Some(notice), Some((_, title))) = (
            self.ivars().auto_next_notice.get(),
            self.ivars().auto_next_target.borrow().as_ref(),
        ) {
            notice.show(title, self.ivars().auto_next_seconds.get());
        }
    }

    fn schedule_auto_next_tick(&self) {
        unsafe {
            let _: () = msg_send![self, performSelector: sel!(tickAutoNext:),
                withObject: Option::<&AnyObject>::None, afterDelay: 1.0f64];
        }
    }

    pub(crate) fn tick_auto_next(&self) {
        if self.ivars().auto_next_target.borrow().is_none() {
            return;
        }
        let seconds = self.ivars().auto_next_seconds.get().saturating_sub(1);
        self.ivars().auto_next_seconds.set(seconds);
        if seconds == 0 {
            self.play_auto_next();
        } else {
            self.update_auto_next_notice();
            self.schedule_auto_next_tick();
        }
    }

    pub(crate) fn cancel_auto_next(&self) {
        unsafe {
            let _: () = msg_send![<NSObject as ClassType>::class(),
                cancelPreviousPerformRequestsWithTarget: self,
                selector: sel!(tickAutoNext:), object: Option::<&AnyObject>::None];
            let _: () = msg_send![<NSObject as ClassType>::class(),
                cancelPreviousPerformRequestsWithTarget: self,
                selector: sel!(checkAutoNextFallback:), object: Option::<&AnyObject>::None];
        }
        self.ivars().auto_next_target.borrow_mut().take();
        self.ivars().auto_next_fallback.borrow_mut().take();
        let generation = &self.ivars().auto_next_generation;
        generation.set(generation.get().wrapping_add(1));
        if let Some(notice) = self.ivars().auto_next_notice.get() {
            notice.root.setHidden(true);
        }
    }

    pub(crate) fn check_auto_next_fallback(&self) {
        unsafe {
            let _: () = msg_send![<NSObject as ClassType>::class(),
                cancelPreviousPerformRequestsWithTarget: self,
                selector: sel!(checkAutoNextFallback:), object: Option::<&AnyObject>::None];
        }
        let Some((source, target)) = self.ivars().auto_next_fallback.borrow_mut().take() else {
            return;
        };
        let current = self.current_url();
        let target_id = Url::parse(&target).ok().and_then(|url| video_id(&url));
        if current.as_ref() == Some(&source) && current.as_ref().and_then(video_id) != target_id {
            self.load_url(&target);
        }
    }

    pub(crate) fn play_auto_next(&self) {
        let target = self
            .ivars()
            .auto_next_target
            .borrow()
            .as_ref()
            .map(|(url, _)| url.clone());
        self.cancel_auto_next();
        let Some(index) = target
            .and_then(|url| Url::parse(&url).ok())
            .and_then(|url| self.with_state(|state| state.position(&url)).flatten())
        else {
            return;
        };
        let Some(url) = self
            .with_state(|state| state.up_next.get(index).map(|item| item.resume_url()))
            .flatten()
        else {
            return;
        };
        let Ok(parsed) = Url::parse(&url) else {
            return;
        };
        let Some(id) = video_id(&parsed).filter(|_| {
            !is_music_url(&parsed)
                && (parsed.path() == "/watch" || parsed.host_str() == Some("youtu.be"))
        }) else {
            self.open_row(self.row_index(Row::Saved(index)));
            return;
        };
        let Some(pane) = self.pane(Mode::YouTube) else {
            return;
        };
        let url_json = serde_json::to_string(&url).expect("URL string");
        self.set_queue_visible(false);
        *pane.last_requested.borrow_mut() = Some(url.clone());
        let id_json = serde_json::to_string(&id).expect("video ID string");
        // Navigate in the current YouTube document so its gesture-approved media element can
        // continue playing. A full WKWebView load would require another user gesture.
        let script = format!(
            r#"(() => {{
  const app = document.querySelector("ytd-app");
  if (!app) return false;
  const u = new URL({url_json});
  const id = {id_json};
  u.searchParams.set("v", id);
  const endpoint = {{ videoId: id }};
  const time = u.searchParams.get("t") || u.searchParams.get("start");
  if (time) {{
    const parts = time.match(/^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s?)?$/);
    if (parts) endpoint.startTimeSeconds = (+parts[1] || 0) * 3600 + (+parts[2] || 0) * 60 + (+parts[3] || 0);
  }}
  const list = u.searchParams.get("list");
  if (list) endpoint.playlistId = list;
  const index = u.searchParams.get("index");
  if (index && /^\d+$/.test(index)) endpoint.index = +index;
  app.dispatchEvent(new CustomEvent("yt-navigate", {{ bubbles: true, composed: true, detail: {{
    endpoint: {{ commandMetadata: {{ webCommandMetadata: {{
      url: "/watch?" + u.searchParams, webPageType: "WEB_PAGE_TYPE_WATCH", rootVe: 3832
    }} }}, watchEndpoint: endpoint }}
  }} }}));
  return true;
}})()"#
        );
        let Some(source) = self.current_url() else {
            return;
        };
        *self.ivars().auto_next_fallback.borrow_mut() = Some((source, url));
        unsafe {
            let _: () = msg_send![self, performSelector: sel!(checkAutoNextFallback:),
                withObject: Option::<&AnyObject>::None, afterDelay: 3.0f64];
        }
        let generation = self.ivars().auto_next_generation.get();
        let this = self.retain();
        let block = RcBlock::new(move |result: *mut AnyObject, _error: *mut NSError| {
            let navigated = unsafe { result.as_ref() }
                .and_then(|value| value.downcast_ref::<NSNumber>())
                .is_some_and(|value| value.doubleValue() != 0.);
            if !navigated && this.ivars().auto_next_generation.get() == generation {
                this.check_auto_next_fallback();
            }
        });
        unsafe {
            pane.web
                .evaluateJavaScript_completionHandler(&ns(&script), Some(&block));
        }
    }

    /// Applies a queue edit, persisting before committing it in memory.
    pub(crate) fn change_queue(&self, update: impl FnOnce(&mut AppState) -> bool) -> bool {
        let mut stored = self.ivars().queue.borrow_mut();
        let Ok(state) = stored.as_mut() else {
            drop(stored);
            self.show_notice("Up Next can't be changed because its saved data couldn't be read.");
            return false;
        };
        let mut candidate = state.clone();
        if !update(&mut candidate) {
            return false;
        }
        if let Err(error) = save_state(&candidate) {
            drop(stored);
            self.show_sticky_notice(&format!("Up Next couldn't be saved: {error}"));
            return false;
        }
        *state = candidate;
        drop(stored);
        self.cancel_auto_next();
        self.reload_sidebar();
        true
    }

    /// Remembers where a saved video was left. Saved quietly: no sidebar reload (it would
    /// interrupt a drag) and no notice for a failed write, since this repeats while a video plays.
    pub(crate) fn on_video_progress(&self, url: &str, seconds: f64, duration: f64) {
        let Ok(Navigation::Video(url)) = parse_input(url) else {
            return;
        };
        let position = resume_point(seconds, duration);
        let mut stored = self.ivars().queue.borrow_mut();
        let Ok(state) = stored.as_mut() else {
            return;
        };
        let mut candidate = state.clone();
        if candidate.set_position(&url, position) && save_state(&candidate).is_ok() {
            *state = candidate;
        }
    }

    /// `Some(saved?)` when the page is a video; `None` otherwise or when the queue is unreadable.
    pub(crate) fn save_state_for_current(&self) -> Option<bool> {
        let url = self.current_video_url()?;
        self.with_state(|state| state.position(&url).is_some())
    }

    pub(crate) fn save_current_video(&self) {
        let Some(url) = self.current_video_url() else {
            return;
        };
        let title = self.current_title_for(&url);
        self.change_queue(|state| state.add(&url, title));
    }

    /// Remembers the video link a page's context menu is opening over, or forgets the last one.
    pub(crate) fn on_context_link(&self, href: &str, title: String) {
        let link = match parse_input(href) {
            Ok(Navigation::Video(url)) => {
                Some((url, Some(title).filter(|title| !title.is_empty())))
            }
            _ => None,
        };
        *self.ivars().context_link.borrow_mut() = link;
    }

    /// Puts the Up Next commands at the top of a page's context menu when it is over a video link.
    pub(crate) fn extend_page_menu(&self, menu: &NSMenu) {
        let Some(url) = self
            .ivars()
            .context_link
            .borrow()
            .as_ref()
            .map(|(url, _)| url.clone())
        else {
            return;
        };
        let mtm = self.mtm();
        let saved = self
            .with_state(|state| state.position(&url).is_some())
            .unwrap_or(false);
        let add = ui::menu_item(
            mtm,
            if saved {
                "Already in Up Next"
            } else {
                "Add to Up Next"
            },
            (!saved).then_some(sel!(addLinkToUpNext:)),
            "",
            Some(self),
        );
        let next = ui::menu_item(mtm, "Play Next", Some(sel!(playLinkNext:)), "", Some(self));
        menu.insertItem_atIndex(&add, 0);
        menu.insertItem_atIndex(&next, 1);
        menu.insertItem_atIndex(&NSMenuItem::separatorItem(mtm), 2);
    }

    /// Saves the right-clicked video at the end of Up Next, or (`next`) right after the open
    /// video, or first when the open page isn't a saved video.
    pub(crate) fn add_context_link(&self, next: bool) {
        let Some((url, title)) = self.ivars().context_link.borrow().clone() else {
            return;
        };
        let current = self.current_video_url();
        let changed = self.change_queue(|state| {
            if next {
                state.play_next(&url, title, current.as_ref())
            } else {
                state.add(&url, title)
            }
        });
        if changed {
            self.show_notice(if next {
                "Playing next in Up Next"
            } else {
                "Added to Up Next"
            });
        }
    }

    /// Display title of the open video, only once the page has caught up with it.
    pub(crate) fn current_title_for(&self, url: &Url) -> Option<String> {
        let pane = self.active()?;
        if is_music_url(url) {
            // YouTube Music keeps a fixed document title; it publishes the track via Media Session.
            return pane.now_playing.borrow().clone();
        }
        if *pane.title_video.borrow() != video_id(url) {
            return None;
        }
        clean_page_title(&unsafe { pane.web.title() }?.to_string())
    }
}
