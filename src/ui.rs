//! Small AppKit builders for DooTube's native chrome.
use objc2::{
    MainThreadOnly, Message,
    rc::Retained,
    runtime::{AnyObject, Bool, Sel},
};
use objc2_app_kit::{
    NSBezierPath, NSBox, NSBoxType, NSButton, NSColor, NSControlSize, NSFont, NSFontWeightRegular,
    NSFontWeightSemibold, NSImage, NSImageScaling, NSImageSymbolConfiguration, NSImageView,
    NSLayoutConstraint, NSLineBreakMode, NSMenu, NSMenuItem, NSProgressIndicator,
    NSProgressIndicatorStyle, NSStackView, NSTableCellView, NSTextAlignment, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSPoint, NSRect, NSSize, NSString};

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
    activate(&[
        text.leadingAnchor()
            .constraintEqualToAnchor_constant(&cell.leadingAnchor(), 4.),
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
        icon.setImage(symbol("wifi.exclamationmark", "Connection problem").as_deref());
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
        placeholder.show_loading();
        placeholder
    }

    pub fn show_loading(&self) {
        for view in [
            self.icon.as_ref() as &NSView,
            self.title.as_ref(),
            self.detail.as_ref(),
            self.retry.as_ref(),
        ] {
            view.setHidden(true);
        }
        self.spinner.setHidden(false);
        unsafe { self.spinner.startAnimation(None) };
        self.root.setHidden(false);
    }

    pub fn show_failure(&self, title: &str, detail: &str) {
        unsafe { self.spinner.stopAnimation(None) };
        self.spinner.setHidden(true);
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
