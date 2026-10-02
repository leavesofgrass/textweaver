//! The voice manager in the window (W7v): every engine's voices in one
//! list, with buttons for what the terminal does with rows and keys.
//!
//! The list is the app's voice list (`ListKind::Voices` in
//! `textweaver-app`, the model in `textweaver_app::voice_manager`), so
//! the window and the terminal list the same voices of every engine
//! (Eloquence, SAPI 5 and OneCore, DECtalk, eSpeak NG, Piper, and
//! Apple's on macOS), with the same labels, favorites (and the favorites
//! "not on this computer"), filters, and questions. The window shows the
//! filters and the fetch row as buttons instead of rows
//! (`App::set_voice_controls_in_list`), so the list holds only voices.
//!
//! From top to bottom: the Language and Engine filter buttons, the list
//! (a `ListBox` named "Voices"), then Use voice, Preview, Favorite, and
//! Remove, which act on the focused voice as Enter, the Say Status key,
//! Space, and Delete do in the list, then Fetch the Piper voice list
//! (when it can be fetched) and Close. Every button has a name, its key
//! as its keyboard shortcut, and a description.

use masonry::core::{NewWidget, Widget, WidgetId, WidgetTag};
use masonry::layout::Length;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::Flex;
use textweaver_app::App;
use textweaver_app::keymap::{ActionId, Key, KeyChord, Modifiers};
use textweaver_app::voice_manager::VoiceControls;

use crate::dialog::{self, ChoiceList, Modal};
use crate::gui::{LIST, label, shortcut_for};
use crate::theme::{self, Palette};
use crate::widgets::{ActionButton, Unavailable};

/// The Language filter button.
pub const VOICE_LANGUAGE: WidgetTag<ActionButton> = WidgetTag::named("tw-voices-language");
/// The Engine filter button.
pub const VOICE_ENGINE: WidgetTag<ActionButton> = WidgetTag::named("tw-voices-engine");
/// The Remove button, unavailable on a voice that cannot be removed.
pub const VOICE_REMOVE: WidgetTag<ActionButton> = WidgetTag::named("tw-voices-remove");

/// Why Remove is unavailable on row `row` of the voice list, in words, or
/// `None` when that voice can be removed (a downloaded Piper voice).
pub fn remove_unavailable(app: &App, row: usize) -> Option<Unavailable> {
    if app.voice_row_removable(row) {
        return None;
    }
    let c = app.catalog();
    Some(Unavailable {
        word: c.tr("gui-voices-remove-unavailable"),
        reason: c.tr("voice-only-piper-removable"),
    })
}

/// A button of the voice manager.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceButton {
    /// Show only the next language.
    Language,
    /// Show only the next engine.
    Engine,
    /// Use the focused voice (Enter in the list).
    Use,
    /// A sample in the focused voice (the Say Status key in the list).
    Preview,
    /// Mark or unmark a favorite (Space in the list).
    Favorite,
    /// Remove a downloaded Piper voice (Delete in the list).
    Remove,
    /// Fetch the Piper voice list.
    Fetch,
    /// Close the dialog (Escape).
    Close,
}

/// The voice manager, built by [`voice_dialog()`].
pub struct VoiceDialog {
    /// The dialog, for `Root::set_dialog`.
    pub modal: NewWidget<dyn Widget>,
    /// The list, which takes the focus.
    pub list: WidgetId,
    /// Each button's id and what it does.
    pub buttons: Vec<(WidgetId, VoiceButton)>,
}

impl VoiceDialog {
    /// What the button `id` does, if it is one of the dialog's.
    pub fn button(&self, id: WidgetId) -> Option<VoiceButton> {
        button_for(&self.buttons, id)
    }
}

/// What the button `id` among `buttons` does.
pub fn button_for(buttons: &[(WidgetId, VoiceButton)], id: WidgetId) -> Option<VoiceButton> {
    buttons.iter().find(|(b, _)| *b == id).map(|(_, b)| *b)
}

/// The key for `action` in the list, as written ("Alt+End"): a chord,
/// never the single key of single-key shortcuts (a letter jumps to a
/// voice in the list), else the main key.
pub fn list_shortcut(app: &App, action: ActionId) -> String {
    app.keymap()
        .chords_for(action)
        .into_iter()
        .find(|c| !c.is_text_input())
        .map_or_else(|| shortcut_for(app, action), |c| c.to_string())
}

/// A key the dialog's own buttons name, as written ("Enter").
fn plain_key(key: Key) -> String {
    KeyChord::new(key, Modifiers::empty()).to_string()
}

/// The voice manager for the app's voice list: its `title` and `items`
/// (the voices, as the app labels them), focused on `selected`.
pub fn voice_dialog(
    p: &Palette,
    app: &App,
    title: &str,
    items: Vec<String>,
    selected: usize,
) -> VoiceDialog {
    let c = app.catalog();
    let controls = app.voice_controls().unwrap_or_else(|| VoiceControls {
        language: c.fmt(
            "voices-language-row",
            &textweaver_app::lexicon::args!["language" => c.tr("voices-all-languages")],
        ),
        engine: c.fmt(
            "voices-engine-row",
            &textweaver_app::lexicon::args!["engine" => c.tr("voices-all-engines")],
        ),
        fetch: None,
    });
    let mut buttons = Vec::new();
    let mut make = |text: String, shortcut: String, help: &str, which: VoiceButton| {
        let w = NewWidget::new(
            ActionButton::new(text)
                .with_shortcut(shortcut)
                .with_description(c.tr(help)),
        );
        buttons.push((w.id(), which));
        w
    };
    let language = make(
        controls.language,
        String::new(),
        "gui-voices-language-help",
        VoiceButton::Language,
    )
    .with_tag(VOICE_LANGUAGE);
    let engine = make(
        controls.engine,
        String::new(),
        "gui-voices-engine-help",
        VoiceButton::Engine,
    )
    .with_tag(VOICE_ENGINE);
    let use_voice = make(
        c.tr("gui-voices-use"),
        plain_key(Key::Enter),
        "gui-voices-use-help",
        VoiceButton::Use,
    );
    let preview = make(
        c.tr("gui-voices-preview"),
        list_shortcut(app, ActionId::SayStatus),
        "gui-voices-preview-help",
        VoiceButton::Preview,
    );
    let favorite = make(
        c.tr("gui-voices-favorite"),
        plain_key(Key::Space),
        "gui-voices-favorite-help",
        VoiceButton::Favorite,
    );
    let fetch = controls.fetch.map(|text| {
        make(
            text,
            String::new(),
            "gui-voices-fetch-help",
            VoiceButton::Fetch,
        )
    });
    let close = make(
        c.tr("gui-button-close"),
        plain_key(Key::Escape),
        "gui-voices-close-help",
        VoiceButton::Close,
    );
    // Remove says when it is unavailable, and why (W8a).
    let remove = NewWidget::new(
        ActionButton::new(c.tr("gui-voices-remove"))
            .with_shortcut(plain_key(Key::Delete))
            .with_description(c.tr("gui-voices-remove-help"))
            .with_unavailable(remove_unavailable(app, selected)),
    )
    .with_tag(VOICE_REMOVE);
    buttons.push((remove.id(), VoiceButton::Remove));
    let list = NewWidget::new(
        ChoiceList::new(c.tr("gui-voices-list"), items, p.clone())
            .with_selected(selected)
            .with_app_keys(true),
    )
    .with_tag(LIST);
    let list_id = list.id();
    let gap = Length::px(10.0);
    let filters = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(language)
        .with_fixed_spacer(gap)
        .with_fixed(engine);
    let actions = Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_fixed(use_voice)
        .with_fixed_spacer(gap)
        .with_fixed(preview)
        .with_fixed_spacer(gap)
        .with_fixed(favorite)
        .with_fixed_spacer(gap)
        .with_fixed(remove);
    let mut footer = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center);
    if let Some(fetch) = fetch {
        footer = footer.with_fixed(fetch);
    }
    let footer = footer.with_spacer(1.0).with_fixed(close);
    let card = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_fixed(NewWidget::new(
            label(title, 20.0, true).accessibility_hidden(true),
        ))
        .with_fixed_spacer(Length::px(12.0))
        .with_fixed(NewWidget::new(filters))
        .with_fixed_spacer(gap)
        .with_fixed(list)
        .with_fixed_spacer(gap)
        .with_fixed(NewWidget::new(actions))
        .with_fixed_spacer(gap)
        .with_fixed(NewWidget::new(footer))
        .with_fixed_spacer(gap)
        .with_fixed(NewWidget::new(
            label(&c.tr("gui-voices-hint"), theme::UI_TEXT, false).accessibility_hidden(true),
        ));
    let card = NewWidget::new(card).with_props(dialog::card_props(p));
    // Keys pressed on a button (the Help and Say Status keys) reach the
    // app as they do from the list.
    let modal = NewWidget::new(
        Modal::new(card, title, p.clone())
            .with_max_width(900.0)
            .with_app_chords(true),
    )
    .erased();
    VoiceDialog {
        modal,
        list: list_id,
        buttons,
    }
}
