#![deny(unsafe_op_in_unsafe_fn)]
mod core;
mod google_domains;
mod ui;
mod web;

use block2::RcBlock;
use core::{
    AppState, Navigation, clean_page_title, is_google_sign_in_url, is_music_url,
    is_supported_youtube_url, load_state, parse_input, save_state, video_id,
};
use dispatch2::DispatchQueue;
use objc2::{
    AllocAnyThread, ClassType, DefinedClass, MainThreadOnly, Message, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSApplicationPresentationOptions, NSAutoresizingMaskOptions, NSBackingStoreType, NSBox,
    NSBoxType, NSColor, NSControl, NSControlStateValueOff, NSControlStateValueOn,
    NSControlTextEditingDelegate, NSEventModifierFlags, NSEventType, NSImage, NSImageScaling,
    NSImageView, NSMenu, NSMenuItem, NSMenuToolbarItem, NSResponder, NSScrollView,
    NSSearchToolbarItem, NSSegmentSwitchTracking, NSSegmentedControl, NSSplitViewController,
    NSSplitViewItem, NSTableColumn, NSTableColumnResizingOptions, NSTableView,
    NSTableViewColumnAutoresizingStyle, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewStyle, NSTitlePosition, NSToolbar, NSToolbarDelegate, NSToolbarDisplayMode,
    NSToolbarFlexibleSpaceItemIdentifier, NSToolbarItem,
    NSToolbarSidebarTrackingSeparatorItemIdentifier, NSToolbarToggleSidebarItemIdentifier, NSView,
    NSViewController, NSViewFrameDidChangeNotification, NSWindow, NSWindowCollectionBehavior,
    NSWindowDelegate, NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility,
    NSWindowToolbarStyle, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSData, NSError, NSHTTPCookie, NSIndexSet, NSInteger,
    NSNotification, NSNotificationCenter, NSNumber, NSObject, NSObjectProtocol, NSPoint, NSRect,
    NSSet, NSSize, NSString, NSURL, NSURLRequest, NSValue, ns_string,
};
use objc2_quartz_core::{
    CABasicAnimation, CAMediaTiming, CAMediaTimingFunction, CATransaction, CATransform3D,
    CATransform3DIdentity, NSValueCATransform3DAdditions, kCAMediaTimingFunctionEaseInEaseOut,
    kCAMediaTimingFunctionEaseOut,
};
use objc2_web_kit::{
    WKAudiovisualMediaTypes, WKContentWorld, WKHTTPCookieStore, WKHTTPCookieStoreObserver,
    WKNavigation, WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate,
    WKNavigationType, WKScriptMessage, WKScriptMessageHandler, WKUIDelegate,
    WKUserContentController, WKUserScript, WKUserScriptInjectionTime, WKWebView,
    WKSnapshotConfiguration, WKWebViewConfiguration, WKWebsiteDataStore, WKWindowFeatures,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    ffi::c_void,
    ptr::NonNull,
    time::{Duration, Instant},
};
use ui::ns;
use url::Url;
use web::{Mode, PageMessage, PageRect, Pane};

const BACK_ID: &str = "dootube.back";
const FORWARD_ID: &str = "dootube.forward";
const HOME_ID: &str = "dootube.home";
const MODE_ID: &str = "dootube.mode";
const SEARCH_ID: &str = "dootube.search";
const SAVE_ID: &str = "dootube.save";
const PIP_ID: &str = "dootube.pip";
const ACCOUNT_ID: &str = "dootube.account";
const SIGN_OUT_URL: &str = "https://www.youtube.com/logout";
/// Tag for sidebar context-menu items, which act on the clicked row rather than the selection.
const CONTEXT_TAG: NSInteger = 1;
const OBSERVED_WEB_KEYS: [&str; 5] = [
    "URL",
    "title",
    "canGoBack",
    "canGoForward",
    "underPageBackgroundColor",
];
fn is_youtube_main_frame(message: &WKScriptMessage) -> bool {
    let frame = unsafe { message.frameInfo() };
    if !unsafe { frame.isMainFrame() } {
        return false;
    }
    let request = unsafe { frame.request() };
    request
        .URL()
        .and_then(|url| url.absoluteString())
        .and_then(|text| Url::parse(&text.to_string()).ok())
        .is_some_and(|url| url.scheme() == "https" && url.host_str() == Some("www.youtube.com"))
}
fn is_video_fullscreen_main_frame(message: &WKScriptMessage) -> bool {
    let frame = unsafe { message.frameInfo() };
    if !unsafe { frame.isMainFrame() } {
        return false;
    }
    let request = unsafe { frame.request() };
    request
        .URL()
        .and_then(|url| url.absoluteString())
        .and_then(|text| Url::parse(&text.to_string()).ok())
        .is_some_and(|url| {
            url.scheme() == "https"
                && matches!(
                    url.host_str(),
                    Some("youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com")
                )
        })
}

fn exit_fullscreen_js(id: u64) -> String {
    format!("window.__dootubeExitFullscreen && window.__dootubeExitFullscreen({id})")
}
fn apply_fullscreen_js(ok: bool, id: u64) -> String {
    format!("window.__dootubeApplyFullscreen && window.__dootubeApplyFullscreen({ok}, {id})")
}

fn run_js(web: &WKWebView, script: &str) {
    unsafe { web.evaluateJavaScript_completionHandler(&ns(script), None) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    /// Index into the current section's destinations.
    Destination(usize),
    UpNextHeader,
    Saved(usize),
    EmptyHint,
}

/// Saved Up Next state. `Err` means the file was unreadable: it is left untouched and edits are blocked.
type Queue = Result<AppState, String>;

struct AppIvars {
    window: OnceCell<Retained<NSWindow>>,
    /// YouTube and YouTube Music panes, indexed by [`Mode::index`].
    panes: OnceCell<[Pane; 2]>,
    mode: Cell<Mode>,
    sidebar_item: OnceCell<Retained<NSSplitViewItem>>,
    /// Pane whose video is entering, in, or leaving fullscreen.
    fullscreen_mode: Cell<Option<Mode>>,
    /// The web view has moved onto the stage window.
    stage_shown: Cell<bool>,
    stage_leaving: Cell<bool>,
    /// The page's id for the current full-screen request; replies carry it.
    fullscreen_id: Cell<u64>,
    /// Bumped by every zoom, so an interrupted zoom-out's completion doesn't end full screen.
    zoom_generation: Cell<u64>,
    /// Where the video sat on the page, in stage coordinates; the zoom starts and ends here.
    stage_start: Cell<NSRect>,
    /// Where the video sat on the page, in the main window's container; the video's last frame
    /// is laid there while the page returns from full screen.
    stage_player: Cell<NSRect>,
    /// Presentation options to restore; set only when the stage hid the Dock and menu bar.
    saved_presentation: Cell<Option<NSApplicationPresentationOptions>>,
    stage: OnceCell<Stage>,
    container: OnceCell<Retained<NSView>>,
    /// Fills the content area, including the strip behind the toolbar, with the page's background.
    backdrop: OnceCell<Retained<NSBox>>,
    native_fullscreen: Cell<bool>,
    sidebar: OnceCell<Retained<NSTableView>>,
    search: OnceCell<Retained<NSSearchToolbarItem>>,
    mode_control: OnceCell<Retained<NSSegmentedControl>>,
    back_item: OnceCell<Retained<NSToolbarItem>>,
    forward_item: OnceCell<Retained<NSToolbarItem>>,
    home_item: OnceCell<Retained<NSToolbarItem>>,
    mode_item: OnceCell<Retained<NSToolbarItem>>,
    save_item: OnceCell<Retained<NSToolbarItem>>,
    pip_item: OnceCell<Retained<NSToolbarItem>>,
    account_item: OnceCell<Retained<NSMenuToolbarItem>>,
    placeholder: OnceCell<ui::Placeholder>,
    notice: OnceCell<ui::Notice>,
    /// `None` until the cookie store has been read once.
    signed_in: Cell<Option<bool>>,
    account_identity: RefCell<Option<web::AccountIdentity>>,
    /// The active account's photo, once downloaded.
    account_photo: RefCell<Option<Retained<NSImage>>>,
    account_lookup_pending: Cell<bool>,
    account_lookup_attempted: Cell<bool>,
    syncing_selection: Cell<bool>,
    app_hidden: Cell<bool>,
    window_minimized: Cell<bool>,
    window_closed: Cell<bool>,
    queue: RefCell<Queue>,
}

/// Seconds for the video zoom into and out of fullscreen.
const STAGE_ZOOM_SECONDS: f64 = 0.32;

define_class!(
    /// Borderless screen-sized window that shows only the video during page fullscreen.
    #[unsafe(super(NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    struct StageWindow;

    impl StageWindow {
        // Borderless windows refuse key status by default, which would drop Escape and player keys.
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }
    }
);

/// The video fullscreen stage: its window, the black backdrop that fades in behind the video, and
/// a snapshot that stands in for the page in the main window while the web view is on stage.
struct Stage {
    window: Retained<StageWindow>,
    backdrop: Retained<NSBox>,
    /// Layer-backed parent of the web view; the zoom animates only its layer transform, so
    /// WebKit never relayouts mid-animation.
    host: Retained<NSView>,
    snapshot: Retained<NSImageView>,
    /// The video's frame as the zoom-out ended, laid over `snapshot` at the player's spot so the
    /// page doesn't flash back to the frame from when full screen began.
    frame: Retained<NSImageView>,
}

impl Stage {
    /// Transform that shrinks the full-size stage so its video sits exactly over `target`,
    /// the player's rect on the page (both in stage coordinates).
    fn zoom_transform(&self, target: NSRect) -> CATransform3D {
        let size = self.host.bounds().size;
        let Some(layer) = self.host.layer() else {
            return unsafe { CATransform3DIdentity };
        };
        // The fullscreen video is letterboxed or pillarboxed to the player's aspect ratio.
        let aspect = target.size.width / target.size.height.max(1.);
        let video_width = if size.width / size.height.max(1.) > aspect {
            size.height * aspect
        } else {
            size.width
        };
        let scale = target.size.width / video_width.max(1.);
        let anchor = layer.anchorPoint();
        let (ax, ay) = (anchor.x * size.width, anchor.y * size.height);
        let mut transform = unsafe { CATransform3DIdentity };
        transform.m11 = scale;
        transform.m22 = scale;
        transform.m41 =
            target.origin.x + target.size.width / 2. - ax - scale * (size.width / 2. - ax);
        transform.m42 =
            target.origin.y + target.size.height / 2. - ay - scale * (size.height / 2. - ay);
        transform
    }

    /// Zooms the video in (to full screen) or out (to `start`, the player's rect) on the GPU,
    /// fading the backdrop with it, then runs `done`. Without `from` it continues from wherever an
    /// interrupted zoom is now, taking only the remaining share of the duration, so quick repeated
    /// toggles reverse smoothly.
    fn zoom(
        &self,
        start: CATransform3D,
        zoom_in: bool,
        from: Option<CATransform3D>,
        done: Option<&block2::DynBlock<dyn Fn()>>,
    ) {
        let (Some(host), Some(backdrop)) = (self.host.layer(), self.backdrop.layer()) else {
            if let Some(done) = done {
                done.call(());
            }
            return;
        };
        let identity = unsafe { CATransform3DIdentity };
        let (to, backdrop_to) = if zoom_in { (identity, 1.) } else { (start, 0.) };
        let current = from.unwrap_or_else(|| {
            unsafe { host.presentationLayer() }
                .map_or_else(|| host.transform(), |layer| layer.transform())
        });
        let current_opacity = if from.is_some() {
            1. - backdrop_to
        } else {
            unsafe { backdrop.presentationLayer() }
                .map_or_else(|| backdrop.opacity(), |layer| layer.opacity()) as f64
        };
        // 0 = at the player's rect, 1 = full screen.
        let span = 1. - start.m11;
        let progress = if span.abs() < 1e-6 {
            1.
        } else {
            ((current.m11 - start.m11) / span).clamp(0., 1.)
        };
        let remaining = if zoom_in { 1. - progress } else { progress };
        let duration = (STAGE_ZOOM_SECONDS * remaining).max(0.06);
        // A fresh zoom eases in and out; a reversal is already moving, so it only eases out.
        let curve = CAMediaTimingFunction::functionWithName(unsafe {
            if from.is_some() {
                kCAMediaTimingFunctionEaseInEaseOut
            } else {
                kCAMediaTimingFunctionEaseOut
            }
        });
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        unsafe { CATransaction::setCompletionBlock(done) };
        host.setTransform(to);
        self.backdrop.setAlphaValue(backdrop_to);
        backdrop.setOpacity(backdrop_to as f32);
        let zoom = CABasicAnimation::animationWithKeyPath(Some(ns_string!("transform")));
        unsafe {
            zoom.setFromValue(Some(&NSValue::valueWithCATransform3D(current)));
            zoom.setToValue(Some(&NSValue::valueWithCATransform3D(to)));
        }
        let fade = CABasicAnimation::animationWithKeyPath(Some(ns_string!("opacity")));
        unsafe {
            fade.setFromValue(Some(&NSNumber::new_f64(current_opacity)));
            fade.setToValue(Some(&NSNumber::new_f64(backdrop_to)));
        }
        for (layer, animation, key) in [
            (&host, &zoom, ns_string!("zoom")),
            (&backdrop, &fade, ns_string!("fade")),
        ] {
            animation.setDuration(duration);
            animation.setTimingFunction(Some(&curve));
            layer.addAnimation_forKey(animation, Some(key));
        }
        CATransaction::commit();
    }

    fn reset_zoom(&self) {
        if let Some(host) = self.host.layer() {
            CATransaction::begin();
            CATransaction::setDisableActions(true);
            host.removeAllAnimations();
            host.setTransform(unsafe { CATransform3DIdentity });
            CATransaction::commit();
        }
        self.host.setAlphaValue(1.);
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppIvars]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            self.build_window();
        }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            self.on_window_reopen();
            true
        }
        #[unsafe(method(applicationDidHide:))]
        fn did_hide(&self, _notification: &NSNotification) {
            self.on_application_hide();
        }
        #[unsafe(method(applicationDidUnhide:))]
        fn did_unhide(&self, _notification: &NSNotification) {
            self.on_application_unhide();
        }
    }

    unsafe impl WKUIDelegate for AppDelegate {
        #[unsafe(method_id(webView:createWebViewWithConfiguration:forNavigationAction:windowFeatures:))]
        unsafe fn create_web_view(
            &self,
            web_view: &WKWebView,
            _configuration: &WKWebViewConfiguration,
            action: &WKNavigationAction,
            _window_features: &WKWindowFeatures,
        ) -> Option<Retained<WKWebView>> {
            let request = unsafe { action.request() };
            let parsed = request
                .URL()
                .and_then(|url| url.absoluteString())
                .and_then(|text| Url::parse(&text.to_string()).ok());
            if let Some(url) = parsed.as_ref()
                && (is_google_sign_in_url(url) || is_supported_youtube_url(url))
            {
                unsafe { web_view.loadRequest(&request) };
            } else if unsafe { action.navigationType() } == WKNavigationType::LinkActivated
                && let (Some(target), Some(url)) = (request.URL(), parsed.as_ref())
                && matches!(url.scheme(), "http" | "https")
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
            {
                if NSWorkspace::sharedWorkspace().openURL(&target) {
                    self.show_notice("Opened in your browser.");
                } else {
                    self.show_notice("Couldn't open that link in your browser.");
                }
            } else {
                self.show_notice("Links that open a new window aren't supported in DooTube.");
            }
            None
        }
    }

    unsafe impl WKNavigationDelegate for AppDelegate {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        unsafe fn decide_navigation(
            &self,
            web_view: &WKWebView,
            action: &WKNavigationAction,
            decision: &block2::DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            let policy = self.navigation_policy(web_view, action);
            decision.call((policy,));
        }
        #[unsafe(method(webView:didCommitNavigation:))]
        fn did_commit_navigation(&self, web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            if self.ivars().fullscreen_mode.get().is_some()
                && self.ivars().fullscreen_mode.get() == self.mode_of(web_view)
            {
                self.exit_video_fullscreen(false);
            }
            if let Some(pane) = self.pane_of(web_view) {
                pane.loaded.set(true);
                pane.failed.set(false);
                self.show_active_pane();
            }
        }
        #[unsafe(method(webView:didFinishNavigation:))]
        fn did_finish_navigation(&self, web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            self.refresh_sign_in_state();
            self.maybe_fetch_account_identity_in(web_view);
        }
        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_provisional(
            &self,
            web_view: &WKWebView,
            _navigation: Option<&WKNavigation>,
            error: &NSError,
        ) {
            self.on_load_error(web_view, error);
        }
        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn did_fail_navigation(
            &self,
            web_view: &WKWebView,
            _navigation: Option<&WKNavigation>,
            error: &NSError,
        ) {
            self.on_load_error(web_view, error);
        }
        #[unsafe(method(webViewWebContentProcessDidTerminate:))]
        fn web_content_process_terminated(&self, web_view: &WKWebView) {
            self.on_web_process_ended(web_view);
        }
    }

    unsafe impl WKScriptMessageHandler for AppDelegate {
        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn did_receive_script_message(
            &self,
            _controller: &WKUserContentController,
            message: &WKScriptMessage,
        ) {
            let (Some(web), body) = (unsafe { message.webView() }, unsafe { message.body() }) else {
                return;
            };
            let report = body
                .downcast_ref::<NSString>()
                .and_then(|text| serde_json::from_str::<PageMessage>(&text.to_string()).ok());
            match report {
                Some(PageMessage::Fullscreen { active, rect, id })
                    if is_video_fullscreen_main_frame(message) =>
                {
                    self.on_video_fullscreen_message(&web, active, rect, id);
                }
                Some(PageMessage::Audible { title }) => self.on_audible_playback(&web, title),
                Some(PageMessage::PipFailed { reason }) => {
                    self.show_notice(&web::pip_failure_notice(&reason));
                }
                Some(PageMessage::Pip { active }) => {
                    if let Some(pane) = self.pane_of(&web) {
                        pane.pip_active.set(active);
                    }
                }
                Some(PageMessage::AccountIdentity { response })
                    if self.mode_of(&web).is_some()
                        && is_youtube_main_frame(message)
                        && self.ivars().signed_in.get() == Some(true)
                        && self.ivars().account_lookup_pending.get() =>
                {
                    self.ivars().account_lookup_pending.set(false);
                    let identity = web::parse_account_identity(&response);
                    let photo = identity.as_ref().and_then(|identity| identity.photo.clone());
                    *self.ivars().account_identity.borrow_mut() = identity;
                    *self.ivars().account_photo.borrow_mut() = None;
                    self.update_account_ui();
                    if let Some(url) = photo {
                        load_account_photo(url);
                    }
                }
                Some(PageMessage::AccountIdentity { .. })
                | Some(PageMessage::Fullscreen { .. })
                | None => {}
            }
        }
    }

    unsafe impl WKHTTPCookieStoreObserver for AppDelegate {
        #[unsafe(method(cookiesDidChangeInCookieStore:))]
        fn cookies_did_change(&self, _store: &WKHTTPCookieStore) {
            // Cookie changes arrive in bursts while pages load; read the store once things settle.
            unsafe {
                let _: () = msg_send![
                    <NSObject as ClassType>::class(),
                    cancelPreviousPerformRequestsWithTarget: self,
                    selector: sel!(refreshSignIn:),
                    object: Option::<&AnyObject>::None
                ];
                let _: () = msg_send![
                    self,
                    performSelector: sel!(refreshSignIn:),
                    withObject: Option::<&AnyObject>::None,
                    afterDelay: 0.5f64
                ];
            }
        }
    }

    unsafe impl NSWindowDelegate for AppDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            self.on_window_close();
        }
        #[unsafe(method(windowDidMiniaturize:))]
        fn window_did_miniaturize(&self, _notification: &NSNotification) {
            self.on_window_minimize();
        }
        #[unsafe(method(windowDidDeminiaturize:))]
        fn window_did_deminiaturize(&self, _notification: &NSNotification) {
            self.on_window_restore();
        }
        #[unsafe(method(windowWillEnterFullScreen:))]
        fn window_will_enter_full_screen(&self, _notification: &NSNotification) {
            self.ivars().native_fullscreen.set(true);
            self.prepare_window_fullscreen();
        }
        #[unsafe(method(windowDidExitFullScreen:))]
        fn window_did_exit_full_screen(&self, _notification: &NSNotification) {
            self.ivars().native_fullscreen.set(false);
            self.finish_window_fullscreen();
            self.exit_video_fullscreen_from_external_transition();
        }
        #[unsafe(method(window:willUseFullScreenPresentationOptions:))]
        fn window_will_use_full_screen_presentation_options(
            &self,
            _window: &NSWindow,
            _proposed_options: NSApplicationPresentationOptions,
        ) -> NSApplicationPresentationOptions {
            NSApplicationPresentationOptions::FullScreen
                | NSApplicationPresentationOptions::AutoHideToolbar
                | NSApplicationPresentationOptions::AutoHideMenuBar
        }
        #[unsafe(method(windowDidFailToEnterFullScreen:))]
        fn window_did_fail_to_enter_full_screen(&self, _window: &NSWindow) {
            self.ivars().native_fullscreen.set(false);
            self.finish_window_fullscreen();
            self.exit_video_fullscreen_from_external_transition();
        }
        #[unsafe(method(windowDidFailToExitFullScreen:))]
        fn window_did_fail_to_exit_full_screen(&self, _window: &NSWindow) {
            self.ivars().native_fullscreen.set(true);
            self.prepare_window_fullscreen();
        }
    }

    unsafe impl NSToolbarDelegate for AppDelegate {
        #[unsafe(method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:))]
        fn toolbar_item(
            &self,
            _toolbar: &NSToolbar,
            identifier: &NSString,
            _will_insert: bool,
        ) -> Option<Retained<NSToolbarItem>> {
            let ivars = self.ivars();
            match identifier.to_string().as_str() {
                BACK_ID => ivars.back_item.get().cloned(),
                FORWARD_ID => ivars.forward_item.get().cloned(),
                HOME_ID => ivars.home_item.get().cloned(),
                MODE_ID => ivars.mode_item.get().cloned(),
                SAVE_ID => ivars.save_item.get().cloned(),
                PIP_ID => ivars.pip_item.get().cloned(),
                SEARCH_ID => ivars.search.get().map(|item| Retained::into_super(item.clone())),
                ACCOUNT_ID => ivars.account_item.get().map(|item| Retained::into_super(item.clone())),
                _ => None,
            }
        }
        #[unsafe(method_id(toolbarDefaultItemIdentifiers:))]
        fn default_identifiers(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSString>> {
            toolbar_identifiers()
        }
        #[unsafe(method_id(toolbarAllowedItemIdentifiers:))]
        fn allowed_identifiers(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSString>> {
            toolbar_identifiers()
        }
    }

    unsafe impl NSTableViewDataSource for AppDelegate {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.rows().len() as NSInteger
        }
    }

    unsafe impl NSControlTextEditingDelegate for AppDelegate {}

    unsafe impl NSTableViewDelegate for AppDelegate {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for_row(
            &self,
            _table: &NSTableView,
            _column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            self.sidebar_view(row)
        }
        #[unsafe(method(tableView:isGroupRow:))]
        fn is_group_row(&self, _table: &NSTableView, row: NSInteger) -> bool {
            self.row_at(row) == Some(Row::UpNextHeader)
        }
        #[unsafe(method(tableView:shouldSelectRow:))]
        fn should_select_row(&self, _table: &NSTableView, row: NSInteger) -> bool {
            matches!(self.row_at(row), Some(Row::Destination(_) | Row::Saved(_)))
        }
        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_did_change(&self, _notification: &NSNotification) {
            if self.ivars().syncing_selection.get() {
                return;
            }
            // Mouse clicks are handled by the table action so re-clicking a row also works.
            let mouse = NSApplication::sharedApplication(self.mtm())
                .currentEvent()
                .is_some_and(|event| {
                    matches!(event.r#type(), NSEventType::LeftMouseDown | NSEventType::LeftMouseUp)
                });
            if !mouse && let Some(table) = self.ivars().sidebar.get() {
                self.open_row(table.selectedRow());
            }
        }
    }

    impl AppDelegate {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            key_path: Option<&NSString>,
            object: Option<&AnyObject>,
            _change: Option<&AnyObject>,
            _context: *mut c_void,
        ) {
            let Some(web) = object.and_then(|object| object.downcast_ref::<WKWebView>()) else {
                return;
            };
            match key_path.map(|key| key.to_string()).as_deref() {
                Some("title") => self.on_page_title_changed(web),
                Some("underPageBackgroundColor") if self.is_active(web) => self.update_backdrop(),
                Some(_) if self.is_active(web) => {
                    if key_path.is_some_and(|key| key.to_string() == "URL") {
                        self.on_page_url_changed();
                    } else {
                        self.update_toolbar();
                    }
                }
                _ => {}
            }
        }
        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, item: &NSMenuItem) -> bool {
            self.validate(item)
        }
        #[unsafe(method(searchSubmitted:))]
        fn action_search(&self, sender: &NSControl) {
            let text = sender.stringValue().to_string();
            if text.trim().is_empty() {
                return;
            }
            if self.navigate(&text)
                && let (Some(window), Some(pane)) = (self.ivars().window.get(), self.active())
            {
                window.makeFirstResponder(Some(&pane.web));
            }
        }
        #[unsafe(method(focusSearch:))]
        fn action_focus_search(&self, _sender: Option<&AnyObject>) {
            if let Some(search) = self.ivars().search.get() {
                search.beginSearchInteraction();
            }
        }
        #[unsafe(method(modeChanged:))]
        fn action_mode_changed(&self, sender: &NSSegmentedControl) {
            let mode = if sender.selectedSegment() == 1 { Mode::Music } else { Mode::YouTube };
            self.switch_mode(mode);
        }
        #[unsafe(method(showYouTube:))]
        fn action_show_youtube(&self, _sender: Option<&AnyObject>) {
            self.switch_mode(Mode::YouTube);
        }
        #[unsafe(method(showMusic:))]
        fn action_show_music(&self, _sender: Option<&AnyObject>) {
            self.switch_mode(Mode::Music);
        }
        #[unsafe(method(sidebarClicked:))]
        fn action_sidebar_clicked(&self, sender: &NSTableView) {
            self.open_row(sender.clickedRow());
        }
        #[unsafe(method(saveCurrentVideo:))]
        fn action_save(&self, _sender: Option<&AnyObject>) {
            self.save_current_video();
        }
        #[unsafe(method(openSaved:))]
        fn action_open_saved(&self, sender: Option<&AnyObject>) {
            if let Some(index) = self.target_saved_index(sender) {
                self.open_row(self.row_index(Row::Saved(index)));
            }
        }
        #[unsafe(method(removeSaved:))]
        fn action_remove_saved(&self, sender: Option<&AnyObject>) {
            if let Some(index) = self.target_saved_index(sender) {
                self.change_queue(|state| state.remove(index));
            }
        }
        #[unsafe(method(moveSavedUp:))]
        fn action_move_up(&self, sender: Option<&AnyObject>) {
            self.move_saved(sender, -1);
        }
        #[unsafe(method(moveSavedDown:))]
        fn action_move_down(&self, sender: Option<&AnyObject>) {
            self.move_saved(sender, 1);
        }
        #[unsafe(method(goBack:))]
        fn action_back(&self, _sender: Option<&AnyObject>) {
            if let Some(pane) = self.active() {
                unsafe { pane.web.goBack() };
            }
        }
        #[unsafe(method(goForward:))]
        fn action_forward(&self, _sender: Option<&AnyObject>) {
            if let Some(pane) = self.active() {
                unsafe { pane.web.goForward() };
            }
        }
        #[unsafe(method(goHome:))]
        fn action_home(&self, _sender: Option<&AnyObject>) {
            let mode = self.ivars().mode.get();
            self.load_in(mode, mode.home());
        }
        #[unsafe(method(reloadPage:))]
        fn action_reload(&self, _sender: Option<&AnyObject>) {
            self.reload_page();
        }
        #[unsafe(method(openBrowser:))]
        fn action_open_browser(&self, _sender: Option<&AnyObject>) {
            self.open_in_browser();
        }
        #[unsafe(method(togglePictureInPicture:))]
        fn action_toggle_pip(&self, _sender: Option<&AnyObject>) {
            self.toggle_picture_in_picture();
        }
        #[unsafe(method(signIn:))]
        fn action_sign_in(&self, _sender: Option<&AnyObject>) {
            self.clear_account_identity();
            let mode = self.ivars().mode.get();
            self.load_in(mode, &mode.sign_in_url());
        }
        #[unsafe(method(signOut:))]
        fn action_sign_out(&self, _sender: Option<&AnyObject>) {
            // YouTube's own sign-out ends on youtube.com, so run it in the YouTube pane.
            self.load_in(Mode::YouTube, SIGN_OUT_URL);
        }
        #[unsafe(method(refreshSignIn:))]
        fn action_refresh_sign_in(&self, _sender: Option<&AnyObject>) {
            self.refresh_sign_in_state();
        }
        #[unsafe(method(sidebarResized:))]
        fn action_sidebar_resized(&self, _notification: &NSNotification) {
            if let Some(table) = self.ivars().sidebar.get() {
                table.sizeLastColumnToFit();
            }
        }
        #[unsafe(method(hideNotice:))]
        fn action_hide_notice(&self, _sender: Option<&AnyObject>) {
            if let Some(notice) = self.ivars().notice.get() {
                notice.hide();
            }
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AppIvars {
            window: OnceCell::new(),
            panes: OnceCell::new(),
            mode: Cell::new(Mode::YouTube),
            sidebar_item: OnceCell::new(),
            fullscreen_mode: Cell::new(None),
            stage_shown: Cell::new(false),
            stage_leaving: Cell::new(false),
            fullscreen_id: Cell::new(0),
            zoom_generation: Cell::new(0),
            stage_start: Cell::new(NSRect::ZERO),
            stage_player: Cell::new(NSRect::ZERO),
            saved_presentation: Cell::new(None),
            stage: OnceCell::new(),
            container: OnceCell::new(),
            backdrop: OnceCell::new(),
            native_fullscreen: Cell::new(false),
            sidebar: OnceCell::new(),
            search: OnceCell::new(),
            mode_control: OnceCell::new(),
            back_item: OnceCell::new(),
            forward_item: OnceCell::new(),
            home_item: OnceCell::new(),
            mode_item: OnceCell::new(),
            save_item: OnceCell::new(),
            pip_item: OnceCell::new(),
            account_item: OnceCell::new(),
            placeholder: OnceCell::new(),
            notice: OnceCell::new(),
            signed_in: Cell::new(None),
            account_identity: RefCell::new(None),
            account_photo: RefCell::new(None),
            account_lookup_pending: Cell::new(false),
            account_lookup_attempted: Cell::new(false),
            syncing_selection: Cell::new(false),
            app_hidden: Cell::new(false),
            window_minimized: Cell::new(false),
            window_closed: Cell::new(false),
            queue: RefCell::new(load_state()),
        });
        unsafe { msg_send![super(this), init] }
    }

    // MARK: Window construction

    fn build_window(&self) {
        let mtm = self.mtm();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(1180., 760.)),
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable
                    | NSWindowStyleMask::FullSizeContentView,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(ns_string!("DooTube"));
        // The page already shows its own title; the toolbar keeps only controls. The title still
        // names the window in the Window menu, Mission Control, and accessibility.
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
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
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        content.addSubview(&backdrop);
        let container = NSView::new(mtm);
        container.setTranslatesAutoresizingMaskIntoConstraints(false);
        let safe = content.safeAreaLayoutGuide();
        content.addSubview(&container);
        ui::activate(&[
            container
                .topAnchor()
                .constraintEqualToAnchor(&safe.topAnchor()),
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
        let _ = self.ivars().container.set(container.clone());
        let _ = self.ivars().backdrop.set(backdrop);
        content.addSubview(&placeholder.root);
        content.addSubview(&notice.root);
        ui::activate(&[
            placeholder
                .root
                .centerXAnchor()
                .constraintEqualToAnchor(&safe.centerXAnchor()),
            placeholder
                .root
                .centerYAnchor()
                .constraintEqualToAnchor(&safe.centerYAnchor()),
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
        let content_controller = NSViewController::new(mtm);
        content_controller.setView(&content);

        let sidebar = self.build_sidebar();
        let sidebar_controller = NSViewController::new(mtm);
        sidebar_controller.setView(&sidebar);

        let sidebar_item = NSSplitViewItem::sidebarWithViewController(&sidebar_controller);
        sidebar_item.setMinimumThickness(180.);
        sidebar_item.setMaximumThickness(320.);
        let content_item = NSSplitViewItem::splitViewItemWithViewController(&content_controller);
        let split = NSSplitViewController::new(mtm);
        split.addSplitViewItem(&sidebar_item);
        split.addSplitViewItem(&content_item);
        self.ivars()
            .sidebar_item
            .set(sidebar_item)
            .expect("sidebar item initialized once");
        split
            .splitView()
            .setAutosaveName(Some(ns_string!("DooTubeSidebar")));
        window.setContentViewController(Some(&split));

        self.build_toolbar_items();
        let toolbar =
            NSToolbar::initWithIdentifier(NSToolbar::alloc(mtm), ns_string!("DooTubeMain"));
        toolbar.setDelegate(Some(ProtocolObject::from_ref(self)));
        toolbar.setAllowsUserCustomization(false);
        toolbar.setDisplayMode(NSToolbarDisplayMode::IconOnly);
        toolbar.setCenteredItemIdentifiers(&NSSet::from_slice(&[&*ns(MODE_ID)]));
        window.setToolbar(Some(&toolbar));
        window.setToolbarStyle(NSWindowToolbarStyle::Unified);

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

    fn build_panes(&self) -> [Pane; 2] {
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

    fn build_sidebar(&self) -> Retained<NSScrollView> {
        let mtm = self.mtm();
        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(220., 700.));
        let table = NSTableView::initWithFrame(NSTableView::alloc(mtm), frame);
        table.setStyle(NSTableViewStyle::SourceList);
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
        scroll
    }

    fn build_toolbar_items(&self) {
        let mtm = self.mtm();
        let ivars = self.ivars();
        let button = |id: &str, label: &str, symbol: &str, action| {
            let item = NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), &ns(id));
            item.setLabel(&ns(label));
            item.setToolTip(Some(&ns(label)));
            item.setImage(ui::symbol(symbol, label).as_deref());
            item.setBordered(true);
            item.setAutovalidates(false);
            unsafe {
                item.setTarget(Some(self));
                item.setAction(Some(action));
            }
            item
        };
        let navigational = |item: Retained<NSToolbarItem>| {
            // Placed by AppKit at the leading edge, after the sidebar, like Finder and Safari.
            item.setNavigational(true);
            item
        };
        ivars
            .back_item
            .set(navigational(button(
                BACK_ID,
                "Back",
                "chevron.left",
                sel!(goBack:),
            )))
            .ok();
        ivars
            .forward_item
            .set(navigational(button(
                FORWARD_ID,
                "Forward",
                "chevron.right",
                sel!(goForward:),
            )))
            .ok();
        ivars
            .home_item
            .set(navigational(button(HOME_ID, "Home", "house", sel!(goHome:))))
            .ok();
        ivars
            .save_item
            .set(button(
                SAVE_ID,
                "Save to Up Next",
                "text.badge.plus",
                sel!(saveCurrentVideo:),
            ))
            .ok();
        ivars
            .pip_item
            .set(button(
                PIP_ID,
                "Picture in Picture",
                "pip.enter",
                sel!(togglePictureInPicture:),
            ))
            .ok();

        let labels = NSArray::from_retained_slice(&[ns("YouTube"), ns("Music")]);
        let control = unsafe {
            NSSegmentedControl::segmentedControlWithLabels_trackingMode_target_action(
                &labels,
                NSSegmentSwitchTracking::SelectOne,
                Some(self),
                Some(sel!(modeChanged:)),
                mtm,
            )
        };
        control.setSelectedSegment(0);
        let mode_item =
            NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), &ns(MODE_ID));
        mode_item.setLabel(ns_string!("Section"));
        mode_item.setView(Some(&control));
        ivars.mode_item.set(mode_item).ok();
        ivars.mode_control.set(control).ok();

        let account = NSMenuToolbarItem::initWithItemIdentifier(
            NSMenuToolbarItem::alloc(mtm),
            &ns(ACCOUNT_ID),
        );
        account.setLabel(ns_string!("Account"));
        account.setShowsIndicator(false);
        ivars.account_item.set(account).ok();

        let search = NSSearchToolbarItem::initWithItemIdentifier(
            NSSearchToolbarItem::alloc(mtm),
            &ns(SEARCH_ID),
        );
        search.setPreferredWidthForSearchField(260.);
        let field = search.searchField();
        field.setSendsWholeSearchString(true);
        unsafe {
            field.setTarget(Some(self));
            field.setAction(Some(sel!(searchSubmitted:)));
        }
        ivars.search.set(search).ok();
    }

    fn build_menus(&self) {
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
                    item("Show Sidebar", sel!(toggleSidebar:), "s"),
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

    // MARK: Panes and sections

    fn pane(&self, mode: Mode) -> Option<&Pane> {
        self.ivars().panes.get().map(|panes| &panes[mode.index()])
    }

    fn active(&self) -> Option<&Pane> {
        self.pane(self.ivars().mode.get())
    }

    fn mode_of(&self, web: &WKWebView) -> Option<Mode> {
        Mode::ALL.into_iter().find(|mode| {
            self.pane(*mode)
                .is_some_and(|pane| std::ptr::eq(&*pane.web, web))
        })
    }

    fn pane_of(&self, web: &WKWebView) -> Option<&Pane> {
        self.pane(self.mode_of(web)?)
    }

    fn is_active(&self, web: &WKWebView) -> bool {
        self.mode_of(web) == Some(self.ivars().mode.get())
    }

    /// Shows a section, opening its home page the first time it is shown.
    fn switch_mode(&self, mode: Mode) {
        if mode != self.ivars().mode.get() && self.ivars().fullscreen_mode.get().is_some() {
            self.exit_video_fullscreen(false);
        }
        let ivars = self.ivars();
        ivars.mode.set(mode);
        if let Some(control) = ivars.mode_control.get() {
            control.setSelectedSegment(mode.index() as NSInteger);
        }
        if let Some(search) = ivars.search.get() {
            let field = search.searchField();
            field.setPlaceholderString(Some(&ns(mode.search_placeholder())));
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
        } else if self.pane(mode).is_some_and(|pane| pane.reload_when_shown.get()) {
            self.reload_in(mode);
        }
        self.show_active_pane();
        self.reload_sidebar();
        self.update_window_title();
        self.update_backdrop();
    }

    /// Matches the content background, and with it the strip behind the toolbar, to the active page.
    fn update_backdrop(&self) {
        if let (Some(backdrop), Some(pane)) = (self.ivars().backdrop.get(), self.active()) {
            let color = unsafe { pane.web.underPageBackgroundColor() };
            backdrop.setFillColor(&color);
        }
    }

    /// Shows the active pane's page, or its loading/failure state, and hides the other pane.
    fn show_active_pane(&self) {
        let active_mode = self.ivars().mode.get();
        for mode in Mode::ALL {
            if let Some(pane) = self.pane(mode) {
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
            placeholder.show_loading();
        }
    }

    /// Pauses the other section when one starts making sound, so only one plays at a time.
    fn on_audible_playback(&self, web: &WKWebView, title: String) {
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
    fn toggle_picture_in_picture(&self) {
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

    // MARK: Sidebar

    fn with_state<T>(&self, read: impl FnOnce(&AppState) -> T) -> Option<T> {
        self.ivars().queue.borrow().as_ref().ok().map(read)
    }

    fn rows(&self) -> Vec<Row> {
        let destinations = self.ivars().mode.get().destinations().len();
        let mut rows: Vec<Row> = (0..destinations).map(Row::Destination).collect();
        rows.push(Row::UpNextHeader);
        let count = self.with_state(|state| state.up_next.len()).unwrap_or(0);
        if count == 0 {
            rows.push(Row::EmptyHint);
        } else {
            rows.extend((0..count).map(Row::Saved));
        }
        rows
    }

    fn row_at(&self, row: NSInteger) -> Option<Row> {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.rows().get(row).copied())
    }

    fn sidebar_view(&self, row: NSInteger) -> Option<Retained<NSView>> {
        let mtm = self.mtm();
        Some(match self.row_at(row)? {
            Row::Destination(index) => {
                let destination = &self.ivars().mode.get().destinations()[index];
                ui::sidebar_cell(mtm, destination.title, destination.symbol, None)
            }
            Row::UpNextHeader => ui::header_cell(mtm, "Up Next"),
            Row::Saved(index) => {
                let (label, music) = self.with_state(|state| {
                    state.up_next.get(index).map(|item| {
                        let music = Url::parse(&item.url).is_ok_and(|url| is_music_url(&url));
                        (item.label(), music)
                    })
                })??;
                let symbol = if music {
                    "music.note"
                } else {
                    "play.rectangle"
                };
                ui::sidebar_cell(mtm, &label, symbol, Some(&label))
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

    fn row_index(&self, target: Row) -> NSInteger {
        self.rows()
            .iter()
            .position(|row| *row == target)
            .map_or(-1, |index| index as NSInteger)
    }

    fn open_row(&self, row: NSInteger) {
        let url = match self.row_at(row) {
            Some(Row::Destination(index)) => {
                self.ivars().mode.get().destinations()[index].url.to_owned()
            }
            Some(Row::Saved(index)) => {
                match self.with_state(|state| state.up_next.get(index).map(|item| item.url.clone()))
                {
                    Some(Some(url)) => url,
                    _ => return,
                }
            }
            _ => return,
        };
        self.load_url(&url);
    }

    fn reload_sidebar(&self) {
        if let Some(table) = self.ivars().sidebar.get() {
            self.ivars().syncing_selection.set(true);
            table.reloadData();
            self.ivars().syncing_selection.set(false);
        }
        self.sync_sidebar_selection();
        self.update_toolbar();
    }

    /// The current section's destination for a page, if any.
    fn destination_for(&self, url: &Url) -> Option<usize> {
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
    fn sync_sidebar_selection(&self) {
        let Some(table) = self.ivars().sidebar.get() else {
            return;
        };
        let target = self.current_url().and_then(|url| {
            if video_id(&url).is_some()
                && let Some(index) = self.with_state(|state| state.position(&url)).flatten()
            {
                return Some(Row::Saved(index));
            }
            self.destination_for(&url).map(Row::Destination)
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
    fn target_saved_index(&self, sender: Option<&AnyObject>) -> Option<usize> {
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

    fn validate(&self, item: &NSMenuItem) -> bool {
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

    fn move_saved(&self, sender: Option<&AnyObject>, delta: isize) {
        let Some(index) = self.target_saved_index(sender) else {
            return;
        };
        let Some(target) = index.checked_add_signed(delta) else {
            return;
        };
        self.change_queue(|state| state.move_item(index, target));
    }

    // MARK: Queue

    /// Applies a queue edit, persisting before committing it in memory.
    fn change_queue(&self, update: impl FnOnce(&mut AppState) -> bool) -> bool {
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

    /// `Some(saved?)` when the page is a video; `None` otherwise or when the queue is unreadable.
    fn save_state_for_current(&self) -> Option<bool> {
        let url = self.current_video_url()?;
        self.with_state(|state| state.position(&url).is_some())
    }

    fn save_current_video(&self) {
        let Some(url) = self.current_video_url() else {
            return;
        };
        let title = self.current_title_for(&url);
        self.change_queue(|state| state.add(&url, title));
    }

    /// Display title of the open video, only once the page has caught up with it.
    fn current_title_for(&self, url: &Url) -> Option<String> {
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

    // MARK: Page state

    fn current_url(&self) -> Option<Url> {
        let text = unsafe { self.active()?.web.URL() }?
            .absoluteString()?
            .to_string();
        Url::parse(&text).ok()
    }

    fn current_video_url(&self) -> Option<Url> {
        match parse_input(self.current_url()?.as_str()) {
            Ok(Navigation::Video(url)) => Some(url),
            _ => None,
        }
    }

    fn on_page_url_changed(&self) {
        let mode = self.ivars().mode.get();
        // Show the open results page's query; clear a stale one elsewhere unless it's being typed.
        if let Some(search) = self.ivars().search.get() {
            let field = search.searchField();
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

    fn on_page_title_changed(&self, web: &WKWebView) {
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

    fn update_window_title(&self) {
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

    fn update_toolbar(&self) {
        let ivars = self.ivars();
        let web = self.active().map(|pane| &pane.web);
        if let Some(item) = ivars.back_item.get() {
            item.setEnabled(web.is_some_and(|web| unsafe { web.canGoBack() }));
        }
        if let Some(item) = ivars.forward_item.get() {
            item.setEnabled(web.is_some_and(|web| unsafe { web.canGoForward() }));
        }
        if let Some(item) = ivars.save_item.get() {
            let saved = self.save_state_for_current();
            let (label, symbol) = if saved == Some(true) {
                ("Saved in Up Next", "text.badge.checkmark")
            } else {
                ("Save to Up Next", "text.badge.plus")
            };
            item.setEnabled(saved == Some(false));
            item.setLabel(&ns(label));
            item.setToolTip(Some(&ns(label)));
            item.setImage(ui::symbol(symbol, label).as_deref());
        }
    }

    // MARK: Account

    /// Reads the shared cookie store: YouTube sets `LOGIN_INFO`/`SAPISID` for a signed-in account.
    fn refresh_sign_in_state(&self) {
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

    fn set_signed_in(&self, signed_in: bool) {
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
    fn on_account_photo(&self, url: &str, bytes: Vec<u8>) {
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

    fn clear_account_identity(&self) {
        *self.ivars().account_identity.borrow_mut() = None;
        *self.ivars().account_photo.borrow_mut() = None;
        self.ivars().account_lookup_pending.set(false);
        self.ivars().account_lookup_attempted.set(false);
        self.update_account_ui();
    }

    fn maybe_fetch_account_identity(&self) {
        if let Some(pane) = self.pane(Mode::YouTube) {
            self.maybe_fetch_account_identity_in(&pane.web);
        }
    }

    fn maybe_fetch_account_identity_in(&self, web: &WKWebView) {
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

    fn update_account_ui(&self) {
        let mtm = self.mtm();
        let Some(item) = self.ivars().account_item.get() else {
            return;
        };
        let signed_in = self.ivars().signed_in.get() == Some(true);
        let identity = self.ivars().account_identity.borrow();
        let display_identity = identity.as_ref().filter(|_| signed_in);
        let (status, symbol) = if let Some(identity) = display_identity {
            let status = identity
                .email
                .as_deref()
                .or(identity.handle.as_deref())
                .map_or_else(
                    || format!("Signed in as {}", identity.name),
                    |contact| format!("Signed in as {} ({contact})", identity.name),
                );
            (status, "person.crop.circle.fill")
        } else if signed_in {
            ("Signed in to YouTube".to_owned(), "person.crop.circle.fill")
        } else {
            ("Not signed in".to_owned(), "person.crop.circle")
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
        item.setMenu(&menu);
        let toolbar_photo = photo.as_deref().map(|photo| ui::circular_image(photo, 20.));
        item.setImage(
            toolbar_photo
                .or_else(|| ui::symbol(symbol, &status))
                .as_deref(),
        );
        item.setToolTip(Some(&ns(&status)));
    }

    // MARK: Navigation

    fn navigation_policy(
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
    fn navigate(&self, text: &str) -> bool {
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
    fn load_url(&self, url: &str) {
        let mode = Url::parse(url).map_or(Mode::YouTube, |url| Mode::for_url(&url));
        self.load_in(mode, url);
    }

    fn load_in(&self, mode: Mode, url: &str) {
        let Some(pane) = self.pane(mode) else {
            return;
        };
        let Some(nsurl) = NSURL::URLWithString(&ns(url)) else {
            self.show_notice("Couldn't open that YouTube link.");
            return;
        };
        *pane.last_requested.borrow_mut() = Some(url.to_owned());
        // This load replaces any pending reload of a page whose web process ended.
        pane.reload_when_shown.set(false);
        if self.ivars().mode.get() != mode {
            self.switch_mode(mode);
        }
        unsafe { pane.web.loadRequest(&NSURLRequest::requestWithURL(&nsurl)) };
        self.show_active_pane();
    }

    fn reload_page(&self) {
        self.reload_in(self.ivars().mode.get());
    }

    fn reload_in(&self, mode: Mode) {
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
    fn on_web_process_ended(&self, web: &WKWebView) {
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

    fn on_load_error(&self, web: &WKWebView, error: &NSError) {
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

    fn open_in_browser(&self) {
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

    // MARK: Notices

    fn show_notice(&self, text: &str) {
        self.present_notice(text, Some(5.0));
    }

    fn show_sticky_notice(&self, text: &str) {
        self.present_notice(text, None);
    }

    fn present_notice(&self, text: &str, hide_after: Option<f64>) {
        let Some(notice) = self.ivars().notice.get() else {
            return;
        };
        unsafe {
            let _: () = msg_send![
                <NSObject as ClassType>::class(),
                cancelPreviousPerformRequestsWithTarget: self,
                selector: sel!(hideNotice:),
                object: Option::<&AnyObject>::None
            ];
        }
        notice.show(text);
        if let Some(delay) = hide_after {
            unsafe {
                let _: () = msg_send![
                    self,
                    performSelector: sel!(hideNotice:),
                    withObject: Option::<&AnyObject>::None,
                    afterDelay: delay
                ];
            }
        }
    }

    // MARK: Lifecycle

    /// Pauses and suspends media while the window is away, except a video in Picture in Picture,
    /// which stays visible in its own floating window.
    fn pause_web(&self) {
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

    fn resume_web_controls(&self) {
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
    fn on_video_fullscreen_message(
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
    fn snapshot(
        &self,
        web: &WKWebView,
        rect: NSRect,
        shown_width: f64,
        then: impl Fn(Option<&NSImage>) + 'static,
    ) {
        let scale = web.window().map_or(1., |window| window.backingScaleFactor());
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

    fn prepare_stage(&self, mode: Mode, rect: PageRect, id: u64, image: Option<&NSImage>) {
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

    fn zoom_onto_stage(&self, mode: Mode, id: u64) {
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
    fn resume_stage(&self, mode: Mode) {
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

    fn run_zoom(&self, mode: Mode, zoom_in: bool, from: Option<CATransform3D>) {
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
    fn finish_after_zoom_out(&self, mode: Mode, generation: u64) {
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
    fn exit_video_fullscreen(&self, animated: bool) {
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

    fn finish_stage(&self, mode: Mode) {
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
    fn run_js_after_paint(&self, web: &WKWebView, script: &str, then: impl Fn() + 'static) {
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

    fn stage(&self) -> &Stage {
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
    fn exit_video_fullscreen_from_external_transition(&self) {
        self.exit_video_fullscreen(false);
    }

    fn on_window_close(&self) {
        self.exit_video_fullscreen(false);
        self.ivars().window_closed.set(true);
        self.pause_web();
    }

    fn on_window_reopen(&self) {
        self.ivars().window_closed.set(false);
        self.ivars().app_hidden.set(false);
        if let Some(window) = self.ivars().window.get() {
            window.makeKeyAndOrderFront(None);
        }
        self.resume_web_controls();
    }

    fn on_window_minimize(&self) {
        self.exit_video_fullscreen(false);
        self.ivars().window_minimized.set(true);
        self.pause_web();
    }

    fn on_window_restore(&self) {
        self.ivars().window_minimized.set(false);
        self.resume_web_controls();
    }

    fn on_application_hide(&self) {
        self.ivars().app_hidden.set(true);
        self.pause_web();
    }

    fn on_application_unhide(&self) {
        self.ivars().app_hidden.set(false);
        self.resume_web_controls();
    }

    /// Keep transient fullscreen geometry from replacing the user's saved window frame.
    fn prepare_window_fullscreen(&self) {
        if let Some(window) = self.ivars().window.get() {
            window.setFrameAutosaveName(ns_string!("DooTubeMainWindow.FullScreen"));
        }
    }

    fn finish_window_fullscreen(&self) {
        if let Some(window) = self.ivars().window.get() {
            window.setFrameAutosaveName(ns_string!("DooTubeMainWindow"));
        }
    }
}

fn toolbar_identifiers() -> Retained<NSArray<NSString>> {
    let custom = [
        BACK_ID, FORWARD_ID, HOME_ID, MODE_ID, SEARCH_ID, PIP_ID, SAVE_ID, ACCOUNT_ID,
    ]
    .map(ns);
    let [back, forward, home, mode, search, pip, save, account] = &custom;
    let ids: [&NSString; 12] = unsafe {
        [
            NSToolbarToggleSidebarItemIdentifier,
            NSToolbarSidebarTrackingSeparatorItemIdentifier,
            back,
            forward,
            home,
            NSToolbarFlexibleSpaceItemIdentifier,
            mode,
            NSToolbarFlexibleSpaceItemIdentifier,
            search,
            pip,
            save,
            account,
        ]
    };
    NSArray::from_slice(&ids)
}

/// Downloads an account photo off the main thread, then hands it to the app delegate.
fn load_account_photo(url: String) {
    // ponytail: one small image per sign-in, so a plain blocking fetch on a worker thread.
    std::thread::spawn(move || {
        let Some(bytes) = NSURL::URLWithString(&ns(&url))
            .and_then(|target| NSData::dataWithContentsOfURL(&target))
            .map(|data| data.to_vec())
        else {
            return;
        };
        DispatchQueue::main().exec_async(move || {
            let mtm = MainThreadMarker::new().expect("main queue runs on the main thread");
            let delegate = NSApplication::sharedApplication(mtm).delegate();
            if let Some(this) = delegate.as_deref().and_then(|delegate| {
                AsRef::<AnyObject>::as_ref(delegate).downcast_ref::<AppDelegate>()
            }) {
                this.on_account_photo(&url, bytes);
            }
        });
    });
}

fn main() {
    let mtm = MainThreadMarker::new().expect("AppKit requires main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
}
