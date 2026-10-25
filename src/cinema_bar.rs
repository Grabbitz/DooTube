use objc2::rc::Retained;
use objc2_app_kit::{
    NSAccessibility, NSBox, NSBoxType, NSButton, NSColor, NSFont, NSFontWeightRegular,
    NSFontWeightSemibold, NSImage, NSImageScaling, NSLayoutConstraint,
    NSLayoutConstraintOrientation, NSSearchField, NSStackView, NSTextAlignment, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{MainThreadMarker, NSEdgeInsets, NSSize};

use crate::{
    ui::{activate, circular_image, ns, symbol},
    web::Mode,
};

// sRGB projections of the canonical OKLCH palette in resources/cinema/tokens.css.
const RED: (f64, f64, f64) = (0.929263, 0.293140, 0.264086);

pub fn background_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.039636, 0.024776, 0.023106, 1.)
}

pub fn raised_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.092784, 0.071825, 0.069001, 1.)
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
    playback: Retained<NSStackView>,
    height: Retained<NSLayoutConstraint>,
}

impl CinemaBar {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let root = NSBox::new(mtm);
        root.setBoxType(NSBoxType::Custom);
        root.setBorderWidth(0.);
        root.setTitlePosition(objc2_app_kit::NSTitlePosition::NoTitle);
        root.setFillColor(&background_color());
        root.setContentViewMargins(NSSize::new(0., 0.));
        root.setTranslatesAutoresizingMaskIntoConstraints(false);

        let brand = NSTextField::labelWithString(&ns("DooTube"), mtm);
        brand.setFont(Some(&NSFont::systemFontOfSize_weight(23., unsafe {
            NSFontWeightSemibold
        })));
        brand.setTextColor(Some(&ink_color()));
        brand.setToolTip(Some(&ns("DooTube")));
        brand.setAccessibilityLabel(Some(&ns("DooTube")));
        brand.setTranslatesAutoresizingMaskIntoConstraints(false);

        let youtube = mode_button(mtm, "YouTube");
        let music = mode_button(mtm, "Music");
        let search = NSSearchField::new(mtm);
        search.setPlaceholderString(Some(&ns("Search YouTube")));
        search.setToolTip(Some(&ns("Search YouTube")));
        search.setAccessibilityLabel(Some(&ns("Search YouTube")));
        search.setTranslatesAutoresizingMaskIntoConstraints(false);

        let queue = icon_button(mtm, "list.bullet", "Up Next");
        let count = NSTextField::labelWithString(&ns("0"), mtm);
        count.setFont(Some(&NSFont::systemFontOfSize(14.)));
        count.setTextColor(Some(&NSColor::secondaryLabelColor()));
        count.setAlignment(NSTextAlignment::Center);
        count.setAccessibilityLabel(Some(&ns("Queued items")));
        count.setTranslatesAutoresizingMaskIntoConstraints(false);

        let account = icon_button(mtm, "person.crop.circle", "Account");
        let back = icon_button(mtm, "chevron.left", "Back");
        let forward = icon_button(mtm, "chevron.right", "Forward");
        let home = icon_button(mtm, "house", "Cinema Home");
        let save = icon_button(mtm, "bookmark", "Save video");
        let pip = icon_button(mtm, "pip", "Picture in Picture");

        let main_line = horizontal(mtm, 10.);
        let spacer = NSView::new(mtm);
        spacer.setTranslatesAutoresizingMaskIntoConstraints(false);
        spacer.setContentHuggingPriority_forOrientation(
            1.,
            NSLayoutConstraintOrientation::Horizontal,
        );
        spacer.setContentCompressionResistancePriority_forOrientation(
            1.,
            NSLayoutConstraintOrientation::Horizontal,
        );
        main_line.addArrangedSubview(&brand);
        main_line.addArrangedSubview(&youtube);
        main_line.addArrangedSubview(&music);
        main_line.addArrangedSubview(&spacer);
        main_line.addArrangedSubview(&search);
        main_line.addArrangedSubview(&queue);
        main_line.addArrangedSubview(&count);
        main_line.addArrangedSubview(&account);
        let playback = horizontal(mtm, 8.);
        for button in [&back, &forward, &home, &save, &pip] {
            playback.addArrangedSubview(button);
        }
        playback.setHidden(true);
        let layout = NSStackView::new(mtm);
        layout.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        layout.setDetachesHiddenViews(true);
        layout.setSpacing(4.);
        layout.setEdgeInsets(NSEdgeInsets {
            top: 8.,
            left: 24.,
            bottom: 8.,
            right: 24.,
        });
        layout.addArrangedSubview(&main_line);
        layout.addArrangedSubview(&playback);
        root.addSubview(&layout);
        // NSBox has no content-driven intrinsic height. Keep it from consuming the viewport.
        let height = root.heightAnchor().constraintEqualToConstant(72.);
        height.setActive(true);

        for view in [
            &brand as &NSView,
            &youtube,
            &music,
            &queue,
            &account,
            &back,
            &forward,
            &home,
            &save,
            &pip,
            &count,
            &search,
            &main_line,
            &playback,
            &layout,
        ] {
            view.setTranslatesAutoresizingMaskIntoConstraints(false);
        }
        search.setContentHuggingPriority_forOrientation(
            250.,
            NSLayoutConstraintOrientation::Horizontal,
        );
        search.setContentCompressionResistancePriority_forOrientation(
            1.,
            NSLayoutConstraintOrientation::Horizontal,
        );
        activate(&[
            layout
                .leadingAnchor()
                .constraintEqualToAnchor(&root.leadingAnchor()),
            layout
                .trailingAnchor()
                .constraintEqualToAnchor(&root.trailingAnchor()),
            layout
                .topAnchor()
                .constraintEqualToAnchor(&root.topAnchor()),
            layout
                .bottomAnchor()
                .constraintEqualToAnchor(&root.bottomAnchor()),
            main_line.heightAnchor().constraintEqualToConstant(56.),
            playback.heightAnchor().constraintEqualToConstant(40.),
            youtube.heightAnchor().constraintEqualToConstant(44.),
            music.heightAnchor().constraintEqualToConstant(44.),
            queue.heightAnchor().constraintEqualToConstant(44.),
            account.heightAnchor().constraintEqualToConstant(44.),
            search.heightAnchor().constraintEqualToConstant(32.),
            search
                .widthAnchor()
                .constraintLessThanOrEqualToConstant(280.),
            brand
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(108.),
            youtube
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(66.),
            music
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(58.),
            search
                .widthAnchor()
                .constraintGreaterThanOrEqualToConstant(100.),
            queue.widthAnchor().constraintEqualToConstant(44.),
            account.widthAnchor().constraintEqualToConstant(44.),
            back.widthAnchor().constraintEqualToConstant(32.),
            forward.widthAnchor().constraintEqualToConstant(32.),
            home.widthAnchor().constraintEqualToConstant(36.),
            save.widthAnchor().constraintEqualToConstant(36.),
            pip.widthAnchor().constraintEqualToConstant(36.),
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
            count,
            playback,
            height,
        };
        bar.set_mode(Mode::YouTube);
        bar.set_player_visible(false);
        bar.set_saved(None);
        bar.set_queue_count(0);
        bar.set_navigation(false, false);
        bar
    }

    pub fn set_mode(&self, mode: Mode) {
        let (active, inactive) = (accent_color(), NSColor::secondaryLabelColor());
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
            button.setFont(Some(&NSFont::systemFontOfSize_weight(14., weight)));
        }
        let placeholder = ns(mode.search_placeholder());
        self.search.setPlaceholderString(Some(&placeholder));
        self.search.setAccessibilityLabel(Some(&placeholder));
    }

    pub fn set_player_visible(&self, visible: bool) {
        self.playback.setHidden(!visible);
        self.height.setConstant(if visible { 116. } else { 72. });
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
