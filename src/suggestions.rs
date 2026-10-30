//! Search suggestions: a dropdown under the bar's search field, filled as the user types.

use super::*;
use objc2_app_kit::{NSCellImagePosition, NSTextAlignment};

const MAX_ROWS: usize = 8;
const ROW_HEIGHT: f64 = 30.;

pub(crate) struct Suggestions {
    pub root: Retained<NSBox>,
    stack: Retained<NSStackView>,
    rows: RefCell<Vec<Retained<NSBox>>>,
    items: RefCell<Vec<String>>,
    selected: Cell<Option<usize>>,
    /// What the user typed; shown again when arrowing back above the list.
    typed: RefCell<String>,
}

impl Suggestions {
    /// Lays the (hidden) dropdown over `content`, just under `search` and as wide as it.
    pub fn new(mtm: MainThreadMarker, content: &NSView, search: &NSView) -> Self {
        let root = NSBox::new(mtm);
        root.setBoxType(NSBoxType::Custom);
        root.setTitlePosition(NSTitlePosition::NoTitle);
        root.setBorderWidth(0.);
        root.setCornerRadius(10.);
        root.setFillColor(&cinema_bar::raised_color());
        root.setContentViewMargins(NSSize::new(0., 0.));
        root.setHidden(true);
        root.setTranslatesAutoresizingMaskIntoConstraints(false);
        let stack = NSStackView::new(mtm);
        stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        stack.setSpacing(0.);
        stack.setEdgeInsets(objc2_foundation::NSEdgeInsets {
            top: 6.,
            left: 6.,
            bottom: 6.,
            right: 6.,
        });
        stack.setTranslatesAutoresizingMaskIntoConstraints(false);
        root.addSubview(&stack);
        content.addSubview(&root);
        ui::activate(&[
            root.topAnchor()
                .constraintEqualToAnchor_constant(&search.bottomAnchor(), 6.),
            root.leadingAnchor()
                .constraintEqualToAnchor(&search.leadingAnchor()),
            root.trailingAnchor()
                .constraintEqualToAnchor(&search.trailingAnchor()),
            stack.topAnchor().constraintEqualToAnchor(&root.topAnchor()),
            stack
                .bottomAnchor()
                .constraintEqualToAnchor(&root.bottomAnchor()),
            stack
                .leadingAnchor()
                .constraintEqualToAnchor(&root.leadingAnchor()),
            stack
                .trailingAnchor()
                .constraintEqualToAnchor(&root.trailingAnchor()),
        ]);
        Self {
            root,
            stack,
            rows: RefCell::new(Vec::new()),
            items: RefCell::new(Vec::new()),
            selected: Cell::new(None),
            typed: RefCell::new(String::new()),
        }
    }

    pub fn is_visible(&self) -> bool {
        !self.root.isHidden()
    }

    pub fn hide(&self) {
        self.root.setHidden(true);
        self.selected.set(None);
    }

    fn select(&self, row: Option<usize>) {
        self.selected.set(row);
        for (index, view) in self.rows.borrow().iter().enumerate() {
            let fill = if row == Some(index) {
                NSColor::colorWithWhite_alpha(1., 0.1)
            } else {
                NSColor::clearColor()
            };
            view.setFillColor(&fill);
        }
    }
}

impl AppDelegate {
    /// The search text changed: ask YouTube for suggestions, or close the list when it's empty.
    pub(crate) fn search_text_changed(&self, text: String) {
        let Some(list) = self.ivars().suggestions.get() else {
            return;
        };
        if text.trim().is_empty() {
            list.hide();
        } else {
            // ponytail: one request per keystroke; replies for older text are dropped on arrival.
            load_image(core::suggestions_url(&text).into(), |this, _, bytes| {
                this.on_suggestions(&bytes)
            });
        }
        *list.typed.borrow_mut() = text;
    }

    fn on_suggestions(&self, bytes: &[u8]) {
        let ivars = self.ivars();
        let (Some(list), Some(bar)) = (ivars.suggestions.get(), ivars.bar.get()) else {
            return;
        };
        let Some((query, mut items)) = core::parse_suggestions(bytes) else {
            return;
        };
        // Only while still typing the text this reply answers.
        if bar.search.currentEditor().is_none() || *list.typed.borrow() != query {
            return;
        }
        items.truncate(MAX_ROWS);
        let mtm = self.mtm();
        for row in list.rows.borrow_mut().drain(..) {
            row.removeFromSuperview();
        }
        let mut rows = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let button = NSButton::new(mtm);
            button.setBordered(false);
            button.setTitle(&ns(item));
            button.setFont(Some(&NSFont::systemFontOfSize(14.)));
            button.setAlignment(NSTextAlignment::Left);
            button.setImage(ui::symbol("magnifyingglass", "Search").as_deref());
            button.setImagePosition(NSCellImagePosition::ImageLeading);
            button.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
            // Clicking must not take focus from the search field.
            button.setRefusesFirstResponder(true);
            button.setTag(index as NSInteger);
            unsafe {
                button.setTarget(Some(self));
                button.setAction(Some(sel!(suggestionClicked:)));
            }
            let row = NSBox::new(mtm);
            row.setBoxType(NSBoxType::Custom);
            row.setTitlePosition(NSTitlePosition::NoTitle);
            row.setBorderWidth(0.);
            row.setCornerRadius(6.);
            row.setContentViewMargins(NSSize::new(8., 0.));
            row.setContentView(Some(&button));
            row.setTranslatesAutoresizingMaskIntoConstraints(false);
            list.stack.addArrangedSubview(&row);
            ui::activate(&[
                row.heightAnchor().constraintEqualToConstant(ROW_HEIGHT),
                row.widthAnchor()
                    .constraintEqualToAnchor_constant(&list.stack.widthAnchor(), -12.),
            ]);
            rows.push(row);
        }
        *list.rows.borrow_mut() = rows;
        list.root.setHidden(items.is_empty());
        *list.items.borrow_mut() = items;
        list.select(None);
    }

    /// Arrow keys move through the list, putting the suggestion in the field as YouTube does;
    /// Escape closes it. Returns whether the key was handled.
    pub(crate) fn search_command(&self, command: Sel) -> bool {
        let ivars = self.ivars();
        let (Some(list), Some(bar)) = (ivars.suggestions.get(), ivars.bar.get()) else {
            return false;
        };
        if !list.is_visible() {
            return false;
        }
        let count = list.items.borrow().len();
        let next = match (command, list.selected.get()) {
            (c, None) if c == sel!(moveDown:) => Some(0),
            (c, Some(row)) if c == sel!(moveDown:) => Some((row + 1).min(count - 1)),
            (c, Some(0)) if c == sel!(moveUp:) => None,
            (c, Some(row)) if c == sel!(moveUp:) => Some(row - 1),
            (c, None) if c == sel!(moveUp:) => return true,
            (c, _) if c == sel!(cancelOperation:) => {
                list.hide();
                return true;
            }
            (c, _) if c == sel!(insertNewline:) => {
                list.hide();
                return false;
            }
            _ => return false,
        };
        list.select(next);
        let text = next.map_or_else(
            || list.typed.borrow().clone(),
            |row| list.items.borrow()[row].clone(),
        );
        bar.search.setStringValue(&ns(&text));
        if let Some(editor) = bar.search.currentEditor() {
            editor.setSelectedRange(objc2_foundation::NSRange::new(
                text.encode_utf16().count(),
                0,
            ));
        }
        true
    }

    pub(crate) fn hide_suggestions(&self) {
        if let Some(list) = self.ivars().suggestions.get() {
            list.hide();
        }
    }

    pub(crate) fn open_suggestion(&self, row: usize) {
        let ivars = self.ivars();
        let (Some(list), Some(bar)) = (ivars.suggestions.get(), ivars.bar.get()) else {
            return;
        };
        let Some(text) = list.items.borrow().get(row).cloned() else {
            return;
        };
        bar.search.setStringValue(&ns(&text));
        self.action_search(sel!(searchSubmitted:), &bar.search);
    }
}
