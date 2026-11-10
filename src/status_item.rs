//! Menu-bar access to YouTube Music's existing player controls.

use super::*;
use objc2_app_kit::{NSAccessibility, NSStatusBar, NSVariableStatusItemLength};
use objc2_foundation::NSUserDefaults;

pub(crate) fn music_menu_bar_enabled() -> bool {
    let defaults = NSUserDefaults::standardUserDefaults();
    let key = ns_string!("ShowMusicInMenuBar");
    defaults.objectForKey(key).is_none() || defaults.boolForKey(key)
}

impl AppDelegate {
    pub(crate) fn toggle_music_menu_bar(&self) {
        NSUserDefaults::standardUserDefaults()
            .setBool_forKey(!music_menu_bar_enabled(), ns_string!("ShowMusicInMenuBar"));
        self.update_music_status_item();
    }

    pub(crate) fn update_music_status_item(&self) {
        let ivars = self.ivars();
        let track = ivars.track.borrow();
        let mut status = ivars.music_status_item.borrow_mut();
        if !music_menu_bar_enabled() || track.title.is_empty() {
            if let Some(item) = status.take() {
                NSStatusBar::systemStatusBar().removeStatusItem(&item);
            }
            return;
        }

        let mtm = self.mtm();
        let item = status.get_or_insert_with(|| {
            let item =
                NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
            if let Some(button) = item.button(mtm) {
                let symbol = ui::symbol("music.note", "YouTube Music");
                if let Some(image) = symbol.as_deref() {
                    image.setTemplate(true);
                }
                button.setImage(symbol.as_deref());
                button.setAccessibilityLabel(Some(ns_string!("YouTube Music")));
            }
            item
        });
        let heading = if track.artist.is_empty() {
            track.title.clone()
        } else {
            format!("{} — {}", track.title, track.artist)
        };
        if let Some(button) = item.button(mtm) {
            button.setToolTip(Some(&ns(&heading)));
        }
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("YouTube Music"));
        menu.setAutoenablesItems(false);
        let header = ui::menu_item(mtm, &heading, None, "", None);
        header.setEnabled(false);
        if let Some((_, image)) = ivars.artwork.borrow().as_ref() {
            header.setImage(Some(&ui::rounded_image(image, 32., 0.)));
        }
        menu.addItem(&header);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
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
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&ui::menu_item(
            mtm,
            "Show YouTube Music",
            Some(sel!(showMusicFromMenuBar:)),
            "",
            Some(self),
        ));
        item.setMenu(Some(&menu));
    }

    pub(crate) fn show_music_from_menu_bar(&self) {
        let app = NSApplication::sharedApplication(self.mtm());
        app.unhide(None);
        if let Some(window) = self.ivars().window.get() {
            window.deminiaturize(None);
        }
        self.on_window_reopen();
        self.switch_mode(Mode::Music);
        app.activate();
    }
}
