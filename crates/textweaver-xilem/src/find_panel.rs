//! The find and replace panel (Ctrl+Shift+F in edit mode; the terminal
//! reader keeps its prompt-driven loop, Alt+F).
//!
//! One dialog, top to bottom: the Find what field, a status line under it
//! (an invalid pattern in words, or the match being asked about, "Match 2
//! of 5, line 12: teh becomes the"), the Replace with field, four check
//! boxes (Match case, Whole words, Regular expression, Across lines), and
//! the Find next, Replace, Replace all, and Close buttons. Tab moves
//! through them in that order; Enter in Find what finds the next match,
//! Enter in Replace with replaces it, Up and Down bring back earlier
//! answers in either field, F3 and Shift+F3 find again, and Escape closes
//! the panel.
//!
//! The panel holds no search of its own: the driver runs the app's model
//! (`Command::Find`, `Command::StartReplace`, `Command::ReplaceStep`,
//! `Command::SetSearchOptions`, `App::replace_preview`), so the window and
//! the terminal find, count, replace, and undo alike. In a window under
//! [`SHORT_HEIGHT`] the panel is compact, as the voice manager is.

use std::collections::VecDeque;

use masonry::core::{NewWidget, StyleProperty, Widget, WidgetId, WidgetTag};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, LineBreaking, Padding};
use masonry::widgets::{Checkbox, Flex, Label, SizedBox, TextArea, TextInput};
use textweaver_app::keymap::ActionId;
use textweaver_app::{App, SearchOptions};

use crate::dialog::{self, Modal};
use crate::gui::{dialog_title, label, shortcut_for};
use crate::theme::{self, Palette};
use crate::widgets::{ActionButton, FocusFrame, Message};

/// The Find what field.
pub const FIND_WHAT: WidgetTag<TextArea<true>> = WidgetTag::named("tw-find-what");
/// The Replace with field.
pub const FIND_WITH: WidgetTag<TextArea<true>> = WidgetTag::named("tw-find-with");
/// The status line under Find what: an invalid pattern, or the match
/// being asked about.
pub const FIND_STATUS: WidgetTag<Label> = WidgetTag::named("tw-find-status");

/// Below this window height, in logical pixels, the panel is compact.
pub const SHORT_HEIGHT: f64 = crate::voices::SHORT_HEIGHT;

/// The two fields' text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindTexts {
    /// The text or pattern to find.
    pub find: String,
    /// What each match becomes.
    pub with: String,
}

/// A button of the panel (Close is every dialog's own).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindButton {
    /// Find the next match (F3), or skip the one asked about.
    Next,
    /// Replace the match asked about; the first press finds it.
    Replace,
    /// Replace every match, after saying how many and asking once.
    ReplaceAll,
}

/// One of the search options, as a check box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindOption {
    /// Match case.
    MatchCase,
    /// Whole words only.
    WholeWords,
    /// The text to find is a regular expression.
    Regex,
    /// With a regular expression, `.` matches a line break too.
    AcrossLines,
}

impl FindOption {
    /// In the panel's order.
    pub const ALL: [FindOption; 4] = [
        FindOption::MatchCase,
        FindOption::WholeWords,
        FindOption::Regex,
        FindOption::AcrossLines,
    ];

    /// Whether the option is on in `o`.
    pub fn get(self, o: SearchOptions) -> bool {
        match self {
            FindOption::MatchCase => o.match_case,
            FindOption::WholeWords => o.whole_words,
            FindOption::Regex => o.regex,
            FindOption::AcrossLines => o.across_lines,
        }
    }

    /// `o` with this option `on` or off.
    pub fn set(self, mut o: SearchOptions, on: bool) -> SearchOptions {
        match self {
            FindOption::MatchCase => o.match_case = on,
            FindOption::WholeWords => o.whole_words = on,
            FindOption::Regex => o.regex = on,
            FindOption::AcrossLines => o.across_lines = on,
        }
        o
    }

    fn label_id(self) -> &'static str {
        match self {
            FindOption::MatchCase => "gui-find-match-case",
            FindOption::WholeWords => "gui-find-whole-words",
            FindOption::Regex => "gui-find-regex",
            FindOption::AcrossLines => "gui-find-across-lines",
        }
    }
}

/// The panel, built by [`find_dialog()`].
pub struct FindDialog {
    /// The dialog, for [`Root::set_dialog`](crate::widgets::Root::set_dialog).
    pub modal: NewWidget<dyn Widget>,
    /// The Find what field, which takes the focus.
    pub find: WidgetId,
    /// The Replace with field.
    pub with: WidgetId,
    /// The check boxes and their options.
    pub options: Vec<(WidgetId, FindOption)>,
    /// The buttons and what they do.
    pub buttons: Vec<(WidgetId, FindButton)>,
}

/// A labelled field: the drawn label (hidden from screen readers, which
/// hear the field's own name) and the field.
fn field_row(
    text: &str,
    initial: &str,
    tag: WidgetTag<TextArea<true>>,
    label_width: f64,
    size: f32,
) -> (NewWidget<Flex>, WidgetId) {
    let field = NewWidget::new(
        TextArea::new_editable(initial)
            .with_accessible_label(text.to_owned())
            .with_style(StyleProperty::FontSize(theme::ui_size(size))),
    )
    .with_tag(tag);
    let id = field.id();
    let input = NewWidget::new(FocusFrame::new(NewWidget::new(TextInput::from_text_area(
        field,
    ))));
    let shown = NewWidget::new(label(text, theme::UI_TEXT, false).accessibility_hidden(true));
    let row = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(NewWidget::new(
            SizedBox::new(shown).width(Length::px(label_width)),
        ))
        .with(input, 1.0);
    (NewWidget::new(row), id)
}

/// The panel for `app`, its fields holding `texts` and its check boxes
/// the app's search options; compact when `short` (a window under
/// [`SHORT_HEIGHT`], down to 420 by 320): the drawn title and hint are
/// left out, and the buttons hide their keys on screen and take less
/// padding. The order of the controls is the same either way.
pub fn find_dialog(p: &Palette, app: &App, texts: &FindTexts, short: bool) -> FindDialog {
    let c = app.catalog();
    let title = c.tr("gui-find-title");
    let label_width = if short { 104.0 } else { 128.0 };
    // The fields' text a step larger than the labels, as in a prompt,
    // except in the compact panel.
    let size = theme::UI_TEXT + if short { 0.0 } else { 2.0 };
    let (what, find) = field_row(
        &c.tr("gui-find-what"),
        &texts.find,
        FIND_WHAT,
        label_width,
        size,
    );
    let (with_row, with) = field_row(
        &c.tr("gui-find-with"),
        &texts.with,
        FIND_WITH,
        label_width,
        size,
    );
    let status = NewWidget::new(label(&status_text(app, &texts.find), theme::UI_TEXT, false))
        .with_props(LineBreaking::WordWrap)
        .with_tag(FIND_STATUS);
    // Two rows of two, which fit the smallest window.
    let opts = app.search_options();
    let mut options = Vec::new();
    let mut rows = Vec::new();
    for pair in FindOption::ALL.chunks(2) {
        let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
        for (i, o) in pair.iter().enumerate() {
            let b = NewWidget::new(Checkbox::from_label(
                o.get(opts),
                NewWidget::new(label(&c.tr(o.label_id()), theme::UI_TEXT, false)),
            ));
            options.push((b.id(), *o));
            if i > 0 {
                row = row.with_fixed_spacer(Length::px(16.0));
            }
            row = row.with_fixed(b);
        }
        rows.push(NewWidget::new(row));
    }
    // Compact: less padding, still well over 24 by 24 (WCAG 2.5.8), so
    // the four buttons share one row in the smallest window.
    let compact = |w: NewWidget<ActionButton>| {
        if short {
            w.with_props(Padding::from_vh(Length::px(5.0), Length::px(8.0)))
        } else {
            w
        }
    };
    let mut buttons = Vec::new();
    let mut make = |name: &str, help: &str, key: String, which: FindButton| {
        let b = compact(NewWidget::new(
            ActionButton::new(c.tr(name))
                .with_shortcut(key)
                .with_description(c.tr(help))
                .with_show_key(!short),
        ));
        buttons.push((b.id(), which));
        b
    };
    let next = make(
        "gui-find-next",
        "gui-find-next-help",
        shortcut_for(app, ActionId::FindNext),
        FindButton::Next,
    );
    let replace = make(
        "gui-find-replace",
        "gui-find-replace-help",
        String::new(),
        FindButton::Replace,
    );
    let all = make(
        "gui-find-replace-all",
        "gui-find-replace-all-help",
        String::new(),
        FindButton::ReplaceAll,
    );
    // Every dialog's Close: Escape is its key ([`crate::gui::DIALOG_CLOSE`]).
    let escape = textweaver_app::keymap::KeyChord::new(
        textweaver_app::keymap::Key::Escape,
        textweaver_app::keymap::Modifiers::empty(),
    );
    let close = compact(
        NewWidget::new(
            ActionButton::new(c.tr("gui-button-close"))
                .with_shortcut(escape.to_string())
                .with_show_key(!short),
        )
        .with_tag(crate::gui::DIALOG_CLOSE),
    );
    // The buttons wrap in a narrow window, as the window's bars do.
    let flow = crate::bars::Flow::new(vec![next, replace, all, close]);
    // The column's own gap between rows; the compact panel adds no more.
    let mut card = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
    if !short {
        card = card.with_fixed(dialog_title(&title));
    }
    card = card
        .with_fixed(what)
        .with_fixed(status)
        .with_fixed(with_row);
    if !short {
        card = card.with_fixed_spacer(Length::px(4.0));
    }
    for row in rows {
        card = card.with_fixed(row);
    }
    if !short {
        card = card.with_fixed_spacer(Length::px(4.0));
    }
    card = card.with_fixed(NewWidget::new(flow));
    if !short {
        card = card.with_fixed(
            NewWidget::new(
                label(&c.tr("gui-find-hint"), theme::UI_TEXT, false).accessibility_hidden(true),
            )
            .with_props(LineBreaking::WordWrap),
        );
    }
    let mut card = NewWidget::new(card).with_props(dialog::card_props(p));
    if short {
        card = card.with_props((Padding::all(Length::px(12.0)), Gap::new(Length::px(4.0))));
    }
    // F3, Shift+F3 and the Help key reach the driver from any control.
    let modal = NewWidget::new(Modal::new(card, title, p.clone()).with_app_chords(true)).erased();
    FindDialog {
        modal,
        find,
        with,
        options,
        buttons,
    }
}

/// What the status line under Find what shows: why the text to find
/// cannot be searched with the current options, or the match the replace
/// loop asks about, or nothing.
pub fn status_text(app: &App, find: &str) -> String {
    app.search_pattern_problem(find)
        .or_else(|| app.replace_preview())
        .unwrap_or_default()
}

/// Up (`up`) or Down in a field whose `history` is oldest first, from
/// position `at` (`None`: the text typed, not a recalled answer). Returns
/// the new position, or `None` when there is nowhere to go: Up stops at
/// the oldest answer, and Down past the newest goes back to the typed
/// text (`Some(None)`).
pub fn recall(history: &[String], at: Option<usize>, up: bool) -> Option<Option<usize>> {
    if history.is_empty() {
        return None;
    }
    match (at, up) {
        (None, true) => Some(Some(history.len() - 1)),
        (Some(0), true) | (None, false) => None,
        (Some(i), true) => Some(Some(i - 1)),
        (Some(i), false) if i + 1 < history.len() => Some(Some(i + 1)),
        (Some(_), false) => Some(None),
    }
}

/// The app says each match it asks about with the terminal's keys after
/// it ("Match 2 of 5, line 12: teh becomes the. The line: ... Press r to
/// replace, ..."); the panel has buttons instead. Each queued message
/// that starts with `preview` becomes the preview alone, said politely
/// (`keep`), or is dropped (Replace all, which asks its question next).
pub fn panel_messages(queue: &mut VecDeque<Message>, preview: &str, keep: bool) {
    if preview.is_empty() {
        return;
    }
    queue.retain_mut(|m| {
        if !m.text.starts_with(preview) {
            return true;
        }
        m.text = preview.to_owned();
        m.priority = textweaver_app::a11y::Priority::Polite;
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_app::a11y::Priority;

    #[test]
    fn up_and_down_walk_the_history_and_back_to_the_typed_text() {
        let h: Vec<String> = ["teh", "colour", "cats?"].map(String::from).to_vec();
        assert_eq!(recall(&h, None, true), Some(Some(2)));
        assert_eq!(recall(&h, Some(2), true), Some(Some(1)));
        assert_eq!(recall(&h, Some(0), true), None, "Up stops at the oldest");
        assert_eq!(recall(&h, Some(1), false), Some(Some(2)));
        assert_eq!(
            recall(&h, Some(2), false),
            Some(None),
            "back to the typed text"
        );
        assert_eq!(recall(&h, None, false), None);
        assert_eq!(recall(&[], None, true), None);
    }

    #[test]
    fn the_match_question_becomes_the_preview_alone() {
        let preview = "Match 2 of 5, line 12: teh becomes the";
        let msg = |text: &str| Message {
            text: text.to_owned(),
            priority: Priority::Assertive,
        };
        let mut q: VecDeque<Message> = [
            msg("Replaced 1."),
            msg(&format!(
                "{preview}. The line: teh cat. Press r to replace."
            )),
        ]
        .into();
        panel_messages(&mut q, preview, true);
        let texts: Vec<&str> = q.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["Replaced 1.", preview]);
        assert_eq!(q[1].priority, Priority::Polite);
        panel_messages(&mut q, preview, false);
        assert_eq!(q.len(), 1, "Replace all drops it before its question");
    }

    #[test]
    fn each_option_sets_its_own_field() {
        let mut o = SearchOptions::default();
        for f in FindOption::ALL {
            assert!(!f.get(o));
            o = f.set(o, true);
            assert!(f.get(o));
        }
        assert!(o.match_case && o.whole_words && o.regex && o.across_lines);
    }
}
