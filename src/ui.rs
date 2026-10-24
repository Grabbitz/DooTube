//! Small AppKit builders for DooTube's native chrome.
use objc2::{
    MainThreadOnly, Message,
    rc::Retained,
    runtime::{AnyObject, Bool, Sel},
};
use objc2_app_kit::{
    NSBezierPath, NSBox, NSBoxType, NSButton, NSColor, NSCompositingOperation, NSControlSize,
    NSFont, NSFontWeightRegular, NSFontWeightSemibold, NSImage, NSImageScaling,
    NSImageSymbolConfiguration, NSImageView, NSLayoutAttribute, NSLayoutConstraint,
    NSLayoutConstraintOrientation, NSLineBreakMode, NSMenu, NSMenuItem, NSProgressIndicator,
    NSProgressIndicatorStyle, NSStackView, NSTableCellView, NSTextAlignment, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSEdgeInsets, NSPoint, NSRect, NSSize, NSString,
};
use std::cell::Cell;

pub fn ns(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}

pub fn symbol(name: &str, description: &str) -> Option<Retained<NSImage>> {
    NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns(name), Some(&ns(description)))
}

/// Clips `image` to a circle of `size` points, drawn lazily at the screen's resolution.
pub fn circular_image(image: &NSImage, size: f64) -> Retained<NSImage> {
    let image = image.retain();
    let draw = block2::RcBlock::new(move |rect: NSRect| -> Bool {
        NSBezierPath::bezierPathWithOvalInRect(rect).addClip();
        image.drawInRect(rect);
        Bool::YES
    });
    NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(size, size), false, &draw)
}

/// Fills a `size`-point square with `image`, cropped to its center, with rounded corners.
pub fn rounded_image(image: &NSImage, size: f64, radius: f64) -> Retained<NSImage> {
    rounded_image_sized(image, NSSize::new(size, size), radius)
}

/// Fills `size` with `image`, cropped to its center at that aspect ratio, with rounded corners.
pub fn rounded_image_sized(image: &NSImage, size: NSSize, radius: f64) -> Retained<NSImage> {
    let image = image.retain();
    let draw = block2::RcBlock::new(move |rect: NSRect| -> Bool {
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, radius, radius).addClip();
        let source = image.size();
        let aspect = rect.size.width / rect.size.height;
        let (width, height) = if source.width / source.height > aspect {
            (source.height * aspect, source.height)
        } else {
            (source.width, source.width / aspect)
        };
        let crop = NSRect::new(
            NSPoint::new((source.width - width) / 2., (source.height - height) / 2.),
            NSSize::new(width, height),
        );
        image.drawInRect_fromRect_operation_fraction(
            rect,
            crop,
            NSCompositingOperation::SourceOver,
            1.,
        );
        Bool::YES
    });
    NSImage::imageWithSize_flipped_drawingHandler(size, false, &draw)
}

pub fn activate(constraints: &[Retained<NSLayoutConstraint>]) {
    NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(constraints));
}

pub fn menu_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<Sel>,
    key: &str,
    target: Option<&AnyObject>,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &ns(title),
            action,
            &ns(key),
        )
    };
    if target.is_some() {
        unsafe { item.setTarget(target) };
    }
    item
}

/// Adds a titled submenu to `main`; `None` entries become separators.
pub fn add_menu(
    mtm: MainThreadMarker,
    main: &NSMenu,
    title: &str,
    items: Vec<Option<Retained<NSMenuItem>>>,
) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &ns(title));
    for item in items {
        match item {
            Some(item) => menu.addItem(&item),
            None => menu.addItem(&NSMenuItem::separatorItem(mtm)),
        }
    }
    let holder = menu_item(mtm, title, None, "", None);
    holder.setSubmenu(Some(&menu));
    main.addItem(&holder);
    menu
}

fn label(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&ns(text), mtm);
    field.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    field.setTranslatesAutoresizingMaskIntoConstraints(false);
    field
}

fn cell_frame() -> NSRect {
    NSRect::new(NSPoint::new(0., 0.), NSSize::new(220., 28.))
}

/// Standard sidebar row: SF Symbol plus a single truncating line.
pub fn sidebar_cell(
    mtm: MainThreadMarker,
    title: &str,
    symbol_name: &str,
    tooltip: Option<&str>,
) -> Retained<NSView> {
    let cell = NSTableCellView::initWithFrame(NSTableCellView::alloc(mtm), cell_frame());
    let icon = NSImageView::new(mtm);
    icon.setTranslatesAutoresizingMaskIntoConstraints(false);
    icon.setImage(symbol(symbol_name, title).as_deref());
    icon.setImageScaling(NSImageScaling::ScaleProportionallyDown);
    // One optical size for every row, so mixed symbols line up. Still a template image:
    // the source list recolors it on selection, and nothing is redrawn while scrolling.
    let config = NSImageSymbolConfiguration::configurationWithPointSize_weight(16., unsafe {
        NSFontWeightRegular
    });
    icon.setSymbolConfiguration(Some(&config));
    let text = label(mtm, title);
    if let Some(tooltip) = tooltip {
        cell.setToolTip(Some(&ns(tooltip)));
    }
    cell.addSubview(&icon);
    cell.addSubview(&text);
    unsafe {
        cell.setImageView(Some(&icon));
        cell.setTextField(Some(&text));
    }
    activate(&[
        icon.leadingAnchor()
            .constraintEqualToAnchor_constant(&cell.leadingAnchor(), 4.),
        icon.centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor()),
        icon.widthAnchor().constraintEqualToConstant(18.),
        icon.heightAnchor().constraintEqualToConstant(18.),
        text.leadingAnchor()
            .constraintEqualToAnchor_constant(&icon.trailingAnchor(), 8.),
        text.trailingAnchor()
            .constraintEqualToAnchor_constant(&cell.trailingAnchor(), -4.),
        text.centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor()),
    ]);
    cell.into_super()
}

/// Height of an Up Next row, which shows the video's thumbnail.
pub const SAVED_ROW_HEIGHT: f64 = 36.;
const THUMBNAIL_SIZE: NSSize = NSSize::new(48., 27.);

/// A downloaded video thumbnail, prepared once for [`saved_cell`].
pub fn thumbnail_image(image: &NSImage) -> Retained<NSImage> {
    rounded_image_sized(image, THUMBNAIL_SIZE, 4.)
}

/// Up Next row: the video's [`thumbnail_image`] (or `symbol_name` until it loads) and its title.
/// The thumbnail box keeps its size either way, so titles don't shift when it arrives.
pub fn saved_cell(
    mtm: MainThreadMarker,
    title: &str,
    thumbnail: Option<&NSImage>,
    symbol_name: &str,
) -> Retained<NSView> {
    let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(220., SAVED_ROW_HEIGHT));
    let cell = NSTableCellView::initWithFrame(NSTableCellView::alloc(mtm), frame);
    let icon = NSImageView::new(mtm);
    icon.setTranslatesAutoresizingMaskIntoConstraints(false);
    match thumbnail {
        Some(image) => {
            icon.setImage(Some(image));
            icon.setImageScaling(NSImageScaling::ScaleNone);
        }
        None => {
            icon.setImage(symbol(symbol_name, title).as_deref());
            icon.setImageScaling(NSImageScaling::ScaleProportionallyDown);
            let config =
                NSImageSymbolConfiguration::configurationWithPointSize_weight(16., unsafe {
                    NSFontWeightRegular
                });
            icon.setSymbolConfiguration(Some(&config));
        }
    }
    let text = label(mtm, title);
    cell.setToolTip(Some(&ns(title)));
    cell.addSubview(&icon);
    cell.addSubview(&text);
    unsafe {
        // Not the cell's imageView: the source list would tint the thumbnail on selection.
        cell.setTextField(Some(&text));
    }
    activate(&[
        icon.leadingAnchor()
            .constraintEqualToAnchor_constant(&cell.leadingAnchor(), 4.),
        icon.centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor()),
        icon.widthAnchor()
            .constraintEqualToConstant(THUMBNAIL_SIZE.width),
        icon.heightAnchor()
            .constraintEqualToConstant(THUMBNAIL_SIZE.height),
        text.leadingAnchor()
            .constraintEqualToAnchor_constant(&icon.trailingAnchor(), 8.),
        text.trailingAnchor()
            .constraintEqualToAnchor_constant(&cell.trailingAnchor(), -4.),
        text.centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor()),
    ]);
    cell.into_super()
}

/// Section header for a source-list group row.
pub fn header_cell(mtm: MainThreadMarker, title: &str) -> Retained<NSView> {
    let cell = NSTableCellView::initWithFrame(NSTableCellView::alloc(mtm), cell_frame());
    let text = label(mtm, title);
    text.setFont(Some(&NSFont::systemFontOfSize_weight(11., unsafe {
        NSFontWeightSemibold
    })));
    text.setTextColor(Some(&NSColor::secondaryLabelColor()));
    cell.addSubview(&text);
    unsafe { cell.setTextField(Some(&text)) };
    // Source lists don't indent group rows; line the header up with the rows' icons below it.
    activate(&[
        text.leadingAnchor()
            .constraintEqualToAnchor_constant(&cell.leadingAnchor(), 14.),
        text.trailingAnchor()
            .constraintEqualToAnchor_constant(&cell.trailingAnchor(), -4.),
        text.centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor()),
    ]);
    cell.into_super()
}

/// Non-selectable explanatory row, used when a section is empty.
pub fn hint_cell(mtm: MainThreadMarker, text: &str) -> Retained<NSView> {
    let cell = NSTableCellView::initWithFrame(NSTableCellView::alloc(mtm), cell_frame());
    let field = label(mtm, text);
    field.setFont(Some(&NSFont::systemFontOfSize(12.)));
    field.setTextColor(Some(&NSColor::tertiaryLabelColor()));
    cell.addSubview(&field);
    activate(&[
        field
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&cell.leadingAnchor(), 30.),
        field
            .trailingAnchor()
            .constraintEqualToAnchor_constant(&cell.trailingAnchor(), -4.),
        field
            .centerYAnchor()
            .constraintEqualToAnchor(&cell.centerYAnchor()),
    ]);
    cell.into_super()
}

/// Centered first-load and failure state shown before the YouTube page renders.
pub struct Placeholder {
    pub root: Retained<NSStackView>,
    spinner: Retained<NSProgressIndicator>,
    icon: Retained<NSImageView>,
    title: Retained<NSTextField>,
    detail: Retained<NSTextField>,
    pub retry: Retained<NSButton>,
}

impl Placeholder {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let spinner = NSProgressIndicator::new(mtm);
        spinner.setStyle(NSProgressIndicatorStyle::Spinning);
        spinner.setControlSize(NSControlSize::Regular);
        spinner.setDisplayedWhenStopped(false);
        let icon = NSImageView::new(mtm);
        let config = NSImageSymbolConfiguration::configurationWithPointSize_weight(34., unsafe {
            NSFontWeightRegular
        });
        icon.setSymbolConfiguration(Some(&config));
        icon.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
        let title = NSTextField::labelWithString(&ns(""), mtm);
        title.setFont(Some(&NSFont::systemFontOfSize_weight(17., unsafe {
            NSFontWeightSemibold
        })));
        title.setAlignment(NSTextAlignment::Center);
        let detail = NSTextField::wrappingLabelWithString(&ns(""), mtm);
        detail.setTextColor(Some(&NSColor::secondaryLabelColor()));
        detail.setAlignment(NSTextAlignment::Center);
        detail.setPreferredMaxLayoutWidth(320.);
        let retry =
            unsafe { NSButton::buttonWithTitle_target_action(&ns("Try Again"), None, None, mtm) };
        let root = NSStackView::new(mtm);
        root.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        root.setSpacing(8.);
        root.setTranslatesAutoresizingMaskIntoConstraints(false);
        for view in [
            spinner.as_ref() as &NSView,
            icon.as_ref(),
            title.as_ref(),
            detail.as_ref(),
            retry.as_ref(),
        ] {
            root.addArrangedSubview(view);
        }
        root.setCustomSpacing_afterView(12., &icon);
        root.setCustomSpacing_afterView(20., &detail);
        let placeholder = Self {
            root,
            spinner,
            icon,
            title,
            detail,
            retry,
        };
        placeholder.show_loading("Loading YouTube");
        placeholder
    }

    /// Spinner plus a single status line. No progress number — the page doesn't report one.
    pub fn show_loading(&self, title: &str) {
        for view in [
            self.icon.as_ref() as &NSView,
            self.detail.as_ref(),
            self.retry.as_ref(),
        ] {
            view.setHidden(true);
        }
        self.title.setStringValue(&ns(title));
        self.title.setHidden(false);
        self.spinner.setHidden(false);
        unsafe { self.spinner.startAnimation(None) };
        self.root.setHidden(false);
    }

    pub fn show_failure(&self, symbol_name: &str, title: &str, detail: &str) {
        unsafe { self.spinner.stopAnimation(None) };
        self.spinner.setHidden(true);
        self.icon.setImage(symbol(symbol_name, title).as_deref());
        self.title.setStringValue(&ns(title));
        self.detail.setStringValue(&ns(detail));
        for view in [
            self.icon.as_ref() as &NSView,
            self.title.as_ref(),
            self.detail.as_ref(),
            self.retry.as_ref(),
        ] {
            view.setHidden(false);
        }
        self.root.setHidden(false);
    }

    pub fn hide(&self) {
        unsafe { self.spinner.stopAnimation(None) };
        self.root.setHidden(true);
    }
}

/// Floating message near the bottom of the content area.
pub struct Notice {
    pub root: Retained<NSBox>,
    label: Retained<NSTextField>,
    pub close: Retained<NSButton>,
}

impl Notice {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let root = NSBox::new(mtm);
        root.setBoxType(NSBoxType::Custom);
        root.setTitlePosition(objc2_app_kit::NSTitlePosition::NoTitle);
        root.setCornerRadius(10.);
        root.setBorderWidth(1.);
        root.setBorderColor(&NSColor::separatorColor());
        root.setFillColor(&NSColor::windowBackgroundColor());
        root.setContentViewMargins(NSSize::new(0., 0.));
        root.setTranslatesAutoresizingMaskIntoConstraints(false);
        let label = NSTextField::wrappingLabelWithString(&ns(""), mtm);
        label.setPreferredMaxLayoutWidth(440.);
        label.setTranslatesAutoresizingMaskIntoConstraints(false);
        let close = unsafe { NSButton::buttonWithTitle_target_action(&ns(""), None, None, mtm) };
        close.setImage(symbol("xmark.circle.fill", "Dismiss").as_deref());
        close.setBordered(false);
        close.setContentTintColor(Some(&NSColor::tertiaryLabelColor()));
        close.setTranslatesAutoresizingMaskIntoConstraints(false);
        let content = NSView::new(mtm);
        content.addSubview(&label);
        content.addSubview(&close);
        root.setContentView(Some(&content));
        activate(&[
            label
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&content.leadingAnchor(), 14.),
            label
                .topAnchor()
                .constraintEqualToAnchor_constant(&content.topAnchor(), 10.),
            label
                .bottomAnchor()
                .constraintEqualToAnchor_constant(&content.bottomAnchor(), -10.),
            close
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&label.trailingAnchor(), 10.),
            close
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&content.trailingAnchor(), -10.),
            close
                .centerYAnchor()
                .constraintEqualToAnchor(&content.centerYAnchor()),
        ]);
        root.setHidden(true);
        Self { root, label, close }
    }

    pub fn show(&self, text: &str) {
        self.label.setStringValue(&ns(text));
        self.root.setHidden(false);
    }

    pub fn hide(&self) {
        self.root.setHidden(true);
    }
}

/// Side length of the Now Playing artwork, in points.
const ARTWORK_SIZE: f64 = 36.;

/// Sidebar card for what YouTube Music is playing: artwork (click to show the Music section),
/// title, artist, and previous / play-pause / next buttons.
pub struct NowPlayingCard {
    pub root: Retained<NSStackView>,
    pub artwork: Retained<NSButton>,
    title: Retained<NSTextField>,
    artist: Retained<NSTextField>,
    pub previous: Retained<NSButton>,
    pub play_pause: Retained<NSButton>,
    pub next: Retained<NSButton>,
    /// Pointer of the source image already clipped for the artwork button.
    /// Play/pause updates must not rasterize the cover again.
    artwork_token: Cell<usize>,
}

impl NowPlayingCard {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let artwork = NSButton::new(mtm);
        artwork.setBordered(false);
        artwork.setTitle(&ns(""));
        artwork.setImageScaling(NSImageScaling::ScaleNone);
        artwork.setToolTip(Some(&ns("Show YouTube Music")));
        artwork.setTranslatesAutoresizingMaskIntoConstraints(false);
        let title = label(mtm, "");
        title.setFont(Some(&NSFont::systemFontOfSize_weight(13., unsafe {
            NSFontWeightSemibold
        })));
        let artist = label(mtm, "");
        artist.setFont(Some(&NSFont::systemFontOfSize(11.)));
        artist.setTextColor(Some(&NSColor::secondaryLabelColor()));
        for field in [&title, &artist] {
            // Long names truncate instead of widening the sidebar.
            field.setContentCompressionResistancePriority_forOrientation(
                1.,
                NSLayoutConstraintOrientation::Horizontal,
            );
        }
        let labels = NSStackView::new(mtm);
        labels.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        labels.setAlignment(NSLayoutAttribute::Leading);
        labels.setSpacing(1.);
        labels.addArrangedSubview(&title);
        labels.addArrangedSubview(&artist);
        let info = NSStackView::new(mtm);
        info.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        info.setSpacing(8.);
        info.addArrangedSubview(&artwork);
        info.addArrangedSubview(&labels);

        // Skip stays quieter than play/pause. Both glyphs keep a fixed button frame,
        // so toggling play does not move the row.
        let skip_config =
            NSImageSymbolConfiguration::configurationWithPointSize_weight(13., unsafe {
                NSFontWeightRegular
            });
        let play_config =
            NSImageSymbolConfiguration::configurationWithPointSize_weight(15., unsafe {
                NSFontWeightSemibold
            });
        let control = |name: &str, description: &str, config: &NSImageSymbolConfiguration| {
            let button = NSButton::new(mtm);
            button.setBordered(false);
            button.setTitle(&ns(""));
            button.setImage(symbol(name, description).as_deref());
            button.setImageScaling(NSImageScaling::ScaleProportionallyDown);
            button.setSymbolConfiguration(Some(config));
            button.setToolTip(Some(&ns(description)));
            button.setTranslatesAutoresizingMaskIntoConstraints(false);
            button
        };
        let previous = control("backward.fill", "Previous", &skip_config);
        let play_pause = control("play.fill", "Play", &play_config);
        let next = control("forward.fill", "Next", &skip_config);
        previous.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
        next.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
        play_pause.setContentTintColor(Some(&NSColor::labelColor()));
        let controls = NSStackView::new(mtm);
        controls.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        controls.setSpacing(0.);
        controls.setAlignment(NSLayoutAttribute::CenterY);
        for button in [&previous, &play_pause, &next] {
            controls.addArrangedSubview(button);
        }

        let body = NSStackView::new(mtm);
        body.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        body.setAlignment(NSLayoutAttribute::CenterX);
        body.setSpacing(6.);
        body.setEdgeInsets(NSEdgeInsets {
            top: 8.,
            left: 0.,
            bottom: 8.,
            right: 0.,
        });
        body.addArrangedSubview(&info);
        body.addArrangedSubview(&controls);

        // Hairline only — no material, shadow, or fill. Those would composite over the
        // sidebar on every frame the window is visible.
        let rule = NSBox::new(mtm);
        rule.setBoxType(NSBoxType::Custom);
        rule.setTitlePosition(objc2_app_kit::NSTitlePosition::NoTitle);
        rule.setBorderWidth(0.);
        rule.setCornerRadius(0.);
        rule.setFillColor(&NSColor::separatorColor());
        rule.setContentHuggingPriority_forOrientation(
            1000.,
            NSLayoutConstraintOrientation::Vertical,
        );

        let root = NSStackView::new(mtm);
        root.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        root.setAlignment(NSLayoutAttribute::Leading);
        root.setSpacing(0.);
        root.addArrangedSubview(&rule);
        root.addArrangedSubview(&body);
        activate(&[
            artwork
                .widthAnchor()
                .constraintEqualToConstant(ARTWORK_SIZE),
            artwork
                .heightAnchor()
                .constraintEqualToConstant(ARTWORK_SIZE),
            info.leadingAnchor()
                .constraintEqualToAnchor_constant(&body.leadingAnchor(), 12.),
            info.trailingAnchor()
                .constraintEqualToAnchor_constant(&body.trailingAnchor(), -12.),
            previous.widthAnchor().constraintEqualToConstant(36.),
            previous.heightAnchor().constraintEqualToConstant(28.),
            play_pause.widthAnchor().constraintEqualToConstant(36.),
            play_pause.heightAnchor().constraintEqualToConstant(28.),
            next.widthAnchor().constraintEqualToConstant(36.),
            next.heightAnchor().constraintEqualToConstant(28.),
            rule.leadingAnchor()
                .constraintEqualToAnchor(&root.leadingAnchor()),
            rule.trailingAnchor()
                .constraintEqualToAnchor(&root.trailingAnchor()),
            rule.heightAnchor().constraintEqualToConstant(1.),
            body.leadingAnchor()
                .constraintEqualToAnchor(&root.leadingAnchor()),
            body.trailingAnchor()
                .constraintEqualToAnchor(&root.trailingAnchor()),
        ]);
        root.setHidden(true);
        Self {
            root,
            artwork,
            title,
            artist,
            previous,
            play_pause,
            next,
            artwork_token: Cell::new(usize::MAX),
        }
    }

    /// Shows the track; `artwork` is the downloaded cover, if any.
    pub fn show(&self, title: &str, artist: &str, playing: bool, artwork: Option<&NSImage>) {
        self.title.setStringValue(&ns(title));
        self.title.setToolTip(Some(&ns(title)));
        self.artist.setStringValue(&ns(artist));
        self.artist.setHidden(artist.is_empty());
        let token = artwork
            .map(|image| image as *const NSImage as usize)
            .unwrap_or(0);
        if token != self.artwork_token.get() {
            self.artwork_token.set(token);
            let image = match artwork {
                Some(artwork) => Some(rounded_image(artwork, ARTWORK_SIZE, 6.)),
                // A template symbol, so it takes the button's tint in light and dark mode.
                None => symbol("music.note", "No artwork"),
            };
            self.artwork.setImage(image.as_deref());
        }
        let (name, description) = if playing {
            ("pause.fill", "Pause")
        } else {
            ("play.fill", "Play")
        };
        self.play_pause
            .setImage(symbol(name, description).as_deref());
        self.play_pause.setToolTip(Some(&ns(description)));
        self.root.setHidden(false);
    }

    pub fn hide(&self) {
        self.root.setHidden(true);
    }
}
