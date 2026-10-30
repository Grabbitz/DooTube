//! Lifecycle.

use super::*;

impl AppDelegate {
    /// Pauses and suspends media while the window is away, except a video in Picture in Picture,
    /// which stays visible in its own floating window.
    pub(crate) fn pause_web(&self) {
        for mode in Mode::ALL {
            if let Some(pane) = self.pane(mode)
                && !pane.pip_active.get()
            {
                unsafe {
                    pane.web.pauseAllMediaPlaybackWithCompletionHandler(None);
                    pane.web
                        .setAllMediaPlaybackSuspended_completionHandler(true, None);
                }
            }
        }
    }

    pub(crate) fn resume_web_controls(&self) {
        let ivars = self.ivars();
        if ivars.app_hidden.get() || ivars.window_minimized.get() || ivars.window_closed.get() {
            return;
        }
        for mode in Mode::ALL {
            if let Some(pane) = self.pane(mode) {
                unsafe {
                    pane.web
                        .setAllMediaPlaybackSuspended_completionHandler(false, None)
                };
            }
        }
    }

    /// The page asked to enter (with the player's rect) or leave video fullscreen.
    pub(crate) fn on_video_fullscreen_message(
        &self,
        web: &WKWebView,
        active: bool,
        rect: Option<PageRect>,
        id: u64,
    ) {
        let Some(mode) = self.mode_of(web) else {
            return;
        };
        let ivars = self.ivars();
        let current = ivars.fullscreen_mode.get() == Some(mode) && ivars.fullscreen_id.get() == id;
        if !active {
            if current {
                self.exit_video_fullscreen(true);
            } else {
                run_js(web, &exit_fullscreen_js(id));
            }
            return;
        }
        if current {
            self.resume_stage(mode);
            return;
        }
        // A new request replaces any other one still on screen.
        self.exit_video_fullscreen(false);
        let Some(rect) = rect.filter(|_| ivars.mode.get() == mode) else {
            run_js(web, &apply_fullscreen_js(false, id));
            return;
        };
        ivars.fullscreen_mode.set(Some(mode));
        ivars.fullscreen_id.set(id);
        // Snapshot the page first: it stands in for the web view in the main window while on stage.
        let this = self.retain();
        let bounds = web.bounds();
        self.snapshot(web, bounds, bounds.size.width, move |image| {
            this.prepare_stage(mode, rect, id, image);
        });
    }

    /// Snapshots `rect` of the web view for display `shown_width` points wide, at standard
    /// (not Retina) resolution. WebKit paints snapshots in the page's process, which then stalls
    /// YouTube; at Retina resolution that took 0.5 s with a 60 fps video, at standard ~0.1 s.
    /// The images only stand in for a moment while full screen begins or ends.
    pub(crate) fn snapshot(
        &self,
        web: &WKWebView,
        rect: NSRect,
        shown_width: f64,
        then: impl Fn(Option<&NSImage>) + 'static,
    ) {
        let scale = web
            .window()
            .map_or(1., |window| window.backingScaleFactor());
        let config = unsafe { WKSnapshotConfiguration::new(self.mtm()) };
        unsafe {
            config.setRect(rect);
            config.setSnapshotWidth(Some(&NSNumber::new_f64(shown_width / scale)));
        }
        let block = RcBlock::new(move |image: *mut NSImage, _error: *mut NSError| {
            then(unsafe { image.as_ref() });
        });
        unsafe { web.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block) };
    }

    pub(crate) fn prepare_stage(
        &self,
        mode: Mode,
        rect: PageRect,
        id: u64,
        image: Option<&NSImage>,
    ) {
        let ivars = self.ivars();
        let (Some(pane), Some(window), Some(container)) =
            (self.pane(mode), ivars.window.get(), ivars.container.get())
        else {
            return;
        };
        if ivars.fullscreen_mode.get() != Some(mode) || ivars.fullscreen_id.get() != id {
            run_js(&pane.web, &apply_fullscreen_js(false, id));
            return;
        }
        let stage = self.stage();
        let Some(content) = stage.window.contentView() else {
            return;
        };
        let screen = window
            .screen()
            .map_or_else(|| window.frame(), |screen| screen.frame());
        let player = NSRect::new(
            NSPoint::new(rect.x, rect.y),
            NSSize::new(rect.width, rect.height),
        );
        let on_screen = window.convertRectToScreen(pane.web.convertRect_toView(player, None));
        ivars.stage_start.set(NSRect::new(
            NSPoint::new(
                on_screen.origin.x - screen.origin.x,
                on_screen.origin.y - screen.origin.y,
            ),
            on_screen.size,
        ));
        ivars
            .stage_player
            .set(pane.web.convertRect_toView(player, Some(container)));
        stage.frame.removeFromSuperview();
        stage.frame.setImage(None);
        // The snapshot stands in for the page in the main window while the web view is on stage.
        stage.snapshot.setImage(image);
        stage.snapshot.setFrame(container.bounds());
        container.addSubview(&stage.snapshot);
        // Move the web view onto the stage at full size but invisible, so the page can lay out
        // and paint as fullscreen before the zoom shows it.
        stage.window.setFrame_display(screen, false);
        stage.host.setFrame(content.bounds());
        stage.host.setAlphaValue(0.);
        stage.backdrop.setAlphaValue(0.);
        let web = &pane.web;
        web.removeFromSuperview();
        web.setFrame(stage.host.bounds());
        stage.host.addSubview(web);
        if !ivars.native_fullscreen.get() {
            let app = NSApplication::sharedApplication(self.mtm());
            ivars
                .saved_presentation
                .set(Some(app.presentationOptions()));
            app.setPresentationOptions(
                NSApplicationPresentationOptions::HideDock
                    | NSApplicationPresentationOptions::HideMenuBar,
            );
        }
        stage.window.makeKeyAndOrderFront(None);
        stage.window.makeFirstResponder(Some(web));
        ivars.stage_shown.set(true);
        let this = self.retain();
        self.run_js_after_paint(
            web,
            &format!("{};", apply_fullscreen_js(true, id)),
            move || {
                this.zoom_onto_stage(mode, id);
            },
        );
    }

    pub(crate) fn zoom_onto_stage(&self, mode: Mode, id: u64) {
        let ivars = self.ivars();
        let Some(stage) = ivars.stage.get() else {
            return;
        };
        if ivars.fullscreen_mode.get() != Some(mode)
            || ivars.fullscreen_id.get() != id
            || !ivars.stage_shown.get()
        {
            return;
        }
        if ivars.stage_leaving.get() {
            self.finish_stage(mode);
            return;
        }
        let start = stage.zoom_transform(ivars.stage_start.get());
        stage.host.setAlphaValue(1.);
        self.run_zoom(mode, true, Some(start));
    }

    /// The page asked for full screen again while zooming out: turn around and zoom back in.
    pub(crate) fn resume_stage(&self, mode: Mode) {
        let ivars = self.ivars();
        // Before the zoom-in started, clearing the flag is enough; it starts once painted.
        if ivars.stage_leaving.replace(false)
            && ivars
                .stage
                .get()
                .is_some_and(|stage| stage.host.alphaValue() != 0.)
        {
            self.run_zoom(mode, true, None);
        }
    }

    pub(crate) fn run_zoom(&self, mode: Mode, zoom_in: bool, from: Option<CATransform3D>) {
        let ivars = self.ivars();
        let Some(stage) = ivars.stage.get() else {
            return;
        };
        let generation = ivars.zoom_generation.get().wrapping_add(1);
        ivars.zoom_generation.set(generation);
        let start = stage.zoom_transform(ivars.stage_start.get());
        let this = self.retain();
        let done = RcBlock::new(move || {
            if this.ivars().zoom_generation.get() == generation {
                this.finish_after_zoom_out(mode, generation);
            }
        });
        stage.zoom(start, zoom_in, from, (!zoom_in).then_some(&*done));
    }

    /// Zoomed back out: captures the video's current frame for the main window, then leaves the
    /// stage. Without it, the page snapshot would show the frame from when full screen began
    /// until the page has laid out again.
    pub(crate) fn finish_after_zoom_out(&self, mode: Mode, generation: u64) {
        let Some(pane) = self.pane(mode) else {
            return;
        };
        // The video is letterboxed or pillarboxed to the player's aspect ratio on the stage.
        let size = pane.web.bounds().size;
        let player = self.ivars().stage_player.get().size;
        let aspect = player.width / player.height.max(1.);
        let video = if size.width / size.height.max(1.) > aspect {
            let width = size.height * aspect;
            NSRect::new(
                NSPoint::new((size.width - width) / 2., 0.),
                NSSize::new(width, size.height),
            )
        } else {
            let height = size.width / aspect;
            NSRect::new(
                NSPoint::new(0., (size.height - height) / 2.),
                NSSize::new(size.width, height),
            )
        };
        let this = self.retain();
        self.snapshot(&pane.web, video, player.width, move |image| {
            let ivars = this.ivars();
            // A quick toggle turned the zoom around, or the stage already closed.
            if ivars.zoom_generation.get() != generation {
                return;
            }
            if let Some(stage) = ivars.stage.get() {
                stage.frame.setImage(image);
            }
            this.finish_stage(mode);
        });
    }

    /// Leaves video fullscreen, zooming the video back into the page when `animated`.
    pub(crate) fn exit_video_fullscreen(&self, animated: bool) {
        let ivars = self.ivars();
        let Some(mode) = ivars.fullscreen_mode.get() else {
            return;
        };
        if !ivars.stage_shown.get() {
            // Still snapshotting; the pending step sees the cleared mode and stops.
            ivars.fullscreen_mode.set(None);
            if let Some(pane) = self.pane(mode) {
                run_js(&pane.web, &exit_fullscreen_js(ivars.fullscreen_id.get()));
            }
            return;
        }
        let Some(stage) = ivars.stage.get() else {
            return;
        };
        if !animated {
            self.finish_stage(mode);
            return;
        }
        if ivars.stage_leaving.replace(true) {
            return;
        }
        // Not zoomed in yet (the page is still painting its full-screen layout): the pending
        // zoom-in sees `stage_leaving` and puts it back, unless a quick toggle resumes first.
        if stage.host.alphaValue() != 0. {
            self.run_zoom(mode, false, None);
        }
    }

    pub(crate) fn finish_stage(&self, mode: Mode) {
        let ivars = self.ivars();
        if !ivars.stage_shown.replace(false) {
            return;
        }
        ivars
            .zoom_generation
            .set(ivars.zoom_generation.get().wrapping_add(1));
        ivars.stage_leaving.set(false);
        ivars.fullscreen_mode.set(None);
        let (Some(pane), Some(stage), Some(container), Some(window)) = (
            self.pane(mode),
            ivars.stage.get(),
            ivars.container.get(),
            ivars.window.get(),
        ) else {
            return;
        };
        let web = &pane.web;
        web.removeFromSuperview();
        web.setFrame(container.bounds());
        // Below the snapshot, so the page repaints at its normal size before it is revealed.
        container.addSubview_positioned_relativeTo(
            web,
            NSWindowOrderingMode::Below,
            Some(&stage.snapshot),
        );
        if stage.frame.image().is_some() {
            stage.frame.setFrame(ivars.stage_player.get());
            container.addSubview_positioned_relativeTo(
                &stage.frame,
                NSWindowOrderingMode::Above,
                Some(&stage.snapshot),
            );
        }
        stage.window.orderOut(None);
        stage.reset_zoom();
        if let Some(options) = ivars.saved_presentation.take() {
            NSApplication::sharedApplication(self.mtm()).setPresentationOptions(options);
        }
        if !ivars.window_closed.get() && !ivars.window_minimized.get() {
            window.makeKeyAndOrderFront(None);
        }
        window.makeFirstResponder(Some(web));
        let this = self.retain();
        let exit = format!("{};", exit_fullscreen_js(ivars.fullscreen_id.get()));
        self.run_js_after_paint(web, &exit, move || {
            if this.ivars().fullscreen_mode.get().is_none()
                && let Some(stage) = this.ivars().stage.get()
            {
                stage.snapshot.removeFromSuperview();
                stage.snapshot.setImage(None);
                stage.frame.removeFromSuperview();
                stage.frame.setImage(None);
            }
        });
    }

    /// Runs `script` in the page, then calls `then` once the page has painted the result (or after
    /// 120 ms, since WebKit may pause animation frames while the stage is still invisible).
    pub(crate) fn run_js_after_paint(
        &self,
        web: &WKWebView,
        script: &str,
        then: impl Fn() + 'static,
    ) {
        let body = format!(
            "{script}\nawait new Promise((resolve) => {{ requestAnimationFrame(() => requestAnimationFrame(resolve)); setTimeout(resolve, 120); }});"
        );
        let block = RcBlock::new(move |_result: *mut AnyObject, _error: *mut NSError| then());
        unsafe {
            web.callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(
                &ns(&body),
                None,
                None,
                &WKContentWorld::pageWorld(self.mtm()),
                Some(&block),
            );
        }
    }

    pub(crate) fn stage(&self) -> &Stage {
        self.ivars().stage.get_or_init(|| {
            let mtm = self.mtm();
            let window: Retained<StageWindow> = unsafe {
                msg_send![
                    StageWindow::alloc(mtm),
                    initWithContentRect: NSRect::ZERO,
                    styleMask: NSWindowStyleMask::Borderless,
                    backing: NSBackingStoreType::Buffered,
                    defer: false
                ]
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setOpaque(false);
            window.setBackgroundColor(Some(&NSColor::clearColor()));
            window.setHasShadow(false);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::MoveToActiveSpace,
            );
            let sizable = NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable;
            let backdrop = NSBox::new(mtm);
            backdrop.setBoxType(NSBoxType::Custom);
            backdrop.setBorderWidth(0.);
            backdrop.setTitlePosition(NSTitlePosition::NoTitle);
            backdrop.setFillColor(&NSColor::blackColor());
            backdrop.setAutoresizingMask(sizable);
            backdrop.setWantsLayer(true);
            if let Some(content) = window.contentView() {
                backdrop.setFrame(content.bounds());
                content.addSubview(&backdrop);
            }
            let host = NSView::new(mtm);
            host.setWantsLayer(true);
            if let Some(content) = window.contentView() {
                content.setWantsLayer(true);
                content.addSubview(&host);
            }
            let snapshot = NSImageView::new(mtm);
            snapshot.setImageScaling(NSImageScaling::ScaleAxesIndependently);
            snapshot.setAutoresizingMask(sizable);
            let frame = NSImageView::new(mtm);
            frame.setImageScaling(NSImageScaling::ScaleAxesIndependently);
            Stage {
                window,
                backdrop,
                host,
                snapshot,
                frame,
            }
        })
    }

    /// Native window fullscreen changed independently of the page (green button, keyboard, or system).
    pub(crate) fn exit_video_fullscreen_from_external_transition(&self) {
        self.exit_video_fullscreen(false);
    }

    pub(crate) fn on_window_close(&self) {
        self.exit_video_fullscreen(false);
        self.ivars().window_closed.set(true);
        self.pause_web();
    }

    pub(crate) fn on_window_reopen(&self) {
        self.ivars().window_closed.set(false);
        self.ivars().app_hidden.set(false);
        if let Some(window) = self.ivars().window.get() {
            window.makeKeyAndOrderFront(None);
        }
        self.resume_web_controls();
    }

    pub(crate) fn on_window_minimize(&self) {
        self.exit_video_fullscreen(false);
        self.ivars().window_minimized.set(true);
        self.pause_web();
    }

    pub(crate) fn on_window_restore(&self) {
        self.ivars().window_minimized.set(false);
        self.resume_web_controls();
    }

    pub(crate) fn on_application_hide(&self) {
        self.ivars().app_hidden.set(true);
        self.pause_web();
    }

    pub(crate) fn on_application_unhide(&self) {
        self.ivars().app_hidden.set(false);
        self.resume_web_controls();
    }

    /// Keep transient fullscreen geometry from replacing the user's saved window frame.
    pub(crate) fn prepare_window_fullscreen(&self) {
        if let Some(window) = self.ivars().window.get() {
            window.setFrameAutosaveName(ns_string!("DooTubeMainWindow.FullScreen"));
        }
    }

    pub(crate) fn finish_window_fullscreen(&self) {
        if let Some(window) = self.ivars().window.get() {
            window.setFrameAutosaveName(ns_string!("DooTubeMainWindow"));
        }
    }
}
