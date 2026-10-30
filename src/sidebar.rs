//! Sidebar.

use super::*;

impl AppDelegate {
    pub(crate) fn with_state<T>(&self, read: impl FnOnce(&AppState) -> T) -> Option<T> {
        self.ivars().queue.borrow().as_ref().ok().map(read)
    }

    pub(crate) fn row_at(&self, row: NSInteger) -> Option<Row> {
        let index = usize::try_from(row).ok()?;
        let count = self.with_state(|state| state.up_next.len()).unwrap_or(0);
        if count == 0 && index == 0 {
            Some(Row::EmptyHint)
        } else if index < count {
            Some(Row::Saved(index))
        } else {
            None
        }
    }

    pub(crate) fn sidebar_view(&self, row: NSInteger) -> Option<Retained<NSView>> {
        let mtm = self.mtm();
        Some(match self.row_at(row)? {
            Row::Saved(index) => {
                let (label, music, thumbnail) = self.with_state(|state| {
                    state.up_next.get(index).map(|item| {
                        let music = Url::parse(&item.url).is_ok_and(|url| is_music_url(&url));
                        (item.label(), music, item.thumbnail_url())
                    })
                })??;
                let symbol = if music {
                    "music.note"
                } else {
                    "play.rectangle"
                };
                let image = thumbnail.and_then(|url| self.thumbnail(url));
                ui::saved_cell(mtm, &label, image.as_deref(), symbol)
            }
            Row::EmptyHint => ui::hint_cell(
                mtm,
                if self.with_state(|_| ()).is_some() {
                    "Save videos with ⌘S"
                } else {
                    "Saved list can't be read"
                },
            ),
        })
    }

    /// What dragging Up Next `row` carries: its index for reordering here, and its video link
    /// for drops outside the sidebar.
    pub(crate) fn saved_row_pasteboard_item(
        &self,
        row: NSInteger,
    ) -> Option<Retained<NSPasteboardItem>> {
        let Some(Row::Saved(index)) = self.row_at(row) else {
            return None;
        };
        let url =
            self.with_state(|state| state.up_next.get(index).map(|item| item.url.clone()))??;
        let item = NSPasteboardItem::new();
        item.setString_forType(&ns(&index.to_string()), &ns(UP_NEXT_DRAG_TYPE));
        item.setString_forType(&ns(&url), unsafe { NSPasteboardTypeURL });
        Some(item)
    }

    /// The downloaded thumbnail at `url`, starting its download the first time it is asked for.
    pub(crate) fn thumbnail(&self, url: String) -> Option<Retained<NSImage>> {
        let mut thumbnails = self.ivars().thumbnails.borrow_mut();
        if let Some(image) = thumbnails.get(&url) {
            return image.clone();
        }
        thumbnails.insert(url.clone(), None);
        load_image(url, AppDelegate::on_thumbnail);
        None
    }

    /// Keeps a downloaded thumbnail and redraws the Up Next rows that show it.
    pub(crate) fn on_thumbnail(&self, url: &str, bytes: Vec<u8>) {
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&bytes))
        else {
            return;
        };
        self.ivars()
            .thumbnails
            .borrow_mut()
            .insert(url.to_owned(), Some(ui::thumbnail_image(&image)));
        let Some(table) = self.ivars().sidebar.get() else {
            return;
        };
        let rows = NSMutableIndexSet::new();
        let saved = self
            .with_state(|state| {
                state
                    .up_next
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.thumbnail_url().as_deref() == Some(url))
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for index in saved {
            let row = self.row_index(Row::Saved(index));
            if row >= 0 {
                rows.addIndex(row as usize);
            }
        }
        if rows.count() > 0 {
            self.ivars().syncing_selection.set(true);
            table.reloadDataForRowIndexes_columnIndexes(&rows, &NSIndexSet::indexSetWithIndex(0));
            self.ivars().syncing_selection.set(false);
        }
    }

    /// For a drag of one of this sidebar's Up Next rows to the gap above `row`: the saved item's
    /// index and where it moves to. `None` for other drags, gaps outside Up Next, and drops that
    /// would leave the item where it is.
    pub(crate) fn drop_target(
        &self,
        table: &NSTableView,
        info: &ProtocolObject<dyn NSDraggingInfo>,
        row: NSInteger,
    ) -> Option<(usize, usize)> {
        let source = info.draggingSource()?;
        if !std::ptr::eq(&*source, AsRef::<AnyObject>::as_ref(table)) {
            return None;
        }
        let from: usize = info
            .draggingPasteboard()
            .stringForType(&ns(UP_NEXT_DRAG_TYPE))?
            .to_string()
            .parse()
            .ok()?;
        let first = usize::try_from(self.row_index(Row::Saved(0))).ok()?;
        let gap = usize::try_from(row).ok()?.checked_sub(first)?;
        let to = self.with_state(|state| state.drop_destination(from, gap))??;
        Some((from, to))
    }

    pub(crate) fn row_index(&self, target: Row) -> NSInteger {
        match target {
            Row::Saved(index)
                if self
                    .with_state(|state| index < state.up_next.len())
                    .unwrap_or(false) =>
            {
                index as NSInteger
            }
            Row::EmptyHint
                if self
                    .with_state(|state| state.up_next.is_empty())
                    .unwrap_or(true) =>
            {
                0
            }
            _ => -1,
        }
    }

    pub(crate) fn open_row(&self, row: NSInteger) {
        let url = match self.row_at(row) {
            Some(Row::Saved(index)) => {
                match self
                    .with_state(|state| state.up_next.get(index).map(|item| item.resume_url()))
                {
                    Some(Some(url)) => url,
                    _ => return,
                }
            }
            _ => return,
        };
        self.load_url(&url);
        self.set_queue_visible(false);
    }

    pub(crate) fn reload_sidebar(&self) {
        if let Some(table) = self.ivars().sidebar.get() {
            self.ivars().syncing_selection.set(true);
            table.reloadData();
            self.ivars().syncing_selection.set(false);
        }
        self.sync_sidebar_selection();
        self.update_toolbar();
        if let Some(bar) = self.ivars().bar.get() {
            bar.set_queue_count(self.with_state(|state| state.up_next.len()).unwrap_or(0));
        }
    }

    /// The current section's destination for a page, if any.
    pub(crate) fn destination_for(&self, url: &Url) -> Option<usize> {
        let mode = self.ivars().mode.get();
        if Mode::for_url(url) != mode {
            return None;
        }
        mode.destinations()
            .iter()
            .position(|destination| match destination.path {
                "/" => url.path() == "/",
                path => url.path().starts_with(path),
            })
    }

    /// Highlights the sidebar row that matches the page the active section is showing.
    pub(crate) fn sync_sidebar_selection(&self) {
        let Some(table) = self.ivars().sidebar.get() else {
            return;
        };
        let target = self.current_url().and_then(|url| {
            if video_id(&url).is_some()
                && let Some(index) = self.with_state(|state| state.position(&url)).flatten()
            {
                return Some(Row::Saved(index));
            }
            None
        });
        self.ivars().syncing_selection.set(true);
        match target
            .map(|row| self.row_index(row))
            .filter(|row| *row >= 0)
        {
            Some(row) => {
                table.selectRowIndexes_byExtendingSelection(
                    &NSIndexSet::indexSetWithIndex(row as usize),
                    false,
                );
                table.scrollRowToVisible(row);
            }
            None => unsafe { table.deselectAll(None) },
        }
        self.ivars().syncing_selection.set(false);
    }

    /// Saved row targeted by an action: the clicked row for the context menu,
    /// otherwise the selected row while the sidebar has keyboard focus.
    pub(crate) fn target_saved_index(&self, sender: Option<&AnyObject>) -> Option<usize> {
        let table = self.ivars().sidebar.get()?;
        let from_context = sender
            .and_then(|sender| sender.downcast_ref::<NSMenuItem>())
            .is_some_and(|item| item.tag() == CONTEXT_TAG);
        let row = if from_context {
            table.clickedRow()
        } else {
            let window = self.ivars().window.get()?;
            let focused = window
                .firstResponder()
                .is_some_and(|responder| responder.isEqual(Some(table.as_ref())));
            if !focused {
                return None;
            }
            table.selectedRow()
        };
        match self.row_at(row)? {
            Row::Saved(index) => Some(index),
            _ => None,
        }
    }

    pub(crate) fn validate(&self, item: &NSMenuItem) -> bool {
        let Some(action) = item.action() else {
            return true;
        };
        let web = self.active().map(|pane| &pane.web);
        let sender: Option<&AnyObject> = Some(item.as_ref());
        let count = self.with_state(|state| state.up_next.len()).unwrap_or(0);
        let mode = self.ivars().mode.get();
        let check = |on: bool| {
            item.setState(if on {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            true
        };
        if action == sel!(showYouTube:) {
            check(mode == Mode::YouTube)
        } else if action == sel!(toggleQueue:) {
            check(
                self.ivars()
                    .queue_panel
                    .get()
                    .is_some_and(|panel| !panel.isHidden()),
            )
        } else if action == sel!(showMusic:) {
            check(mode == Mode::Music)
        } else if action == sel!(togglePictureInPicture:) {
            let active = Mode::ALL
                .into_iter()
                .any(|mode| self.pane(mode).is_some_and(|pane| pane.pip_active.get()));
            check(active)
        } else if action == sel!(signOut:) {
            self.ivars().signed_in.get() == Some(true)
        } else if action == sel!(goBack:) {
            web.is_some_and(|web| unsafe { web.canGoBack() })
        } else if action == sel!(goForward:) {
            web.is_some_and(|web| unsafe { web.canGoForward() })
        } else if action == sel!(musicPlayPause:) {
            let track = self.ivars().track.borrow();
            item.setTitle(&ns(if track.playing { "Pause" } else { "Play" }));
            !track.title.is_empty()
        } else if action == sel!(musicNext:) || action == sel!(musicPrevious:) {
            // While text is being edited, Command-Left/Right keep moving the insertion point:
            // a disabled menu item lets the key equivalent through to the field editor.
            let editing = self
                .ivars()
                .window
                .get()
                .and_then(|window| window.firstResponder())
                .is_some_and(|responder| responder.isKindOfClass(NSText::class()));
            !editing && !self.ivars().track.borrow().title.is_empty()
        } else if action == sel!(saveCurrentVideo:) {
            self.save_state_for_current() == Some(false)
        } else if action == sel!(openSaved:) || action == sel!(removeSaved:) {
            self.target_saved_index(sender).is_some()
        } else if action == sel!(moveSavedUp:) {
            self.target_saved_index(sender)
                .is_some_and(|index| index > 0)
        } else if action == sel!(moveSavedDown:) {
            self.target_saved_index(sender)
                .is_some_and(|index| index + 1 < count)
        } else {
            true
        }
    }

    pub(crate) fn move_saved(&self, sender: Option<&AnyObject>, delta: isize) {
        let Some(index) = self.target_saved_index(sender) else {
            return;
        };
        let Some(target) = index.checked_add_signed(delta) else {
            return;
        };
        self.change_queue(|state| state.move_item(index, target));
    }
}
