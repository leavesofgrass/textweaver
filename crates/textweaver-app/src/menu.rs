//! Menus from one model (Wave 6, W6u; ADR-0043).
//!
//! The terminal reader, the GUI, and JSON-RPC build their menus from the
//! same model: File, Edit, View, Reading, Speech, Tools, and Help, each an
//! ordered list of commands ([`ActionId`]), submenus, and separators. Every
//! label comes from the catalog in the interface's language (`menu-*` for
//! menus, `name-*` for commands, the same short names the command palette
//! shows), and every shortcut is read from the live keymap, so a key
//! changed in `keymap.toml` shows at once and a menu can never advertise a
//! key that is not bound.
//!
//! - **Access keys** are chosen per menu and per language: a label may
//!   mark its letter with `&` (`E&xport as`); the rest take the first free
//!   letter of a word, then any free letter. No two items of one menu
//!   share one (a test, in six languages), and the top menus never take a
//!   letter a GUI `Alt` chord uses, so `Alt+O` stays the outline.
//! - **Toggles and choices** show their state from the settings or the
//!   live mode, never from a stored flag: edit mode, RSVP, and Speech
//!   Cursor say "checked" only while they are on.
//! - **Commands other modules provide** (browse files, batch conversion,
//!   audio export, dictation) have their ids and menu entries here. Until a
//!   module registers a handler with [`App::register_handler`], the command
//!   is left out of the menus and the command palette, so nothing ever
//!   offers what does not work ([`PENDING`]).
//! - **The terminal** shows the menus as a list on F10 ("Menus, 1 of 7,
//!   File"): Enter or Right opens a menu, a letter moves to the item with
//!   that access key, Enter runs a command, Left or Backspace goes up, and
//!   Escape closes. The GUI builds native menus from [`App::menu_bar`]
//!   (W6a6).
//! - **Recent commands**: the last eight run from a menu or the command
//!   palette, listed first in an empty palette.

use std::path::PathBuf;
use std::sync::LazyLock;

use textweaver_a11y::Priority;
use textweaver_keymap::{ActionId, KeyChord, Keymap};
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;

use crate::app::{App, ListKind, Mode};
use crate::command::{Command, Effect};
use crate::list_model::ListKey;

/// A command other modules implement, run by [`App::register_handler`].
pub type Handler = fn(&mut App) -> Vec<Effect>;

/// Commands whose ids and menu entries exist before their modules do:
/// browse files (W6f), batch conversion (W6k), audio export (W6v), and
/// dictation (W6d). Each is hidden from the menus and the command palette
/// until its module registers a handler.
pub const PENDING: &[ActionId] = &[
    ActionId::BrowseFiles,
    ActionId::BatchConvert,
    ActionId::ExportAudio,
    ActionId::Dictate,
    ActionId::DownloadDictationModel,
];

/// Commands that are deliberately in no menu: the key that opens the
/// menus.
pub const NOT_IN_MENUS: &[ActionId] = &[ActionId::Menu];

/// How many recent commands are kept.
pub const RECENT_COMMANDS: usize = 8;

/// How many recent documents the File menu lists.
pub const RECENT_DOCUMENTS: usize = 8;

/// A menu or submenu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuId {
    /// File.
    File,
    /// Edit.
    Edit,
    /// View.
    View,
    /// Reading.
    Reading,
    /// Speech.
    Speech,
    /// Tools.
    Tools,
    /// Help.
    Help,
    /// File, Recent documents.
    Recent,
    /// File, Export as.
    ExportAs,
    /// File, Preview.
    Preview,
    /// File, Settings.
    SettingsFiles,
    /// Edit, Find.
    Find,
    /// Edit, Format.
    Format,
    /// Edit, Insert.
    Insert,
    /// Edit, Proofing.
    Proofing,
    /// Edit, Citations.
    Citations,
    /// View, Text size.
    TextSize,
    /// View, Reading aids.
    ReadingAids,
    /// View, Reading aids, RSVP.
    Rsvp,
    /// Reading, Say.
    Say,
    /// Reading, Move by.
    MoveBy,
    /// Reading, Headings.
    Headings,
    /// Reading, Go to.
    GoTo,
    /// Reading, Cursor and selection.
    Cursor,
    /// Reading, Bookmarks and notes.
    Bookmarks,
    /// Reading, Tables.
    Tables,
    /// Reading, Speech Cursor.
    SpeechCursor,
    /// Tools, Sync.
    Sync,
}

/// One entry of a menu's definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Do(ActionId),
    Sub(MenuId),
    Sep,
    RecentDocuments,
}

use ActionId as A;
use Entry::{Do, Sep, Sub};

impl MenuId {
    /// The menu bar, in order.
    pub const TOP: [MenuId; 7] = [
        MenuId::File,
        MenuId::Edit,
        MenuId::View,
        MenuId::Reading,
        MenuId::Speech,
        MenuId::Tools,
        MenuId::Help,
    ];

    /// Every menu and submenu.
    pub const ALL: [MenuId; 28] = [
        MenuId::File,
        MenuId::Edit,
        MenuId::View,
        MenuId::Reading,
        MenuId::Speech,
        MenuId::Tools,
        MenuId::Help,
        MenuId::Recent,
        MenuId::ExportAs,
        MenuId::Preview,
        MenuId::SettingsFiles,
        MenuId::Find,
        MenuId::Format,
        MenuId::Insert,
        MenuId::Proofing,
        MenuId::Citations,
        MenuId::TextSize,
        MenuId::ReadingAids,
        MenuId::Rsvp,
        MenuId::Say,
        MenuId::MoveBy,
        MenuId::Headings,
        MenuId::GoTo,
        MenuId::Cursor,
        MenuId::Bookmarks,
        MenuId::Tables,
        MenuId::SpeechCursor,
        MenuId::Sync,
    ];

    /// The stable id, used in catalog ids (`menu-export-as`) and JSON-RPC.
    pub fn id(self) -> &'static str {
        match self {
            MenuId::File => "file",
            MenuId::Edit => "edit",
            MenuId::View => "view",
            MenuId::Reading => "reading",
            MenuId::Speech => "speech",
            MenuId::Tools => "tools",
            MenuId::Help => "help",
            MenuId::Recent => "recent",
            MenuId::ExportAs => "export-as",
            MenuId::Preview => "preview",
            MenuId::SettingsFiles => "settings",
            MenuId::Find => "find",
            MenuId::Format => "format",
            MenuId::Insert => "insert",
            MenuId::Proofing => "proofing",
            MenuId::Citations => "citations",
            MenuId::TextSize => "text-size",
            MenuId::ReadingAids => "reading-aids",
            MenuId::Rsvp => "rsvp",
            MenuId::Say => "say",
            MenuId::MoveBy => "move-by",
            MenuId::Headings => "headings",
            MenuId::GoTo => "go-to",
            MenuId::Cursor => "cursor",
            MenuId::Bookmarks => "bookmarks",
            MenuId::Tables => "tables",
            MenuId::SpeechCursor => "speech-cursor",
            MenuId::Sync => "sync",
        }
    }

    /// The menu for a stable id.
    pub fn from_id(id: &str) -> Option<MenuId> {
        Self::ALL.into_iter().find(|m| m.id() == id)
    }

    /// True for the seven menus of the menu bar.
    pub fn is_top(self) -> bool {
        Self::TOP.contains(&self)
    }

    fn entries(self) -> &'static [Entry] {
        match self {
            MenuId::File => &[
                Do(A::Open),
                Do(A::OpenPath),
                Do(A::BrowseFiles),
                Do(A::OpenLibrary),
                Do(A::AddLibraryFolder),
                Do(A::ContinueReading),
                Do(A::EditDocumentDetails),
                Sub(MenuId::Recent),
                Sep,
                Do(A::NewDocument),
                Do(A::NewFromTemplate),
                Do(A::Save),
                Do(A::SaveAs),
                Sep,
                Sub(MenuId::ExportAs),
                Do(A::ExportAudio),
                Do(A::BatchConvert),
                Sub(MenuId::Preview),
                Sep,
                Sub(MenuId::SettingsFiles),
                Sep,
                Do(A::Quit),
            ],
            MenuId::Recent => &[Entry::RecentDocuments],
            MenuId::ExportAs => &[
                Do(A::ExportPdf),
                Do(A::ExportHtml),
                Do(A::ExportDocx),
                Do(A::ExportEpub),
                Do(A::ExportBrf),
                Sep,
                Do(A::ExportStudySheet),
            ],
            MenuId::Preview => &[
                Do(A::PreviewInBrowser),
                Do(A::TogglePreviewAutoReload),
                Do(A::TogglePreviewLive),
            ],
            MenuId::SettingsFiles => &[
                Do(A::Settings),
                Do(A::SettingsProfiles),
                Sep,
                Do(A::ExportSettings),
                Do(A::ImportSettings),
            ],
            MenuId::Edit => &[
                Do(A::ToggleEditMode),
                Do(A::Undo),
                Do(A::Redo),
                Sep,
                Do(A::Cut),
                Do(A::Copy),
                Do(A::Paste),
                Do(A::SelectAll),
                Do(A::DeleteWordBefore),
                Do(A::DeleteWordAfter),
                Sep,
                Sub(MenuId::Find),
                Sub(MenuId::Format),
                Sub(MenuId::Insert),
                Sub(MenuId::Proofing),
                Sub(MenuId::Citations),
                Sep,
                Do(A::Dictate),
            ],
            MenuId::Find => &[
                Do(A::Find),
                Do(A::FindNext),
                Do(A::FindPrevious),
                Do(A::Replace),
                Do(A::SearchOptions),
            ],
            MenuId::Format => &[
                Do(A::Bold),
                Do(A::Italic),
                Do(A::Underline),
                Do(A::Strikethrough),
                Do(A::InlineCode),
                Do(A::CodeBlock),
                Sep,
                Do(A::Heading),
                Do(A::BulletList),
                Do(A::NumberedList),
                Do(A::BlockQuote),
            ],
            MenuId::Insert => &[
                Do(A::InsertLink),
                Do(A::InsertImage),
                Do(A::InsertTable),
                Do(A::AddTableRow),
                Do(A::NextTableCell),
                Do(A::PreviousTableCell),
                Do(A::HorizontalRule),
            ],
            MenuId::Proofing => &[
                Do(A::NextMisspelling),
                Do(A::PreviousMisspelling),
                Do(A::SpellingSuggestions),
                Sep,
                Do(A::NextGrammarProblem),
                Do(A::PreviousGrammarProblem),
                Sep,
                Do(A::NextLintProblem),
                Do(A::PreviousLintProblem),
            ],
            MenuId::Citations => &[
                Do(A::InsertCitation),
                Do(A::InsertBibliography),
                Do(A::CheckCitations),
                Sep,
                Do(A::AddReference),
                Do(A::ImportReferences),
            ],
            MenuId::View => &[
                Do(A::ContentsPanel),
                Do(A::NotesPanel),
                Do(A::NextRegion),
                Do(A::PreviousRegion),
                Do(A::ToggleHeader),
                Do(A::ToggleToolbar),
                Sep,
                Do(A::NextTheme),
                Do(A::ColorSettings),
                Do(A::ReadingForm),
                Sub(MenuId::TextSize),
                Do(A::ChooseFont),
                Do(A::ToggleLineNumbers),
                Do(A::ShowOriginalBraille),
                Sub(MenuId::ReadingAids),
                Sep,
                Do(A::CycleAccessMode),
                Do(A::ToggleCharacterKeys),
            ],
            MenuId::TextSize => &[Do(A::TextLarger), Do(A::TextSmaller), Do(A::TextSizeReset)],
            MenuId::ReadingAids => &[
                Do(A::BionicToggle),
                Do(A::RulerCycle),
                Do(A::SyllablesToggle),
                Do(A::DifficultWordsToggle),
                Sub(MenuId::Rsvp),
            ],
            MenuId::Rsvp => &[
                Do(A::RsvpToggle),
                Do(A::RsvpPlayPause),
                Do(A::RsvpFaster),
                Do(A::RsvpSlower),
                Do(A::RsvpPositionNext),
            ],
            MenuId::Reading => &[
                Do(A::PlayPause),
                Do(A::Stop),
                Do(A::ReadFromCursor),
                Do(A::ReadDocument),
                Do(A::ReplaySentence),
                Do(A::RepeatSentenceSlower),
                Do(A::ReplayParagraph),
                Do(A::ReadingPass),
                Sep,
                Sub(MenuId::Say),
                Sub(MenuId::MoveBy),
                Sub(MenuId::Headings),
                Sub(MenuId::GoTo),
                Sub(MenuId::Cursor),
                Sub(MenuId::Bookmarks),
                Sub(MenuId::Tables),
                Sub(MenuId::SpeechCursor),
                Sep,
                Do(A::ExploreMath),
                Do(A::ToggleCitations),
                Do(A::ListenRendered),
            ],
            MenuId::Say => &[
                Do(A::ReadCurrentCharacter),
                Do(A::ReadCurrentWord),
                Do(A::ReadCurrentSentence),
                Do(A::ReadCurrentLine),
                Do(A::ReadParagraph),
                Do(A::ReadSelection),
                Sep,
                Do(A::SayPosition),
                Do(A::DocumentOverview),
                Do(A::SayStatus),
                Do(A::RepeatMessage),
                Do(A::WordCount),
                Do(A::LinkAddress),
            ],
            MenuId::MoveBy => &[
                Do(A::NextSentence),
                Do(A::PreviousSentence),
                Do(A::NextParagraph),
                Do(A::PreviousParagraph),
                Do(A::NextChapter),
                Do(A::PreviousChapter),
                Do(A::NextList),
                Do(A::PreviousList),
                Do(A::NextListItem),
                Do(A::PreviousListItem),
                Do(A::NextLink),
                Do(A::PreviousLink),
                Do(A::NextBlockQuote),
                Do(A::PreviousBlockQuote),
                Do(A::NextSeparator),
                Do(A::PreviousSeparator),
                Do(A::NextGraphic),
                Do(A::PreviousGraphic),
            ],
            MenuId::Headings => &[
                Do(A::SkipNextHeading),
                Do(A::SkipPreviousHeading),
                Do(A::NextHeading),
                Do(A::PreviousHeading),
                Do(A::Outline),
                Sep,
                Do(A::NextHeadingLevel1),
                Do(A::NextHeadingLevel2),
                Do(A::NextHeadingLevel3),
                Do(A::NextHeadingLevel4),
                Do(A::NextHeadingLevel5),
                Do(A::NextHeadingLevel6),
                Sep,
                Do(A::PreviousHeadingLevel1),
                Do(A::PreviousHeadingLevel2),
                Do(A::PreviousHeadingLevel3),
                Do(A::PreviousHeadingLevel4),
                Do(A::PreviousHeadingLevel5),
                Do(A::PreviousHeadingLevel6),
            ],
            MenuId::GoTo => &[
                Do(A::GoTo),
                Do(A::DocumentStart),
                Do(A::DocumentEnd),
                Sep,
                Do(A::HistoryBack),
                Do(A::HistoryForward),
                Do(A::FollowLink),
                Sep,
                Do(A::PageDown),
                Do(A::PageUp),
                Do(A::ScrollDown),
                Do(A::ScrollUp),
            ],
            MenuId::Cursor => &[
                Do(A::CaretNextWord),
                Do(A::CaretPreviousWord),
                Do(A::CaretNextLine),
                Do(A::CaretPreviousLine),
                Sep,
                Do(A::SelectNextWord),
                Do(A::SelectPreviousWord),
                Do(A::SelectNextLine),
                Do(A::SelectPreviousLine),
            ],
            MenuId::Bookmarks => &[
                Do(A::AddBookmark),
                Do(A::ListBookmarks),
                Do(A::NextBookmark),
                Do(A::PreviousBookmark),
                Sep,
                Do(A::AddNote),
                Do(A::ListNotes),
                Do(A::NextNote),
                Do(A::PreviousNote),
                Do(A::HighlightSelection),
                Do(A::DeleteNote),
                Sep,
                Do(A::ListChanges),
                Do(A::AcceptAllChanges),
                Do(A::RejectAllChanges),
                Do(A::AddComment),
                Do(A::SelfTest),
            ],
            MenuId::Tables => &[
                Do(A::NextTable),
                Do(A::PreviousTable),
                Sep,
                Do(A::TableNextRow),
                Do(A::TablePreviousRow),
                Do(A::TableNextColumn),
                Do(A::TablePreviousColumn),
            ],
            MenuId::SpeechCursor => &[
                Do(A::SpeechCursorToggle),
                Do(A::SpeechCursorNextLine),
                Do(A::SpeechCursorPreviousLine),
                Do(A::SpeechCursorRereadLine),
                Do(A::SpeechCursorExitAndRead),
            ],
            MenuId::Speech => &[
                Do(A::ChooseVoice),
                Sep,
                Do(A::RateUp),
                Do(A::RateDown),
                Do(A::CycleSpeedPreset),
                Do(A::PitchUp),
                Do(A::PitchDown),
                Do(A::VolumeUp),
                Do(A::VolumeDown),
                Sep,
                Do(A::CycleVerbosity),
                Do(A::CyclePunctuation),
                Do(A::CycleTypingEcho),
                Do(A::CycleInterfaceAnnouncements),
                Sep,
                Do(A::RestartSpeech),
            ],
            MenuId::Tools => &[
                Do(A::CommandPalette),
                Sep,
                Do(A::DefineWord),
                Do(A::Summarize),
                Do(A::ReadingLevel),
                Do(A::ReadingStatistics),
                Sep,
                Sub(MenuId::Sync),
                Sep,
                Do(A::ManageComponents),
                Do(A::DownloadDictationModel),
                Do(A::AskFirstRunAgain),
            ],
            MenuId::Sync => &[
                Do(A::SyncSetup),
                Do(A::SyncStatus),
                Do(A::SyncNow),
                Sep,
                Do(A::SyncGoToPlace),
                Do(A::SyncReplacedNotes),
                Sep,
                Do(A::SyncStop),
            ],
            MenuId::Help => &[
                Do(A::Help),
                Do(A::KeyboardHelp),
                Do(A::WhatDoesThisKeyDo),
                Sep,
                Do(A::QuickStart),
                Do(A::Documentation),
                Do(A::ReportProblem),
                Sep,
                Do(A::About),
            ],
        }
    }
}

/// The setting a command toggles or cycles, whose value its menu item
/// shows ("checked", or "Reading ruler: current line").
pub fn bound_setting(a: ActionId) -> Option<&'static str> {
    Some(match a {
        A::BionicToggle => "reading_aids.bionic",
        A::SyllablesToggle => "reading_aids.syllables",
        A::DifficultWordsToggle => "reading_aids.difficult_words",
        A::RulerCycle => "reading_aids.ruler.mode",
        A::ToggleLineNumbers => "display.show_line_numbers",
        A::ToggleCharacterKeys => "keyboard.character_keys",
        A::CycleAccessMode => "accessibility.mode",
        A::CycleInterfaceAnnouncements => "accessibility.interface_announcements",
        A::CycleVerbosity => "speech.verbosity",
        A::CyclePunctuation => "speech.punctuation",
        A::ToggleCitations => "reading.citations",
        A::TogglePreviewAutoReload => "preview.auto_reload",
        A::TogglePreviewLive => "preview.live",
        A::ToggleHeader => "gui.header",
        A::ToggleToolbar => "gui.toolbar",
        _ => return None,
    })
}

/// What a menu item is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItemKind {
    /// Runs a command.
    Action(ActionId),
    /// Opens a submenu.
    Submenu(MenuId),
    /// A line between groups.
    Separator,
    /// Opens a recent document.
    Document(PathBuf),
    /// Says there is nothing here (no recent documents).
    Empty,
}

/// One menu item as a frontend shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// What it is.
    pub kind: MenuItemKind,
    /// The label, without the access key's mark.
    pub label: String,
    /// The access key: a letter or digit of the label, lowercase.
    pub access: Option<char>,
    /// The main key, written ("Ctrl+O"), from the live keymap.
    pub keys: Option<String>,
    /// For a toggle, whether it is on.
    pub checked: Option<bool>,
    /// For a choice, its value ("current line").
    pub value: Option<String>,
}

impl MenuItem {
    /// The label with its access key marked with `&` (a literal `&` is
    /// doubled), as native menus take it.
    pub fn marked_label(&self) -> String {
        mark_access(&self.label, self.access)
    }

    /// The item as one line, meaning first, as the terminal shows and says
    /// it: "Open, Ctrl+O", "Bionic reading, checked, Alt+Shift+B",
    /// "Reading ruler: current line, Alt+Shift+U", "Export as, submenu".
    pub fn line(&self, c: &Catalog) -> String {
        let mut s = match (&self.kind, self.checked, &self.value) {
            (MenuItemKind::Submenu(_), _, _) => {
                return c.fmt("menu-submenu", &args!["name" => self.label.as_str()]);
            }
            (_, Some(true), _) => c.fmt("menu-checked", &args!["name" => self.label.as_str()]),
            (_, Some(false), _) => c.fmt("menu-not-checked", &args!["name" => self.label.as_str()]),
            (_, None, Some(v)) => c.fmt(
                "menu-value",
                &args!["name" => self.label.as_str(), "value" => v.as_str()],
            ),
            _ => self.label.clone(),
        };
        if let Some(k) = &self.keys {
            s = c.fmt("menu-with-keys", &args!["item" => s, "keys" => k.as_str()]);
        }
        s
    }
}

/// A menu as a frontend shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuView {
    /// Which menu.
    pub id: MenuId,
    /// Its title, without the access key's mark.
    pub title: String,
    /// Its access key among the menus beside it.
    pub access: Option<char>,
    /// Its items, in order.
    pub items: Vec<MenuItem>,
}

impl MenuView {
    /// The title with its access key marked with `&`.
    pub fn marked_title(&self) -> String {
        mark_access(&self.title, self.access)
    }
}

fn mark_access(label: &str, access: Option<char>) -> String {
    let mut out = String::with_capacity(label.len() + 1);
    let mut done = access.is_none();
    for ch in label.chars() {
        if !done && ch.to_lowercase().eq(access) {
            out.push('&');
            done = true;
        }
        if ch == '&' {
            out.push('&');
        }
        out.push(ch);
    }
    out
}

/// Splits a catalog label into its text and the letter marked with `&`
/// (`&&` is a literal ampersand).
pub fn split_access(label: &str) -> (String, Option<char>) {
    let mut out = String::with_capacity(label.len());
    let mut access = None;
    let mut chars = label.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            match chars.peek() {
                Some('&') => {
                    out.push('&');
                    chars.next();
                }
                Some(&next) if access.is_none() && next.is_alphanumeric() => {
                    access = next.to_lowercase().next();
                }
                _ => {}
            }
            continue;
        }
        out.push(ch);
    }
    (out, access)
}

/// Chooses access keys for `labels` (each with the letter its catalog
/// label marks, if any): marked letters first, then the first free letter
/// starting a word, then any free letter or digit. Letters in `reserved`
/// are never taken.
pub fn assign_access_keys(
    labels: &[(String, Option<char>)],
    reserved: &[char],
) -> Vec<Option<char>> {
    let mut taken: Vec<char> = reserved.to_vec();
    let mut out: Vec<Option<char>> = vec![None; labels.len()];
    for (i, (_, marked)) in labels.iter().enumerate() {
        if let Some(m) = marked
            && !taken.contains(m)
        {
            taken.push(*m);
            out[i] = Some(*m);
        }
    }
    for (i, (label, _)) in labels.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        let lower: Vec<char> = label.to_lowercase().chars().collect();
        let word_starts = lower.iter().enumerate().filter_map(|(k, ch)| {
            let starts = k == 0 || !lower[k - 1].is_alphanumeric();
            (starts && ch.is_alphanumeric()).then_some(*ch)
        });
        let any = lower.iter().copied().filter(|ch| ch.is_alphanumeric());
        if let Some(ch) = word_starts.chain(any).find(|ch| !taken.contains(ch)) {
            taken.push(ch);
            out[i] = Some(ch);
        }
    }
    out
}

/// A command's short name in the catalog's language: "Export PDF" (the
/// `name-*` message, without its access key's mark). The command palette
/// and the menus both use it; the id stays typeable.
pub fn action_name(c: &Catalog, a: ActionId) -> String {
    split_access(&action_label(c, a)).0
}

/// A command's name as the catalog has it, access mark included.
fn action_label(c: &Catalog, a: ActionId) -> String {
    let id = format!("name-{}", a.id().replace('_', "-"));
    if c.has(&id) {
        c.tr(&id)
    } else {
        let name = a.palette_name();
        let mut chars = name.chars();
        chars.next().map_or_else(String::new, |f| {
            f.to_uppercase().collect::<String>() + chars.as_str()
        })
    }
}

/// A menu's title as the catalog has it (`menu-*`), access mark included.
fn menu_label(c: &Catalog, m: MenuId) -> String {
    c.tr(&format!("menu-{}", m.id()))
}

/// The letters the top menus' access keys must avoid, from the window's
/// default keymap on Windows: the same in every session, so worked out once.
static RESERVED_LETTERS: LazyLock<Vec<char>> = LazyLock::new(|| {
    gui_alt_letters(&Keymap::defaults(
        textweaver_keymap::Platform::Windows,
        textweaver_keymap::Frontend::Gui,
    ))
});

/// The settings what the menus show depends on: the language, the theme
/// (its name is shown on Next theme), and every setting a menu item shows
/// ([`bound_setting`]). A frontend that keeps native menus compares these,
/// not all the settings, so a rate key does not rebuild the menus.
pub fn menu_setting_paths() -> &'static [&'static str] {
    static PATHS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let mut out = vec!["interface.language", "display.theme"];
        for a in ActionId::ALL {
            if let Some(p) = bound_setting(*a)
                && !out.contains(&p)
            {
                out.push(p);
            }
        }
        out
    });
    &PATHS
}

/// Letters the GUI's `Alt` chords use, which a top menu's access key must
/// not take, so `Alt+O` stays the outline in every language.
pub fn gui_alt_letters(keymap: &Keymap) -> Vec<char> {
    let mut out = Vec::new();
    for b in keymap.bindings() {
        if b.chord.mods == textweaver_keymap::Modifiers::ALT
            && let textweaver_keymap::Key::Char(ch) = b.chord.key
            && (ch.is_lowercase() || ch.is_ascii_digit())
        {
            // An uppercase letter is Alt+Shift with it, which leaves Alt with
            // the letter alone to a menu.
            let lower = ch;
            if !out.contains(&lower) {
                out.push(lower);
            }
        }
    }
    out
}

/// The key a menu shows for a command: its first chord that works with
/// single-key shortcuts off, else its single key; `None` without keys.
pub fn menu_chord(keymap: &Keymap, a: ActionId) -> Option<KeyChord> {
    let chords = keymap.chords_for(a);
    chords
        .iter()
        .find(|c| !c.is_text_input())
        .or_else(|| {
            chords
                .iter()
                .find(|c| c.is_text_input() && keymap.character_keys())
        })
        .copied()
}

/// The menu state the app keeps: registered handlers, recent commands,
/// and the terminal's open menu.
#[derive(Default)]
pub(crate) struct MenuState {
    handlers: Vec<(ActionId, Handler)>,
    /// Commands run from a menu or the palette, newest first.
    pub(crate) recent: Vec<ActionId>,
    /// The next command key is described, not run (Help, "What does this
    /// key do?").
    describe_next: bool,
    /// The menu shown as a list: the path of menus opened, and what each
    /// row of the list is.
    list: Option<MenuList>,
}

/// The menu shown as a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MenuList {
    /// The menus opened, from the bar down; empty on the bar itself.
    path: Vec<MenuId>,
    /// The rows shown.
    rows: Vec<MenuItem>,
}

impl App {
    /// Registers the handler of a command a module provides (the pending
    /// ones, [`PENDING`]); the command then appears in the menus and the
    /// command palette, and runs `handler`. A later registration replaces
    /// an earlier one.
    pub fn register_handler(&mut self, action: ActionId, handler: Handler) {
        self.menu.handlers.retain(|(a, _)| *a != action);
        self.menu.handlers.push((action, handler));
    }

    /// True when `action` can run: every command but a pending one whose
    /// module has not registered a handler ([`PENDING`]).
    pub fn is_available(&self, action: ActionId) -> bool {
        !PENDING.contains(&action) || self.menu.handlers.iter().any(|(a, _)| *a == action)
    }

    /// Runs a command registered with [`register_handler`](Self::register_handler).
    pub(crate) fn run_registered(&mut self, action: ActionId) -> Vec<Effect> {
        if let Some(h) = self
            .menu
            .handlers
            .iter()
            .find(|(a, _)| *a == action)
            .map(|(_, h)| *h)
        {
            return h(self);
        }
        let name = action_name(self.cat(), action);
        let msg = self.msg_args("menu-not-available", &args!["name" => name]);
        self.error(&msg);
        vec![Effect::Redraw]
    }

    /// Commands run from a menu or the command palette, newest first (at
    /// most [`RECENT_COMMANDS`]).
    pub fn recent_commands(&self) -> &[ActionId] {
        &self.menu.recent
    }

    /// Runs `action` as a menu or the palette does: it becomes the most
    /// recent command ([`Command::RunCommand`]).
    pub(crate) fn run_command(&mut self, action: ActionId) -> Vec<Effect> {
        self.remember_command(action);
        self.action(action)
    }

    /// Records `action` as the most recent command.
    pub fn remember_command(&mut self, action: ActionId) {
        if matches!(action, A::CommandPalette | A::Menu) {
            return;
        }
        self.menu.recent.retain(|a| *a != action);
        self.menu.recent.insert(0, action);
        self.menu.recent.truncate(RECENT_COMMANDS);
    }

    /// The menu bar: the seven menus, each with its items, in the
    /// interface's language, with keys from the live keymap. Submenus are
    /// items of kind [`MenuItemKind::Submenu`]; build them with
    /// [`menu_view`](Self::menu_view).
    pub fn menu_bar(&self) -> Vec<MenuView> {
        MenuId::TOP.iter().map(|&m| self.menu_view(m)).collect()
    }

    /// The top menus' titles and access keys, in order.
    fn top_titles(&self) -> Vec<(String, Option<char>)> {
        let c = self.cat();
        let labels: Vec<(String, Option<char>)> = MenuId::TOP
            .iter()
            .map(|&m| split_access(&menu_label(c, m)))
            .collect();
        let keys = assign_access_keys(&labels, &RESERVED_LETTERS);
        labels
            .into_iter()
            .zip(keys)
            .map(|((t, _), k)| (t, k))
            .collect()
    }

    /// One menu, with its items as a frontend shows them.
    pub fn menu_view(&self, id: MenuId) -> MenuView {
        let c = self.cat();
        let (title, access) = if id.is_top() {
            let i = MenuId::TOP.iter().position(|m| *m == id).unwrap_or(0);
            self.top_titles().swap_remove(i)
        } else {
            (split_access(&menu_label(c, id)).0, None)
        };
        let mut items: Vec<MenuItem> = Vec::new();
        for entry in id.entries() {
            match *entry {
                Do(a) if self.is_available(a) => items.push(self.action_item(a)),
                Do(_) => {}
                Sub(m) => items.push(MenuItem {
                    kind: MenuItemKind::Submenu(m),
                    label: menu_label(c, m),
                    access: None,
                    keys: None,
                    checked: None,
                    value: None,
                }),
                Sep => {
                    // No separator first, last, or twice in a row (a
                    // hidden command can leave one alone).
                    if items
                        .last()
                        .is_some_and(|i| i.kind != MenuItemKind::Separator)
                    {
                        items.push(MenuItem {
                            kind: MenuItemKind::Separator,
                            label: String::new(),
                            access: None,
                            keys: None,
                            checked: None,
                            value: None,
                        });
                    }
                }
                Entry::RecentDocuments => items.extend(self.recent_document_items()),
            }
        }
        while items
            .last()
            .is_some_and(|i| i.kind == MenuItemKind::Separator)
        {
            items.pop();
        }
        // Access keys: marked letters, then free ones; recent documents
        // keep their digits.
        let labels: Vec<(String, Option<char>)> = items
            .iter()
            .map(|i| match i.kind {
                MenuItemKind::Separator | MenuItemKind::Empty => (String::new(), None),
                MenuItemKind::Document(_) => (String::new(), i.access),
                _ => split_access(&i.label),
            })
            .collect();
        let keys = assign_access_keys(&labels, &[]);
        for ((item, (plain, _)), key) in items.iter_mut().zip(labels).zip(keys) {
            if matches!(item.kind, MenuItemKind::Document(_)) {
                continue;
            }
            if !matches!(item.kind, MenuItemKind::Separator | MenuItemKind::Empty) {
                item.label = plain;
            }
            item.access = key;
        }
        MenuView {
            id,
            title,
            access,
            items,
        }
    }

    /// A command's menu item: its name, main key, and state.
    fn action_item(&self, a: ActionId) -> MenuItem {
        let c = self.cat();
        let keys = menu_chord(&self.keymap, a).map(|k| k.to_string());
        let (checked, value) = self.item_state(a);
        MenuItem {
            kind: MenuItemKind::Action(a),
            label: action_label(c, a),
            access: None,
            keys,
            checked,
            value,
        }
    }

    /// Whether a toggle is on, or a choice's value: from the live mode
    /// for modes that end with the session, else from the settings.
    fn item_state(&self, a: ActionId) -> (Option<bool>, Option<String>) {
        match a {
            A::ToggleEditMode => return (Some(self.edit.is_some()), None),
            A::RsvpToggle => return (Some(self.rsvp.is_some()), None),
            A::SpeechCursorToggle => return (Some(self.mode == Mode::SpeechCursor), None),
            A::NextTheme => {
                let name = self
                    .themes
                    .resolve(&self.settings.display.theme)
                    .0
                    .meta
                    .display_name
                    .clone();
                return (None, Some(name));
            }
            _ => {}
        }
        let Some(path) = bound_setting(a) else {
            return (None, None);
        };
        let (Some(setting), Some(now)) = (
            crate::settings_schema::base_schema().get(path),
            self.setting_value(path),
        ) else {
            return (None, None);
        };
        // The window shows its two modes, not the three stored ones.
        let window_mode;
        let setting = if path == "accessibility.mode" && self.uses_window_modes() {
            window_mode = crate::settings_schema::window_access_mode(setting);
            &window_mode
        } else {
            setting
        };
        match (&setting.kind, &now) {
            (crate::settings_schema::SettingKind::Toggle, serde_json::Value::Bool(b)) => {
                (Some(*b), None)
            }
            _ => (None, Some(setting.describe_in(self.cat(), &now))),
        }
    }

    /// The File menu's recent documents, newest first, with the reading
    /// position: "essay.md, 43 percent". Digits 1 to 8 are their keys.
    fn recent_document_items(&self) -> Vec<MenuItem> {
        let c = self.cat();
        let Some(paths) = self.paths.as_ref() else {
            return vec![empty_item(c)];
        };
        let recent = textweaver_store::Recent::load(&paths.recent_file());
        let states = textweaver_store::StateStore::new(paths.state_dir());
        let items: Vec<MenuItem> = recent
            .entries
            .iter()
            .take(RECENT_DOCUMENTS)
            .enumerate()
            .map(|(i, e)| {
                let name = e.path.file_name().map_or_else(
                    || e.path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let pct = states
                    .load(&textweaver_store::DocKey::for_path(&e.path))
                    .filter(textweaver_store::DocState::has_position)
                    .map(|s| s.pct);
                let label = match pct {
                    Some(p) => c.fmt(
                        "menu-recent-document",
                        &args!["name" => name.as_str(), "pct" => p],
                    ),
                    None => name,
                };
                MenuItem {
                    kind: MenuItemKind::Document(e.path.clone()),
                    label,
                    access: char::from_digit(i as u32 + 1, 10),
                    keys: None,
                    checked: None,
                    value: None,
                }
            })
            .collect();
        if items.is_empty() {
            vec![empty_item(c)]
        } else {
            items
        }
    }

    // ---- The menus as a list (the terminal, and the GUI on Linux) ----

    /// The Menu command (F10): the menu bar as a list, "Menus, 1 of 7,
    /// File".
    pub(crate) fn open_menu(&mut self) -> Vec<Effect> {
        self.show_menu_list(Vec::new(), 0)
    }

    /// The menu path shown, from the bar down (empty on the bar), while
    /// the menus are shown as a list.
    pub fn menu_path(&self) -> Option<&[MenuId]> {
        match &self.list {
            Some(ListKind::Menu) => self.menu.list.as_ref().map(|l| l.path.as_slice()),
            _ => None,
        }
    }

    fn show_menu_list(&mut self, path: Vec<MenuId>, focus: usize) -> Vec<Effect> {
        let c = self.catalog();
        let (title, rows) = match path.last() {
            None => {
                let rows: Vec<MenuItem> = self
                    .top_titles()
                    .into_iter()
                    .zip(MenuId::TOP)
                    .map(|((label, access), m)| MenuItem {
                        kind: MenuItemKind::Submenu(m),
                        label,
                        access,
                        keys: None,
                        checked: None,
                        value: None,
                    })
                    .collect();
                (c.tr("menu-bar"), rows)
            }
            Some(&m) => {
                let view = self.menu_view(m);
                let rows: Vec<MenuItem> = view
                    .items
                    .into_iter()
                    .filter(|i| i.kind != MenuItemKind::Separator)
                    .collect();
                (c.fmt("menu-title", &args!["name" => view.title]), rows)
            }
        };
        let items: Vec<String> = rows
            .iter()
            .map(|r| {
                if path.is_empty() {
                    r.label.clone()
                } else {
                    r.line(&c)
                }
            })
            .collect();
        self.menu.list = Some(MenuList { path, rows });
        self.list = Some(ListKind::Menu);
        self.pending_list_focus = Some(focus);
        self.say_result(&title);
        vec![Effect::ShowList { title, items }]
    }

    /// Keys the menu list handles itself: Right and Enter open a submenu
    /// or run a command, a letter moves to its access key, Left and
    /// Backspace go up, Escape closes. `None` leaves the key to the list.
    pub(crate) fn menu_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        if self.list != Some(ListKind::Menu) {
            return None;
        }
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        match key {
            ListKey::Right | ListKey::Enter => Some(self.menu_choose(n, key == ListKey::Enter)),
            ListKey::Left | ListKey::Backspace => Some(self.menu_up()),
            ListKey::Escape => Some(self.close_menu()),
            ListKey::Char(ch) if ch.is_alphanumeric() => Some(self.menu_access_key(ch)),
            ListKey::Introduce => Some(self.menu_keys_help()),
            _ => None,
        }
    }

    /// Chooses row `n` of the menu list (Enter, Right, or a GUI's or
    /// JSON-RPC's [`Command::Choose`]). Right on a command does nothing.
    pub(crate) fn menu_choose(&mut self, n: usize, run: bool) -> Vec<Effect> {
        let Some(list) = self.menu.list.clone() else {
            return vec![Effect::Redraw];
        };
        let Some(row) = list.rows.get(n) else {
            return vec![Effect::Redraw];
        };
        match &row.kind {
            MenuItemKind::Submenu(m) => {
                let mut path = list.path.clone();
                path.push(*m);
                self.show_menu_list(path, 0)
            }
            MenuItemKind::Action(a) if run => {
                let a = *a;
                self.leave_menu();
                self.run_command(a)
            }
            MenuItemKind::Document(p) if run => {
                let p = p.clone();
                self.leave_menu();
                self.dispatch_inner(Command::Open(p))
            }
            _ => {
                self.list = Some(ListKind::Menu);
                vec![Effect::Redraw]
            }
        }
    }

    /// Left or Backspace: up one menu, on the item that opened it; on the
    /// bar, Backspace closes and Left stays.
    fn menu_up(&mut self) -> Vec<Effect> {
        let Some(list) = self.menu.list.clone() else {
            return vec![Effect::Redraw];
        };
        let mut path = list.path;
        let Some(left) = path.pop() else {
            return self.close_menu();
        };
        let focus = match path.last() {
            None => MenuId::TOP.iter().position(|m| *m == left).unwrap_or(0),
            Some(&parent) => self
                .menu_view(parent)
                .items
                .iter()
                .filter(|i| i.kind != MenuItemKind::Separator)
                .position(|i| i.kind == MenuItemKind::Submenu(left))
                .unwrap_or(0),
        };
        self.show_menu_list(path, focus)
    }

    /// A letter: the focus moves to the item with that access key, as in
    /// a platform menu (it does not run it).
    fn menu_access_key(&mut self, ch: char) -> Vec<Effect> {
        let want = ch.to_lowercase().next().unwrap_or(ch);
        let found = self
            .menu
            .list
            .as_ref()
            .and_then(|l| l.rows.iter().position(|r| r.access == Some(want)));
        match found {
            Some(i) => {
                if let Some(l) = self.list_model.as_mut() {
                    l.selected = i;
                }
                let text = self
                    .list_model
                    .as_ref()
                    .and_then(|l| l.spoken_item_text(self.cat()))
                    .unwrap_or_default();
                self.announce(&text, Priority::Assertive);
            }
            None => {
                let msg = self.msg_args("menu-no-access-key", &args!["letter" => ch.to_string()]);
                self.announce(&msg, Priority::Polite);
            }
        }
        self.list = Some(ListKind::Menu);
        vec![Effect::Redraw]
    }

    /// F1 or Say Status in the menus: where the focus is, and the keys.
    fn menu_keys_help(&mut self) -> Vec<Effect> {
        let item = self
            .list_model
            .as_ref()
            .and_then(|l| l.spoken_item_text(self.cat()))
            .unwrap_or_default();
        let msg = self.msg_args("menu-keys", &args!["item" => item]);
        self.tell(&msg);
        self.list = Some(ListKind::Menu);
        vec![Effect::Redraw]
    }

    /// Escape: the menus close; the reader is where it was.
    fn close_menu(&mut self) -> Vec<Effect> {
        self.leave_menu();
        self.list_model = None;
        let msg = self.msg("menu-closed");
        self.say_dialog(&msg);
        vec![Effect::Redraw]
    }

    /// Forgets the menu list (it closed, or a command from it runs).
    pub(crate) fn leave_menu(&mut self) {
        self.menu.list = None;
        if self.list == Some(ListKind::Menu) {
            self.list = None;
        }
        self.list_model = None;
    }

    /// Help, "What does this key do?": the next command key pressed is
    /// described, not run.
    pub(crate) fn what_does_this_key_do(&mut self) -> Vec<Effect> {
        self.menu.describe_next = true;
        let msg = self.msg("menu-press-a-key");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// True while the next command key is to be described (Help, "What
    /// does this key do?", Shift+F1): a frontend that runs some commands
    /// itself sends them to [`dispatch`](Self::dispatch) instead, so they
    /// are described, not run.
    pub fn describing_next_key(&self) -> bool {
        self.menu.describe_next
    }

    /// Describes `action` instead of running it: its name, what it does,
    /// its keys, and where it is in the menus.
    pub(crate) fn describe_action(&mut self, action: ActionId) -> Vec<Effect> {
        self.menu.describe_next = false;
        let c = self.catalog();
        let name = action_name(&c, action);
        let help = crate::help::action_help(&c, action);
        let keys = self.keys(action);
        let msg = match self.menu_path_of(action) {
            Some(path) => c.fmt(
                "menu-key-described",
                &args!["name" => name, "help" => help, "keys" => keys, "path" => path],
            ),
            None => c.fmt(
                "menu-key-described-no-menu",
                &args!["name" => name, "help" => help, "keys" => keys],
            ),
        };
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// The menu path of `action` in the interface's language: "File,
    /// Export as, Export PDF", the first place it appears; `None` for a
    /// command in no menu.
    pub fn menu_path_of(&self, action: ActionId) -> Option<String> {
        let c = self.cat();
        let path = find_path(action)?;
        let mut parts: Vec<String> = path
            .iter()
            .map(|&m| split_access(&menu_label(c, m)).0)
            .collect();
        parts.push(action_name(c, action));
        Some(parts.join(", "))
    }
}

/// The first menu path that holds `action`: its menus, from the bar down.
pub fn find_path(action: ActionId) -> Option<Vec<MenuId>> {
    fn walk(m: MenuId, action: ActionId, path: &mut Vec<MenuId>) -> bool {
        path.push(m);
        for e in m.entries() {
            match *e {
                Do(a) if a == action => return true,
                Sub(s) if walk(s, action, path) => return true,
                _ => {}
            }
        }
        path.pop();
        false
    }
    let mut path = Vec::new();
    MenuId::TOP
        .iter()
        .any(|&m| walk(m, action, &mut path))
        .then_some(path)
}

/// Every command some menu lists, in the model (whether or not it is
/// available now).
pub fn actions_in_menus() -> Vec<ActionId> {
    let mut out = Vec::new();
    for m in MenuId::ALL {
        for e in m.entries() {
            if let Do(a) = *e
                && !out.contains(&a)
            {
                out.push(a);
            }
        }
    }
    out
}

fn empty_item(c: &Catalog) -> MenuItem {
    MenuItem {
        kind: MenuItemKind::Empty,
        label: c.tr("menu-recent-none"),
        access: None,
        keys: None,
        checked: None,
        value: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_keymap::{Frontend, Platform};
    use textweaver_lexicon::i18n::LANGUAGES;

    #[test]
    fn access_marks_split_and_join() {
        assert_eq!(split_access("E&xport as"), ("Export as".into(), Some('x')));
        assert_eq!(split_access("Q&&A"), ("Q&A".into(), None));
        assert_eq!(split_access("Plain"), ("Plain".into(), None));
        let item = MenuItem {
            kind: MenuItemKind::Separator,
            label: "Export as".into(),
            access: Some('x'),
            keys: None,
            checked: None,
            value: None,
        };
        assert_eq!(item.marked_label(), "E&xport as");
        assert_eq!(mark_access("Q&A", Some('a')), "Q&&&A");
    }

    #[test]
    fn access_keys_take_marks_then_word_starts() {
        let labels = vec![
            ("Open".to_owned(), None),
            ("Open path".to_owned(), None),
            ("Export as".to_owned(), Some('x')),
            ("Exit".to_owned(), None),
        ];
        let keys = assign_access_keys(&labels, &['o']);
        assert_eq!(keys, vec![Some('p'), Some('e'), Some('x'), Some('i')]);
    }

    /// Every command is in a menu, except the key that opens them and the
    /// pending commands, whose modules have not registered handlers yet.
    #[test]
    fn every_action_is_in_a_menu() {
        let listed = actions_in_menus();
        let missing: Vec<ActionId> = ActionId::ALL
            .iter()
            .copied()
            .filter(|a| !listed.contains(a) && !NOT_IN_MENUS.contains(a))
            .collect();
        assert!(missing.is_empty(), "in no menu: {missing:?}");
        let app = App::new(crate::AppConfig::for_tests());
        let shown: Vec<ActionId> = MenuId::ALL
            .iter()
            .flat_map(|&m| app.menu_view(m).items)
            .filter_map(|i| match i.kind {
                MenuItemKind::Action(a) => Some(a),
                _ => None,
            })
            .collect();
        let hidden: Vec<ActionId> = ActionId::ALL
            .iter()
            .copied()
            .filter(|a| !shown.contains(a) && !NOT_IN_MENUS.contains(a))
            .collect();
        let pending: Vec<ActionId> = PENDING
            .iter()
            .copied()
            .filter(|a| !app.is_available(*a))
            .collect();
        assert_eq!(
            hidden, pending,
            "hidden commands must be exactly the pending ones"
        );
    }

    /// Every command a menu shows runs something: a pending one only once
    /// its module registered a handler. In a full build every pending
    /// command has registered (browse W6f, batch W6k, audio export W6v,
    /// dictation W6d), so all of them show.
    #[test]
    fn every_visible_menu_command_has_a_handler() {
        let app = App::new(crate::AppConfig::for_tests());
        let registered = |a: A| app.menu.handlers.iter().any(|(h, _)| *h == a);
        for m in MenuId::ALL {
            for item in app.menu_view(m).items {
                if let MenuItemKind::Action(a) = item.kind
                    && PENDING.contains(&a)
                {
                    assert!(registered(a), "{a:?} shows in {m:?} with no handler");
                }
            }
        }
        if cfg!(all(
            feature = "publish",
            feature = "dictation",
            feature = "audio-export"
        )) {
            for &a in PENDING {
                assert!(
                    registered(a) && app.is_available(a),
                    "{a:?} is not registered"
                );
            }
        }
    }

    /// A later registration replaces an earlier one, and the menu runs the
    /// command registered last.
    #[test]
    fn a_registered_handler_shows_and_runs() {
        fn handler(app: &mut App) -> Vec<Effect> {
            app.tell("Exporting.");
            vec![Effect::Redraw]
        }
        let mut app = App::new(crate::AppConfig::for_tests());
        app.register_handler(A::ExportAudio, handler);
        assert!(app.is_available(A::ExportAudio));
        let file = app.menu_view(MenuId::File);
        assert!(
            file.items
                .iter()
                .any(|i| i.kind == MenuItemKind::Action(A::ExportAudio))
        );
        app.dispatch(Command::Action(A::ExportAudio));
        assert_eq!(app.status_text(), "Exporting.");
    }

    /// Commands deliberately in more than one menu, each with its reason.
    /// Empty: every command has one place.
    const IN_TWO_MENUS: &[ActionId] = &[];

    /// Each command is in exactly one menu, and each submenu hangs from
    /// exactly one menu: no redundant items (the owner, alpha.9). The
    /// command palette lists every command once; the menus do too.
    #[test]
    fn every_command_and_submenu_has_one_place() {
        let mut places: Vec<(ActionId, Vec<MenuId>)> = Vec::new();
        let mut parents: Vec<(MenuId, Vec<MenuId>)> = Vec::new();
        for m in MenuId::ALL {
            for e in m.entries() {
                match *e {
                    Do(a) => match places.iter_mut().find(|(x, _)| *x == a) {
                        Some((_, ms)) => ms.push(m),
                        None => places.push((a, vec![m])),
                    },
                    Sub(sub) => match parents.iter_mut().find(|(x, _)| *x == sub) {
                        Some((_, ms)) => ms.push(m),
                        None => parents.push((sub, vec![m])),
                    },
                    _ => {}
                }
            }
        }
        let repeated: Vec<_> = places
            .iter()
            .filter(|(a, ms)| ms.len() > 1 && !IN_TWO_MENUS.contains(a))
            .collect();
        assert!(repeated.is_empty(), "in more than one menu: {repeated:?}");
        let shared: Vec<_> = parents.iter().filter(|(_, ms)| ms.len() > 1).collect();
        assert!(
            shared.is_empty(),
            "submenu in more than one menu: {shared:?}"
        );
        for m in MenuId::ALL {
            if !m.is_top() {
                assert!(
                    parents.iter().any(|(x, _)| *x == m),
                    "{m:?} hangs from no menu"
                );
            }
        }
    }

    /// Each menu stays short enough to hear through (about 20 items).
    #[test]
    fn menus_stay_short() {
        for m in MenuId::ALL {
            let n = m.entries().iter().filter(|e| **e != Sep).count();
            assert!(n <= 21, "{m:?} has {n} items");
        }
    }

    /// The menu bar is exactly these menus, in this order, so a new menu
    /// is a decision, not drift.
    #[test]
    fn the_menu_bar_is_pinned() {
        let ids: Vec<&str> = MenuId::TOP.iter().map(|m| m.id()).collect();
        assert_eq!(
            ids,
            ["file", "edit", "view", "reading", "speech", "tools", "help"]
        );
        for m in MenuId::ALL {
            assert_eq!(MenuId::from_id(m.id()), Some(m));
        }
    }

    /// In every language: no access key repeats in a menu, the top menus'
    /// keys never equal a GUI Alt chord, every label has a catalog
    /// message, and every line fits the meaning into 40 cells.
    #[test]
    fn menus_in_six_languages() {
        let gui = Keymap::defaults(Platform::Windows, Frontend::Gui);
        let alt = gui_alt_letters(&gui);
        for lang in LANGUAGES {
            let (cat, warn) = Catalog::for_language(lang.tag, None);
            assert!(warn.is_none(), "{}", lang.tag);
            for m in MenuId::ALL {
                assert!(
                    cat.has(&format!("menu-{}", m.id())),
                    "{}: menu-{}",
                    lang.tag,
                    m.id()
                );
            }
            let mut config = crate::AppConfig::for_tests();
            config.settings.interface.language = lang.tag.to_owned();
            let app = App::new(config);
            let bar = app.menu_bar();
            let keys: Vec<Option<char>> = bar.iter().map(|m| m.access).collect();
            for k in keys.iter().flatten() {
                assert!(
                    !alt.contains(k),
                    "{}: top menu key {k} is an Alt chord",
                    lang.tag
                );
            }
            assert!(keys.iter().all(Option::is_some), "{}: {keys:?}", lang.tag);
            for m in MenuId::ALL {
                let view = app.menu_view(m);
                let mut seen: Vec<char> = Vec::new();
                for item in &view.items {
                    if let MenuItemKind::Action(a) = item.kind {
                        assert!(
                            cat.has(&format!("name-{}", a.id().replace('_', "-"))),
                            "{}: name-{}",
                            lang.tag,
                            a.id()
                        );
                    }
                    if let Some(k) = item.access {
                        assert!(!seen.contains(&k), "{}: {k} repeats in {m:?}", lang.tag);
                        seen.push(k);
                    }
                    // The name comes first; at most 40 cells before the keys.
                    assert!(
                        item.label.chars().count() <= 40,
                        "{}: {:?} is longer than 40 cells",
                        lang.tag,
                        item.label
                    );
                }
            }
        }
    }

    #[test]
    fn toggles_show_live_state() {
        let mut app = App::new(crate::AppConfig::for_tests());
        let item = |app: &App, a: ActionId| {
            MenuId::ALL
                .iter()
                .flat_map(|&m| app.menu_view(m).items)
                .find(|i| i.kind == MenuItemKind::Action(a))
                .unwrap()
        };
        assert_eq!(item(&app, A::BionicToggle).checked, Some(false));
        app.dispatch(Command::Action(A::BionicToggle));
        let bionic = item(&app, A::BionicToggle);
        assert_eq!(bionic.checked, Some(true));
        assert!(
            bionic
                .line(&Catalog::english())
                .starts_with("Bionic reading, checked")
        );
        assert_eq!(item(&app, A::ToggleEditMode).checked, Some(false));
        let ruler = item(&app, A::RulerCycle);
        assert_eq!(ruler.value.as_deref(), Some("off"));
    }

    #[test]
    fn f10_opens_the_menus_as_a_list() {
        let mut app = App::new(crate::AppConfig::for_tests());
        app.dispatch(Command::Action(A::Menu));
        let list = app.list_model().unwrap();
        assert_eq!(list.title, "Menus");
        assert_eq!(list.items.len(), 7);
        assert_eq!(list.items[0], "File");
        assert!(
            app.status_text().starts_with("Menus"),
            "{}",
            app.status_text()
        );
        assert!(
            app.status_text().contains("1 of 7, File"),
            "{}",
            app.status_text()
        );
        // Right opens File; its items say name then keys.
        app.dispatch(Command::ListKey(ListKey::Right));
        let list = app.list_model().unwrap();
        assert_eq!(list.title, "File menu");
        assert_eq!(list.items[0], "Open, Ctrl+O");
        assert_eq!(app.menu_path(), Some(&[MenuId::File][..]));
        // x lands on Export as, without running it.
        app.dispatch(Command::ListKey(ListKey::Char('x')));
        let list = app.list_model().unwrap();
        assert_eq!(list.current(), Some("Export as, submenu"));
        // Right opens it; Left comes back to it.
        app.dispatch(Command::ListKey(ListKey::Right));
        assert_eq!(app.list_model().unwrap().title, "Export as menu");
        app.dispatch(Command::ListKey(ListKey::Left));
        let list = app.list_model().unwrap();
        assert_eq!(list.title, "File menu");
        assert_eq!(list.current(), Some("Export as, submenu"));
        // Left again: the bar, on File.
        app.dispatch(Command::ListKey(ListKey::Left));
        let list = app.list_model().unwrap();
        assert_eq!(list.title, "Menus");
        assert_eq!(list.current(), Some("File"));
        // Escape closes.
        app.dispatch(Command::ListKey(ListKey::Escape));
        assert!(app.list_model().is_none());
        assert!(app.menu_path().is_none());
    }

    #[test]
    fn a_command_from_the_menu_runs_and_becomes_recent() {
        let mut app = App::new(crate::AppConfig::for_tests());
        app.dispatch(Command::Action(A::Menu));
        app.dispatch(Command::ListKey(ListKey::Char('v')));
        app.dispatch(Command::ListKey(ListKey::Enter));
        assert_eq!(app.list_model().unwrap().title, "View menu");
        // Reading aids, then Bionic reading.
        let rows = app.list_model().unwrap().items.clone();
        let aids = rows
            .iter()
            .position(|r| r.starts_with("Reading aids"))
            .unwrap();
        app.dispatch(Command::ListFocus(aids));
        app.dispatch(Command::ListKey(ListKey::Enter));
        app.dispatch(Command::ListKey(ListKey::Enter));
        assert!(app.settings().reading_aids.bionic);
        assert!(app.list_model().is_none());
        assert_eq!(app.recent_commands(), &[A::BionicToggle]);
    }

    #[test]
    fn menu_paths() {
        let app = App::new(crate::AppConfig::for_tests());
        assert_eq!(
            app.menu_path_of(A::ExportPdf).as_deref(),
            Some("File, Export as, Export PDF")
        );
        assert_eq!(app.menu_path_of(A::Menu), None);
    }
}
