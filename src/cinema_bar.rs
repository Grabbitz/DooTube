// Hallmark · component: toolbar · genre: modern-minimal · theme: project tokens (teal + amber)
// states: default · hover · focus (search) · active (AppKit) · disabled · selected
// pre-emit critique: P4 H4 E4 S4 R5 V4
use objc2::{AllocAnyThread, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSBox, NSBoxType, NSButton, NSColor, NSControl, NSEvent,
    NSFont, NSFontWeightMedium, NSFontWeightRegular, NSFontWeightSemibold, NSImage, NSImageScaling,
    NSImageSymbolConfiguration, NSLayoutConstraintOrientation, NSResponder, NSSearchField,
    NSShadow, NSStackView, NSTextAlignment, NSTextField, NSTrackingArea, NSTrackingAreaOptions,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSRect, NSSize};

use crate::{
    ui::{self, activate, circular_image, ns, symbol},
    web::Mode,
};

// The page keeps YouTube's own dark gray; the chrome adds the app icon's teal and amber.
pub fn background_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.059, 0.059, 0.059, 1.)
}

/// Floating surfaces: the Up Next panel, notices, and search suggestions.
pub fn raised_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.086, 0.098, 0.102, 1.)
}

/// Amber from the app icon: saved videos and the Up Next count.
pub fn warm_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.949, 0.722, 0.294, 1.)
}

/// One-point edge of capsules and floating surfaces.
pub fn hairline_color() -> Retained<NSColor> {
    NSColor::colorWithWhite_alpha(1., 0.1)
}

/// Resting fill of capsule controls over the page color.
pub fn glass_color() -> Retained<NSColor> {
    NSColor::colorWithWhite_alpha(1., 0.07)
}

/// Teal from the app icon, brightened for dark backgrounds, at `alpha`: capsule edges, their glow, and the selected section's fill.
fn accent_alpha(alpha: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.271, 0.878, 0.796, alpha)
}

/// Lit glass: a thin accent edge that bleeds a soft glow of the same color.
fn glow(view: &NSBox, edge: f64, halo: f64) {
    view.setBorderWidth(1.);
    view.setBorderColor(&accent_alpha(edge));
    let shadow = NSShadow::new();
    shadow.setShadowColor(Some(&accent_alpha(halo)));
    shadow.setShadowBlurRadius(9.);
    shadow.setShadowOffset(NSSize::new(0., 0.));
    view.setWantsLayer(true);
    view.setShadow(Some(&shadow));
}

/// Resting tint of toolbar glyphs: brighter than secondary text, quieter than a title.
fn icon_color() -> Retained<NSColor> {
    NSColor::colorWithWhite_alpha(1., 0.88)
}

fn ink_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.957527, 0.945432, 0.934476, 1.)
}

pub struct CinemaBar {
    pub root: Retained<NSBox>,
    pub back: Retained<NSButton>,
    pub forward: Retained<NSButton>,
    pub home: Retained<NSButton>,
    pub youtube: Retained<NSButton>,
    pub music: Retained<NSButton>,
    pub save: Retained<NSButton>,
    pub pip: Retained<NSButton>,
    pub queue: Retained<NSButton>,
    pub account: Retained<NSButton>,
    pub search: Retained<NSSearchField>,
    /// The capsule around `search`; the suggestions list hangs under it.
    pub search_frame: Retained<NSBox>,
    count: Retained<NSTextField>,
    youtube_pill: Retained<NSBox>,
    music_pill: Retained<NSBox>,
}

/// Height of the bar, which shares its row with the window's traffic lights.
pub const HEIGHT: f64 = 52.;
/// Strength of a capsule's accent edge and of the glow around it, at rest.
const EDGE: f64 = 0.3;
const HALO: f64 = 0.45;
/// Height of every capsule in the bar, and of the pills inside them.
const ISLAND_HEIGHT: f64 = 36.;
const PILL_HEIGHT: f64 = 30.;

impl CinemaBar {
    /// One row of equal-height capsules: the app mark, Back/Forward, and the YouTube/Music switch
    /// on the left; search centered; video and queue actions and the account on the right.
    /// Clear, so the page color shows through.
    pub fn new(mtm: MainThreadMarker) -> Self {
        let root = NSBox::new(mtm);
        root.setBoxType(NSBoxType::Custom);
        root.setBorderWidth(0.);
        root.setTitlePosition(objc2_app_kit::NSTitlePosition::NoTitle);
        root.setFillColor(&NSColor::clearColor());
        root.setContentViewMargins(NSSize::new(0., 0.));
        root.setTranslatesAutoresizingMaskIntoConstraints(false);

        // The app's own icon is the brand mark and the way home.
        let home = icon_button(mtm, "play.fill", "Home");
        if let Some(icon) = NSApplication::sharedApplication(mtm).applicationIconImage() {
            home.setImage(Some(&ui::rounded_image(&icon, 36., 0.)));
            home.setImageScaling(NSImageScaling::ScaleNone);
        }

        let back = icon_button(mtm, "chevron.left", "Back");
        let forward = icon_button(mtm, "chevron.right", "Forward");
        let nav = island(mtm, &[&hover_pill(mtm, &back), &hover_pill(mtm, &forward)]);

        // Segmented switch: a lighter pill sits behind the current section.
        let youtube = mode_button(mtm, "YouTube");
        let music = mode_button(mtm, "Music");
        let youtube_pill = mode_pill(mtm, &youtube);
        let music_pill = mode_pill(mtm, &music);
        let switch = island(mtm, &[&youtube_pill, &music_pill]);

        // The field draws nothing itself; the capsule around it is the visible control.
        let search = FocusSearchField::new(mtm).into_super();
        search.setPlaceholderString(Some(&ns("Search YouTube")));
        search.setToolTip(Some(&ns("Search YouTube")));
        search.setAccessibilityLabel(Some(&ns("Search YouTube")));
        search.setFont(Some(&NSFont::systemFontOfSize(14.5)));
        search.setBezeled(false);
        search.setDrawsBackground(false);
        search.setFocusRingType(objc2_app_kit::NSFocusRingType::None);
        search.setTranslatesAutoresizingMaskIntoConstraints(false);
        // Without its bezel the field draws its own magnifier over the text, so the capsule
        // carries the glyph instead.
        if let Some(cell) = search
            .cell()
            .and_then(|cell| cell.downcast::<objc2_app_kit::NSSearchFieldCell>().ok())
        {
            cell.setSearchButtonCell(None);
        }
        let glyph = objc2_app_kit::NSImageView::new(mtm);
        glyph.setImage(symbol("magnifyingglass", "Search").as_deref());
        glyph.setContentTintColor(Some(&accent_alpha(0.8)));
        glyph.setTranslatesAutoresizingMaskIntoConstraints(false);
        let search_frame = ui::surface(mtm, ISLAND_HEIGHT / 2., &glass_color());
        glow(&search_frame, EDGE, HALO);
        search_frame.addSubview(&glyph);
        search_frame.addSubview(&search);

        let save = icon_button(mtm, "bookmark", "Save video");
        let pip = icon_button(mtm, "pip.enter", "Picture in Picture");
        let queue = icon_button(mtm, "list.bullet", "Up Next");
        let count = NSTextField::labelWithString(&ns("0"), mtm);
        count.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
            13.,
            unsafe { NSFontWeightSemibold },
        )));
        count.setTextColor(Some(&warm_color()));
        count.setAlignment(NSTextAlignment::Center);
        count.setAccessibilityLabel(Some(&ns("Queued items")));
        let actions = island(
            mtm,
            &[
                &hover_pill(mtm, &save),
                &hover_pill(mtm, &pip),
                &hover_pill(mtm, &queue),
                &count,
            ],
        );

        let account = icon_button(mtm, "person.crop.circle", "Account");

        let left = horizontal(mtm, 10.);
        left.addArrangedSubview(&home);
        left.addArrangedSubview(&nav);
        // The switch decides what search looks through, so the two travel together.
        let center = horizontal(mtm, 10.);
        center.addArrangedSubview(&switch);
        center.addArrangedSubview(&search_frame);
        let right = horizontal(mtm, 10.);
        right.addArrangedSubview(&actions);
        right.addArrangedSubview(&account);
        for view in [&left as &NSView, &right, &center] {
            root.addSubview(view);
        }
        let height = root.heightAnchor().constraintEqualToConstant(HEIGHT);
        // Switch and search stay centered in the window but yield to both sides when narrow.
        let centered = center
            .centerXAnchor()
            .constraintEqualToAnchor(&root.centerXAnchor());
        centered.setPriority(700.);
        let wide = search_frame.widthAnchor().constraintEqualToConstant(480.);
        wide.setPriority(600.);
        for view in [&search as &NSView, &search_frame] {
            view.setContentCompressionResistancePriority_forOrientation(
                250.,
                NSLayoutConstraintOrientation::Horizontal,
            );
        }
        activate(&[
            height,
            centered,
            wide,
            // Clears the traffic lights, which sit in this row.
            left.leadingAnchor()
                .constraintEqualToAnchor_constant(&root.leadingAnchor(), 88.),
            left.centerYAnchor()
                .constraintEqualToAnchor(&root.centerYAnchor()),
            right
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&root.trailingAnchor(), -12.),
            right
                .centerYAnchor()
                .constraintEqualToAnchor(&root.centerYAnchor()),
            center
                .centerYAnchor()
                .constraintEqualToAnchor(&root.centerYAnchor()),
            center
                .leadingAnchor()
                .constraintGreaterThanOrEqualToAnchor_constant(&left.trailingAnchor(), 16.),
            center
                .trailingAnchor()
                .constraintLessThanOrEqualToAnchor_constant(&right.leadingAnchor(), -16.),
            search_frame
                .heightAnchor()
                .constraintEqualToConstant(ISLAND_HEIGHT),
            search_frame
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(140.),
            search_frame
                .widthAnchor()
                .constraintLessThanOrEqualToConstant(480.),
            glyph
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&search_frame.leadingAnchor(), 13.),
            glyph
                .centerYAnchor()
                .constraintEqualToAnchor(&search_frame.centerYAnchor()),
            glyph.widthAnchor().constraintEqualToConstant(17.),
            search
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&glyph.trailingAnchor(), 7.),
            search
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&search_frame.trailingAnchor(), -10.),
            search
                .centerYAnchor()
                .constraintEqualToAnchor(&search_frame.centerYAnchor()),
            home.widthAnchor().constraintEqualToConstant(ISLAND_HEIGHT),
            home.heightAnchor().constraintEqualToConstant(ISLAND_HEIGHT),
            account
                .widthAnchor()
                .constraintEqualToConstant(ISLAND_HEIGHT),
            account
                .heightAnchor()
                .constraintEqualToConstant(ISLAND_HEIGHT),
            count
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(22.),
        ]);
        let bar = Self {
            root,
            back,
            forward,
            home,
            youtube,
            music,
            save,
            pip,
            queue,
            account,
            search,
            search_frame,
            count,
            youtube_pill,
            music_pill,
        };
        bar.set_mode(Mode::YouTube);
        bar.set_saved(None);
        bar.set_queue_count(0);
        bar.set_navigation(false, false);
        bar
    }

    pub fn set_mode(&self, mode: Mode) {
        let (active, inactive) = (ink_color(), NSColor::secondaryLabelColor());
        let youtube_on = matches!(mode, Mode::YouTube);
        let music_on = !youtube_on;
        let (selected, clear) = (accent_alpha(0.22), NSColor::clearColor());
        for (pill, on) in [
            (&self.youtube_pill, youtube_on),
            (&self.music_pill, music_on),
        ] {
            pill.setFillColor(if on { &selected } else { &clear });
            pill.setBorderWidth(if on { 1. } else { 0. });
        }
        self.youtube.setAccessibilityLabel(Some(&ns(if youtube_on {
            "YouTube, selected"
        } else {
            "YouTube"
        })));
        self.music.setAccessibilityLabel(Some(&ns(if music_on {
            "Music, selected"
        } else {
            "Music"
        })));
        self.youtube
            .setContentTintColor(Some(if youtube_on { &active } else { &inactive }));
        self.music
            .setContentTintColor(Some(if music_on { &active } else { &inactive }));
        for (button, selected) in [(&self.youtube, youtube_on), (&self.music, music_on)] {
            let weight = if selected {
                unsafe { NSFontWeightSemibold }
            } else {
                unsafe { NSFontWeightRegular }
            };
            button.setFont(Some(&NSFont::systemFontOfSize_weight(14., weight)));
        }
        let placeholder = ns(mode.search_placeholder());
        self.search.setPlaceholderString(Some(&placeholder));
        self.search.setAccessibilityLabel(Some(&placeholder));
        self.search.setToolTip(Some(&placeholder));
    }

    pub fn set_saved(&self, saved: Option<bool>) {
        let (name, tip, color) = match saved {
            Some(true) => ("bookmark.fill", "Saved in Up Next", warm_color()),
            Some(false) => ("bookmark", "Save video", icon_color()),
            None => (
                "bookmark",
                "Save unavailable",
                NSColor::tertiaryLabelColor(),
            ),
        };
        self.save.setImage(symbol(name, tip).as_deref());
        self.save.setToolTip(Some(&ns(tip)));
        self.save.setAccessibilityLabel(Some(&ns(tip)));
        self.save.setEnabled(saved == Some(false));
        self.save.setContentTintColor(Some(&color));
    }

    pub fn set_account(&self, name: &str, image: Option<&NSImage>) {
        let label = if name.is_empty() { "Account" } else { name };
        self.account.setToolTip(Some(&ns(label)));
        self.account.setAccessibilityLabel(Some(&ns(label)));
        if let Some(image) = image {
            self.account.setImage(Some(&circular_image(image, 32.)));
            self.account
                .setImageScaling(NSImageScaling::ScaleProportionallyDown);
        } else {
            self.account
                .setImage(symbol("person.crop.circle", "Account").as_deref());
        }
    }

    pub fn set_queue_count(&self, count: usize) {
        let text = count.to_string();
        self.count.setStringValue(&ns(&text));
        self.count.setHidden(count == 0);
        self.count
            .setAccessibilityLabel(Some(&ns(&format!("{count} queued items"))));
        self.queue
            .setToolTip(Some(&ns(&format!("Up Next, {count} items"))));
        self.queue
            .setAccessibilityLabel(Some(&ns(&format!("Up Next, {count} items"))));
    }

    pub fn set_navigation(&self, can_back: bool, can_forward: bool) {
        self.back.setEnabled(can_back);
        self.forward.setEnabled(can_forward);
    }
}

fn horizontal(mtm: MainThreadMarker, spacing: f64) -> Retained<NSStackView> {
    let stack = NSStackView::new(mtm);
    stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    stack.setAlignment(objc2_app_kit::NSLayoutAttribute::CenterY);
    stack.setSpacing(spacing);
    stack.setTranslatesAutoresizingMaskIntoConstraints(false);
    stack
}

fn mode_button(mtm: MainThreadMarker, title: &str) -> Retained<NSButton> {
    let button = NSButton::new(mtm);
    button.setBordered(false);
    button.setTitle(&ns(title));
    button.setToolTip(Some(&ns(title)));
    button.setAccessibilityLabel(Some(&ns(title)));
    button.setTranslatesAutoresizingMaskIntoConstraints(false);
    button
}

/// The rounded highlight behind one section of the switch, with `button` filling it.
fn mode_pill(mtm: MainThreadMarker, button: &NSButton) -> Retained<NSBox> {
    let pill = ui::surface(mtm, PILL_HEIGHT / 2., &NSColor::clearColor());
    pill.setBorderColor(&accent_alpha(0.6));
    pill.setBorderWidth(0.);
    fill_pill(&pill, button, 15.);
    pill
}

/// An icon button's click target, which lights up under the pointer.
fn hover_pill(mtm: MainThreadMarker, button: &NSButton) -> Retained<NSBox> {
    let pill = HoverBox::new(mtm);
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveInKeyWindow
                | NSTrackingAreaOptions::InVisibleRect,
            Some(&pill),
            None,
        )
    };
    pill.addTrackingArea(&area);
    let pill = pill.into_super();
    pill.setBoxType(NSBoxType::Custom);
    pill.setTitlePosition(objc2_app_kit::NSTitlePosition::NoTitle);
    pill.setBorderWidth(0.);
    pill.setCornerRadius(PILL_HEIGHT / 2.);
    pill.setFillColor(&NSColor::clearColor());
    pill.setTranslatesAutoresizingMaskIntoConstraints(false);
    fill_pill(&pill, button, 10.);
    pill
}

fn fill_pill(pill: &NSBox, button: &NSButton, inset: f64) {
    pill.addSubview(button);
    activate(&[
        pill.heightAnchor().constraintEqualToConstant(PILL_HEIGHT),
        button
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&pill.leadingAnchor(), inset),
        button
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&pill.trailingAnchor(), -inset),
        button
            .topAnchor()
            .constraintEqualToAnchor(&pill.topAnchor()),
        button
            .bottomAnchor()
            .constraintEqualToAnchor(&pill.bottomAnchor()),
    ]);
}

/// A capsule holding one group of controls. Every island in the bar has the same height.
fn island(mtm: MainThreadMarker, views: &[&NSView]) -> Retained<NSBox> {
    let island = ui::surface(mtm, ISLAND_HEIGHT / 2., &glass_color());
    glow(&island, EDGE, HALO);
    let stack = horizontal(mtm, 1.);
    for view in views {
        stack.addArrangedSubview(view);
    }
    island.addSubview(&stack);
    let inset = (ISLAND_HEIGHT - PILL_HEIGHT) / 2.;
    activate(&[
        island
            .heightAnchor()
            .constraintEqualToConstant(ISLAND_HEIGHT),
        stack
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&island.leadingAnchor(), inset),
        stack
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&island.trailingAnchor(), -inset),
        stack
            .centerYAnchor()
            .constraintEqualToAnchor(&island.centerYAnchor()),
    ]);
    island
}

fn any_disabled_button(view: &NSView) -> bool {
    view.downcast_ref::<NSButton>()
        .is_some_and(|button| !button.isEnabled())
        || view
            .subviews()
            .iter()
            .any(|child| any_disabled_button(&child))
}

define_class!(
    /// A pill that lightens while the pointer is over an enabled button inside it.
    #[unsafe(super(NSBox, NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    struct HoverBox;

    impl HoverBox {
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            if !any_disabled_button(self) {
                self.setFillColor(&NSColor::colorWithWhite_alpha(1., 0.1));
            }
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.setFillColor(&NSColor::clearColor());
        }
    }
);

impl HoverBox {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

define_class!(
    /// Search field that outlines its capsule in the accent color while it has focus.
    #[unsafe(super(NSSearchField, NSTextField, NSControl, NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    struct FocusSearchField;

    impl FocusSearchField {
        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            let became: bool = unsafe { msg_send![super(self), becomeFirstResponder] };
            if became {
                self.light_capsule(1., 0.9);
            }
            became
        }

        #[unsafe(method(textDidEndEditing:))]
        fn text_did_end_editing(&self, notification: &NSNotification) {
            let _: () = unsafe { msg_send![super(self), textDidEndEditing: notification] };
            self.light_capsule(EDGE, HALO);
        }
    }
);

impl FocusSearchField {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }

    fn light_capsule(&self, edge: f64, halo: f64) {
        let mut view = unsafe { self.superview() };
        while let Some(current) = view {
            if let Some(capsule) = current.downcast_ref::<NSBox>() {
                glow(capsule, edge, halo);
                return;
            }
            view = unsafe { current.superview() };
        }
    }
}

fn icon_button(mtm: MainThreadMarker, name: &str, description: &str) -> Retained<NSButton> {
    let button = NSButton::new(mtm);
    button.setBordered(false);
    button.setTitle(&ns(""));
    button.setImage(symbol(name, description).as_deref());
    button.setImageScaling(NSImageScaling::ScaleProportionallyDown);
    let config = NSImageSymbolConfiguration::configurationWithPointSize_weight(16., unsafe {
        NSFontWeightMedium
    });
    button.setSymbolConfiguration(Some(&config));
    button.setContentTintColor(Some(&icon_color()));
    button.setToolTip(Some(&ns(description)));
    button.setAccessibilityLabel(Some(&ns(description)));
    button.setTranslatesAutoresizingMaskIntoConstraints(false);
    button
}
