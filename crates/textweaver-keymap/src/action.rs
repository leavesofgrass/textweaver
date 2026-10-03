//! Every user command, with its help text and default keys.
//!
//! The browse layer mirrors the quick navigation keys of NVDA's and
//! JAWS's browse mode (the owner's decision, 2026-09-26): `h` headings, `1` to
//! `6` heading levels, `l` lists, `i` list items, `t` tables, `k` links,
//! `q` block quotes, `s` separators, `g` graphics, and `d` sections or
//! chapters, each with Shift for the previous one; `Backspace` goes back.
//! Sentences move with `Alt+Down` and `Alt+Up` and paragraphs with
//! `Ctrl+Down` and `Ctrl+Up`, as in both screen readers. The keys this
//! layout displaced keep chords, and the whole earlier layout is the
//! `classic` preset ([`Preset::Classic`](crate::Preset::Classic)).
//!
//! GUI chords follow Star (the Star parity reference Part 1 §6), and new keys
//! cover what Star lacked (pitch, volume, read the current unit, list and
//! link navigation, bookmark stepping).
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
//!   many layouts and types characters;
//! - an action that quits or destroys work asks for confirmation before it
//!   runs ([`ActionId::needs_confirmation`]); only such actions may have a
//!   single printable key (Star's `q` quit at once, even when dictated text
//!   reached the reader, WCAG 2.1.4 Character Key Shortcuts), and every
//!   action stays usable with single-key shortcuts turned off
//!   ([`Keymap::set_character_keys`](crate::Keymap::set_character_keys)),
//!   through a modifier chord or the command palette.
//!
//! Deliberate departures from Star: Ctrl+Q (and `q` in the classic
//! preset) asks "Quit textweaver? y or n" before quitting; `Ctrl+T` means next table in the GUI
//! and nothing in the terminal (Star's TUI used it for the voice picker,
//! Part 1 §7 item 37); `Ctrl+S` saves in both (Star's TUI exported);
//! Redo also answers to `Ctrl+Shift+Z` in the GUI (Star used that chord for
//! its preview pane, which textweaver does not have); in the GUI, `Ctrl+=`
//! (Ctrl+Plus), `Ctrl+-`, and `Ctrl+0` size the text, as screen reader users
//! expect (the owner's session 2, 2026-09-28), so Star's rate chords moved
//! from them to `F11` and `Shift+F11`, beside volume on `F7`.

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
    /// Bookmarks, notes, and highlights.
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
            Category::Bookmarks => "Bookmarks and notes",
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
        gui ["g:Ctrl+Shift+Space"], term ["g:Alt+P"], shared ["b:Space"];
    Stop = "stop", Reading, "Stop reading",
        gui ["g:Escape"], term ["g:Escape", "b:Ctrl+X"], shared [];
    ReadFromCursor = "read_from_cursor", Reading, "Read continuously from the cursor",
        gui ["g:Ctrl+Space"], term ["g:Ctrl+Space"], shared ["b:Enter"];
    ReadDocument = "read_document", Reading, "Read the whole document from the start",
        gui [], term [], shared ["b:Shift+R"];
    ReadCurrentCharacter = "read_current_character", Reading, "Say the character at the cursor",
        gui ["g:Ctrl+Shift+C"], term ["g:Alt+Shift+C"], shared ["b:c"];
    ReadCurrentWord = "read_current_word", Reading, "Say the word at the cursor",
        gui ["g:Ctrl+Shift+W"], term ["g:Alt+Shift+W"], shared ["b:w"];
    ReadCurrentSentence = "read_current_sentence", Reading, "Say the sentence at the cursor without moving",
        gui ["g:Ctrl+Shift+E"], term ["g:Alt+Shift+S"], shared ["b:."];
    ReadCurrentLine = "read_current_line", Reading, "Say the line at the cursor",
        gui ["g:Ctrl+L"], term ["g:Alt+Shift+L"], shared [];
    ReadParagraph = "read_paragraph", Reading, "Say the paragraph at the cursor without moving",
        gui [], term [], shared ["b:,"];
    ReadSelection = "read_selection", Reading, "Read the selected text",
        gui [], term [], shared ["b:v"];
    SayPosition = "say_position", Reading,
        "Say the position: line, percentage, word number, and heading",
        gui ["g:Alt+Shift+Y"], term ["g:Alt+Shift+Y"], shared ["b:Shift+W"];
    SayStatus = "say_status", Reading,
        "Say the last message again, then the status: mode, reading state, position, rate, and speech engine; in a list, the list's introduction",
        gui ["g:Alt+End"], term ["g:Alt+End"], shared ["b:z"];
    RepeatMessage = "repeat_message", Reading, "Say the last message again",
        gui ["g:Alt+'"], term ["g:Alt+'"], shared ["b:'"];
    WordCount = "word_count", Reading, "Say how many words are in the document, or in the selection",
        gui ["g:Alt+Shift+T"], term ["g:Alt+Shift+T"], shared [];
    LinkAddress = "link_address", Reading, "Say the address of the link at the cursor",
        gui ["g:Alt+Shift+K"], term ["g:Alt+Shift+K"], shared [];
    ReplaySentence = "replay_sentence", Reading, "Read again from the start of the current sentence",
        gui ["g:Alt+;"], term ["g:Alt+;"], shared ["b:;"];
    ReplayParagraph = "replay_paragraph", Reading, "Read again from the start of the current paragraph",
        gui ["g:Ctrl+R"], term ["g:Ctrl+R"], shared ["b:r"];
    RsvpToggle = "rsvp_toggle", Reading, "Show or hide RSVP: one word at a time, from the cursor",
        gui ["g:Alt+Shift+R"], term ["g:Alt+Shift+R"], shared [];
    RsvpPlayPause = "rsvp_play_pause", Reading, "Start or pause RSVP",
        gui ["g:Alt+Shift+P"], term ["g:Alt+Shift+P"], shared [];
    RsvpFaster = "rsvp_faster", Reading, "RSVP faster",
        gui ["g:Alt+Shift+Up", "g:Alt+Shift+PageUp"], term ["g:Alt+Shift+Up", "g:Alt+Shift+PageUp"], shared [];
    RsvpSlower = "rsvp_slower", Reading, "RSVP slower",
        gui ["g:Alt+Shift+Down", "g:Alt+Shift+PageDown"], term ["g:Alt+Shift+Down", "g:Alt+Shift+PageDown"], shared [];
    RsvpPositionNext = "rsvp_position_next", Reading, "Move the RSVP word to the next place on the screen",
        gui ["g:Alt+Shift+O"], term ["g:Alt+Shift+O"], shared [];
    ReadingLevel = "reading_level", Reading, "Say the reading level of the document or the selection",
        gui ["g:Alt+Shift+G"], term ["g:Alt+Shift+G"], shared [];
    DefineWord = "define_word", Reading,
        "Define the word at the cursor, or the selected words: senses, examples, synonyms, and pronunciation",
        gui ["g:Ctrl+Shift+D"], term ["g:Alt+E"], shared [];
    Summarize = "summarize", Reading,
        "Summarize the selection, the chapter, or the document: its most central sentences in a list; Enter goes to one",
        gui [], term [], shared [];
    ToggleCitations = "toggle_citations", Reading,
        "Turn citations on or off in continuous reading: off skips them, on says them in words",
        gui ["g:Alt+Shift+Q"], term ["g:Alt+Shift+Q"], shared [];
    ExploreMath = "explore_math", Reading,
        "Explore the math at the cursor term by term: arrows move, Down goes into a part, Up comes out, Escape leaves",
        gui ["g:Alt+Shift+X"], term ["g:Alt+Shift+X"], shared [];
    ListenRendered = "listen_rendered", Reading,
        "Listen to the document as it will render, without leaving edit mode",
        gui [], term [], shared [];

    // Navigation
    NextSentence = "next_sentence", Navigation, "Move to the next sentence",
        gui ["g:Alt+.", "g:Alt+Down"], term ["g:Alt+.", "g:Alt+Down"], shared [];
    PreviousSentence = "previous_sentence", Navigation,
        "Move to the previous sentence, or to the start of this one when more than three words in",
        gui ["g:Alt+,", "g:Alt+Up"], term ["g:Alt+,", "g:Alt+Up"], shared [];
    NextParagraph = "next_paragraph", Navigation, "Move to the next paragraph",
        gui ["g:Ctrl+P"], term ["g:Ctrl+P"], shared ["b:p", "b:]", "b:Ctrl+Down", "s:PageDown"];
    PreviousParagraph = "previous_paragraph", Navigation, "Move to the previous paragraph",
        gui ["g:Ctrl+Shift+P"], term [], shared ["b:Shift+P", "b:[", "b:Ctrl+Up", "s:PageUp"];
    NextHeading = "next_heading", Navigation, "Read from the next heading",
        gui ["g:Ctrl+H"], term [], shared ["b:>"];
    PreviousHeading = "previous_heading", Navigation, "Read from the previous heading",
        gui ["g:Ctrl+Shift+H"], term [], shared ["b:<"];
    SkipNextHeading = "skip_next_heading", Navigation, "Move to the next heading without reading",
        gui [], term ["g:Alt+H"], shared ["b:h", "b:}"];
    SkipPreviousHeading = "skip_previous_heading", Navigation, "Move to the previous heading without reading",
        gui [], term ["g:Alt+Shift+H"], shared ["b:Shift+H", "b:{"];
    Outline = "outline", Navigation, "List the headings: type to filter, Enter jumps to one",
        gui ["g:Alt+O"], term ["g:Alt+O"], shared [];
    NextHeadingLevel1 = "next_heading_level_1", Navigation, "Move to the next heading at level 1",
        gui [], term [], shared ["b:1"];
    NextHeadingLevel2 = "next_heading_level_2", Navigation, "Move to the next heading at level 2",
        gui [], term [], shared ["b:2"];
    NextHeadingLevel3 = "next_heading_level_3", Navigation, "Move to the next heading at level 3",
        gui [], term [], shared ["b:3"];
    NextHeadingLevel4 = "next_heading_level_4", Navigation, "Move to the next heading at level 4",
        gui [], term [], shared ["b:4"];
    NextHeadingLevel5 = "next_heading_level_5", Navigation, "Move to the next heading at level 5",
        gui [], term [], shared ["b:5"];
    NextHeadingLevel6 = "next_heading_level_6", Navigation, "Move to the next heading at level 6",
        gui [], term [], shared ["b:6"];
    PreviousHeadingLevel1 = "previous_heading_level_1", Navigation, "Move to the previous heading at level 1",
        gui [], term [], shared ["b:!"];
    PreviousHeadingLevel2 = "previous_heading_level_2", Navigation, "Move to the previous heading at level 2",
        gui [], term [], shared ["b:@"];
    PreviousHeadingLevel3 = "previous_heading_level_3", Navigation, "Move to the previous heading at level 3",
        gui [], term [], shared ["b:#"];
    PreviousHeadingLevel4 = "previous_heading_level_4", Navigation, "Move to the previous heading at level 4",
        gui [], term [], shared ["b:$"];
    PreviousHeadingLevel5 = "previous_heading_level_5", Navigation, "Move to the previous heading at level 5",
        gui [], term [], shared ["b:%"];
    PreviousHeadingLevel6 = "previous_heading_level_6", Navigation, "Move to the previous heading at level 6",
        gui [], term [], shared ["b:^"];
    NextTable = "next_table", Navigation, "Move to the next table",
        gui ["g:Ctrl+T"], term [], shared ["b:t"];
    PreviousTable = "previous_table", Navigation, "Move to the previous table",
        gui ["g:Ctrl+Shift+T"], term [], shared ["b:Shift+T"];
    NextList = "next_list", Navigation, "Move to the next list",
        gui [], term [], shared ["b:l"];
    PreviousList = "previous_list", Navigation, "Move to the previous list",
        gui [], term [], shared ["b:Shift+L"];
    NextListItem = "next_list_item", Navigation, "Move to the next list item",
        gui [], term [], shared ["b:i"];
    PreviousListItem = "previous_list_item", Navigation, "Move to the previous list item",
        gui [], term [], shared ["b:Shift+I"];
    NextLink = "next_link", Navigation, "Move to the next link",
        gui [], term [], shared ["b:k", "b:u"];
    PreviousLink = "previous_link", Navigation, "Move to the previous link",
        gui [], term [], shared ["b:Shift+K", "b:Shift+U"];
    NextBlockQuote = "next_block_quote", Navigation, "Move to the next block quote",
        gui [], term [], shared ["b:q"];
    PreviousBlockQuote = "previous_block_quote", Navigation, "Move to the previous block quote",
        gui [], term [], shared ["b:Shift+Q"];
    NextSeparator = "next_separator", Navigation, "Move to the next separator (horizontal rule)",
        gui [], term [], shared ["b:s"];
    PreviousSeparator = "previous_separator", Navigation, "Move to the previous separator (horizontal rule)",
        gui [], term [], shared ["b:Shift+S"];
    NextGraphic = "next_graphic", Navigation, "Move to the next graphic (image)",
        gui [], term [], shared ["b:g"];
    PreviousGraphic = "previous_graphic", Navigation, "Move to the previous graphic (image)",
        gui [], term [], shared ["b:Shift+G"];
    FollowLink = "follow_link", Navigation,
        "Follow the link at the cursor, or go between a footnote and its note",
        gui ["g:Alt+Shift+F"], term ["g:Alt+Shift+F"], shared [];
    TableNextRow = "table_next_row", Navigation, "In a table, move down a row in the same column",
        gui ["g:Ctrl+Alt+Down"], term ["g:Ctrl+Alt+Down"], shared [];
    TablePreviousRow = "table_previous_row", Navigation, "In a table, move up a row in the same column",
        gui ["g:Ctrl+Alt+Up"], term ["g:Ctrl+Alt+Up"], shared [];
    TableNextColumn = "table_next_column", Navigation, "In a table, move to the next cell in the row",
        gui ["g:Ctrl+Alt+Right"], term ["g:Ctrl+Alt+Right"], shared [];
    TablePreviousColumn = "table_previous_column", Navigation, "In a table, move to the previous cell in the row",
        gui ["g:Ctrl+Alt+Left"], term ["g:Ctrl+Alt+Left"], shared [];
    NextChapter = "next_chapter", Navigation, "Move to the next chapter or section",
        gui ["g:Alt+PageDown"], term ["g:F11", "g:Alt+PageDown"], shared ["b:d"];
    PreviousChapter = "previous_chapter", Navigation, "Move to the previous chapter or section",
        gui ["g:Alt+PageUp"], term ["g:Alt+PageUp"], shared ["b:Shift+D"];
    HistoryBack = "history_back", Navigation, "Go back to where you were before the last jump",
        gui ["g:Alt+Left"], term ["g:Alt+Left"], shared ["b:Backspace"];
    HistoryForward = "history_forward", Navigation, "Go forward again after going back",
        gui ["g:Alt+Right"], term ["g:Alt+Right"], shared ["b:\\"];
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
    SelectNextWord = "select_next_word", Navigation, "Extend the selection to the next word",
        gui [], term [], shared ["b:Shift+Right"];
    SelectPreviousWord = "select_previous_word", Navigation, "Extend the selection to the previous word",
        gui [], term [], shared ["b:Shift+Left"];
    SelectNextLine = "select_next_line", Navigation, "Extend the selection to the next line",
        gui [], term [], shared ["b:Shift+Down"];
    SelectPreviousLine = "select_previous_line", Navigation, "Extend the selection to the previous line",
        gui [], term [], shared ["b:Shift+Up"];
    PageDown = "page_down", Navigation, "Move down one screen",
        gui [], term [], shared ["b:PageDown"];
    PageUp = "page_up", Navigation, "Move up one screen",
        gui [], term [], shared ["b:PageUp"];
    ScrollDown = "scroll_down", Navigation, "Scroll down one line without moving the cursor",
        gui [], term [], shared ["b:j"];
    ScrollUp = "scroll_up", Navigation, "Scroll up one line without moving the cursor",
        gui [], term [], shared ["b:Shift+J"];

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
        gui ["g:F11"], term [], shared ["b:+", "b:="];
    RateDown = "rate_down", Voice, "Speak slower",
        gui ["g:Shift+F11"], term [], shared ["b:-"];
    PitchUp = "pitch_up", Voice, "Raise the pitch",
        gui ["g:Alt+="], term ["g:Alt+="], shared ["b:)"];
    PitchDown = "pitch_down", Voice, "Lower the pitch",
        gui ["g:Alt+-"], term ["g:Alt+-"], shared ["b:("];
    VolumeUp = "volume_up", Voice, "Louder",
        gui ["g:F7"], term ["g:F7"], shared ["b:0"];
    VolumeDown = "volume_down", Voice, "Quieter",
        gui ["g:Shift+F7"], term ["g:Shift+F7"], shared ["b:9"];
    CycleSpeedPreset = "cycle_speed_preset", Voice, "Cycle the speed presets (skim, normal, study, slow)",
        gui ["g:F8"], term ["g:F8"], shared [];
    ChooseVoice = "choose_voice", Voice, "Choose a voice",
        gui ["g:Ctrl+Shift+V"], term ["g:Alt+V"], shared [];
    RestartSpeech = "restart_speech", Voice,
        "Restart speech with the current settings (after the speech engine stopped working)",
        gui ["g:Shift+F8"], term ["g:Shift+F8"], shared [];
    CycleVerbosity = "cycle_verbosity", Voice, "Cycle how much textweaver says: low, normal, high",
        gui ["g:Alt+Shift+V"], term ["g:Alt+Shift+V"], shared [];
    CyclePunctuation = "cycle_punctuation", Voice, "Cycle how much punctuation is spoken: none, some, all",
        gui ["g:Alt+Shift+N"], term ["g:Alt+Shift+N"], shared [];

    // Search
    Find = "find", Search, "Find text in the document",
        gui ["g:Ctrl+F"], term ["g:Ctrl+F"], shared ["b:/"];
    FindNext = "find_next", Search, "Find the next match",
        gui [], term ["g:F3"], shared ["b:n"];
    FindPrevious = "find_previous", Search, "Find the previous match",
        gui [], term ["g:F4"], shared ["b:Shift+N"];
    NextMisspelling = "next_misspelling", Search, "Move to the next misspelled word, and spell it",
        gui ["g:Alt+M"], term ["g:Alt+M"], shared [];
    PreviousMisspelling = "previous_misspelling", Search, "Move to the previous misspelled word, and spell it",
        gui ["g:Alt+Shift+M"], term ["g:Alt+Shift+M"], shared [];
    SpellingSuggestions = "spelling_suggestions", Search,
        "List suggestions for the misspelled word at the cursor, or add it to your word list",
        gui ["g:Alt+J"], term ["g:Alt+J"], shared [];
    NextGrammarProblem = "next_grammar_problem", Search,
        "Move to the next grammar problem, and say it and its fix",
        gui ["g:Ctrl+F7"], term ["g:Ctrl+F7"], shared [];
    PreviousGrammarProblem = "previous_grammar_problem", Search,
        "Move to the previous grammar problem, and say it and its fix",
        gui ["g:Ctrl+Shift+F7"], term ["g:Ctrl+Shift+F7"], shared [];
    NextLintProblem = "next_lint_problem", Search,
        "In edit mode, move to the next Markdown lint problem, and say it",
        gui ["g:Ctrl+F8"], term ["g:Ctrl+F8"], shared [];
    PreviousLintProblem = "previous_lint_problem", Search,
        "In edit mode, move to the previous Markdown lint problem, and say it",
        gui ["g:Ctrl+Shift+F8"], term ["g:Ctrl+Shift+F8"], shared [];

    // Bookmarks
    AddBookmark = "add_bookmark", Bookmarks, "Add a bookmark at the cursor",
        gui ["g:Ctrl+M"], term [], shared ["b:m"];
    ListBookmarks = "list_bookmarks", Bookmarks, "List bookmarks",
        gui [], term [], shared ["b:Shift+M"];
    NextBookmark = "next_bookmark", Bookmarks, "Move to the next bookmark",
        gui [], term [], shared ["b:b"];
    PreviousBookmark = "previous_bookmark", Bookmarks, "Move to the previous bookmark",
        gui [], term [], shared ["b:Shift+B"];
    AddNote = "add_note", Bookmarks, "Add a note to the selection or the sentence at the cursor",
        gui ["g:Alt+N"], term ["g:Alt+N"], shared ["b:a"];
    ListNotes = "list_notes", Bookmarks, "List notes",
        gui ["g:Ctrl+Shift+N"], term [], shared ["b:Shift+A"];
    NextNote = "next_note", Bookmarks, "Move to the next note",
        gui ["g:F12"], term ["g:F12"], shared ["b:e"];
    PreviousNote = "previous_note", Bookmarks, "Move to the previous note",
        gui ["g:Shift+F12"], term ["g:Shift+F12"], shared ["b:Shift+E"];
    DeleteNote = "delete_note", Bookmarks, "Delete the note or highlight at the cursor",
        gui [], term [], shared ["b:Delete"];
    HighlightSelection = "highlight_selection", Bookmarks, "Highlight the selection, or the sentence at the cursor",
        gui [], term [], shared ["b:y"];
    ExportStudySheet = "export_study_sheet", Bookmarks,
        "Export the notes and highlights as a Markdown study sheet, grouped by heading",
        gui [], term [], shared [];

    // File
    Open = "open", File, "Open a document",
        gui ["g:Ctrl+O"], term ["g:Ctrl+O"], shared [];
    OpenPath = "open_path", File, "Open a document by typing its path",
        gui ["g:Ctrl+Shift+G"], term [], shared [];
    OpenLibrary = "open_library", File, "Open the library: documents in your library folders and recent files",
        gui ["g:Ctrl+Shift+B"], term ["g:Alt+L"], shared [];
    ContinueReading = "continue_reading", File,
        "Continue reading: the documents on this computer with a saved place, from any computer, newest first",
        gui [], term [], shared [];
    EditDocumentDetails = "edit_document_details", File,
        "Edit the document's details: title, author, DOI, and ISBN",
        gui [], term [], shared [];
    NewDocument ="new_document", File, "Start a new document in edit mode",
        gui ["g:Ctrl+N"], term ["g:Ctrl+N"], shared [];
    Save = "save", File, "Save (Markdown and text in place; other formats as Markdown)",
        gui ["g:Ctrl+S"], term ["g:Ctrl+S"], shared [];
    SaveAs = "save_as", File, "Save under a new name",
        gui ["g:Ctrl+Shift+S"], term ["g:Alt+S"], shared [];
    ExportSettings = "export_settings", File, "Export settings and key overrides to a JSON or TOML file",
        gui ["g:Alt+Shift+E"], term ["g:Alt+Shift+E"], shared [];
    ImportSettings = "import_settings", File, "Import settings from a JSON or TOML file, after a yes or no",
        gui ["g:Alt+Shift+I"], term ["g:Alt+Shift+I"], shared [];
    ReadingStatistics = "reading_statistics", File,
        "List reading statistics: time read, the furthest point, sessions, and the most read documents",
        gui ["g:Ctrl+Shift+Y"], term ["g:Alt+Y"], shared [];
    NewFromTemplate = "new_from_template", File,
        "Start a new document from a template, with a title, author, date, and References heading",
        gui [], term [], shared [];
    ExportHtml = "export_html", File, "Export the document as a web page (HTML) next to it",
        gui [], term [], shared [];
    ExportPdf = "export_pdf", File, "Export the document as a tagged PDF next to it",
        gui [], term [], shared [];
    ExportDocx = "export_docx", File, "Export the document as a Word file (DOCX) next to it",
        gui [], term [], shared [];
    ExportEpub = "export_epub", File, "Export the document as an EPUB book next to it",
        gui [], term [], shared [];
    ExportBrf = "export_brf", File, "Export the document as braille (BRF) next to it",
        gui [], term [], shared [];
    PreviewInBrowser = "preview_in_browser", File,
        "Preview the document in the web browser, with math; each save rewrites the preview",
        gui [], term [], shared [];
    TogglePreviewAutoReload = "toggle_preview_auto_reload", File,
        "Turn automatic reloading of the browser preview on or off",
        gui [], term [], shared [];
    TogglePreviewLive = "toggle_preview_live", File,
        "Turn live preview on or off: with automatic reloading, the preview also reloads when typing pauses",
        gui [], term [], shared [];
    BrowseFiles = "browse_files", File,
        "Browse files and archives: Enter opens a folder, an archive, or a document; Backspace goes up",
        gui [], term [], shared [];
    BatchConvert = "batch_convert", File,
        "Convert a folder of documents to another format, in the background",
        gui [], term [], shared [];
    ExportAudio = "export_audio", File,
        "Export the document as spoken audio: MP3, FLAC, Opus, WAV, or an M4B audiobook",
        gui [], term [], shared [];
    SyncSetup = "sync_setup", File,
        "Set up sync: choose the sync folder, name this computer, and choose what syncs",
        gui [], term [], shared [];
    SyncStatus = "sync_status", File,
        "Say how sync stands (up to date, the folder missing, or a problem) and name the other computers",
        gui ["g:Shift+F5"], term ["g:Shift+F5"], shared [];
    SyncNow = "sync_now", File,
        "Sync now: send this computer's changes and take the other computers' for every document",
        gui [], term [], shared [];
    SyncGoToPlace = "sync_go_to_place", File,
        "List the other computers' places in this document; Enter goes to one",
        gui [], term [], shared [];
    SyncReplacedNotes = "sync_replaced_notes", File,
        "List notes replaced by another computer's newer edit; Enter puts one back",
        gui [], term [], shared [];
    SyncStop = "sync_stop", File,
        "Stop syncing on this computer; the sync folder is left as it is",
        gui [], term [], shared [];
    Quit = "quit", File, "Quit, saving the reading position",
        gui ["g:Ctrl+Q"], term ["g:Ctrl+Q"], shared [];

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
    Copy = "copy", Editing, "Copy the selection, or the sentence at the cursor, to the clipboard",
        gui ["g:Ctrl+C"], term ["g:Ctrl+C"], shared [];
    Cut = "cut", Editing, "Cut the selection to the clipboard",
        gui ["e:Ctrl+X"], term ["e:Ctrl+X"], shared [];
    NextTableCell = "next_table_cell", Editing,
        "In a table, move to the next cell and say its column; elsewhere, type a tab",
        gui ["e:Tab"], term ["e:Tab"], shared [];
    PreviousTableCell = "previous_table_cell", Editing,
        "In a table, move to the previous cell and say its column",
        gui ["e:Shift+Tab"], term ["e:Shift+Tab"], shared [];
    CycleTypingEcho = "cycle_typing_echo", Editing,
        "Cycle typing echo: characters and words, characters, words, or none",
        gui ["g:Shift+F9"], term ["g:Shift+F9"], shared [];
    SelectAll = "select_all", Editing, "Select all the text",
        gui ["e:Ctrl+A"], term ["e:Ctrl+A"], shared [];
    DeleteWordBefore = "delete_word_before", Editing, "Delete the word before the cursor",
        gui ["e:Ctrl+Backspace"], term ["e:Alt+Backspace"], shared [];
    DeleteWordAfter = "delete_word_after", Editing, "Delete the word after the cursor",
        gui ["e:Ctrl+Delete"], term ["e:Ctrl+Delete"], shared [];
    Paste = "paste", Editing,
        "Paste the text last copied or cut in textweaver; the terminal paste works too",
        gui ["e:Ctrl+V"], term ["e:Ctrl+V"], shared [];
    InsertCitation = "insert_citation", Editing,
        "Insert a citation: pick a reference, then give a page or other locator",
        gui ["e:Alt+C"], term ["e:Alt+C"], shared [];
    AddReference = "add_reference", Editing, "Add a reference to your library by DOI or ISBN",
        gui ["g:Alt+Shift+D"], term ["g:Alt+B"], shared [];
    InsertBibliography = "insert_bibliography", Editing,
        "Insert the bibliography of the works cited, at the cursor",
        gui [], term [], shared [];
    CheckCitations = "check_citations", Editing,
        "Check the citations: how many there are, and which keys are not in your library",
        gui [], term [], shared [];
    ImportReferences = "import_references", Editing,
        "Import references from a BibTeX, RIS, or CSL-JSON file into your library",
        gui [], term [], shared [];
    Dictate = "dictate", Editing,
        "Start or stop dictation: spoken words are typed at the cursor in edit mode",
        gui ["g:Ctrl+Shift+F9"], term ["g:Ctrl+Shift+F9"], shared [];
    DownloadDictationModel = "download_dictation_model", Editing,
        "Download the dictation model chosen in the settings, after saying its size and license",
        gui [], term [], shared [];

    // View and help
    NextTheme = "next_theme", View, "Switch to the next color theme",
        gui ["g:F5"], term ["g:F5"], shared [];
    // The terminal's gutter: the window has none, and gives F6 to its
    // regions (Wave 8d), as Windows programs do.
    ToggleLineNumbers = "toggle_line_numbers", View, "Show or hide line numbers",
        gui [], term ["g:F6"], shared [];
    ToggleCharacterKeys = "toggle_character_keys", View,
        "Turn single-key shortcuts on or off, so dictation and typing never trigger commands",
        gui ["g:F9"], term ["g:F9"], shared [];
    CycleAccessMode = "cycle_access_mode", View,
        "Cycle the accessibility mode: self-voicing, hybrid, or screen reader",
        gui ["g:Alt+Shift+A"], term ["g:Alt+Shift+A"], shared [];
    SettingsProfiles = "settings_profiles", View,
        "List settings profiles: switch to one, save the current settings as one, rename, delete, import, or export",
        gui ["g:Ctrl+Shift+U"], term ["g:Alt+U"], shared [];
    BionicToggle = "bionic_toggle", View, "Turn bionic reading on or off: the start of each word in bold",
        gui ["g:Alt+Shift+B"], term ["g:Alt+Shift+B"], shared [];
    RulerCycle = "ruler_cycle", View, "Cycle the reading ruler: off, current line, ruler",
        gui ["g:Alt+Shift+U"], term ["g:Alt+Shift+U"], shared [];
    SyllablesToggle = "syllables_toggle", View, "Show or hide syllables: words split with a middle dot",
        gui ["g:Alt+Shift+Z"], term ["g:Alt+Shift+Z"], shared [];
    DifficultWordsToggle = "difficult_words_toggle", View,
        "Mark difficult words on or off: underlined, and named on word moves at high verbosity",
        gui ["g:Alt+Shift+J"], term ["g:Alt+Shift+J"], shared [];
    TextLarger = "text_larger", View, "Make the document text larger",
        gui ["g:Ctrl+="], term [], shared [];
    TextSmaller = "text_smaller", View, "Make the document text smaller",
        gui ["g:Ctrl+-"], term [], shared [];
    TextSizeReset = "text_size_reset", View, "Return the document text to its standard size",
        gui ["g:Ctrl+0"], term [], shared [];
    ChooseFont = "choose_font", View, "Choose the font of the document text",
        gui ["g:Ctrl+D"], term [], shared [];
    ContentsPanel = "contents_panel", View,
        "Show the Contents panel beside the document and go to it, or close it from inside it: Enter goes to a heading",
        gui ["g:Ctrl+1"], term [], shared [];
    NotesPanel = "notes_panel", View,
        "Show the Notes panel beside the document and go to it, or close it from inside it: Enter goes to a note",
        gui ["g:Ctrl+2"], term [], shared [];
    NextRegion = "next_region", View,
        "Move to the next part of the window: the header, the panel, the document, or the toolbar",
        gui ["g:F6"], term [], shared [];
    PreviousRegion = "previous_region", View,
        "Move to the previous part of the window",
        gui ["g:Shift+F6"], term [], shared [];
    ColorSettings = "color_settings", View,
        "Open the color settings: the reading highlight, the ruler, marks, and each part of the screen, with their contrast",
        gui [], term [], shared [];
    CycleInterfaceAnnouncements = "cycle_interface_announcements", View,
        "Cycle how much textweaver announces about itself: off, minimal, normal, or full; errors and answers are always said",
        gui ["g:Ctrl+F9"], term ["g:Ctrl+F9"], shared [];
    Menu = "menu", View, "Open the menus: File, Edit, View, Reading, Speech, Tools, and Help",
        gui ["g:F10"], term ["g:F10"], shared [];
    CommandPalette = "command_palette", View, "Run any command by name",
        gui ["g:F2"], term ["g:F2", "g:Alt+X"], shared ["b::"];
    Settings = "settings", View,
        "Open the settings: every option with its help; Left and Right change a value",
        gui ["g:Ctrl+,"], term ["g:Shift+F10"], shared [];
    KeyboardHelp = "keyboard_help", View, "List keyboard shortcuts",
        gui ["g:F3"], term [], shared ["b:?"];
    WhatDoesThisKeyDo = "what_does_this_key_do", View,
        "Press a key to hear what it does and where it is in the menus, without running it",
        gui ["g:Shift+F1"], term ["g:Shift+F1"], shared [];
    About = "about", View, "Say textweaver's version and license",
        gui [], term [], shared [];
    ManageComponents = "manage_components", View,
        "Manage optional components: the models, fonts, and voices textweaver can download, with their size and license",
        gui [], term [], shared [];
    Help = "help", View, "Open the help",
        gui ["g:F1"], term ["g:F1"], shared [];
}

impl ActionId {
    /// The next-heading action for level 1 to 6 (the browse keys 1 to 6).
    pub fn next_heading_at(level: u8) -> Option<ActionId> {
        use ActionId as A;
        Some(match level {
            1 => A::NextHeadingLevel1,
            2 => A::NextHeadingLevel2,
            3 => A::NextHeadingLevel3,
            4 => A::NextHeadingLevel4,
            5 => A::NextHeadingLevel5,
            6 => A::NextHeadingLevel6,
            _ => return None,
        })
    }

    /// The previous-heading action for level 1 to 6 (Shift with 1 to 6).
    pub fn previous_heading_at(level: u8) -> Option<ActionId> {
        use ActionId as A;
        Some(match level {
            1 => A::PreviousHeadingLevel1,
            2 => A::PreviousHeadingLevel2,
            3 => A::PreviousHeadingLevel3,
            4 => A::PreviousHeadingLevel4,
            5 => A::PreviousHeadingLevel5,
            6 => A::PreviousHeadingLevel6,
            _ => return None,
        })
    }

    /// The heading level and direction (true for next) of a heading-level
    /// action.
    pub fn heading_level_jump(self) -> Option<(u8, bool)> {
        (1..=6u8).find_map(|l| {
            if Self::next_heading_at(l) == Some(self) {
                Some((l, true))
            } else if Self::previous_heading_at(l) == Some(self) {
                Some((l, false))
            } else {
                None
            }
        })
    }

    /// True for actions the app must confirm before running, however they
    /// are triggered (key, palette, or script): quitting ("Quit textweaver?
    /// y or n": `y` quits; `n`, `a`, or Escape aborts) and deleting a note
    /// or highlight. A single printable key reaches them only through that
    /// confirmation, so a stray or dictated keystroke cannot quit or delete.
    pub fn needs_confirmation(self) -> bool {
        matches!(self, ActionId::Quit | ActionId::DeleteNote)
    }

    /// The confirmation question for an action that
    /// [`needs_confirmation`](Self::needs_confirmation), worded to be read
    /// aloud.
    pub fn confirmation_prompt(self) -> Option<&'static str> {
        match self {
            ActionId::Quit => Some("Quit textweaver? y or n"),
            ActionId::DeleteNote => Some("Delete this note or highlight? y or n"),
            _ => None,
        }
    }

    /// True for commands with no default keys, run from the command
    /// palette (exports, templates, citation checks). Users may still bind
    /// keys to them in `keymap.toml`.
    pub fn is_palette_command(self) -> bool {
        let d = self.defaults();
        d.gui.is_empty() && d.terminal.is_empty() && d.shared.is_empty()
    }

    /// True for commands only the window has: its text size and font,
    /// Open by typed path (the terminal's Open is a typed path already),
    /// its Contents and Notes panels, and moving between its regions (F6).
    /// They have GUI keys and no terminal keys; the terminal's palette still
    /// lists them, and says where they work.
    pub fn is_window_only(self) -> bool {
        matches!(
            self,
            ActionId::OpenPath
                | ActionId::TextLarger
                | ActionId::TextSmaller
                | ActionId::TextSizeReset
                | ActionId::ChooseFont
                | ActionId::ContentsPanel
                | ActionId::NotesPanel
                | ActionId::NextRegion
                | ActionId::PreviousRegion
        )
    }

    /// True for commands only the terminal reader has keys for: line
    /// numbers, which are the terminal's gutter. They have terminal keys
    /// and no window keys, so F6 is the window's, for its regions; the
    /// window's palette leaves them out.
    pub fn is_terminal_only(self) -> bool {
        matches!(self, ActionId::ToggleLineNumbers)
    }

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
                a.id()
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{}",
                a.id()
            );
        }
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ActionId::ALL.len());
    }

    #[test]
    fn confirmation_marks() {
        let marked: Vec<ActionId> = ActionId::ALL
            .iter()
            .copied()
            .filter(|a| a.needs_confirmation())
            .collect();
        assert_eq!(marked, vec![ActionId::DeleteNote, ActionId::Quit]);
        for a in ActionId::ALL {
            assert_eq!(
                a.needs_confirmation(),
                a.confirmation_prompt().is_some(),
                "{a:?}"
            );
        }
        assert_eq!(
            ActionId::Quit.confirmation_prompt(),
            Some("Quit textweaver? y or n")
        );
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
