//! Queue.

use super::*;

impl AppDelegate {
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
