//! Account.

use super::*;

impl AppDelegate {
    /// Reads the shared cookie store: YouTube sets `LOGIN_INFO`/`SAPISID` for a signed-in account.
    pub(crate) fn refresh_sign_in_state(&self) {
        let this = self.retain();
        let block = RcBlock::new(move |cookies: NonNull<NSArray<NSHTTPCookie>>| {
            let cookies = unsafe { cookies.as_ref() };
            let signed_in = cookies.iter().any(|cookie| {
                let name = cookie.name().to_string();
                let domain = cookie.domain().to_string();
                matches!(name.as_str(), "LOGIN_INFO" | "SAPISID")
                    && domain.trim_start_matches('.').ends_with("youtube.com")
            });
            this.set_signed_in(signed_in);
        });
        let store = unsafe { WKWebsiteDataStore::defaultDataStore(self.mtm()).httpCookieStore() };
        unsafe { store.getAllCookies(&block) };
    }

    pub(crate) fn set_signed_in(&self, signed_in: bool) {
        let previous = self.ivars().signed_in.replace(Some(signed_in));
        if previous != Some(signed_in) {
            self.clear_account_identity();
            self.update_account_ui();
        }
        if signed_in {
            self.maybe_fetch_account_identity();
        }
        if previous == Some(signed_in) || previous.is_none() {
            return;
        }
        self.show_notice(if signed_in {
            "Signed in to YouTube."
        } else {
            "Signed out of YouTube."
        });
        // The other section still shows the previous account; refresh it.
        let active = self.ivars().mode.get();
        for mode in Mode::ALL.into_iter().filter(|mode| *mode != active) {
            if let Some(pane) = self.pane(mode)
                && pane.loaded.get()
            {
                unsafe { pane.web.reload() };
            }
        }
    }

    /// Shows a downloaded account photo if it still belongs to the active account.
    pub(crate) fn on_account_photo(&self, url: &str, bytes: Vec<u8>) {
        let current = self
            .ivars()
            .account_identity
            .borrow()
            .as_ref()
            .and_then(|identity| identity.photo.clone());
        if current.as_deref() != Some(url) {
            return;
        }
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&bytes))
        else {
            return;
        };
        *self.ivars().account_photo.borrow_mut() = Some(image);
        self.update_account_ui();
    }

    pub(crate) fn clear_account_identity(&self) {
        *self.ivars().account_identity.borrow_mut() = None;
        *self.ivars().account_photo.borrow_mut() = None;
        self.ivars().account_lookup_pending.set(false);
        self.ivars().account_lookup_attempted.set(false);
        self.update_account_ui();
    }

    pub(crate) fn maybe_fetch_account_identity(&self) {
        if let Some(pane) = self.pane(Mode::YouTube) {
            self.maybe_fetch_account_identity_in(&pane.web);
        }
    }

    pub(crate) fn maybe_fetch_account_identity_in(&self, web: &WKWebView) {
        let ivars = self.ivars();
        if ivars.signed_in.get() != Some(true)
            || ivars.account_identity.borrow().is_some()
            || ivars.account_lookup_pending.get()
            || ivars.account_lookup_attempted.get()
        {
            return;
        }
        let Some(pane) = self.pane_of(web) else {
            return;
        };
        let url = unsafe { web.URL() }
            .and_then(|url| url.absoluteString())
            .and_then(|text| Url::parse(&text.to_string()).ok());
        if !pane.loaded.get()
            || !url.is_some_and(|url| {
                url.scheme() == "https" && url.host_str() == Some("www.youtube.com")
            })
        {
            return;
        }
        ivars.account_lookup_pending.set(true);
        ivars.account_lookup_attempted.set(true);
        unsafe {
            web.evaluateJavaScript_completionHandler(&ns(&web::account_identity_script()), None);
        }
    }

    pub(crate) fn update_account_ui(&self) {
        let mtm = self.mtm();
        let Some(bar) = self.ivars().bar.get() else {
            return;
        };
        let signed_in = self.ivars().signed_in.get() == Some(true);
        let identity = self.ivars().account_identity.borrow();
        let display_identity = identity.as_ref().filter(|_| signed_in);
        let status = if let Some(identity) = display_identity {
            identity
                .email
                .as_deref()
                .or(identity.handle.as_deref())
                .map_or_else(
                    || format!("Signed in as {}", identity.name),
                    |contact| format!("Signed in as {} ({contact})", identity.name),
                )
        } else if signed_in {
            "Signed in to YouTube".to_owned()
        } else {
            "Not signed in".to_owned()
        };
        let name = display_identity.map_or(
            if signed_in {
                "Signed in to YouTube"
            } else {
                "Not signed in"
            },
            |identity| identity.name.as_str(),
        );
        let photo = self
            .ivars()
            .account_photo
            .borrow()
            .clone()
            .filter(|_| display_identity.is_some());
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Account"));
        // One header like Google's account chip: photo, name, and email (or handle) beneath.
        let header = ui::menu_item(mtm, name, None, "", None);
        header.setEnabled(false);
        let detail = display_identity
            .and_then(|identity| identity.email.as_deref().or(identity.handle.as_deref()));
        // `subtitle` is macOS 14.4+; earlier systems show the name alone (detail stays in the tooltip).
        if let Some(detail) = detail
            && header.respondsToSelector(sel!(setSubtitle:))
        {
            header.setSubtitle(Some(&ns(detail)));
        }
        if let Some(photo) = photo.as_deref() {
            header.setImage(Some(&ui::circular_image(photo, 32.)));
        }
        menu.addItem(&header);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        if signed_in {
            menu.addItem(&ui::menu_item(
                mtm,
                "Switch Account…",
                Some(sel!(signIn:)),
                "",
                Some(self),
            ));
            menu.addItem(&ui::menu_item(
                mtm,
                "Sign Out",
                Some(sel!(signOut:)),
                "",
                Some(self),
            ));
        } else {
            menu.addItem(&ui::menu_item(
                mtm,
                "Sign In…",
                Some(sel!(signIn:)),
                "",
                Some(self),
            ));
        }
        bar.set_account(&status, photo.as_deref());
        *self.ivars().account_menu.borrow_mut() = Some(menu);
    }
}
