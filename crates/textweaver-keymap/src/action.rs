//! Every user command, with its help text and default keys.
//!
//! Defaults follow Star (docs/star-parity.md Part 1 §6): Star's GUI chords
//! for the GUI, Star's TUI single keys as the browse layer shared by both
//! frontends, and new keys for what Star lacked (pitch, volume, read the
//! current unit, list and link navigation, bookmark stepping).
//!
//! Rules the defaults follow, each enforced by a test:
//!
//! - every action has at least one binding on each frontend and platform;
//! - no chord reaches two actions in the same mode;
//! - terminal defaults avoid what terminals cannot distinguish or send
//!   (`Ctrl+H` is Backspace, `Ctrl+I` is Tab, `Ctrl+M` and `Ctrl+J` are
//!   Enter, `Ctrl+Shift+letter` arrives as `Ctrl+letter`, `Ctrl` with
//!   digits or punctuation is not sent, and Command never arrives), see
//!   [`KeyChord::terminal_limitation`](crate::KeyChord::terminal_limitation);
//! - modified chords use only letters, digits, and unshifted punctuation,
//!   so they do not depend on the keyboard layout (on a US layout
//!   `Ctrl+Shift+8` arrives as `Ctrl+*`, on others as something else);
//! - edit-layer chords never use Ctrl+Alt with a letter, which is AltGr on
//!   many layouts and types characters.
//!
//! Deliberate departures from Star: `Ctrl+T` means next table in the GUI
//! and nothing in the terminal (Star's TUI used it for the voice picker,
//! Part 1 §7 item 37); `Ctrl+S` saves in both (Star's TUI exported);
//! Redo also answers to `Ctrl+Shift+Z` in the GUI (Star used that chord for
//! its preview pane, which textweaver does not have).

use serde::{Deserialize, Serialize};

/// Groups for help and the command palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Start, stop, and what to read.
    Reading,
    /// Moving through the document.
    Navigation,
    /// Speech Cursor (line) mode.
    SpeechCursor,
    /// Rate, pitch, volume.
    Voice,
    /// Find.
    Search,
    /// Bookmarks.
    Bookmarks,
    /// Files and the application.
    File,
    /// Editing and Markdown formatting.
    Editing,
    /// Display and help.
    View,
}

impl Category {
    /// Every category, in help order.
    pub const ALL: [Category; 9] = [
        Category::Reading,
        Category::Navigation,
        Category::SpeechCursor,
        Category::Voice,
        Category::Search,
        Category::Bookmarks,
        Category::File,
        Category::Editing,
        Category::View,
    ];

    /// Heading used in help.
    pub fn title(self) -> &'static str {
        match self {
            Category::Reading => "Reading",
            Category::Navigation => "Navigation",
            Category::SpeechCursor => "Speech Cursor",
            Category::Voice => "Voice",
            Category::Search => "Search",
            Category::Bookmarks => "Bookmarks",
            Category::File => "File",
            Category::Editing => "Editing",
            Category::View => "View and help",
        }
    }
}

/// Default keys for one action.
///
/// Chord strings are prefixed with their layer: `g:` global, `b:` browse
/// (single keys, both frontends), `s:` Speech Cursor, `e:` edit mode.
pub(crate) struct Defaults {
    pub(crate) gui: &'static [&'static str],
    pub(crate) terminal: &'static [&'static str],
    /// Browse and Speech Cursor keys shared by both frontends.
    pub(crate) shared: &'static [&'static str],
}

macro_rules! actions {
    ($( $variant:ident = $id:literal, $cat:ident, $help:literal,
        gui [$($g:literal),*], term [$($t:literal),*], shared [$($s:literal),*]; )*) => {
        /// Every user command.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum ActionId {
            $(
                #[doc = $help]
                $variant,
            )*
        }

        impl ActionId {
            /// Every action, in help order.
            pub const ALL: &'static [ActionId] = &[$(ActionId::$variant),*];

            /// Stable id used in `keymap.toml` and the command palette.
            pub fn id(self) -> &'static str {
                match self { $(ActionId::$variant => $id,)* }
            }

            /// The action for a stable id.
            pub fn from_id(id: &str) -> Option<Self> {
                match id { $($id => Some(ActionId::$variant),)* _ => None }
            }

            /// Help category.
            pub fn category(self) -> Category {
                match self { $(ActionId::$variant => Category::$cat,)* }
            }

            /// One-line help text.
            pub fn help(self) -> &'static str {
                match self { $(ActionId::$variant => $help,)* }
            }

            pub(crate) fn defaults(self) -> Defaults {
                match self {
                    $(ActionId::$variant => Defaults {
                        gui: &[$($g),*],
                        terminal: &[$($t),*],
                        shared: &[$($s),*],
                    },)*
                }
            }
        }
    };
}

actions! {
    // Reading
    PlayPause = "play_pause", Reading, "Play or pause reading from the current word",
        gui [], term [], shared ["b:Space"];
    Stop = "stop", Reading, "Stop reading",
        gui ["g:Escape"], term ["g:Escape", "b:Ctrl+X"], shared [];
    ReadFromCursor = "read_from_cursor", Reading, "Read continuously from the cursor",
        gui ["g:Ctrl+Space"], term ["g:Ctrl+Space"], shared ["b:Enter"];
    ReadDocument = "read_document", Reading, "Read the whole document from the start",
        gui [], term [], shared ["b:Shift+R"];
    ReadCurrentCharacter = "read_current_character", Reading, "Say the character at the cursor",
        gui [], term [], shared ["b:c"];
    ReadCurrentWord = "read_current_word", Reading, "Say the word at the cursor",
        gui [], term [], shared ["b:w"];
    ReadCurrentSentence = "read_current_sentence", Reading, "Say the sentence at the cursor without moving",
        gui [], term [], shared ["b:s"];
    ReadCurrentLine = "read_current_line", Reading, "Say the line at the cursor",
        gui [], term [], shared ["b:l"];
    ReadSelection = "read_selection", Reading, "Read the selected text",
        gui [], term [], shared ["b:v"];
    SayPosition = "say_position", Reading, "Say the position: line, percentage, and heading",
        gui [], term [], shared ["b:%"];
    ReplaySentence = "replay_sentence", Reading, "Read again from the start of the current sentence",
        gui ["g:Alt+;"], term ["g:Alt+;"], shared ["b:;"];
    ReplayParagraph = "replay_paragraph", Reading, "Read again from the start of the current paragraph",
        gui ["g:Ctrl+R"], term ["g:Ctrl+R"], shared ["b:r"];

    // Navigation
    NextSentence = "next_sentence", Navigation, "Move to the next sentence",
        gui ["g:Alt+."], term ["g:Alt+."], shared ["b:."];
    PreviousSentence = "previous_sentence", Navigation,
        "Move to the previous sentence, or to the start of this one when more than three words in",
        gui ["g:Alt+,"], term ["g:Alt+,"], shared ["b:,"];
    NextParagraph = "next_paragraph", Navigation, "Move to the next paragraph",
        gui ["g:Ctrl+P"], term ["g:Ctrl+P"], shared ["b:p", "b:]", "s:PageDown"];
    PreviousParagraph = "previous_paragraph", Navigation, "Move to the previous paragraph",
        gui ["g:Ctrl+Shift+P"], term [], shared ["b:Shift+P", "b:[", "s:PageUp"];
    NextHeading = "next_heading", Navigation, "Read from the next heading",
        gui ["g:Ctrl+H"], term [], shared ["b:>"];
    PreviousHeading = "previous_heading", Navigation, "Read from the previous heading",
        gui ["g:Ctrl+Shift+H"], term [], shared ["b:<"];
    SkipNextHeading = "skip_next_heading", Navigation, "Move to the next heading without reading",
        gui [], term [], shared ["b:h", "b:}"];
    SkipPreviousHeading = "skip_previous_heading", Navigation, "Move to the previous heading without reading",
        gui [], term [], shared ["b:{"];
    NextTable = "next_table", Navigation, "Move to the next table",
        gui ["g:Ctrl+T"], term [], shared ["b:t"];
    PreviousTable = "previous_table", Navigation, "Move to the previous table",
        gui ["g:Ctrl+Shift+T"], term [], shared ["b:Shift+T"];
    NextList = "next_list", Navigation, "Move to the next list",
        gui [], term [], shared ["b:o"];
    PreviousList = "previous_list", Navigation, "Move to the previous list",
        gui [], term [], shared ["b:Shift+O"];
    NextListItem = "next_list_item", Navigation, "Move to the next list item",
        gui [], term [], shared ["b:i"];
    PreviousListItem = "previous_list_item", Navigation, "Move to the previous list item",
        gui [], term [], shared ["b:Shift+I"];
    NextLink = "next_link", Navigation, "Move to the next link",
        gui [], term [], shared ["b:u"];
    PreviousLink = "previous_link", Navigation, "Move to the previous link",
        gui [], term [], shared ["b:Shift+U"];
    NextChapter = "next_chapter", Navigation, "Move to the next chapter or section",
        gui ["g:Alt+PageDown"], term ["g:F11", "g:Alt+PageDown"], shared [];
    PreviousChapter = "previous_chapter", Navigation, "Move to the previous chapter or section",
        gui ["g:Alt+PageUp"], term ["g:F10", "g:Alt+PageUp"], shared [];
    HistoryBack = "history_back", Navigation, "Go back to where you were before the last jump",
        gui ["g:Alt+Left"], term ["g:Alt+Left"], shared ["b:Shift+H"];
    HistoryForward = "history_forward", Navigation, "Go forward again after going back",
        gui ["g:Alt+Right"], term ["g:Alt+Right"], shared ["b:Shift+L"];
    GoTo = "go_to", Navigation, "Go to a line, percentage, or position",
        gui ["g:Ctrl+G"], term ["g:Ctrl+G"], shared [];
    DocumentStart = "document_start", Navigation, "Move to the start of the document",
        gui ["g:Ctrl+Home"], term ["g:Ctrl+Home"], shared ["b:Home"];
    DocumentEnd = "document_end", Navigation, "Move to the end of the document",
        gui ["g:Ctrl+End"], term ["g:Ctrl+End"], shared ["b:End"];
    CaretNextWord = "caret_next_word", Navigation, "Move the cursor to the next word",
        gui [], term [], shared ["b:Right"];
    CaretPreviousWord = "caret_previous_word", Navigation, "Move the cursor to the previous word",
        gui [], term [], shared ["b:Left"];
    CaretNextLine = "caret_next_line", Navigation, "Move the cursor to the next line",
        gui [], term [], shared ["b:Down"];
    CaretPreviousLine = "caret_previous_line", Navigation, "Move the cursor to the previous line",
        gui [], term [], shared ["b:Up"];
    PageDown = "page_down", Navigation, "Move down one screen",
        gui [], term [], shared ["b:PageDown"];
    PageUp = "page_up", Navigation, "Move up one screen",
        gui [], term [], shared ["b:PageUp"];
    ScrollDown = "scroll_down", Navigation, "Scroll down one line without moving the cursor",
        gui [], term [], shared ["b:j"];
    ScrollUp = "scroll_up", Navigation, "Scroll up one line without moving the cursor",
        gui [], term [], shared ["b:k"];

    // Speech Cursor
    SpeechCursorToggle = "speech_cursor_toggle", SpeechCursor, "Enter or leave Speech Cursor (line) mode",
        gui [], term [], shared ["b:Tab", "s:Tab"];
    SpeechCursorNextLine = "speech_cursor_next_line", SpeechCursor, "Speech Cursor: read the next line",
        gui [], term [], shared ["s:Down", "s:j"];
    SpeechCursorPreviousLine = "speech_cursor_previous_line", SpeechCursor, "Speech Cursor: read the previous line",
        gui [], term [], shared ["s:Up", "s:k"];
    SpeechCursorRereadLine = "speech_cursor_reread_line", SpeechCursor, "Speech Cursor: read the current line again",
        gui [], term [], shared ["s:r"];
    SpeechCursorExitAndRead = "speech_cursor_exit_and_read", SpeechCursor, "Speech Cursor: leave and read on from this line",
        gui [], term [], shared ["s:Enter"];

    // Voice
    RateUp = "rate_up", Voice, "Speak faster",
        gui ["g:Ctrl+="], term [], shared ["b:+", "b:="];
    RateDown = "rate_down", Voice, "Speak slower",
        gui ["g:Ctrl+-"], term [], shared ["b:-"];
    PitchUp = "pitch_up", Voice, "Raise the pitch",
        gui ["g:Alt+="], term ["g:Alt+="], shared ["b:)"];
    PitchDown = "pitch_down", Voice, "Lower the pitch",
        gui ["g:Alt+-"], term ["g:Alt+-"], shared ["b:("];
    VolumeUp = "volume_up", Voice, "Louder",
        gui [], term [], shared ["b:0"];
    VolumeDown = "volume_down", Voice, "Quieter",
        gui [], term [], shared ["b:9"];
    CycleSpeedPreset = "cycle_speed_preset", Voice, "Cycle the speed presets (skim, normal, study, slow)",
        gui ["g:F8"], term ["g:F8"], shared [];
    ChooseVoice = "choose_voice", Voice, "Choose a voice",
        gui ["g:Ctrl+Shift+V"], term ["g:Alt+V"], shared [];

    // Search
    Find = "find", Search, "Find text in the document",
        gui ["g:Ctrl+F"], term ["g:Ctrl+F"], shared ["b:/"];
    FindNext = "find_next", Search, "Find the next match",
        gui [], term ["g:F3"], shared ["b:n"];
    FindPrevious = "find_previous", Search, "Find the previous match",
        gui [], term ["g:F4"], shared ["b:Shift+N"];

    // Bookmarks
    AddBookmark = "add_bookmark", Bookmarks, "Add a bookmark at the cursor",
        gui ["g:Ctrl+M"], term [], shared ["b:m"];
    ListBookmarks = "list_bookmarks", Bookmarks, "List bookmarks",
        gui [], term [], shared ["b:Shift+M"];
    NextBookmark = "next_bookmark", Bookmarks, "Move to the next bookmark",
        gui [], term [], shared ["b:b"];
    PreviousBookmark = "previous_bookmark", Bookmarks, "Move to the previous bookmark",
        gui [], term [], shared ["b:Shift+B"];

    // File
    Open = "open", File, "Open a document",
        gui ["g:Ctrl+O"], term ["g:Ctrl+O"], shared [];
    NewDocument = "new_document", File, "Start a new document in edit mode",
        gui ["g:Ctrl+N"], term ["g:Ctrl+N"], shared [];
    Save = "save", File, "Save (Markdown and text in place; other formats as Markdown)",
        gui ["g:Ctrl+S"], term ["g:Ctrl+S"], shared [];
    SaveAs = "save_as", File, "Save under a new name",
        gui ["g:Ctrl+Shift+S"], term ["g:Alt+S"], shared [];
    Quit = "quit", File, "Quit, saving the reading position",
        gui ["g:Ctrl+Q"], term ["g:Ctrl+Q"], shared ["b:q", "b:Shift+Q"];

    // Editing
    ToggleEditMode = "toggle_edit_mode", Editing, "Switch between reading and editing",
        gui ["g:Ctrl+E"], term ["g:Ctrl+E"], shared [];
    Undo = "undo", Editing, "Undo",
        gui ["e:Ctrl+Z"], term ["e:Ctrl+Z"], shared [];
    Redo = "redo", Editing, "Redo",
        gui ["e:Ctrl+Y", "e:Ctrl+Shift+Z"], term ["e:Ctrl+Y"], shared [];
    Bold = "bold", Editing, "Make the selection bold",
        gui ["e:Ctrl+B"], term ["e:Ctrl+B"], shared [];
    Italic = "italic", Editing, "Make the selection italic",
        gui ["e:Ctrl+I"], term ["e:Alt+I"], shared [];
    Underline = "underline", Editing, "Underline the selection",
        gui ["e:Ctrl+U"], term ["e:Ctrl+U"], shared [];
    Strikethrough = "strikethrough", Editing, "Strike through the selection",
        gui ["e:Ctrl+Shift+X"], term ["e:Alt+D"], shared [];
    InlineCode = "inline_code", Editing, "Mark the selection as code",
        gui ["e:Ctrl+`"], term ["e:Alt+`"], shared [];
    CodeBlock = "code_block", Editing, "Make the selected lines a code block",
        gui ["e:Ctrl+Shift+K"], term ["e:Alt+K"], shared [];
    InsertLink = "insert_link", Editing, "Make the selection a link",
        gui ["e:Ctrl+K"], term ["e:Ctrl+K"], shared [];
    Heading = "heading", Editing, "Make the current line a heading",
        gui ["e:Ctrl+Alt+1"], term ["e:Alt+1"], shared [];
    BulletList = "bullet_list", Editing, "Make the selected lines a bulleted list",
        gui ["e:Ctrl+Shift+L"], term ["e:Alt+8"], shared [];
    NumberedList = "numbered_list", Editing, "Make the selected lines a numbered list",
        gui ["e:Ctrl+Shift+O"], term ["e:Alt+7"], shared [];
    BlockQuote = "block_quote", Editing, "Make the selected lines a block quote",
        gui ["e:Ctrl+Shift+Q"], term ["e:Alt+9"], shared [];
    HorizontalRule = "horizontal_rule", Editing, "Insert a horizontal rule",
        gui ["e:Ctrl+Shift+R"], term ["e:Alt+R"], shared [];
    InsertTable = "insert_table", Editing, "Insert a table",
        gui ["e:Ctrl+Shift+A"], term ["e:Alt+T"], shared [];
    AddTableRow = "add_table_row", Editing, "Add a row to the table at the cursor",
        gui ["e:Ctrl+Shift+Enter"], term ["e:Alt+W"], shared [];
    InsertImage = "insert_image", Editing, "Insert an image",
        gui ["e:Ctrl+Shift+I"], term ["e:Alt+G"], shared [];
    Replace = "replace", Editing, "Find and replace",
        gui ["e:Ctrl+Shift+F"], term ["e:Alt+F"], shared [];

    // View and help
    NextTheme = "next_theme", View, "Switch to the next color theme",
        gui ["g:F5"], term ["g:F5"], shared [];
    ToggleLineNumbers = "toggle_line_numbers", View, "Show or hide line numbers",
        gui ["g:F6"], term ["g:F6"], shared [];
    CommandPalette = "command_palette", View, "Run any command by name",
        gui ["g:F2"], term ["g:F2", "g:Alt+X"], shared ["b::"];
    KeyboardHelp = "keyboard_help", View, "List keyboard shortcuts",
        gui ["g:F3"], term [], shared ["b:?"];
    Help = "help", View, "Open the help",
        gui ["g:F1"], term ["g:F1"], shared [];
}

impl ActionId {
    /// The command palette name: the id with spaces, e.g. `next sentence`.
    pub fn palette_name(self) -> String {
        self.id().replace('_', " ")
    }

    /// Actions in `category`, in help order.
    pub fn in_category(category: Category) -> impl Iterator<Item = ActionId> {
        ActionId::ALL
            .iter()
            .copied()
            .filter(move |a| a.category() == category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_are_unique() {
        let mut ids: Vec<&str> = ActionId::ALL.iter().map(|a| a.id()).collect();
        for a in ActionId::ALL {
            assert_eq!(ActionId::from_id(a.id()), Some(*a));
            assert!(!a.help().is_empty());
            assert!(
                a.id().bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{}",
                a.id()
            );
        }
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ActionId::ALL.len());
    }

    #[test]
    fn help_reads_as_a_sentence() {
        for a in ActionId::ALL {
            let h = a.help();
            assert!(h.chars().next().is_some_and(char::is_uppercase), "{h}");
            assert!(!h.ends_with('.'), "{h}");
        }
    }

    #[test]
    fn every_category_has_actions_and_actions_are_grouped() {
        for c in Category::ALL {
            assert!(ActionId::in_category(c).next().is_some(), "{c:?}");
        }
        // `ALL` lists actions category by category, in `Category::ALL` order,
        // so help, the palette, and keyboard.md group the same way.
        let order: Vec<Category> = ActionId::ALL.iter().map(|a| a.category()).collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
        assert_eq!(ActionId::NextSentence.palette_name(), "next sentence");
    }
}
