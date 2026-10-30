//! Window construction.

use super::*;

impl AppDelegate {
    pub(crate) fn build_window(&self) {
        let mtm = self.mtm();
        let window: Retained<QuietWindow> = unsafe {
            msg_send![
                QuietWindow::alloc(mtm),
                initWithContentRect: NSRect::new(NSPoint::new(0., 0.), NSSize::new(1180., 760.)),
                styleMask: NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable
                    | NSWindowStyleMask::FullSizeContentView,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ]
        };
        let window = window.into_super();
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(ns_string!("DooTube"));
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        window.setTitlebarAppearsTransparent(true);
        window.setBackgroundColor(Some(&cinema_bar::background_color()));
        window.setAppearance(
            NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }).as_deref(),
        );
        // An empty toolbar only sizes the titlebar row the Cinema bar sits in.
        let toolbar = NSToolbar::new(mtm);
        window.setToolbar(Some(&toolbar));
        window.setToolbarStyle(NSWindowToolbarStyle::Unified);
        self.build_menus();

        let content = NSView::new(mtm);
        let panes = self.build_panes();
        let placeholder = ui::Placeholder::new(mtm);
        unsafe {
            placeholder.retry.setTarget(Some(self));
            placeholder.retry.setAction(Some(sel!(reloadPage:)));
        }
        let notice = ui::Notice::new(mtm);
        unsafe {
            notice.close.setTarget(Some(self));
            notice.close.setAction(Some(sel!(hideNotice:)));
        }
        let backdrop = NSBox::new(mtm);
        backdrop.setBoxType(NSBoxType::Custom);
        backdrop.setTitlePosition(NSTitlePosition::NoTitle);
        backdrop.setBorderWidth(0.);
        backdrop.setFrame(content.bounds());
        backdrop.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        content.addSubview(&backdrop);
        let container = NSView::new(mtm);
        container.setTranslatesAutoresizingMaskIntoConstraints(false);
        let bar = cinema_bar::CinemaBar::new(mtm);
        for (button, action) in [
            (&bar.back, sel!(goBack:)),
            (&bar.forward, sel!(goForward:)),
            (&bar.home, sel!(goHome:)),
            (&bar.youtube, sel!(showYouTube:)),
            (&bar.music, sel!(showMusic:)),
            (&bar.save, sel!(saveCurrentVideo:)),
            (&bar.pip, sel!(togglePictureInPicture:)),
            (&bar.queue, sel!(toggleQueue:)),
            (&bar.account, sel!(showAccountMenu:)),
        ] {
            unsafe {
                button.setTarget(Some(self));
                button.setAction(Some(action));
            }
        }
        bar.search.setSendsWholeSearchString(true);
        unsafe {
            bar.search.setDelegate(Some(ProtocolObject::from_ref(self)));
            bar.search.setTarget(Some(self));
            bar.search.setAction(Some(sel!(searchSubmitted:)));
        }
        content.addSubview(&bar.root);
        let safe = content.safeAreaLayoutGuide();
        content.addSubview(&container);
        ui::activate(&[
            // The bar shares the (empty, unified) toolbar row with the traffic lights.
            bar.root
                .topAnchor()
                .constraintEqualToAnchor(&content.topAnchor()),
            bar.root
                .leadingAnchor()
                .constraintEqualToAnchor(&content.leadingAnchor()),
            bar.root
                .trailingAnchor()
                .constraintEqualToAnchor(&content.trailingAnchor()),
            container
                .topAnchor()
                .constraintEqualToAnchor(&bar.root.bottomAnchor()),
            container
                .leadingAnchor()
                .constraintEqualToAnchor(&content.leadingAnchor()),
            container
                .trailingAnchor()
                .constraintEqualToAnchor(&content.trailingAnchor()),
            container
                .bottomAnchor()
                .constraintEqualToAnchor(&content.bottomAnchor()),
        ]);
        for pane in &panes {
            pane.web.setFrame(container.bounds());
            pane.web.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            pane.web.setTranslatesAutoresizingMaskIntoConstraints(true);
            container.addSubview(&pane.web);
        }
        let suggestions = suggestions::Suggestions::new(mtm, &content, &bar.search);
        let _ = self.ivars().bar.set(bar);
        let _ = self.ivars().container.set(container.clone());
        let _ = self.ivars().backdrop.set(backdrop);
        content.addSubview(&placeholder.root);
        content.addSubview(&notice.root);
        // Above the page and the overlays added before it.
        content.addSubview_positioned_relativeTo(
            &suggestions.root,
            NSWindowOrderingMode::Above,
            None,
        );
        let _ = self.ivars().suggestions.set(suggestions);
        ui::activate(&[
            placeholder
                .root
                .centerXAnchor()
                .constraintEqualToAnchor(&container.centerXAnchor()),
            placeholder
                .root
                .centerYAnchor()
                .constraintEqualToAnchor(&container.centerYAnchor()),
            notice
                .root
                .centerXAnchor()
                .constraintEqualToAnchor(&safe.centerXAnchor()),
            notice
                .root
                .bottomAnchor()
                .constraintEqualToAnchor_constant(&safe.bottomAnchor(), -20.),
            notice
                .root
                .widthAnchor()
                .constraintLessThanOrEqualToAnchor_constant(&safe.widthAnchor(), -40.),
        ]);
        let scrim = NSBox::new(mtm);
        scrim.setBoxType(NSBoxType::Custom);
        scrim.setTitlePosition(NSTitlePosition::NoTitle);
        scrim.setBorderWidth(0.);
        scrim.setFillColor(&NSColor::colorWithWhite_alpha(0., 0.65));
        scrim.setTranslatesAutoresizingMaskIntoConstraints(false);
        let dismiss = NSButton::new(mtm);
        dismiss.setTitle(ns_string!(""));
        dismiss.setBordered(false);
        dismiss.setTranslatesAutoresizingMaskIntoConstraints(false);
        unsafe {
            dismiss.setTarget(Some(self));
            dismiss.setAction(Some(sel!(toggleQueue:)));
        }
        scrim.addSubview(&dismiss);
        content.addSubview(&scrim);

        let drawer = NSBox::new(mtm);
        drawer.setBoxType(NSBoxType::Custom);
        drawer.setTitlePosition(NSTitlePosition::NoTitle);
        drawer.setBorderWidth(0.);
        drawer.setFillColor(&cinema_bar::raised_color());
        drawer.setTranslatesAutoresizingMaskIntoConstraints(false);
        let title = NSTextField::labelWithString(ns_string!("Up Next"), mtm);
        title.setFont(Some(&NSFont::boldSystemFontOfSize(28.)));
        title.setTranslatesAutoresizingMaskIntoConstraints(false);
        let close = NSButton::new(mtm);
        close.setTitle(ns_string!(""));
        close.setImage(ui::symbol("xmark", "Close Up Next").as_deref());
        close.setBordered(false);
        close.setToolTip(Some(ns_string!("Close Up Next")));
        close.setKeyEquivalent(&ns("\u{1b}"));
        close.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
        close.setTranslatesAutoresizingMaskIntoConstraints(false);
        unsafe {
            close.setTarget(Some(self));
            close.setAction(Some(sel!(toggleQueue:)));
        }
        let sidebar = self.build_sidebar();
        sidebar.setTranslatesAutoresizingMaskIntoConstraints(false);
        drawer.addSubview(&title);
        drawer.addSubview(&close);
        drawer.addSubview(&sidebar);
        content.addSubview(&drawer);
        ui::activate(&[
            scrim
                .topAnchor()
                .constraintEqualToAnchor(&container.topAnchor()),
            scrim
                .leadingAnchor()
                .constraintEqualToAnchor(&container.leadingAnchor()),
            scrim
                .trailingAnchor()
                .constraintEqualToAnchor(&container.trailingAnchor()),
            scrim
                .bottomAnchor()
                .constraintEqualToAnchor(&container.bottomAnchor()),
            dismiss
                .topAnchor()
                .constraintEqualToAnchor(&scrim.topAnchor()),
            dismiss
                .leadingAnchor()
                .constraintEqualToAnchor(&scrim.leadingAnchor()),
            dismiss
                .trailingAnchor()
                .constraintEqualToAnchor(&scrim.trailingAnchor()),
            dismiss
                .bottomAnchor()
                .constraintEqualToAnchor(&scrim.bottomAnchor()),
            drawer
                .topAnchor()
                .constraintEqualToAnchor(&container.topAnchor()),
            drawer
                .trailingAnchor()
                .constraintEqualToAnchor(&container.trailingAnchor()),
            drawer
                .bottomAnchor()
                .constraintEqualToAnchor(&container.bottomAnchor()),
            drawer.widthAnchor().constraintEqualToConstant(340.),
            title
                .topAnchor()
                .constraintEqualToAnchor_constant(&drawer.topAnchor(), 24.),
            title
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&drawer.leadingAnchor(), 24.),
            close
                .centerYAnchor()
                .constraintEqualToAnchor(&title.centerYAnchor()),
            close
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&drawer.trailingAnchor(), -16.),
            close.widthAnchor().constraintEqualToConstant(44.),
            close.heightAnchor().constraintEqualToConstant(44.),
            sidebar
                .topAnchor()
                .constraintEqualToAnchor_constant(&title.bottomAnchor(), 24.),
            sidebar
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&drawer.leadingAnchor(), 16.),
            sidebar
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&drawer.trailingAnchor(), -16.),
            sidebar
                .bottomAnchor()
                .constraintEqualToAnchor_constant(&drawer.bottomAnchor(), -16.),
        ]);
        drawer.setHidden(true);
        scrim.setHidden(true);
        self.ivars().queue_panel.set(drawer).ok();
        self.ivars().queue_scrim.set(scrim).ok();
        window.setContentView(Some(&content));

        window.setContentMinSize(NSSize::new(820., 480.));
        window.setContentSize(NSSize::new(1180., 760.));
        window.center();
        window.setFrameUsingName(ns_string!("DooTubeMainWindow"));
        window.setFrameAutosaveName(ns_string!("DooTubeMainWindow"));
        window.setDelegate(Some(ProtocolObject::from_ref(self)));

        let ivars = self.ivars();
        ivars.window.set(window.clone()).ok();
        ivars.panes.set(panes).ok();
        ivars.placeholder.set(placeholder).ok();
        ivars.notice.set(notice).ok();
        self.update_account_ui();
        window.makeKeyAndOrderFront(None);

        let cookies = unsafe { WKWebsiteDataStore::defaultDataStore(mtm).httpCookieStore() };
        unsafe { cookies.addObserver(ProtocolObject::from_ref(self)) };
        self.refresh_sign_in_state();

        if let Err(error) = ivars.queue.borrow().as_ref() {
            self.show_sticky_notice(&format!(
                "Up Next couldn't be read and was left untouched. {error}"
            ));
        }
        let args: Vec<String> = std::env::args().skip(1).collect();
        let initial = args.join(" ");
        if initial.trim().is_empty() || !self.navigate(&initial) {
            self.switch_mode(Mode::YouTube);
        }
    }

    pub(crate) fn set_queue_visible(&self, visible: bool) {
        let ivars = self.ivars();
        let was_visible = ivars
            .queue_panel
            .get()
            .is_some_and(|panel| !panel.isHidden());
        if let Some(panel) = ivars.queue_panel.get() {
            panel.setHidden(!visible);
        }
        if let Some(scrim) = ivars.queue_scrim.get() {
            scrim.setHidden(!visible);
        }
        if visible {
            if let (Some(window), Some(table)) = (ivars.window.get(), ivars.sidebar.get()) {
                window.makeFirstResponder(Some(table));
            }
        } else if was_visible {
            if let (Some(window), Some(bar)) = (ivars.window.get(), ivars.bar.get()) {
                window.makeFirstResponder(Some(&bar.queue));
            }
        }
    }

    pub(crate) fn build_panes(&self) -> [Pane; 2] {
        let mtm = self.mtm();
        let config = unsafe { WKWebViewConfiguration::new(mtm) };
        unsafe {
            config.setMediaTypesRequiringUserActionForPlayback(WKAudiovisualMediaTypes::All);
            // Safari's application name: YouTube then follows the system appearance.
            config.setApplicationNameForUserAgent(Some(ns_string!("Version/27.0 Safari/605.1.15")));
            // Page fullscreen uses DooTube's native-window shim instead of WebKit's handoff.
            config.preferences().setElementFullscreenEnabled(false);
            let controller = config.userContentController();
            let script = WKUserScript::initWithSource_injectionTime_forMainFrameOnly(
                WKUserScript::alloc(mtm),
                &ns(&web::page_script()),
                WKUserScriptInjectionTime::AtDocumentStart,
                true,
            );
            controller.addUserScript(&script);
            // All frames: parts of Google's sign-in render in iframes.
            let passkeys = WKUserScript::initWithSource_injectionTime_forMainFrameOnly(
                WKUserScript::alloc(mtm),
                &ns(&web::sign_in_passkey_script()),
                WKUserScriptInjectionTime::AtDocumentStart,
                false,
            );
            controller.addUserScript(&passkeys);
            controller.addScriptMessageHandler_name(
                ProtocolObject::from_ref(self),
                &ns(web::MESSAGE_HANDLER),
            );
        }
        // Both panes share the configuration, so they share the default website data store and sign-in.
        Mode::ALL.map(|_| {
            let web = unsafe {
                WKWebView::initWithFrame_configuration(
                    WKWebView::alloc(mtm),
                    NSRect::new(NSPoint::new(0., 0.), NSSize::new(960., 700.)),
                    &config,
                )
            };
            unsafe {
                web.setNavigationDelegate(Some(ProtocolObject::from_ref(self)));
                web.setUIDelegate(Some(ProtocolObject::from_ref(self)));
                web.setAllowsBackForwardNavigationGestures(true);
            }
            web.setHidden(true);
            for key in OBSERVED_WEB_KEYS {
                let _: () = unsafe {
                    msg_send![
                        &*web,
                        addObserver: self,
                        forKeyPath: &*ns(key),
                        options: 0usize,
                        context: std::ptr::null_mut::<c_void>()
                    ]
                };
            }
            Pane::new(web)
        })
    }

    /// The source list, with the Now Playing card pinned below it.
    pub(crate) fn build_sidebar(&self) -> Retained<NSView> {
        let mtm = self.mtm();
        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(220., 700.));
        let table = NSTableView::initWithFrame(NSTableView::alloc(mtm), frame);
        table.setStyle(NSTableViewStyle::FullWidth);
        table.setBackgroundColor(&cinema_bar::raised_color());
        table.setRowHeight(56.);
        table.setHeaderView(None);
        table.setFloatsGroupRows(false);
        table.setAllowsEmptySelection(true);
        table.setColumnAutoresizingStyle(
            NSTableViewColumnAutoresizingStyle::LastColumnOnlyAutoresizingStyle,
        );
        let column =
            NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), ns_string!("main"));
        column.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
        table.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        table.addTableColumn(&column);
        table.sizeLastColumnToFit();
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(self)));
            table.setDelegate(Some(ProtocolObject::from_ref(self)));
            table.setTarget(Some(self));
            table.setAction(Some(sel!(sidebarClicked:)));
        }
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Up Next"));
        for item in [
            Some(ui::menu_item(
                mtm,
                "Open",
                Some(sel!(openSaved:)),
                "",
                Some(self),
            )),
            None,
            Some(ui::menu_item(
                mtm,
                "Move Up",
                Some(sel!(moveSavedUp:)),
                "",
                Some(self),
            )),
            Some(ui::menu_item(
                mtm,
                "Move Down",
                Some(sel!(moveSavedDown:)),
                "",
                Some(self),
            )),
            None,
            Some(ui::menu_item(
                mtm,
                "Remove from Up Next",
                Some(sel!(removeSaved:)),
                "",
                Some(self),
            )),
        ] {
            match item {
                Some(item) => {
                    item.setTag(CONTEXT_TAG);
                    menu.addItem(&item);
                }
                None => menu.addItem(&NSMenuItem::separatorItem(mtm)),
            }
        }
        unsafe { table.setMenu(Some(&menu)) };
        // Up Next rows reorder by dragging within the sidebar and drag out as video links.
        table.registerForDraggedTypes(&NSArray::from_retained_slice(&[ns(UP_NEXT_DRAG_TYPE)]));
        table.setDraggingSourceOperationMask_forLocal(NSDragOperation::Move, true);
        table.setDraggingSourceOperationMask_forLocal(NSDragOperation::Copy, false);
        let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frame);
        scroll.setDocumentView(Some(&table));
        scroll.setHasVerticalScroller(true);
        scroll.setAutohidesScrollers(true);
        scroll.setDrawsBackground(false);
        // Keep the single column as wide as the sidebar so long titles truncate at its edge.
        let clip = scroll.contentView();
        clip.setPostsFrameChangedNotifications(true);
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                self,
                sel!(sidebarResized:),
                Some(NSViewFrameDidChangeNotification),
                Some(&clip),
            );
        }
        self.ivars().sidebar.set(table).ok();
        let card = ui::NowPlayingCard::new(mtm);
        for (button, action) in [
            (&card.artwork, sel!(resumeMusic:)),
            (&card.previous, sel!(musicPrevious:)),
            (&card.play_pause, sel!(musicPlayPause:)),
            (&card.next, sel!(musicNext:)),
        ] {
            unsafe {
                button.setTarget(Some(self));
                button.setAction(Some(action));
            }
        }
        let column = NSStackView::new(mtm);
        column.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        column.setSpacing(0.);
        column.addArrangedSubview(&scroll);
        column.addArrangedSubview(&card.root);
        ui::activate(&[
            scroll
                .widthAnchor()
                .constraintEqualToAnchor(&column.widthAnchor()),
            card.root
                .widthAnchor()
                .constraintEqualToAnchor(&column.widthAnchor()),
        ]);
        self.ivars().now_playing.set(card).ok();
        column.into_super()
    }

    pub(crate) fn build_menus(&self) {
        let mtm = self.mtm();
        let app = NSApplication::sharedApplication(mtm);
        let target: &AnyObject = self.as_ref();
        let item =
            |title: &str, action, key: &str| ui::menu_item(mtm, title, Some(action), key, None);
        let mine = |title: &str, action, key: &str| {
            ui::menu_item(mtm, title, Some(action), key, Some(target))
        };
        let with_mods = |item: Retained<NSMenuItem>, mods: NSEventModifierFlags| {
            item.setKeyEquivalentModifierMask(mods);
            item
        };
        let main = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Main"));
        ui::add_menu(
            mtm,
            &main,
            "DooTube",
            vec![
                Some(item(
                    "About DooTube",
                    sel!(orderFrontStandardAboutPanel:),
                    "",
                )),
                None,
                Some(mine("Sign In to YouTube…", sel!(signIn:), "")),
                Some(mine("Sign Out of YouTube", sel!(signOut:), "")),
                None,
                Some(item("Hide DooTube", sel!(hide:), "h")),
                Some(with_mods(
                    item("Hide Others", sel!(hideOtherApplications:), "h"),
                    NSEventModifierFlags::Option | NSEventModifierFlags::Command,
                )),
                Some(item("Show All", sel!(unhideAllApplications:), "")),
                None,
                Some(item("Quit DooTube", sel!(terminate:), "q")),
            ],
        );
        ui::add_menu(
            mtm,
            &main,
            "File",
            vec![
                Some(mine("Search…", sel!(focusSearch:), "l")),
                Some(mine("Open in Browser", sel!(openBrowser:), "")),
                None,
                Some(item("Close Window", sel!(performClose:), "w")),
            ],
        );
        ui::add_menu(
            mtm,
            &main,
            "Edit",
            vec![
                Some(item("Undo", sel!(undo:), "z")),
                Some(item("Redo", sel!(redo:), "Z")),
                None,
                Some(item("Cut", sel!(cut:), "x")),
                Some(item("Copy", sel!(copy:), "c")),
                Some(item("Paste", sel!(paste:), "v")),
                Some(item("Select All", sel!(selectAll:), "a")),
            ],
        );
        let control_command = NSEventModifierFlags::Control | NSEventModifierFlags::Command;
        ui::add_menu(
            mtm,
            &main,
            "View",
            vec![
                Some(mine("YouTube", sel!(showYouTube:), "1")),
                Some(mine("YouTube Music", sel!(showMusic:), "2")),
                None,
                Some(with_mods(
                    mine("Show Up Next", sel!(toggleQueue:), "s"),
                    control_command,
                )),
                None,
                Some(mine("Back", sel!(goBack:), "[")),
                Some(mine("Forward", sel!(goForward:), "]")),
                Some(mine("Home", sel!(goHome:), "H")),
                Some(mine("Reload Page", sel!(reloadPage:), "r")),
                None,
                Some(with_mods(
                    mine("Picture in Picture", sel!(togglePictureInPicture:), "p"),
                    control_command,
                )),
                None,
                Some(with_mods(
                    item("Enter Full Screen", sel!(toggleFullScreen:), "f"),
                    control_command,
                )),
            ],
        );
        ui::add_menu(
            mtm,
            &main,
            "Controls",
            vec![
                Some(mine("Play", sel!(musicPlayPause:), "p")),
                None,
                Some(mine("Next", sel!(musicNext:), "\u{F703}")),
                Some(mine("Previous", sel!(musicPrevious:), "\u{F702}")),
                None,
                Some(with_mods(
                    mine("Speed Up", sel!(speedUp:), "."),
                    NSEventModifierFlags::Option | NSEventModifierFlags::Command,
                )),
                Some(with_mods(
                    mine("Slow Down", sel!(slowDown:), ","),
                    NSEventModifierFlags::Option | NSEventModifierFlags::Command,
                )),
                Some(with_mods(
                    mine("Normal Speed", sel!(normalSpeed:), "0"),
                    NSEventModifierFlags::Option | NSEventModifierFlags::Command,
                )),
            ],
        );
        let option_command = NSEventModifierFlags::Option | NSEventModifierFlags::Command;
        ui::add_menu(
            mtm,
            &main,
            "Up Next",
            vec![
                Some(mine("Save Current Video", sel!(saveCurrentVideo:), "s")),
                None,
                Some(with_mods(
                    mine("Move Up", sel!(moveSavedUp:), "\u{F700}"),
                    option_command,
                )),
                Some(with_mods(
                    mine("Move Down", sel!(moveSavedDown:), "\u{F701}"),
                    option_command,
                )),
                None,
                Some(mine("Remove from Up Next", sel!(removeSaved:), "\u{8}")),
            ],
        );
        let window_menu = ui::add_menu(
            mtm,
            &main,
            "Window",
            vec![
                Some(item("Minimize", sel!(performMiniaturize:), "m")),
                Some(item("Zoom", sel!(performZoom:), "")),
                None,
                Some(item("Bring All to Front", sel!(arrangeInFront:), "")),
            ],
        );
        app.setMainMenu(Some(&main));
        app.setWindowsMenu(Some(&window_menu));
    }
}
