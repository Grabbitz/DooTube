#![deny(unsafe_op_in_unsafe_fn)]
mod account;
mod cinema_bar;
mod core;
mod google_domains;
mod lifecycle;
mod navigation;
mod notices;
mod page_state;
mod panes;
mod queue;
mod sidebar;
mod suggestions;
mod ui;
mod web;
mod window;

use block2::RcBlock;
use core::{
    AppState, Navigation, clean_page_title, is_google_sign_in_url, is_music_url,
    is_supported_youtube_url, load_state, parse_input, resume_point, save_state, video_id,
};
use dispatch2::DispatchQueue;
use objc2::{
    AllocAnyThread, ClassType, DefinedClass, MainThreadOnly, Message, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSApplication,
    NSApplicationActivationPolicy, NSApplicationDelegate, NSApplicationPresentationOptions,
    NSAutoresizingMaskOptions, NSBackingStoreType, NSBox, NSBoxType, NSButton, NSColor, NSControl,
    NSControlStateValueOff, NSControlStateValueOn, NSControlTextEditingDelegate, NSDragOperation,
    NSDraggingInfo, NSEventModifierFlags, NSEventType, NSFont, NSImage, NSImageScaling,
    NSImageView, NSMenu, NSMenuItem, NSPasteboardItem, NSPasteboardTypeURL, NSPasteboardWriting,
    NSResponder, NSScrollView, NSSearchFieldDelegate, NSStackView, NSTableColumn,
    NSTableColumnResizingOptions, NSTableView, NSTableViewColumnAutoresizingStyle,
    NSTableViewDataSource, NSTableViewDelegate, NSTableViewDropOperation, NSTableViewStyle, NSText,
    NSTextField, NSTextFieldDelegate, NSTitlePosition, NSToolbar, NSUserInterfaceLayoutOrientation,
    NSView, NSViewFrameDidChangeNotification, NSWindow, NSWindowCollectionBehavior,
    NSWindowDelegate, NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility,
    NSWindowToolbarStyle, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSData, NSError, NSHTTPCookie, NSIndexSet, NSInteger,
    NSMutableIndexSet, NSNotification, NSNotificationCenter, NSNumber, NSObject, NSObjectProtocol,
    NSPoint, NSRect, NSSize, NSString, NSURL, NSURLRequest, NSValue, ns_string,
};
use objc2_quartz_core::{
    CABasicAnimation, CAMediaTiming, CAMediaTimingFunction, CATransaction, CATransform3D,
    CATransform3DIdentity, NSValueCATransform3DAdditions, kCAMediaTimingFunctionEaseInEaseOut,
    kCAMediaTimingFunctionEaseOut,
};
use objc2_web_kit::{
    WKAudiovisualMediaTypes, WKContentWorld, WKHTTPCookieStore, WKHTTPCookieStoreObserver,
    WKNavigation, WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate,
    WKNavigationType, WKScriptMessage, WKScriptMessageHandler, WKSnapshotConfiguration,
    WKUIDelegate, WKUserContentController, WKUserScript, WKUserScriptInjectionTime, WKWebView,
    WKWebViewConfiguration, WKWebsiteDataStore, WKWindowFeatures,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    ffi::c_void,
    ptr::NonNull,
    time::{Duration, Instant},
};
use ui::ns;
use url::Url;
use web::{Mode, PageMessage, PageRect, Pane};

const SIGN_OUT_URL: &str = "https://www.youtube.com/logout";
/// Pasteboard type carrying the Up Next index of a row being dragged within the sidebar.
const UP_NEXT_DRAG_TYPE: &str = "com.dootube.up-next-index";
/// Tag for sidebar context-menu items, which act on the clicked row rather than the selection.
const CONTEXT_TAG: NSInteger = 1;
const OBSERVED_WEB_KEYS: [&str; 5] = [
    "URL",
    "title",
    "canGoBack",
    "canGoForward",
    "underPageBackgroundColor",
];
/// The HTTPS page URL of the main frame that posted `message`; `None` for subframes.
fn main_frame_url(message: &WKScriptMessage) -> Option<Url> {
    let frame = unsafe { message.frameInfo() };
    if !unsafe { frame.isMainFrame() } {
        return None;
    }
    let request = unsafe { frame.request() };
    request
        .URL()
        .and_then(|url| url.absoluteString())
        .and_then(|text| Url::parse(&text.to_string()).ok())
        .filter(|url| url.scheme() == "https")
}
fn is_youtube_main_frame(message: &WKScriptMessage) -> bool {
    main_frame_url(message).is_some_and(|url| url.host_str() == Some("www.youtube.com"))
}
fn is_music_main_frame(message: &WKScriptMessage) -> bool {
    main_frame_url(message).is_some_and(|url| url.host_str() == Some("music.youtube.com"))
}
fn is_video_fullscreen_main_frame(message: &WKScriptMessage) -> bool {
    main_frame_url(message).is_some_and(|url| {
        matches!(
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
    bar: OnceCell<cinema_bar::CinemaBar>,
    suggestions: OnceCell<suggestions::Suggestions>,
    queue_panel: OnceCell<Retained<NSBox>>,
    queue_scrim: OnceCell<Retained<NSBox>>,
    account_menu: RefCell<Option<Retained<NSMenu>>>,
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
    /// Matches the current content while the Cinema bar keeps its own dark palette.
    backdrop: OnceCell<Retained<NSBox>>,
    native_fullscreen: Cell<bool>,
    sidebar: OnceCell<Retained<NSTableView>>,
    placeholder: OnceCell<ui::Placeholder>,
    notice: OnceCell<ui::Notice>,
    /// `None` until the cookie store has been read once.
    signed_in: Cell<Option<bool>>,
    account_identity: RefCell<Option<web::AccountIdentity>>,
    /// The active account's photo, once downloaded.
    account_photo: RefCell<Option<Retained<NSImage>>>,
    /// YouTube Music's player-bar track; an empty title means nothing is loaded.
    track: RefCell<web::Track>,
    /// Downloaded artwork for `track`, with the URL it came from.
    artwork: RefCell<Option<(String, Retained<NSImage>)>>,
    now_playing: OnceCell<ui::NowPlayingCard>,
    /// Up Next thumbnails by URL, kept for the session; `None` while loading or after a failed
    /// download, so each is fetched at most once.
    thumbnails: RefCell<HashMap<String, Option<Retained<NSImage>>>>,
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
    /// Window that doesn't beep for keys a web page already acted on.
    #[unsafe(super(NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    struct QuietWindow;

    impl QuietWindow {
        // WebKit hands every key the page didn't cancel back up the responder chain; YouTube acts
        // on T, F, and other player keys without cancelling them, so AppKit would beep for each.
        #[unsafe(method(noResponderFor:))]
        fn no_responder_for(&self, selector: Sel) {
            let mut view = self
                .firstResponder()
                .and_then(|responder| responder.downcast::<NSView>().ok());
            while let Some(current) = view {
                if current.isKindOfClass(WKWebView::class()) {
                    if selector == sel!(keyDown:) {
                        return;
                    }
                    break;
                }
                view = unsafe { current.superview() };
            }
            unsafe { msg_send![super(self), noResponderFor: selector] }
        }
    }
);

define_class!(
    /// Borderless screen-sized window that shows only the video during page fullscreen.
    #[unsafe(super(QuietWindow, NSWindow, NSResponder, NSObject))]
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
        #[unsafe(method_id(applicationDockMenu:))]
        fn dock_menu(&self, _app: &NSApplication) -> Option<Retained<NSMenu>> {
            self.music_dock_menu()
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
            if self.mode_of(&web).is_none() {
                return;
            }
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
                        load_image(url, AppDelegate::on_account_photo);
                    }
                }
                Some(PageMessage::Progress { url, seconds, duration })
                    if self.mode_of(&web) == Some(Mode::YouTube) && is_youtube_main_frame(message) =>
                {
                    self.on_video_progress(&url, seconds, duration);
                }
                Some(PageMessage::NowPlaying(track))
                    if self.mode_of(&web) == Some(Mode::Music) && is_music_main_frame(message) =>
                {
                    self.on_now_playing(track);
                }
                Some(PageMessage::AccountIdentity { .. })
                | Some(PageMessage::Fullscreen { .. })
                | Some(PageMessage::NowPlaying(_))
                | Some(PageMessage::Progress { .. })
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


    unsafe impl NSTableViewDataSource for AppDelegate {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.with_state(|state| state.up_next.len()).unwrap_or(0).max(1) as NSInteger
        }
        #[unsafe(method_id(tableView:pasteboardWriterForRow:))]
        fn pasteboard_writer(
            &self,
            _table: &NSTableView,
            row: NSInteger,
        ) -> Option<Retained<ProtocolObject<dyn NSPasteboardWriting>>> {
            self.saved_row_pasteboard_item(row)
                .map(ProtocolObject::from_retained)
        }
        #[unsafe(method(tableView:validateDrop:proposedRow:proposedDropOperation:))]
        fn validate_drop(
            &self,
            table: &NSTableView,
            info: &ProtocolObject<dyn NSDraggingInfo>,
            row: NSInteger,
            _operation: NSTableViewDropOperation,
        ) -> NSDragOperation {
            match self.drop_target(table, info, row) {
                Some(_) => {
                    // Always a gap between rows, never onto a row.
                    table.setDropRow_dropOperation(row, NSTableViewDropOperation::Above);
                    NSDragOperation::Move
                }
                None => NSDragOperation::None,
            }
        }
        #[unsafe(method(tableView:acceptDrop:row:dropOperation:))]
        fn accept_drop(
            &self,
            table: &NSTableView,
            info: &ProtocolObject<dyn NSDraggingInfo>,
            row: NSInteger,
            _operation: NSTableViewDropOperation,
        ) -> bool {
            match self.drop_target(table, info, row) {
                Some((from, to)) => self.change_queue(|state| state.move_item(from, to)),
                None => false,
            }
        }
    }

    unsafe impl NSControlTextEditingDelegate for AppDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, _notification: &NSNotification) {
            if let Some(bar) = self.ivars().bar.get() {
                self.search_text_changed(bar.search.stringValue().to_string());
            }
        }
        #[unsafe(method(controlTextDidEndEditing:))]
        fn control_text_did_end_editing(&self, _notification: &NSNotification) {
            self.hide_suggestions();
        }
        // NSTextView in AppKit's declaration; only its pointer passes through here.
        #[unsafe(method(control:textView:doCommandBySelector:))]
        fn control_do_command(&self, _control: &NSControl, _text_view: &NSText, command: Sel) -> bool {
            self.search_command(command)
        }
    }

    unsafe impl NSTextFieldDelegate for AppDelegate {}

    unsafe impl NSSearchFieldDelegate for AppDelegate {}

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
        #[unsafe(method(tableView:heightOfRow:))]
        fn height_of_row(&self, table: &NSTableView, row: NSInteger) -> f64 {
            match self.row_at(row) {
                Some(Row::Saved(_)) => ui::SAVED_ROW_HEIGHT,
                _ => table.rowHeight(),
            }
        }
        #[unsafe(method(tableView:shouldSelectRow:))]
        fn should_select_row(&self, _table: &NSTableView, row: NSInteger) -> bool {
            matches!(self.row_at(row), Some(Row::Saved(_)))
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
            self.hide_suggestions();
            if text.trim().is_empty() {
                return;
            }
            if self.navigate(&text)
                && let (Some(window), Some(pane)) = (self.ivars().window.get(), self.active())
            {
                window.makeFirstResponder(Some(&pane.web));
            }
        }
        #[unsafe(method(suggestionClicked:))]
        fn action_suggestion(&self, sender: &NSButton) {
            self.open_suggestion(sender.tag() as usize);
        }
        #[unsafe(method(focusSearch:))]
        fn action_focus_search(&self, _sender: Option<&AnyObject>) {
            if let (Some(bar), Some(window)) = (self.ivars().bar.get(), self.ivars().window.get()) {
                window.makeFirstResponder(Some(&bar.search));
                unsafe { bar.search.selectText(None) };
            }
        }
        #[unsafe(method(showYouTube:))]
        fn action_show_youtube(&self, _sender: Option<&AnyObject>) {
            self.switch_mode(Mode::YouTube);
        }
        #[unsafe(method(showMusic:))]
        fn action_show_music(&self, _sender: Option<&AnyObject>) {
            self.switch_mode(Mode::Music);
        }
        #[unsafe(method(resumeMusic:))]
        fn action_resume_music(&self, _sender: Option<&AnyObject>) {
            self.switch_mode(Mode::Music);
        }
        #[unsafe(method(toggleQueue:))]
        fn action_toggle_queue(&self, _sender: Option<&AnyObject>) {
            let visible = self.ivars().queue_panel.get().is_some_and(|panel| !panel.isHidden());
            self.set_queue_visible(!visible);
        }
        #[unsafe(method(showAccountMenu:))]
        fn action_account_menu(&self, _sender: Option<&AnyObject>) {
            let menu = self.ivars().account_menu.borrow().clone();
            if let (Some(menu), Some(bar)) = (menu, self.ivars().bar.get()) {
                menu.popUpMenuPositioningItem_atLocation_inView(
                    None, NSPoint::new(0., 0.), Some(&bar.account),
                );
            }
        }
        #[unsafe(method(musicPlayPause:))]
        fn action_music_play_pause(&self, _sender: Option<&AnyObject>) {
            self.music_control("playpause");
        }
        #[unsafe(method(speedUp:))]
        fn action_speed_up(&self, _sender: Option<&AnyObject>) {
            self.change_speed(0.25);
        }
        #[unsafe(method(slowDown:))]
        fn action_slow_down(&self, _sender: Option<&AnyObject>) {
            self.change_speed(-0.25);
        }
        #[unsafe(method(normalSpeed:))]
        fn action_normal_speed(&self, _sender: Option<&AnyObject>) {
            self.change_speed(0.);
        }
        #[unsafe(method(musicNext:))]
        fn action_music_next(&self, _sender: Option<&AnyObject>) {
            self.music_control("next");
        }
        #[unsafe(method(musicPrevious:))]
        fn action_music_previous(&self, _sender: Option<&AnyObject>) {
            self.music_control("previous");
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
            self.set_queue_visible(false);
            if let Some(pane) = self.active() {
                unsafe { pane.web.goBack() };
            }
            self.show_active_pane();
        }
        #[unsafe(method(goForward:))]
        fn action_forward(&self, _sender: Option<&AnyObject>) {
            self.set_queue_visible(false);
            if let Some(pane) = self.active() {
                unsafe { pane.web.goForward() };
            }
            self.show_active_pane();
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
            bar: OnceCell::new(),
            suggestions: OnceCell::new(),
            queue_panel: OnceCell::new(),
            queue_scrim: OnceCell::new(),
            account_menu: RefCell::new(None),
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
            placeholder: OnceCell::new(),
            notice: OnceCell::new(),
            signed_in: Cell::new(None),
            account_identity: RefCell::new(None),
            account_photo: RefCell::new(None),
            track: RefCell::new(web::Track::default()),
            artwork: RefCell::new(None),
            now_playing: OnceCell::new(),
            thumbnails: RefCell::new(HashMap::new()),
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
}

/// Downloads a small image (account photo or artwork) off the main thread, then hands it to
/// `done` on the app delegate.
fn load_image(url: String, done: fn(&AppDelegate, &str, Vec<u8>)) {
    // ponytail: one small image per account or track, so a plain blocking fetch on a worker thread.
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
                done(this, &url, bytes);
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
