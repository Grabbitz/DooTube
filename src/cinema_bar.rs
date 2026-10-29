use objc2::rc::Retained;
use objc2_app_kit::{
    NSAccessibility, NSBox, NSBoxType, NSButton, NSColor, NSFont, NSFontWeightRegular,
    NSFontWeightSemibold, NSImage, NSImageScaling, NSLayoutConstraintOrientation, NSSearchField,
    NSStackView, NSTextAlignment, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{MainThreadMarker, NSSize};

use crate::{
    ui::{activate, circular_image, ns, symbol},
    web::Mode,
};

// Close to YouTube's own dark theme, so the bar and drawer read as part of the page.
const RED: (f64, f64, f64) = (0.929263, 0.293140, 0.264086);

pub fn background_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.059, 0.059, 0.059, 1.)
}

pub fn raised_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.13, 0.13, 0.13, 1.)
}

pub fn accent_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(RED.0, RED.1, RED.2, 1.)
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
    count: Retained<NSTextField>,
}

/// Height of the bar, which shares its row with the window's traffic lights.
pub const HEIGHT: f64 = 52.;

impl CinemaBar {
    /// One YouTube-style row: navigation and sections on the left, search centered, and
    /// video, queue, and account actions on the right. Clear, so the page color shows through.
    pub fn new(mtm: MainThreadMarker) -> Self {
        let root = NSBox::new(mtm);
        root.setBoxType(NSBoxType::Custom);
        root.setBorderWidth(0.);
        root.setTitlePosition(objc2_app_kit::NSTitlePosition::NoTitle);
        root.setFillColor(&NSColor::clearColor());
        root.setContentViewMargins(NSSize::new(0., 0.));
        root.setTranslatesAutoresizingMaskIntoConstraints(false);

        let home = icon_button(mtm, "play.rectangle.fill", "Home");
        home.setContentTintColor(Some(&accent_color()));
        home.setTitle(&ns("DooTube"));
        home.setImagePosition(objc2_app_kit::NSCellImagePosition::ImageLeading);
        home.setFont(Some(&NSFont::systemFontOfSize_weight(17., unsafe {
            NSFontWeightSemibold
        })));
        home.setToolTip(Some(&ns("Home")));

        let youtube = mode_button(mtm, "YouTube");
        let music = mode_button(mtm, "Music");
        let search = NSSearchField::new(mtm);
        search.setPlaceholderString(Some(&ns("Search YouTube")));
        search.setToolTip(Some(&ns("Search YouTube")));
        search.setAccessibilityLabel(Some(&ns("Search YouTube")));
        search.setFont(Some(&NSFont::systemFontOfSize(14.)));
        search.setTranslatesAutoresizingMaskIntoConstraints(false);

        let queue = icon_button(mtm, "list.bullet", "Up Next");
        let count = NSTextField::labelWithString(&ns("0"), mtm);
        count.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
            12.,
            unsafe { NSFontWeightRegular },
        )));
        count.setTextColor(Some(&NSColor::secondaryLabelColor()));
        count.setAlignment(NSTextAlignment::Left);
        count.setAccessibilityLabel(Some(&ns("Queued items")));

        let account = icon_button(mtm, "person.crop.circle", "Account");
        let back = icon_button(mtm, "chevron.left", "Back");
        let forward = icon_button(mtm, "chevron.right", "Forward");
        let save = icon_button(mtm, "bookmark", "Save video");
        let pip = icon_button(mtm, "pip.enter", "Picture in Picture");

        let left = horizontal(mtm, 4.);
        for view in [&back as &NSView, &forward, &home] {
            left.addArrangedSubview(view);
        }
        left.setCustomSpacing_afterView(14., &home);
        left.addArrangedSubview(&youtube);
        left.addArrangedSubview(&music);
        let right = horizontal(mtm, 6.);
        for view in [&save as &NSView, &pip, &queue] {
            right.addArrangedSubview(view);
        }
        right.addArrangedSubview(&count);
        right.setCustomSpacing_afterView(10., &count);
        right.addArrangedSubview(&account);
        for view in [&left as &NSView, &right, &search] {
            root.addSubview(view);
        }
        let height = root.heightAnchor().constraintEqualToConstant(HEIGHT);
        // The search field stays centered in the window but yields to both sides when narrow.
        let centered = search
            .centerXAnchor()
            .constraintEqualToAnchor(&root.centerXAnchor());
        centered.setPriority(700.);
        let wide = search.widthAnchor().constraintEqualToConstant(520.);
        wide.setPriority(600.);
        search.setContentCompressionResistancePriority_forOrientation(
            250.,
            NSLayoutConstraintOrientation::Horizontal,
        );
        activate(&[
            height,
            centered,
            wide,
            // Clears the traffic lights, which sit in this row.
            left.leadingAnchor()
                .constraintEqualToAnchor_constant(&root.leadingAnchor(), 80.),
            left.centerYAnchor()
                .constraintEqualToAnchor(&root.centerYAnchor()),
            right
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&root.trailingAnchor(), -16.),
            right
                .centerYAnchor()
                .constraintEqualToAnchor(&root.centerYAnchor()),
            search
                .centerYAnchor()
                .constraintEqualToAnchor(&root.centerYAnchor()),
            search
                .leadingAnchor()
                .constraintGreaterThanOrEqualToAnchor_constant(&left.trailingAnchor(), 16.),
            search
                .trailingAnchor()
                .constraintLessThanOrEqualToAnchor_constant(&right.leadingAnchor(), -16.),
            search.heightAnchor().constraintEqualToConstant(30.),
            search
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(140.),
            search
                .widthAnchor()
                .constraintLessThanOrEqualToConstant(520.),
            back.widthAnchor().constraintEqualToConstant(28.),
            forward.widthAnchor().constraintEqualToConstant(28.),
            save.widthAnchor().constraintEqualToConstant(32.),
            pip.widthAnchor().constraintEqualToConstant(32.),
            queue.widthAnchor().constraintEqualToConstant(32.),
            account.widthAnchor().constraintEqualToConstant(32.),
            account.heightAnchor().constraintEqualToConstant(32.),
            count
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(14.),
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
            count,
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
            button.setFont(Some(&NSFont::systemFontOfSize_weight(13., weight)));
        }
        let placeholder = ns(mode.search_placeholder());
        self.search.setPlaceholderString(Some(&placeholder));
        self.search.setAccessibilityLabel(Some(&placeholder));
    }

    pub fn set_saved(&self, saved: Option<bool>) {
        let (name, tip, color) = match saved {
            Some(true) => ("bookmark.fill", "Saved in Up Next", accent_color()),
            Some(false) => ("bookmark", "Save video", NSColor::secondaryLabelColor()),
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
            self.account.setImage(Some(&circular_image(image, 28.)));
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

fn icon_button(mtm: MainThreadMarker, name: &str, description: &str) -> Retained<NSButton> {
    let button = NSButton::new(mtm);
    button.setBordered(false);
    button.setTitle(&ns(""));
    button.setImage(symbol(name, description).as_deref());
    button.setImageScaling(NSImageScaling::ScaleProportionallyDown);
    button.setToolTip(Some(&ns(description)));
    button.setAccessibilityLabel(Some(&ns(description)));
    button.setTranslatesAutoresizingMaskIntoConstraints(false);
    button
}
