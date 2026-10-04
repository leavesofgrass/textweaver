//! `DocumentView`: the document as one focusable node for screen readers,
//! drawn with Parley and Vello, written against Masonry and AccessKit
//! directly (ADR-0027).
//!
//! - **Accessibility.** One node, role `Document` and read-only, whose
//!   children are text runs of at most 255 characters ([`crate::runs`]).
//!   The caret is the node's text selection; the spoken word is its own run
//!   with a background colour; headings carry their level's size and
//!   weight. Only the runs of lines that changed are sent again, so a
//!   highlight move costs a line or two, not the paragraph, even when the
//!   paragraph is one very long line.
//! - **Drawing.** Only the lines of a paragraph that are on screen are
//!   drawn, with their syllable marks and the ruler's rows, and positions
//!   in a long paragraph are found without counting from its start.
//! - **Windowing.** The view holds the window's paragraphs only
//!   ([`crate::window`]); positions are document-absolute. Only the
//!   paragraphs on screen are laid out, from a cache.
//! - **Keys.** The platform's caret keys ([`keys::caret_keys`]: arrows,
//!   Home, End, Page Up, and Page Down, with Ctrl on Windows and Linux or
//!   Option and Command on macOS for words, paragraphs, and the ends, and
//!   Shift to select) move the caret here, and the screen reader reads what
//!   the caret moved over. Every other key goes on to the keymap.
//! - **Actions.** A user's caret move is reported as [`DocAction`]; the
//!   driver passes it to the app as `Command::SetCursor`.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use masonry::accesskit::{
    Action, ActionData, Color as AkColor, Node, NodeId, Role, TextDirection, TextPosition,
    TextSelection,
};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, AccessEvent, BrushIndex, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
    PointerButtonEvent, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, StyleProperty, TextEvent, Update, UpdateCtx, Widget, WidgetMut, render_text,
};
use masonry::dpi::{LogicalPosition, PhysicalPosition};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, BezPath, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::style::{FontFamily, FontStyle, FontWeight, LineHeight};
use masonry::parley::{Affinity, Cursor, FontContext, Layout, LayoutContext, Selection};
use textweaver_app::aids::{RowMark, RulerMode, RulerSettings, TextSpacing, ViewRow, ruler_rows};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::KeyChord;
use textweaver_app::keymap::Platform;

use crate::caret::{self, CharBytes};
use crate::keys::{self, CaretStep};
use crate::runs::{ParaRuns, Paragraph, Run, RunMark, RunSet};
use crate::theme::{self, Palette};
use crate::window::{SpanStyle, StyledSpan};

/// Space around the text column (design system C3: on the 4 px scale).
const INSET: f64 = 24.0;
/// The narrowest column the measure gives, in logical pixels (a window
/// narrower than this keeps its own width).
const MIN_COLUMN: f64 = 200.0;
/// The line length's range in characters (`[display] measure`); 0 fills
/// the window.
pub const MEASURE_RANGE: (u16, u16) = (25, 90);
/// The default line length in characters.
pub const DEFAULT_MEASURE: u16 = 66;
/// The text whose width, divided by its length, is the average advance
/// the measure counts in: every letter once, and eight spaces.
const PANGRAM: &str = "the quick brown fox jumps over the lazy dog";
/// Paragraph layouts kept before the cache is trimmed.
const CACHE_LIMIT: usize = 600;
/// Scroll distance of one wheel line.
const WHEEL_LINE: f64 = 48.0;

// Brush indices into the palette ([`DocumentView::brushes`]).
const B_TEXT: usize = 0;
const B_H1: usize = 2;
const B_LINK: usize = 8;
const B_CODE: usize = 9;
const B_QUOTE: usize = 10;
const B_DIFFICULT: usize = 11;
const B_SYLLABLE: usize = 12;
const BRUSHES: usize = 13;

/// The reading font.
#[derive(Clone, Debug, PartialEq)]
pub struct DocFont {
    /// CSS-style family list, most wanted first.
    pub family: String,
    /// Size in logical pixels.
    pub size: f32,
    /// Bold body text.
    pub bold: bool,
}

impl Default for DocFont {
    fn default() -> Self {
        DocFont {
            family: crate::fonts::DEFAULT_STACK.to_owned(),
            size: theme::DOC_TEXT,
            bold: false,
        }
    }
}

/// The reading aids the view draws itself (ADR-0022): text spacing and the
/// reading ruler. Bionic reading and difficult words arrive as styled
/// spans in the model. All of them are drawn only; the text runs a screen
/// reader gets do not change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocAids {
    /// Line height, paragraph, letter, and word spacing, in multiples of
    /// the font size.
    pub spacing: TextSpacing,
    /// Off, the current line, or the ruler band (and the mask around it).
    pub ruler: RulerSettings,
    /// The line length in characters (`[display] measure`), 25 to 90; 0
    /// fills the window. The column is this many times the reading font's
    /// average advance, so it follows the size and spacing.
    pub measure: u16,
}

impl Default for DocAids {
    fn default() -> Self {
        DocAids {
            spacing: TextSpacing::default(),
            ruler: RulerSettings::default(),
            measure: DEFAULT_MEASURE,
        }
    }
}

/// What the reading highlight draws when it is not the spoken word and
/// sentence themselves (`[highlight] enabled`, `granularity`, and
/// `lead_words`). Drawn only: the caret a screen reader follows while
/// reading stays on the word being spoken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HighlightShown {
    /// The word drawn, if any.
    pub word: Option<CharRange>,
    /// The sentence drawn, if any.
    pub sentence: Option<CharRange>,
}

/// What the document view shows: the window's text and its styles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocModel {
    /// The window's paragraphs.
    pub paragraphs: Vec<Paragraph>,
    /// Styles over the window.
    pub spans: Vec<StyledSpan>,
    /// The whole document's length in chars.
    pub doc_len: usize,
    /// The document's title (the node's description).
    pub title: String,
    /// Syllables (a reading aid): the separator is drawn before the char
    /// at each of these positions, in order. Empty when syllables are off.
    /// Drawn only: the text runs a screen reader gets stay the words.
    pub breaks: Vec<CharPos>,
    /// What is drawn between syllables (a middle dot by default).
    pub separator: String,
}

impl DocModel {
    /// The styles drawn on each paragraph: the spans that touch it and its
    /// syllable breaks. A paragraph whose text and styles are the same in
    /// two models can keep its layout.
    fn styles_by_paragraph(&self) -> Vec<(Vec<StyledSpan>, Vec<CharPos>)> {
        let paras = &self.paragraphs;
        let mut out: Vec<(Vec<StyledSpan>, Vec<CharPos>)> = self
            .spans_by_paragraph()
            .into_iter()
            .map(|s| (s, Vec::new()))
            .collect();
        for &k in &self.breaks {
            out[caret::paragraph_at(paras, k)].1.push(k);
        }
        out
    }

    /// The spans that touch each paragraph, in the model's order.
    fn spans_by_paragraph(&self) -> Vec<Vec<StyledSpan>> {
        let paras = &self.paragraphs;
        let mut out = vec![Vec::new(); paras.len()];
        if paras.is_empty() {
            return out;
        }
        for s in &self.spans {
            let a = caret::paragraph_at(paras, s.range.start);
            let b = caret::paragraph_at(paras, CharPos(s.range.end.0.saturating_sub(1)));
            for slot in out.iter_mut().take(b.max(a) + 1).skip(a) {
                slot.push(*s);
            }
        }
        out
    }
}

/// Something the reader marked or found, drawn over the text (as the
/// terminal draws them). Each has a shape as well as a color, so no color
/// carries it alone: a reader's highlight has a solid line under it, a note
/// a dashed one, a bookmark a bar at its start, a search match a box around
/// it, and the match at the caret a heavier box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocMark {
    /// A highlight the reader made.
    Highlight,
    /// Text with a note.
    Note,
    /// A bookmark.
    Bookmark,
    /// A search match.
    FindHit,
    /// The search match at the caret.
    CurrentFindHit,
}

/// One step of the reading highlights in the last paint, in the order it
/// was drawn ([`DocumentView::painted`]). Rectangles are in the view's
/// coordinates. Kept so tests can check what the window draws, and in what
/// order, without reading pixels.
///
/// The order on each paragraph (design system E): the sentence band, code
/// backgrounds, the marks' bands, the selection band, the word band, the
/// text, the word in its own color with its attribute, then every line,
/// box, bar, and dot (the marks' shapes, syllable dots, the double line of
/// a writing suggestion), then the line under the sentence, the dots under
/// a misspelling, and the caret. So no band covers a shape, a mark inside
/// the spoken sentence stays visible while it is read, and the sentence is
/// underlined in every palette, high contrast too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaintStep {
    /// The band behind one line of the spoken sentence.
    SentenceBand(Rect),
    /// The band of a mark ([`DocMark`]) on one line.
    MarkBand(DocMark, Rect),
    /// The shape of a mark on one line: its line, dashes, bar or box (the
    /// mark's band rectangle).
    MarkShape(DocMark, Rect),
    /// The band behind one line of the spoken word.
    WordBand(Rect),
    /// A paragraph's text.
    Text(usize),
    /// A list item's bullet or number at this depth (from 1), in the
    /// hanging indent left of its text: the box it is drawn in.
    ListMarker(u8, Rect),
    /// The empty window's hint (no document is open), drawn in the
    /// document's place: the box it is drawn in.
    Hint(Rect),
    /// The edit-mode badge, the word "Editing" in a box in the top right
    /// corner: the box.
    Badge(Rect),
    /// The "reading from here" mark: a play-shaped triangle in the left
    /// margin on the caret's line while not reading, where Play starts.
    ReadingFrom(Rect),
    /// The spoken word's text again in its own color, inside `clip`, with
    /// its attribute. Bold is drawn by thickening the glyphs' outlines in
    /// place, so the line never reflows.
    WordText {
        /// The word's band on one line.
        clip: Rect,
        /// Drawn bold.
        bold: bool,
        /// Drawn slanted.
        italic: bool,
        /// Underlined.
        underline: bool,
    },
    /// The line under one line of the spoken sentence, at the font's
    /// underline position, in the sentence's text color.
    SentenceUnderline(Rect, textweaver_theme::Rgb),
    /// The reading ruler's bar at the start of one row: 4 px on the
    /// reading line, 2 px on the band's rows around it.
    RulerBar(RowMark, Rect),
    /// A reading aid's line or dots over one line of text, in its color
    /// (the theme's role, or `[colors]`): the rectangle they fill.
    Aid(AidMark, Rect, textweaver_theme::Rgb),
}

/// The reading aids' marks drawn as lines and dots ([`PaintStep::Aid`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AidMark {
    /// A difficult word: an underline 0.1 em thick.
    DifficultWord,
    /// A syllable separator: a middle dot between syllables.
    Syllable,
    /// A misspelled word: 2 px dots every 4 px at its foot.
    Misspelling,
    /// A writing suggestion (lint or grammar): a double underline, two
    /// 1 px lines 2 px apart.
    Lint,
}

/// The width of the reading ruler's bar on the reading line.
const RULER_FOCUS_BAR: f64 = 4.0;
/// The width of its bar on the band's rows: half the reading line's.
const RULER_BAND_BAR: f64 = 2.0;

/// The moving state: where the caret and the highlights are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DocState {
    /// The caret.
    pub caret: CharPos,
    /// The fixed end of the selection, when there is one.
    pub anchor: Option<CharPos>,
    /// The word being spoken.
    pub spoken: Option<CharRange>,
    /// The sentence being spoken.
    pub sentence: Option<CharRange>,
    /// True while reading aloud.
    pub reading: bool,
}

/// What the document view tells the driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocAction {
    /// The user moved the caret (keys, pointer, or a screen reader).
    CaretMoved {
        /// The new caret.
        caret: CharPos,
        /// The selection, if any.
        selection: Option<CharRange>,
        /// What textweaver's own voice says for a caret key (the driver
        /// passes it to `App::echo`, which speaks only in the self-voicing
        /// mode; a screen reader reads the caret itself). `None` for the
        /// pointer and a screen reader's own moves.
        echo: Option<CaretEcho>,
    },
    /// Edit mode: Tab (`forward`) or Shift+Tab. The driver runs the app's
    /// `next_table_cell` or `previous_table_cell`, as the terminal does:
    /// the next cell in a table, else a tab typed. Ctrl+Tab moves the
    /// focus out instead, so the edit never traps the keyboard.
    TableCell {
        /// Tab, not Shift+Tab.
        forward: bool,
    },
    /// Edit mode: text typed at the caret, with no selection (a key, Enter
    /// as a new line, or an input method's text). The driver sends it as
    /// `Command::Insert`, so it is echoed as the access mode says.
    Typed(String),
    /// Edit mode: Backspace (`forward` false) or Delete with no selection.
    /// The driver sends `Command::DeleteBack` or `DeleteForward`.
    Delete {
        /// Delete, not Backspace.
        forward: bool,
    },
    /// The window took the focus (Alt+Tab, a click) with the document
    /// focused. The driver says the document's and the window's names in
    /// textweaver's own voice (the self-voicing mode); a screen reader says
    /// the window's title and the focused document itself.
    WindowFocused,
    /// Edit mode: replace `range` with `text`: typing or deleting over a
    /// selection, or a screen reader's or dictation's edit
    /// (`ReplaceSelectedText`, `SetValue`). The driver sends it as
    /// `Command::ReplaceRange`, which is quiet: the screen reader says it.
    Replace {
        /// The chars replaced (document positions).
        range: CharRange,
        /// Their replacement.
        text: String,
    },
}

/// What a caret key moved onto, for textweaver's own voice in the
/// self-voicing mode (as the terminal's caret keys say it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaretEcho {
    /// The character now at the caret (Left, Right, Home, End).
    Char(char),
    /// The caret is at the end of a line.
    LineEnd,
    /// The caret is at the end of the document.
    DocEnd,
    /// The word at the caret (Ctrl+Left, Ctrl+Right).
    Word(String),
    /// The line the caret is on, as drawn (Up, Down, the page keys,
    /// Ctrl+Up, Ctrl+Down, Ctrl+Home, Ctrl+End).
    Line(String),
    /// Shift with a caret key: the text the selection gained (`selected`)
    /// or lost.
    Selection {
        /// The text.
        text: String,
        /// True when the selection grew.
        selected: bool,
    },
}

/// A laid-out paragraph.
struct ParaLayout {
    layout: Layout<BrushIndex>,
    /// Height including the space after it.
    height: f64,
    /// Space above the text (headings get more).
    top_gap: f64,
    /// Syllable separators drawn in the paragraph, if any.
    seps: Option<SepMarks>,
    /// Byte indices of the paragraph's chars, for lookups that do not count
    /// from its start (a very long line).
    bytes: CharBytes,
    /// Each visual line's chars, as offsets into the paragraph's text.
    lines: Vec<(usize, usize)>,
    /// How far the text starts right of the column (a list item's hanging
    /// indent); 0 for other paragraphs. The layout's own coordinates start
    /// there, so hit testing, the caret and the bands add it back.
    indent: f64,
    /// A list item's bullet or number, laid out once, and its origin
    /// relative to the text's (left of it, on the first line's baseline).
    marker: Option<(Layout<BrushIndex>, Vec2)>,
}

/// The hanging indent of one list level, in multiples of the font size.
const LIST_INDENT_EM: f64 = 1.75;
/// The space between a list item's marker and its text, in multiples of the
/// font size.
const LIST_GAP_EM: f64 = 0.4;

/// The syllable separators of a paragraph: the separator laid out once in
/// the paragraph's font, and where it goes. The char before each break is
/// laid out with extra letter spacing as wide as the separator, so the
/// text keeps its own bytes (the caret, hit testing, and the screen
/// reader's text are the words) and gains no line-break opportunity.
struct SepMarks {
    layout: Layout<BrushIndex>,
    /// The separator's advance.
    width: f32,
    /// Its baseline within its own layout.
    baseline: f32,
    /// Byte offsets in the paragraph's text of the chars the separator is
    /// drawn before.
    at: Vec<usize>,
    /// Where each separator is drawn, worked out once at layout (not on
    /// every paint): its visual line, and its origin in the layout's
    /// coordinates. In line order.
    placed: Vec<(usize, Vec2)>,
}

/// The document view widget. See the module documentation.
pub struct DocumentView {
    palette: Palette,
    font: DocFont,
    aids: DocAids,
    model: DocModel,
    state: DocState,
    /// Speech-cursor style: select the spoken word instead of a caret.
    select_spoken: bool,
    /// Expose the document as a read-only multi-line edit instead of a
    /// Document (an experiment: screen readers may treat a Document as a
    /// web-style page in browse mode and keep single-letter keys).
    edit_role: bool,
    /// Edit mode (ADR-0033): a multi-line edit that takes typing.
    editing: bool,
    /// Whose caret keys and command modifier the view follows
    /// ([`keys::caret_keys`]).
    platform: Platform,
    /// Caret keys left to the keymap in the app's mode
    /// ([`keys::yielded_caret_keys`]: Speech Cursor mode's line keys).
    yielded: Vec<KeyChord>,
    focused: bool,
    /// The node's name, in the interface language ("Document").
    label: String,
    /// Misspelled words (edit mode), drawn with a dotted underline. Paint
    /// only: the text, its layout, and its runs do not change.
    misspelled: Vec<CharRange>,
    /// Notes, bookmarks, the reader's highlights, and search matches
    /// ([`DocMark`]), drawn only.
    marks: Vec<(CharRange, DocMark)>,
    /// Writing suggestions (edit mode, Markdown lint), drawn with a double
    /// underline. Paint only.
    lint: Vec<CharRange>,
    /// What the reading highlight draws when it is not the spoken word and
    /// sentence ([`HighlightShown`]); `None` draws them.
    shown: Option<HighlightShown>,
    /// The reading font's average advance with the current spacing, in
    /// logical pixels ([`PANGRAM`]); `None` until measured again.
    advance: Option<f64>,
    /// The reading highlights of the last paint, in order.
    painted: Vec<PaintStep>,

    /// The model's spans that touch each paragraph, built when the model
    /// changes, so laying out or painting a paragraph looks at its own
    /// spans only, not every span of the window.
    para_spans: Vec<Vec<StyledSpan>>,

    // Layout.
    layouts: HashMap<usize, ParaLayout>,
    /// Visual line starts (char offsets) per paragraph, for the current
    /// width and font; `None` until laid out.
    line_starts: Vec<Option<Vec<usize>>>,
    column: f64,
    column_x: f64,
    size: Size,
    /// Scroll anchor: a paragraph and how far down it the view starts.
    top: (usize, f64),
    /// The last scroll went down, so the anchor snaps to the next line
    /// boundary rather than the one above ([`DocumentView::snap_top`]).
    scrolled_down: bool,
    /// Paragraphs on screen and their top y, from the last layout.
    visible: Vec<(usize, f64)>,
    follow: bool,
    goal_x: Option<f32>,

    // Accessibility.
    /// Each paragraph's runs, by visual line, built when first asked for.
    para_runs: Vec<Option<ParaRuns>>,
    /// The ids of each paragraph's runs, by line, as last sent.
    para_ids: Vec<Option<Vec<Vec<NodeId>>>>,
    /// Paragraphs whose runs are all sent again in the next pass.
    dirty_paras: Vec<bool>,
    /// Lines whose runs changed since the last pass, by paragraph (a moved
    /// highlight): only their runs are sent again.
    changed_lines: HashMap<usize, Vec<usize>>,
    run_ids: HashMap<CharPos, NodeId>,
    ids_to_pos: HashMap<NodeId, CharPos>,
    /// The window slid: forget the ids of runs no longer shown after the
    /// next accessibility pass.
    prune_ids: bool,
    full_passes: Rc<Cell<u64>>,
    seen_full: u64,
    /// Nodes sent in the last pass, when measuring.
    pub last_nodes_sent: usize,

    /// What the view draws when no document is open; also the node's
    /// description then, so the screen reader has what is drawn.
    empty_hint: String,
    /// The hint laid out, while no document is open.
    hint_layout: Option<Layout<BrushIndex>>,
    /// The word drawn in the corner in edit mode ("Editing").
    editing_word: String,
    /// The word laid out, in edit mode.
    badge_layout: Option<Layout<BrushIndex>>,
}

impl std::fmt::Debug for DocumentView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentView")
            .field("paragraphs", &self.model.paragraphs.len())
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

fn ak_color(c: textweaver_theme::Rgb) -> AkColor {
    AkColor {
        red: c.r,
        green: c.g,
        blue: c.b,
        alpha: 255,
    }
}

impl DocumentView {
    /// An empty view. `full_passes` counts the accessibility passes that
    /// rebuild every node (the root widget bumps it); the view then sends
    /// all its runs again.
    pub fn new(palette: Palette, font: DocFont, full_passes: Rc<Cell<u64>>) -> Self {
        DocumentView {
            palette,
            font,
            aids: DocAids::default(),
            model: DocModel::default(),
            para_spans: Vec::new(),
            state: DocState::default(),
            select_spoken: false,
            edit_role: false,
            editing: false,
            platform: Platform::current(),
            yielded: Vec::new(),
            focused: false,
            label: "Document".to_owned(),
            misspelled: Vec::new(),
            marks: Vec::new(),
            lint: Vec::new(),
            shown: None,
            advance: None,
            painted: Vec::new(),
            layouts: HashMap::new(),
            line_starts: Vec::new(),
            column: 0.0,
            column_x: 0.0,
            size: Size::ZERO,
            top: (0, 0.0),
            scrolled_down: false,
            visible: Vec::new(),
            follow: true,
            goal_x: None,
            para_runs: Vec::new(),
            para_ids: Vec::new(),
            dirty_paras: Vec::new(),
            changed_lines: HashMap::new(),
            run_ids: HashMap::new(),
            ids_to_pos: HashMap::new(),
            prune_ids: false,
            full_passes,
            seen_full: u64::MAX,
            last_nodes_sent: 0,
            empty_hint: String::new(),
            hint_layout: None,
            editing_word: String::new(),
            badge_layout: None,
        }
    }

    /// What the view draws, and its node describes, when no document is
    /// open ("No document is open. Press Ctrl+O to open one."), in the
    /// interface language.
    pub fn with_empty_hint(mut self, hint: impl Into<String>) -> Self {
        self.empty_hint = hint.into();
        self
    }

    /// A new empty-window hint (the interface language or the key changed).
    pub fn set_empty_hint(this: &mut WidgetMut<'_, Self>, hint: impl Into<String>) {
        let hint = hint.into();
        if this.widget.empty_hint != hint {
            this.widget.empty_hint = hint;
            this.ctx.request_layout();
            this.ctx.request_accessibility_update();
        }
    }

    /// The word drawn in the corner in edit mode ("Editing"), in the
    /// interface language.
    pub fn with_editing_word(mut self, word: impl Into<String>) -> Self {
        self.editing_word = word.into();
        self
    }

    /// A new edit-mode word (the interface language changed).
    pub fn set_editing_word(this: &mut WidgetMut<'_, Self>, word: impl Into<String>) {
        let word = word.into();
        if this.widget.editing_word != word {
            this.widget.editing_word = word;
            this.ctx.request_layout();
        }
    }

    /// The empty-window hint, when it was drawn in the last paint (no
    /// document is open).
    pub fn hint_shown(&self) -> Option<&str> {
        (self.hint_layout.is_some() && self.model.paragraphs.is_empty())
            .then_some(self.empty_hint.as_str())
    }

    /// The edit-mode badge's word, when the view draws it (edit mode).
    pub fn badge_shown(&self) -> Option<&str> {
        (self.editing && self.badge_layout.is_some()).then_some(self.editing_word.as_str())
    }

    /// Selects the spoken word while reading, instead of placing a caret on
    /// it (an experiment for screen-reader listening sessions).
    pub fn with_select_spoken(mut self, on: bool) -> Self {
        self.select_spoken = on;
        self
    }

    /// Follows `platform`'s caret keys and command modifier instead of
    /// this system's (tests of another platform's keys).
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// Leaves these caret keys to the keymap (the app's mode binds them).
    pub fn set_yielded_keys(this: &mut WidgetMut<'_, Self>, keys: Vec<KeyChord>) {
        this.widget.yielded = keys;
    }

    /// Names the node for screen readers, in the interface language
    /// (`gui-document`); "Document" until then.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// A new name for the node (the interface language changed).
    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: impl Into<String>) {
        this.widget.label = label.into();
        this.ctx.request_accessibility_update();
    }

    /// Exposes the view as a read-only multi-line edit (UI Automation's
    /// Edit, AT-SPI's text) instead of a Document.
    pub fn with_edit_role(mut self, on: bool) -> Self {
        self.edit_role = on;
        self
    }

    fn brushes(&self) -> [masonry::peniko::Brush; BRUSHES] {
        let p = &self.palette;
        let c = |rgb| theme::color(rgb).into();
        [
            c(p.text),
            c(p.dim_text),
            c(p.headings[0]),
            c(p.headings[1]),
            c(p.headings[2]),
            c(p.headings[3]),
            c(p.headings[4]),
            c(p.headings[5]),
            c(p.link),
            c(p.code),
            c(p.quote),
            c(p.difficult_word),
            c(p.syllable_mark),
        ]
    }

    fn para_count(&self) -> usize {
        self.model.paragraphs.len()
    }

    fn reset_caches(&mut self) {
        let n = self.para_count();
        self.layouts.clear();
        self.line_starts = vec![None; n];
        self.para_runs = vec![None; n];
        self.para_ids = vec![None; n];
        self.dirty_paras = vec![true; n];
        self.changed_lines.clear();
    }

    // --- Mutation from the driver.

    /// Replaces the window's text: a new document, or a new window around
    /// a jump. Every text run is a new node.
    pub fn set_model(this: &mut WidgetMut<'_, Self>, model: DocModel) {
        Self::replace_model(this, model, false);
    }

    /// The window slid while reading or moving (`WindowChange::Forward` or
    /// `Backward`): the text that stays keeps its run nodes, so a screen
    /// reader's place and the caret stay on the same nodes; runs that left
    /// the window are dropped after the next accessibility pass, and the
    /// caret (the text selection) is sent again with it.
    pub fn slide_model(this: &mut WidgetMut<'_, Self>, model: DocModel) {
        Self::replace_model(this, model, true);
    }

    /// Edit mode: the text changed at the caret (a key, an undo, a
    /// command). The paragraphs before the change keep everything; those
    /// after it keep their layouts, their visual lines, and their run nodes,
    /// only moved by the change's length, so a key costs one paragraph's
    /// layout and runs instead of the window's, and the screen reader gets
    /// the edited paragraph's nodes only.
    ///
    /// # Errors
    /// Gives `model` back, changing nothing, when the two models share
    /// neither a first nor a last paragraph; the caller then replaces or
    /// slides the model.
    pub fn edit_model(
        this: &mut WidgetMut<'_, Self>,
        model: DocModel,
    ) -> Result<(), Box<DocModel>> {
        let w = &mut *this.widget;
        let old = &w.model;
        let (n_old, n_new) = (old.paragraphs.len(), model.paragraphs.len());
        let delta = model.doc_len as isize - old.doc_len as isize;
        let shift = |p: CharPos| CharPos((p.0 as isize + delta).max(0) as usize);
        let same_text = |a: &Paragraph, b: &Paragraph| {
            a.text == b.text && a.heading == b.heading && a.has_break == b.has_break
        };
        if model.separator != old.separator {
            return Err(Box::new(model));
        }
        let old_styles = old.styles_by_paragraph();
        let new_styles = model.styles_by_paragraph();
        let shifted = |s: &(Vec<StyledSpan>, Vec<CharPos>)| {
            (
                s.0.iter()
                    .map(|x| StyledSpan {
                        range: CharRange::new(shift(x.range.start).0, shift(x.range.end).0),
                        style: x.style,
                    })
                    .collect::<Vec<_>>(),
                s.1.iter().map(|&b| shift(b)).collect::<Vec<_>>(),
            )
        };
        let limit = n_old.min(n_new);
        let mut pre = 0;
        while pre < limit
            && old.paragraphs[pre].start == model.paragraphs[pre].start
            && same_text(&old.paragraphs[pre], &model.paragraphs[pre])
            && old_styles[pre] == new_styles[pre]
        {
            pre += 1;
        }
        let mut suf = 0;
        while suf < limit - pre {
            let (i, j) = (n_old - 1 - suf, n_new - 1 - suf);
            let (a, b) = (&old.paragraphs[i], &model.paragraphs[j]);
            if shift(a.start) != b.start
                || !same_text(a, b)
                || shifted(&old_styles[i]) != new_styles[j]
            {
                break;
            }
            suf += 1;
        }
        if pre == 0 && suf == 0 {
            return Err(Box::new(model));
        }
        // Where the old suffix starts: runs at or after it move by `delta`;
        // runs of the old middle paragraphs are gone.
        let old_mid = pre..n_old - suf;
        let suf_start = old.paragraphs.get(n_old - suf).map(|p| p.start);
        let anchor_pos = old.paragraphs.get(w.top.0).map(|p| p.start);

        let mut layouts = std::mem::take(&mut w.layouts);
        let old_lines = std::mem::take(&mut w.line_starts);
        let mut old_runs = std::mem::take(&mut w.para_runs);
        let mut old_ids = std::mem::take(&mut w.para_ids);
        let mut old_dirty = std::mem::take(&mut w.dirty_paras);
        // Lines waiting to be sent go with their paragraph, sent whole.
        for (i, _) in w.changed_lines.drain() {
            if let Some(d) = old_dirty.get_mut(i) {
                *d = true;
            }
        }
        let mut new_layouts = HashMap::new();
        let mut lines = vec![None; n_new];
        let mut runs = vec![None; n_new];
        let mut ids = vec![None; n_new];
        let mut dirty = vec![true; n_new];
        let keep = (0..pre)
            .map(|i| (i, i))
            .chain((0..suf).map(|k| (n_old - suf + k, n_new - suf + k)));
        for (i, j) in keep {
            if let Some(l) = layouts.remove(&i) {
                new_layouts.insert(j, l);
            }
            lines[j] = old_lines.get(i).cloned().flatten();
            ids[j] = old_ids.get_mut(i).and_then(Option::take);
            dirty[j] = old_dirty.get(i).copied().unwrap_or(true);
            // The prefix's runs are where they were; the suffix's are
            // rebuilt from the paragraph when next asked for.
            if i < pre {
                runs[j] = old_runs.get_mut(i).and_then(Option::take);
            }
        }
        // The ids of runs in the old middle paragraphs are forgotten; the
        // suffix's ids move with their text.
        for i in old_mid {
            for id in old_ids
                .get_mut(i)
                .and_then(Option::take)
                .into_iter()
                .flatten()
                .flatten()
            {
                if let Some(pos) = w.ids_to_pos.remove(&id) {
                    w.run_ids.remove(&pos);
                }
            }
        }
        if let Some(from) = suf_start
            && delta != 0
        {
            let moved: Vec<(NodeId, CharPos)> = w
                .ids_to_pos
                .iter()
                .filter(|(_, p)| **p >= from)
                .map(|(id, p)| (*id, *p))
                .collect();
            for (_, p) in &moved {
                w.run_ids.remove(p);
            }
            for (id, p) in moved {
                let to = shift(p);
                w.ids_to_pos.insert(id, to);
                w.run_ids.insert(to, id);
            }
        }
        w.model = model;
        w.para_spans = w.model.spans_by_paragraph();
        w.layouts = new_layouts;
        w.line_starts = lines;
        w.para_runs = runs;
        w.para_ids = ids;
        w.dirty_paras = dirty;
        let anchor = anchor_pos.map(|p| {
            if suf_start.is_some_and(|s| p >= s) {
                shift(p)
            } else {
                p
            }
        });
        w.top = (
            anchor.map_or(0, |pos| caret::paragraph_at(&w.model.paragraphs, pos)),
            w.top.1,
        );
        w.follow = true;
        this.ctx.request_layout();
        // No accessibility request of its own: the root would take it for
        // a full pass, and every run would be sent again. The layout pass
        // this asks for brings the accessibility pass with it.
        this.ctx.request_render();
        Ok(())
    }

    fn replace_model(this: &mut WidgetMut<'_, Self>, model: DocModel, keep_ids: bool) {
        let w = &mut *this.widget;
        // Keep the scroll anchor on the same text when the window moves.
        let anchor_pos = w.model.paragraphs.get(w.top.0).map(|p| p.start);
        let old = std::mem::replace(&mut w.model, model);
        w.para_spans = w.model.spans_by_paragraph();
        let mut old_layouts = std::mem::take(&mut w.layouts);
        let mut old_lines = std::mem::take(&mut w.line_starts);
        w.reset_caches();
        if keep_ids {
            w.prune_ids = true;
            // Paragraphs that stay keep their layout and visual lines, so
            // their runs, split at the same lines, keep their text. Only
            // when their styles are the same too: a reading aid turned on
            // or off (bionic reading, difficult words, syllables) keeps the
            // text and the run nodes but needs new layouts.
            let by_start: HashMap<CharPos, usize> = old
                .paragraphs
                .iter()
                .enumerate()
                .map(|(i, p)| (p.start, i))
                .collect();
            let old_styles = old.styles_by_paragraph();
            let new_styles = w.model.styles_by_paragraph();
            let same_sep = old.separator == w.model.separator;
            for (j, p) in w.model.paragraphs.iter().enumerate().skip(1) {
                let Some(&i) = by_start.get(&p.start) else {
                    continue;
                };
                let q = &old.paragraphs[i];
                if q.text != p.text || q.heading != p.heading {
                    continue;
                }
                if !same_sep || old_styles[i] != new_styles[j] {
                    continue;
                }
                if let Some(lines) = old_lines.get_mut(i).and_then(Option::take) {
                    w.line_starts[j] = Some(lines);
                }
                if let Some(l) = old_layouts.remove(&i) {
                    w.layouts.insert(j, l);
                }
            }
        } else {
            w.run_ids.clear();
            w.ids_to_pos.clear();
        }
        w.top = (
            anchor_pos.map_or(0, |pos| caret::paragraph_at(&w.model.paragraphs, pos)),
            0.0,
        );
        w.follow = true;
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    /// Updates the caret and highlights.
    pub fn set_state(this: &mut WidgetMut<'_, Self>, state: DocState) {
        let w = &mut *this.widget;
        if w.state == state {
            return;
        }
        let old = w.state;
        w.state = state;
        // Runs change only where the spoken word was or is.
        if old.spoken != state.spoken {
            for r in [old.spoken, state.spoken].into_iter().flatten() {
                w.mark_dirty(r);
            }
        }
        if old.caret != state.caret || old.spoken != state.spoken {
            w.goal_x = None;
            w.follow = true;
            this.ctx.request_layout();
        }
        this.ctx.request_render();
    }

    /// Edit mode on or off: the view becomes a multi-line edit that takes
    /// typed text, and back to a read-only document.
    pub fn set_editing(this: &mut WidgetMut<'_, Self>, on: bool) {
        if this.widget.editing != on {
            this.widget.editing = on;
            this.widget.state.anchor = None;
            this.ctx.request_accessibility_update();
            // The badge is laid out in the next layout pass.
            this.ctx.request_layout();
            this.ctx.request_render();
        }
    }

    /// True in edit mode.
    pub fn editing(&self) -> bool {
        self.editing
    }

    /// Changes the colours.
    pub fn set_palette(this: &mut WidgetMut<'_, Self>, palette: Palette) {
        this.widget.palette = palette;
        this.widget.dirty_paras.iter_mut().for_each(|d| *d = true);
        this.ctx.request_render();
    }

    /// Changes the reading font.
    pub fn set_font(this: &mut WidgetMut<'_, Self>, font: DocFont) {
        if this.widget.font != font {
            this.widget.font = font;
            this.widget.advance = None;
            this.widget.reset_caches();
            this.widget.follow = true;
            this.ctx.request_layout();
            this.ctx.request_render();
        }
    }

    /// Changes the reading aids: new spacing lays the text out again; a
    /// new ruler is only drawn.
    pub fn set_aids(this: &mut WidgetMut<'_, Self>, aids: DocAids) {
        let w = &mut *this.widget;
        if w.aids == aids {
            return;
        }
        if w.aids.spacing != aids.spacing {
            w.advance = None;
        }
        let relayout = w.aids.spacing != aids.spacing || w.aids.measure != aids.measure;
        w.aids = aids;
        if relayout {
            w.reset_caches();
            w.follow = true;
            this.ctx.request_layout();
        }
        this.ctx.request_render();
    }

    /// The misspelled words to mark (edit mode; W4a3 left them unmarked).
    /// Only drawn: a dotted underline, a shape unlike a link's line or a
    /// difficult word's thick one, so no color carries it.
    pub fn set_misspelled(this: &mut WidgetMut<'_, Self>, ranges: Vec<CharRange>) {
        if this.widget.misspelled != ranges {
            this.widget.misspelled = ranges;
            this.ctx.request_render();
        }
    }

    /// The notes, bookmarks, highlights, and search matches to mark. Only
    /// drawn, each with a shape of its own besides its color (see
    /// [`DocMark`]); a screen reader finds them with their keys and lists.
    pub fn set_marks(this: &mut WidgetMut<'_, Self>, marks: Vec<(CharRange, DocMark)>) {
        if this.widget.marks != marks {
            this.widget.marks = marks;
            this.ctx.request_render();
        }
    }

    /// The writing suggestions to mark (edit mode on Markdown): a double
    /// underline, a shape no other mark has. Only drawn; Ctrl+F8 finds
    /// them and says them.
    pub fn set_lint(this: &mut WidgetMut<'_, Self>, ranges: Vec<CharRange>) {
        if this.widget.lint != ranges {
            this.widget.lint = ranges;
            this.ctx.request_render();
        }
    }

    /// What the reading highlight draws, when not the spoken word and
    /// sentence themselves (`None`). Only drawn.
    pub fn set_highlight_shown(this: &mut WidgetMut<'_, Self>, shown: Option<HighlightShown>) {
        if this.widget.shown != shown {
            this.widget.shown = shown;
            this.ctx.request_render();
        }
    }

    /// The text column's width in logical pixels, for tests.
    pub fn column_width(&self) -> f64 {
        self.column
    }

    /// The chars on each visual line of paragraph `i` as laid out, for
    /// tests; `None` before it is laid out.
    pub fn line_lengths(&self, i: usize) -> Option<Vec<usize>> {
        self.layouts
            .get(&i)
            .map(|pl| pl.lines.iter().map(|(a, b)| b - a).collect())
    }

    /// The marks drawn, for tests.
    pub fn marks(&self) -> &[(CharRange, DocMark)] {
        &self.marks
    }

    /// The misspelled words marked, for tests.
    pub fn misspelled(&self) -> &[CharRange] {
        &self.misspelled
    }

    /// The reading font in use.
    pub fn font(&self) -> &DocFont {
        &self.font
    }

    /// The reading aids in use.
    pub fn aids(&self) -> DocAids {
        self.aids
    }

    /// The current state.
    pub fn state(&self) -> DocState {
        self.state
    }

    /// The paragraphs on screen (index and top y), for tests.
    pub fn visible_paragraphs(&self) -> &[(usize, f64)] {
        &self.visible
    }

    /// How many syllable separators the paragraphs on screen draw, for
    /// tests.
    pub fn syllable_marks_on_screen(&self) -> usize {
        self.visible
            .iter()
            .filter_map(|(i, _)| self.layouts.get(i))
            .map(|pl| pl.seps.as_ref().map_or(0, |s| s.at.len()))
            .sum()
    }

    /// The highlights the text runs carry (the spoken word).
    fn run_marks(&self) -> Vec<(CharRange, RunMark)> {
        self.state
            .spoken
            .map(|r| (r, RunMark::SpokenWord))
            .into_iter()
            .collect()
    }

    /// The runs over `r` changed (the spoken word moved there or away):
    /// builds again only the lines of runs that hold it, and sends only
    /// those in the next pass.
    fn mark_dirty(&mut self, r: CharRange) {
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return;
        }
        let a = caret::paragraph_at(paras, r.start);
        let b = caret::paragraph_at(paras, CharPos(r.end.0.saturating_sub(1).max(r.start.0)));
        let marks = self.run_marks();
        for i in a..=b.min(paras.len() - 1) {
            match self.para_runs[i].as_mut() {
                Some(pr) => {
                    let lines = pr.lines_touching(r);
                    pr.rebuild(lines.clone(), &self.model.paragraphs[i].text, &marks);
                    self.changed_lines.entry(i).or_default().extend(lines);
                }
                None => self.dirty_paras[i] = true,
            }
        }
    }

    // --- Layout.

    fn heading_scale(level: u8) -> f32 {
        match level {
            1 => 1.75,
            2 => 1.45,
            3 => 1.25,
            4 => 1.12,
            _ => 1.0,
        }
    }

    fn build_layout(
        &self,
        i: usize,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> ParaLayout {
        let p = &self.model.paragraphs[i];
        let size = self.font.size;
        let sp = self.aids.spacing;
        let text = p.text.as_str();
        let bytes = CharBytes::new(text);
        // Syllables: the separator in the paragraph's own font, laid out
        // once, before the paragraph's builder takes the contexts.
        let breaks = self.breaks_in(i);
        let seps = (!breaks.is_empty() && !self.model.separator.is_empty()).then(|| {
            let sep = self.model.separator.as_str();
            let mut sb = lcx.ranged_builder(fcx, sep, 1.0, true);
            sb.push_default(StyleProperty::FontFamily(FontFamily::Source(
                self.font.family.clone().into(),
            )));
            sb.push_default(StyleProperty::FontSize(
                size * p.heading.map_or(1.0, Self::heading_scale),
            ));
            if p.heading.is_some() || self.font.bold {
                sb.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
            }
            // In the syllable mark's color (the theme's role, or
            // `[colors] syllables`), a glyph that is its own cue.
            sb.push_default(StyleProperty::Brush(BrushIndex(B_SYLLABLE)));
            let mut layout = sb.build(sep);
            layout.break_all_lines(None);
            let width = layout.full_width();
            let baseline = layout.lines().next().map_or(0.0, |l| l.metrics().baseline);
            SepMarks {
                layout,
                width,
                baseline,
                at: breaks.iter().map(|&k| bytes.byte(text, k)).collect(),
                placed: Vec::new(),
            }
        });
        // A list item: its hanging indent by depth, and its bullet or
        // number in the body font, laid out once (drawn only).
        let em = f64::from(size);
        let indent = p.list.as_ref().map_or(0.0, |l| {
            (f64::from(l.level.max(1)) * em * LIST_INDENT_EM).min(self.column / 2.0)
        });
        let marker = p.list.as_ref().map(|l| {
            let glyph = l.glyph();
            let mut mb = lcx.ranged_builder(fcx, glyph, 1.0, true);
            mb.push_default(StyleProperty::FontFamily(FontFamily::Source(
                self.font.family.clone().into(),
            )));
            mb.push_default(StyleProperty::FontSize(size));
            mb.push_default(StyleProperty::Brush(BrushIndex(B_TEXT)));
            if self.font.bold {
                mb.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
            }
            let mut layout = mb.build(glyph);
            layout.break_all_lines(None);
            layout
        });
        let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(
            self.font.family.clone().into(),
        )));
        b.push_default(StyleProperty::FontSize(size));
        b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            sp.line_height,
        )));
        // Letter and word spacing are extra space in multiples of the size
        // (WCAG 1.4.12), so they grow with headings too.
        if sp.letter_spacing > 0.0 {
            b.push_default(StyleProperty::LetterSpacing(size * sp.letter_spacing));
        }
        if sp.word_spacing > 0.0 {
            b.push_default(StyleProperty::WordSpacing(size * sp.word_spacing));
        }
        b.push_default(StyleProperty::Brush(BrushIndex(B_TEXT)));
        if self.font.bold {
            b.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if let Some(level) = p.heading {
            b.push_default(StyleProperty::FontSize(size * Self::heading_scale(level)));
            b.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
            // Headings are set tighter than body text, in proportion.
            b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                sp.line_height * 1.25 / 1.5,
            )));
            b.push_default(StyleProperty::Brush(BrushIndex(
                B_H1 + usize::from(level - 1),
            )));
        }
        let p_end = p.start.0 + p.len_chars();
        for s in self.spans_in(i) {
            if s.range.end.0 <= p.start.0 || s.range.start.0 >= p_end {
                continue;
            }
            let a = bytes.byte(text, s.range.start.0.saturating_sub(p.start.0));
            let e = bytes.byte(text, s.range.end.0.min(p_end) - p.start.0);
            if a >= e {
                continue;
            }
            match s.style {
                SpanStyle::Heading(_) => {}
                SpanStyle::Link => {
                    b.push(StyleProperty::Brush(BrushIndex(B_LINK)), a..e);
                    b.push(StyleProperty::Underline(true), a..e);
                }
                SpanStyle::Code => {
                    b.push(
                        StyleProperty::FontFamily(FontFamily::Source(
                            crate::fonts::MONO_STACK.into(),
                        )),
                        a..e,
                    );
                    b.push(StyleProperty::Brush(BrushIndex(B_CODE)), a..e);
                }
                SpanStyle::Quote => {
                    b.push(StyleProperty::Brush(BrushIndex(B_QUOTE)), a..e);
                    b.push(StyleProperty::FontStyle(FontStyle::Italic), a..e);
                }
                SpanStyle::Bold => b.push(StyleProperty::FontWeight(FontWeight::BOLD), a..e),
                SpanStyle::Italic => b.push(StyleProperty::FontStyle(FontStyle::Italic), a..e),
                SpanStyle::Underline => b.push(StyleProperty::Underline(true), a..e),
                SpanStyle::Strikethrough => b.push(StyleProperty::Strikethrough(true), a..e),
                SpanStyle::Bionic => b.push(StyleProperty::FontWeight(FontWeight::BOLD), a..e),
                SpanStyle::Difficult => {
                    // A thicker underline than a link's (0.1 em), in the
                    // difficult word mark's color (the theme's role, or
                    // `[colors] difficult_words`): the shape carries the
                    // meaning, not the color.
                    b.push(StyleProperty::Underline(true), a..e);
                    b.push(StyleProperty::UnderlineSize(Some(size * 0.1)), a..e);
                    b.push(
                        StyleProperty::UnderlineBrush(Some(BrushIndex(B_DIFFICULT))),
                        a..e,
                    );
                }
            }
        }
        // Room for each separator: the char before a break gets extra
        // letter spacing as wide as the separator (plus the spacing it
        // already has), so no byte is added to the text.
        if let Some(s) = &seps {
            let base = if sp.letter_spacing > 0.0 {
                size * sp.letter_spacing
            } else {
                0.0
            };
            for &k in &breaks {
                let a = bytes.byte(text, k - 1);
                let e = bytes.byte(text, k);
                b.push(StyleProperty::LetterSpacing(base + s.width), a..e);
            }
        }
        let mut layout = b.build(text);
        layout.break_all_lines(Some((self.column - indent) as f32));
        // The marker sits right-aligned in the indent, on the first line's
        // baseline.
        let marker = marker.map(|m| {
            let base =
                |l: &Layout<BrushIndex>| l.lines().next().map_or(0.0, |l| l.metrics().baseline);
            let x = -(em * LIST_GAP_EM) - f64::from(m.full_width());
            let y = f64::from(base(&layout) - base(&m));
            (m, Vec2::new(x, y))
        });
        let text_h = if text.is_empty() {
            em * 0.6
        } else {
            f64::from(layout.height())
        };
        let top_gap = if p.heading.is_some() && i > 0 {
            em * 0.6
        } else {
            0.0
        };
        // Paragraph spacing, in multiples of the size.
        let after = if text.is_empty() {
            0.0
        } else {
            em * f64::from(sp.paragraph_spacing)
        };
        ParaLayout {
            layout,
            height: top_gap + text_h + after,
            top_gap,
            seps,
            bytes,
            lines: Vec::new(),
            indent,
            marker,
        }
    }

    /// Paragraph `i`'s syllable breaks, as char offsets inside it (never at
    /// its start or end).
    /// The spans that touch paragraph `i` (see `para_spans`).
    fn spans_in(&self, i: usize) -> &[StyledSpan] {
        self.para_spans.get(i).map_or(&[], Vec::as_slice)
    }

    fn breaks_in(&self, i: usize) -> Vec<usize> {
        let p = &self.model.paragraphs[i];
        let (start, len) = (p.start.0, p.len_chars());
        let all = &self.model.breaks;
        let from = all.partition_point(|b| b.0 <= start);
        all[from..]
            .iter()
            .take_while(|b| b.0 < start + len)
            .map(|b| b.0 - start)
            .collect()
    }

    /// Lays out paragraph `i` if it is not cached; records its lines.
    fn ensure_layout(
        &mut self,
        i: usize,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) {
        if self.layouts.contains_key(&i) || i >= self.para_count() {
            return;
        }
        if self.layouts.len() >= CACHE_LIMIT {
            // Keep the ones near the view.
            let keep = self.top.0;
            self.layouts
                .retain(|&k, _| k.abs_diff(keep) < CACHE_LIMIT / 4);
        }
        let mut pl = self.build_layout(i, fcx, lcx);
        let p = &self.model.paragraphs[i];
        pl.lines = pl
            .layout
            .lines()
            .map(|l| {
                let tr = l.text_range();
                (
                    pl.bytes.char(&p.text, tr.start),
                    pl.bytes.char(&p.text, tr.end),
                )
            })
            .collect();
        // The syllable separators' places, once per layout.
        if let Some(seps) = pl.seps.as_mut() {
            let mut placed = Vec::with_capacity(seps.at.len());
            for (n, line) in pl.layout.lines().enumerate() {
                let tr = line.text_range();
                let from = seps.at.partition_point(|&b| b < tr.start);
                let y = f64::from(line.metrics().baseline - seps.baseline);
                for &b in seps.at[from..].iter().take_while(|&&b| b < tr.end) {
                    let x = Cursor::from_byte_index(&pl.layout, b, Affinity::Downstream)
                        .geometry(&pl.layout, 1.0)
                        .x0;
                    placed.push((n, Vec2::new(x - f64::from(seps.width), y)));
                }
            }
            seps.placed = placed;
        }
        let starts: Vec<usize> = pl.lines.iter().skip(1).map(|l| l.0).collect();
        if self.line_starts[i].as_ref() != Some(&starts) {
            self.line_starts[i] = Some(starts);
            self.dirty_paras[i] = true;
            self.para_runs[i] = None;
        }
        self.layouts.insert(i, pl);
    }

    fn height_of(&self, i: usize) -> f64 {
        match self.layouts.get(&i) {
            Some(l) => l.height,
            None => {
                // An estimate: average glyph width about half the size.
                let p = &self.model.paragraphs[i];
                let em = f64::from(self.font.size);
                let sp = self.aids.spacing;
                let per_line = (self.column / (em * 0.5)).max(10.0);
                let lines = (p.len_chars() as f64 / per_line).ceil().max(1.0);
                if p.text.is_empty() {
                    em * 0.6
                } else {
                    lines * em * f64::from(sp.line_height) + em * f64::from(sp.paragraph_spacing)
                }
            }
        }
    }

    /// The paragraph and caret rectangle (in paragraph-local coordinates)
    /// of `pos`, laying the paragraph out if needed.
    fn caret_rect(
        &mut self,
        pos: CharPos,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> Option<(usize, Rect)> {
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return None;
        }
        let i = caret::paragraph_at(paras, pos);
        self.ensure_layout(i, fcx, lcx);
        let pl = self.layouts.get(&i)?;
        let p = &self.model.paragraphs[i];
        let off = pos.0.saturating_sub(p.start.0).min(p.len_chars());
        let b = pl.bytes.byte(&p.text, off);
        let bb =
            Cursor::from_byte_index(&pl.layout, b, Affinity::Downstream).geometry(&pl.layout, 2.0);
        let h = if p.text.is_empty() {
            f64::from(self.font.size) * 1.2
        } else {
            bb.y1 - bb.y0
        };
        Some((
            i,
            Rect::new(
                pl.indent + bb.x0,
                pl.top_gap + bb.y0,
                pl.indent + bb.x1,
                pl.top_gap + bb.y0 + h,
            ),
        ))
    }

    /// The reading font's average advance with the current letter and
    /// word spacing: [`PANGRAM`] laid out once, its width divided by its
    /// length. Cached until the font or the spacing changes.
    fn average_advance(
        &mut self,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> f64 {
        if let Some(a) = self.advance {
            return a;
        }
        let size = self.font.size;
        let sp = self.aids.spacing;
        let mut b = lcx.ranged_builder(fcx, PANGRAM, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(
            self.font.family.clone().into(),
        )));
        b.push_default(StyleProperty::FontSize(size));
        if self.font.bold {
            b.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if sp.letter_spacing > 0.0 {
            b.push_default(StyleProperty::LetterSpacing(size * sp.letter_spacing));
        }
        if sp.word_spacing > 0.0 {
            b.push_default(StyleProperty::WordSpacing(size * sp.word_spacing));
        }
        let mut layout = b.build(PANGRAM);
        layout.break_all_lines(None);
        let chars = PANGRAM.chars().count() as f64;
        let mut a = f64::from(layout.full_width()) / chars;
        if !(a.is_finite() && a > 0.0) {
            // No font answered: about half an em, as an estimate.
            a = f64::from(size) * 0.5;
        }
        self.advance = Some(a);
        a
    }

    /// The text column for a view `width` wide: the measure times the
    /// average advance, at least [`MIN_COLUMN`], and never wider than the
    /// window less its insets; the whole width when the measure is 0.
    fn column_for(
        &mut self,
        width: f64,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> f64 {
        let room = (width - 2.0 * INSET).max(80.0);
        if self.aids.measure == 0 {
            return room;
        }
        let m = f64::from(self.aids.measure.clamp(MEASURE_RANGE.0, MEASURE_RANGE.1));
        // Room for the last glyph's spacing: a line of `m` average chars
        // fits.
        (m * self.average_advance(fcx, lcx) + 1.0)
            .max(MIN_COLUMN)
            .min(room)
    }

    /// Lays out the paragraphs from the scroll anchor down, filling the
    /// view, after scrolling to the focus if asked.
    fn layout_view(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>) {
        let n = self.para_count();
        self.visible.clear();
        if n == 0 {
            return;
        }
        let view_h = self.size.height;
        if self.follow {
            self.follow = false;
            let focus = match (self.state.reading, self.state.spoken) {
                (true, Some(r)) => r.start,
                _ => self.state.caret,
            };
            self.scroll_to(focus, view_h, fcx, lcx);
        }
        self.normalize_top(fcx, lcx);
        self.snap_top();
        let mut y = INSET - self.top.1;
        let mut i = self.top.0;
        while i < n && y < view_h {
            self.ensure_layout(i, fcx, lcx);
            self.visible.push((i, y));
            y += self.height_of(i);
            i += 1;
        }
    }

    /// Keeps the anchor paragraph's offset within it, laying out the
    /// paragraphs it moves over.
    fn normalize_top(&mut self, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>) {
        let n = self.para_count();
        self.top.0 = self.top.0.min(n.saturating_sub(1));
        while self.top.1 < 0.0 && self.top.0 > 0 {
            self.top.0 -= 1;
            self.ensure_layout(self.top.0, fcx, lcx);
            self.top.1 += self.height_of(self.top.0);
        }
        if self.top.1 < 0.0 {
            self.top.1 = 0.0;
        }
        loop {
            self.ensure_layout(self.top.0, fcx, lcx);
            let h = self.height_of(self.top.0);
            if self.top.1 >= h && self.top.0 + 1 < n {
                self.top.1 -= h;
                self.top.0 += 1;
            } else {
                break;
            }
        }
    }

    /// Moves the anchor to a line boundary (W9b-n), so the view's first
    /// line is never cut through at the top: the next boundary after a
    /// scroll down, else the one above. A boundary is the paragraph's top
    /// or a line's top.
    fn snap_top(&mut self) {
        let down = std::mem::take(&mut self.scrolled_down);
        let (i, off) = self.top;
        if off <= 0.0 {
            return;
        }
        let Some(pl) = self.layouts.get(&i) else {
            return;
        };
        let mut bounds = vec![0.0];
        bounds.extend(
            pl.layout
                .lines()
                .map(|l| pl.top_gap + f64::from(l.metrics().block_min_coord)),
        );
        if down {
            if let Some(&b) = bounds.iter().find(|&&b| b >= off - 0.5) {
                self.top.1 = b;
            } else if i + 1 < self.para_count() {
                self.top = (i + 1, 0.0);
            }
        } else if let Some(&b) = bounds.iter().rev().find(|&&b| b <= off + 0.5) {
            self.top.1 = b;
        }
    }

    /// Scrolls so `pos` is on screen, keeping it a third of the way down
    /// when it has to move.
    fn scroll_to(
        &mut self,
        pos: CharPos,
        view_h: f64,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) {
        let Some((i, rect)) = self.caret_rect(pos, fcx, lcx) else {
            return;
        };
        // The caret's y relative to the view's top, if the anchor is
        // above it and near.
        let mut y = INSET - self.top.1;
        let mut j = self.top.0;
        let mut found = None;
        if i >= j {
            while j <= i && y < view_h * 2.0 {
                if j == i {
                    found = Some(y + rect.y0);
                    break;
                }
                self.ensure_layout(j, fcx, lcx);
                y += self.height_of(j);
                j += 1;
            }
        }
        let margin = rect.height() * 2.0;
        let on_screen = found.is_some_and(|cy| {
            cy >= margin.min(view_h / 4.0)
                && cy + rect.height() <= view_h - margin.min(view_h / 4.0)
        });
        if !on_screen {
            self.top = (i, rect.y0 - view_h / 3.0);
        }
    }

    /// The visual lines on screen, as the reading ruler counts rows: each
    /// line's chars (document positions) and paragraph, and its top and
    /// bottom in the view. Lines more than a screen above or below the view
    /// are left out, so a paragraph of a thousand lines (one very long
    /// line) costs only the lines near the screen; the ruler's band never
    /// reaches that far.
    fn rows_on_screen(&self) -> Vec<(ViewRow, f64, f64)> {
        let mut rows = Vec::new();
        let view_h = self.size.height;
        for &(i, y) in &self.visible {
            let Some(pl) = self.layouts.get(&i) else {
                continue;
            };
            let p = &self.model.paragraphs[i];
            let top = y + pl.top_gap;
            let mut any = false;
            let near = (-view_h - top, 2.0 * view_h - top);
            for (line, &(a, b)) in pl.layout.lines().zip(&pl.lines) {
                any = true;
                let m = line.metrics();
                if f64::from(m.block_max_coord) < near.0 {
                    continue;
                }
                if f64::from(m.block_min_coord) > near.1 {
                    break;
                }
                let (a, b) = (p.start.0 + a, p.start.0 + b);
                let y0 = top + f64::from(m.block_min_coord);
                let mut y1 = top + f64::from(m.block_max_coord);
                if p.text.is_empty() {
                    // A blank line is laid out shorter (0.6 em) than its
                    // one empty line: its row ends where it does, so the
                    // band does not reach over the next paragraph or
                    // heading (GUI audit QW3).
                    y1 = y1.min(y + pl.height).max(y0);
                }
                rows.push((
                    ViewRow {
                        range: CharRange::new(a, b),
                        line: i,
                    },
                    y0,
                    y1,
                ));
            }
            if !any {
                // A blank line: one empty row at its position, as tall as
                // the blank paragraph is laid out (0.6 em), so the band
                // does not reach over the next paragraph or heading.
                let h = f64::from(self.font.size) * 0.6;
                rows.push((
                    ViewRow {
                        range: CharRange::new(p.start.0, p.start.0),
                        line: i,
                    },
                    top,
                    top + h,
                ));
            }
        }
        rows
    }

    /// The reading highlights the last paint drew, in order: sentence and
    /// word bands, marks, text, the word's attribute and the sentence's
    /// underline ([`PaintStep`]).
    pub fn painted(&self) -> &[PaintStep] {
        &self.painted
    }

    /// The lines under the spoken sentence in the last paint, one per
    /// visual line of the sentence on screen.
    pub fn sentence_underlines(&self) -> Vec<(Rect, textweaver_theme::Rgb)> {
        self.painted
            .iter()
            .filter_map(|s| match *s {
                PaintStep::SentenceUnderline(r, c) => Some((r, c)),
                _ => None,
            })
            .collect()
    }

    /// The reading ruler's marks for the rows on screen (empty when it is
    /// off): the reading line, the band around it, and the masked rows.
    pub fn ruler_marks(&self) -> Vec<(RowMark, f64, f64)> {
        if self.aids.ruler.mode == RulerMode::Off || self.model.paragraphs.is_empty() {
            return Vec::new();
        }
        let rows = self.rows_on_screen();
        let focus = match (self.state.reading, self.state.spoken) {
            (true, Some(r)) => r.start,
            _ => self.state.caret,
        };
        let view: Vec<ViewRow> = rows.iter().map(|r| r.0).collect();
        ruler_rows(&view, focus, &self.aids.ruler)
            .into_iter()
            .zip(rows)
            .map(|(mark, (_, y0, y1))| (mark, y0, y1))
            .collect()
    }

    // --- Caret movement that needs layout.

    /// Visual line move within and across paragraphs.
    fn move_lines(
        &mut self,
        delta: isize,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> CharPos {
        let pos = self.state.caret;
        let Some((i, rect)) = self.caret_rect(pos, fcx, lcx) else {
            return pos;
        };
        let x = *self.goal_x.get_or_insert(rect.x0 as f32);
        let pl = &self.layouts[&i];
        let p = &self.model.paragraphs[i];
        let line_count = pl.layout.len().max(1);
        let cur_line = {
            let b = pl
                .bytes
                .byte(&p.text, pos.0.saturating_sub(p.start.0).min(p.len_chars()));
            pl.layout
                .lines()
                .position(|l| l.text_range().contains(&b))
                .unwrap_or(line_count - 1)
        };
        let target = cur_line as isize + delta;
        let (pi, line) = if target < 0 {
            if i == 0 {
                return self.model.paragraphs[0].start;
            }
            self.ensure_layout(i - 1, fcx, lcx);
            (i - 1, self.layouts[&(i - 1)].layout.len().max(1) - 1)
        } else if target as usize >= line_count {
            if i + 1 >= self.para_count() {
                return caret::window_end(&self.model.paragraphs);
            }
            (i + 1, 0)
        } else {
            (i, target as usize)
        };
        self.ensure_layout(pi, fcx, lcx);
        let pl = &self.layouts[&pi];
        let p = &self.model.paragraphs[pi];
        let y = pl.layout.get(line).map_or(0.0, |l| {
            (l.metrics().block_min_coord + l.metrics().block_max_coord) / 2.0
        });
        // The goal is in the column's coordinates, so a move into a list
        // item keeps the same place on screen.
        let c = Cursor::from_point(&pl.layout, x - pl.indent as f32, y);
        CharPos(p.start.0 + pl.bytes.char(&p.text, c.index()))
    }

    fn line_edge(
        &mut self,
        end: bool,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> CharPos {
        let pos = self.state.caret;
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return pos;
        }
        let i = caret::paragraph_at(paras, pos);
        self.ensure_layout(i, fcx, lcx);
        let pl = &self.layouts[&i];
        let p = &self.model.paragraphs[i];
        let b = pl
            .bytes
            .byte(&p.text, pos.0.saturating_sub(p.start.0).min(p.len_chars()));
        let sel = Selection::from_byte_index(&pl.layout, b, Affinity::Downstream);
        let moved = if end {
            sel.line_end(&pl.layout, false)
        } else {
            sel.line_start(&pl.layout, false)
        };
        CharPos(p.start.0 + pl.bytes.char(&p.text, moved.focus().index()))
    }

    fn page(
        &mut self,
        down: bool,
        fcx: &mut FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) -> CharPos {
        let view = (self.size.height - 2.0 * INSET).max(100.0);
        let line = f64::from(self.font.size) * f64::from(self.aids.spacing.line_height);
        let lines = ((view / line).floor() as isize - 1).max(1);
        let pos = self.move_lines(if down { lines } else { -lines }, fcx, lcx);
        // Scroll by the same page, so the caret keeps its place on screen.
        self.top.1 += if down { view } else { -view };
        self.scrolled_down = down;
        pos
    }

    /// Handles a caret key: `chord` as the platform's caret keys read it
    /// ([`keys::caret_move`]). Returns true when the key was a caret key.
    fn caret_key(&mut self, ctx: &mut EventCtx<'_>, chord: &KeyChord) -> bool {
        let Some((m, shift)) = keys::caret_move(chord, self.platform) else {
            return false;
        };
        let paras = &self.model.paragraphs;
        let pos = self.state.caret;
        let doc_end = CharPos(self.model.doc_len);
        let (fcx, lcx) = ctx.text_contexts();
        let new = match (m.step, m.forward) {
            (CaretStep::Char, false) => caret::prev_char(paras, pos),
            (CaretStep::Char, true) => caret::next_char(paras, pos),
            (CaretStep::Word, false) => caret::prev_word(paras, pos),
            (CaretStep::Word, true) => caret::next_word(paras, pos),
            (CaretStep::Paragraph, false) => caret::prev_paragraph(paras, pos),
            (CaretStep::Paragraph, true) => caret::next_paragraph(paras, pos),
            (CaretStep::Line, forward) => self.move_lines(if forward { 1 } else { -1 }, fcx, lcx),
            (CaretStep::LineEdge, forward) => self.line_edge(forward, fcx, lcx),
            (CaretStep::Page, forward) => self.page(forward, fcx, lcx),
            (CaretStep::DocumentEdge, false) => CharPos::ZERO,
            (CaretStep::DocumentEdge, true) => doc_end,
        };
        let vertical = matches!(m.step, CaretStep::Line | CaretStep::Page);
        let echo = if shift {
            self.selection_echo(new)
        } else {
            match m.step {
                CaretStep::Word => self.word_echo(new),
                CaretStep::Char | CaretStep::LineEdge => self.char_echo(new),
                _ => self.line_echo(new),
            }
        };
        let goal = self.goal_x;
        self.move_caret_saying(ctx, new, shift, echo);
        if vertical {
            self.goal_x = goal;
        }
        true
    }

    fn move_caret(&mut self, ctx: &mut EventCtx<'_>, new: CharPos, extend: bool) {
        self.move_caret_saying(ctx, new, extend, None);
    }

    fn move_caret_saying(
        &mut self,
        ctx: &mut EventCtx<'_>,
        new: CharPos,
        extend: bool,
        echo: Option<CaretEcho>,
    ) {
        let old = self.state;
        let anchor = if extend {
            Some(old.anchor.unwrap_or(old.caret))
        } else {
            None
        };
        let anchor = anchor.filter(|a| *a != new);
        self.state.caret = new;
        self.state.anchor = anchor;
        self.goal_x = None;
        self.follow = true;
        ctx.request_layout();
        ctx.request_render();
        ctx.submit_action::<DocAction>(DocAction::CaretMoved {
            caret: new,
            selection: anchor.map(|a| CharRange::new(a.0.min(new.0), a.0.max(new.0))),
            echo,
        });
    }

    // --- What a caret key says (the self-voicing mode).

    /// The paragraph holding `pos` and `pos`'s char offset in it.
    fn para_offset(&self, pos: CharPos) -> Option<(usize, usize)> {
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return None;
        }
        let i = caret::paragraph_at(paras, pos);
        Some((i, pos.0.saturating_sub(paras[i].start.0)))
    }

    /// The char at `pos`, or the end of its line or of the document.
    fn char_echo(&self, pos: CharPos) -> Option<CaretEcho> {
        let (i, off) = self.para_offset(pos)?;
        let p = &self.model.paragraphs[i];
        Some(match p.text.chars().nth(off) {
            Some(ch) => CaretEcho::Char(ch),
            None if pos.0 >= self.model.doc_len => CaretEcho::DocEnd,
            None => CaretEcho::LineEnd,
        })
    }

    /// The word at `pos` (from `pos` to the word's end), else its char.
    fn word_echo(&self, pos: CharPos) -> Option<CaretEcho> {
        use unicode_segmentation::UnicodeSegmentation;
        let (i, off) = self.para_offset(pos)?;
        let p = &self.model.paragraphs[i];
        let b = caret::byte_of(&p.text, off);
        let word = p
            .text
            .split_word_bound_indices()
            .find(|(start, w)| *start <= b && b < start + w.len())
            .map(|(start, w)| &p.text[b.max(start)..start + w.len()]);
        match word {
            Some(w) if w.chars().any(char::is_alphanumeric) => Some(CaretEcho::Word(w.to_owned())),
            _ => self.char_echo(pos),
        }
    }

    /// The visual line holding `pos`, as laid out (the paragraph when it
    /// has not been laid out yet).
    fn line_echo(&self, pos: CharPos) -> Option<CaretEcho> {
        let (i, off) = self.para_offset(pos)?;
        let p = &self.model.paragraphs[i];
        let text = match self.layouts.get(&i) {
            Some(pl) => {
                let b = pl.bytes.byte(&p.text, off);
                pl.layout
                    .lines()
                    .map(|l| l.text_range())
                    .find(|r| r.contains(&b))
                    .or_else(|| pl.layout.lines().last().map(|l| l.text_range()))
                    // An empty paragraph is laid out with a stand-in
                    // char, so its line can reach past the text.
                    .map_or(p.text.as_str(), |r| p.text.get(r).unwrap_or(""))
            }
            None => p.text.as_str(),
        };
        let text = text.trim_end();
        if text.is_empty() {
            return Some(if pos.0 >= self.model.doc_len {
                CaretEcho::DocEnd
            } else {
                CaretEcho::LineEnd
            });
        }
        Some(CaretEcho::Line(text.to_owned()))
    }

    /// Shift with a caret key: the text between the old caret and `new`,
    /// and whether the selection grew by it.
    fn selection_echo(&self, new: CharPos) -> Option<CaretEcho> {
        let old = self.state.caret;
        if old == new {
            return None;
        }
        let anchor = self.state.anchor.unwrap_or(old);
        let text = caret::text_between(&self.model.paragraphs, old, new);
        let (lo, hi) = (old.0.min(new.0), old.0.max(new.0));
        let selected = anchor.0.min(new.0) <= lo && hi <= anchor.0.max(new.0);
        Some(CaretEcho::Selection { text, selected })
    }

    /// The selection, when there is one.
    fn selection(&self) -> Option<CharRange> {
        let a = self.state.anchor?;
        let c = self.state.caret;
        (a != c).then(|| CharRange::new(a.0.min(c.0), a.0.max(c.0)))
    }

    /// Edit mode: `text` typed, over the selection when there is one.
    fn type_text(&mut self, ctx: &mut EventCtx<'_>, text: &str) {
        if text.is_empty() {
            return;
        }
        let action = match self.selection() {
            Some(range) => DocAction::Replace {
                range,
                text: text.to_owned(),
            },
            None => DocAction::Typed(text.to_owned()),
        };
        self.goal_x = None;
        ctx.submit_action::<DocAction>(action);
    }

    /// Edit mode: Backspace or Delete, of the selection when there is one.
    fn delete_text(&mut self, ctx: &mut EventCtx<'_>, forward: bool) {
        let action = match self.selection() {
            Some(range) => DocAction::Replace {
                range,
                text: String::new(),
            },
            None => DocAction::Delete { forward },
        };
        self.goal_x = None;
        ctx.submit_action::<DocAction>(action);
    }

    /// Edit mode: a key that types or deletes. Returns true when it did.
    /// Keys with Ctrl, Alt, or Command are commands and go on to the keymap,
    /// except AltGr (Ctrl with Alt) typing a symbol, as on many layouts.
    fn edit_key(
        &mut self,
        ctx: &mut EventCtx<'_>,
        k: &masonry::core::keyboard::KeyboardEvent,
    ) -> bool {
        let m = k.modifiers;
        // The Windows key types nothing either, though it is no command.
        let command = keys::is_command(m, self.platform) || m.meta();
        let altgr = m.ctrl() && m.alt() && !m.meta();
        match &k.key {
            Key::Named(NamedKey::Enter) if !command => self.type_text(ctx, "\n"),
            Key::Named(NamedKey::Tab) if !command => {
                self.goal_x = None;
                ctx.submit_action::<DocAction>(DocAction::TableCell {
                    forward: !m.shift(),
                });
            }
            Key::Named(NamedKey::Backspace) if !command => self.delete_text(ctx, false),
            Key::Named(NamedKey::Delete) if !command => self.delete_text(ctx, true),
            Key::Character(s)
                if !command || (altgr && s.chars().all(|c| !c.is_ascii_alphanumeric())) =>
            {
                if s.chars().any(char::is_control) {
                    return false;
                }
                let s = s.clone();
                self.type_text(ctx, &s);
            }
            _ => return false,
        }
        true
    }

    fn hit(&mut self, ctx: &mut EventCtx<'_>, pos: Point) -> Option<CharPos> {
        let (fcx, lcx) = ctx.text_contexts();
        for k in 0..self.visible.len() {
            let (i, y) = self.visible[k];
            self.ensure_layout(i, fcx, lcx);
            let h = self.height_of(i);
            if pos.y < y + h || k + 1 == self.visible.len() {
                let pl = &self.layouts[&i];
                let p = &self.model.paragraphs[i];
                let c = Cursor::from_point(
                    &pl.layout,
                    (pos.x - self.column_x - pl.indent) as f32,
                    (pos.y - y - pl.top_gap) as f32,
                );
                return Some(CharPos(p.start.0 + pl.bytes.char(&p.text, c.index())));
            }
        }
        None
    }

    // --- Accessibility.

    /// Paragraph `i`'s runs, built if they are not cached.
    fn runs_of(&mut self, i: usize) -> &ParaRuns {
        if self.para_runs[i].is_none() {
            let marks = self.run_marks();
            let runs = ParaRuns::new(
                i,
                &self.model.paragraphs[i],
                self.line_starts[i].as_deref(),
                &marks,
            );
            self.para_runs[i] = Some(runs);
        }
        self.para_runs[i].get_or_insert_default()
    }

    /// The ids and nodes of one line's runs.
    fn line_nodes(&mut self, line: &[Run], out: &mut Vec<(NodeId, Node)>) -> Vec<NodeId> {
        let ids: Vec<NodeId> = line.iter().map(|r| self.id_for(r.start)).collect();
        for (k, r) in line.iter().enumerate() {
            let next = (r.continues_line && k + 1 < line.len()).then(|| ids[k + 1]);
            let prev = (k > 0 && line[k - 1].continues_line).then(|| ids[k - 1]);
            out.push((ids[k], self.run_node(r, next, prev)));
        }
        ids
    }

    /// Forgets the ids in `old` that are not in `new` (runs gone).
    fn forget_ids(&mut self, old: &[NodeId], new: &[NodeId]) {
        let gone: Vec<NodeId> = if new.len() > 32 {
            let keep: std::collections::HashSet<&NodeId> = new.iter().collect();
            old.iter()
                .filter(|id| !keep.contains(id))
                .copied()
                .collect()
        } else {
            old.iter().filter(|id| !new.contains(id)).copied().collect()
        };
        for id in gone {
            if let Some(k) = self.ids_to_pos.remove(&id) {
                self.run_ids.remove(&k);
            }
        }
    }

    fn run_node(&self, run: &Run, next: Option<NodeId>, prev: Option<NodeId>) -> Node {
        let mut node = Node::new(Role::TextRun);
        node.set_value(run.text.as_str());
        node.set_character_lengths(run.char_lens.clone());
        node.set_word_starts(run.word_starts.clone());
        node.set_text_direction(TextDirection::LeftToRight);
        let size = self.font.size * run.heading.map_or(1.0, Self::heading_scale);
        node.set_font_size(size);
        if run.heading.is_some() || self.font.bold {
            node.set_font_weight(700.0);
        }
        let p = &self.palette;
        let fg = run.heading.map_or(p.text, |l| p.heading(l));
        node.set_foreground_color(ak_color(fg));
        if let Some(RunMark::SpokenWord) = run.mark {
            node.set_background_color(ak_color(p.spoken_word.1));
            node.set_foreground_color(ak_color(p.spoken_word.0));
        }
        if let Some(n) = next {
            node.set_next_on_line(n);
        }
        if let Some(pv) = prev {
            node.set_previous_on_line(pv);
        }
        node
    }

    fn id_for(&mut self, key: CharPos) -> NodeId {
        if let Some(id) = self.run_ids.get(&key) {
            return *id;
        }
        let id = AccessCtx::next_node_id();
        self.run_ids.insert(key, id);
        self.ids_to_pos.insert(id, key);
        id
    }

    /// The accessibility position of `pos`.
    fn text_position(&mut self, pos: CharPos) -> Option<TextPosition> {
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return None;
        }
        let i = caret::paragraph_at(paras, pos);
        let (key, index) = self
            .runs_of(i)
            .position(pos)
            .map(|(run, index)| (run.start, index))?;
        let id = self.id_for(key);
        Some(TextPosition {
            node: id,
            character_index: index,
        })
    }

    /// The document position of an accessibility position.
    fn doc_position(&mut self, tp: &TextPosition) -> Option<CharPos> {
        let start = *self.ids_to_pos.get(&tp.node)?;
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return None;
        }
        let i = caret::paragraph_at(paras, start);
        self.runs_of(i).char_pos(start, tp.character_index)
    }
}

impl Widget for DocumentView {
    type Action = DocAction;

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FocusChanged(f) => {
                self.focused = *f;
                ctx.request_render();
            }
            Update::FontsChanged => {
                self.reset_caches();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                ctx.request_focus();
                let local = ctx.local_position(state.position);
                if let Some(pos) = self.hit(ctx, local) {
                    let shift = state.modifiers.shift();
                    self.move_caret(ctx, pos, shift);
                }
                ctx.set_handled();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                let scale = ctx.scale_factor();
                let line = PhysicalPosition::new(WHEEL_LINE * scale, WHEEL_LINE * scale);
                let page =
                    PhysicalPosition::new(self.size.width * scale, self.size.height * 0.9 * scale);
                let px = delta.to_pixel_delta(line, page);
                let LogicalPosition { y, .. } = px.to_logical::<f64>(scale);
                self.top.1 -= y;
                self.scrolled_down = y < 0.0;
                ctx.request_layout();
                ctx.request_render();
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if self.editing
            && let TextEvent::Ime(masonry::core::Ime::Commit(text))
            | TextEvent::ClipboardPaste(text) = event
        {
            // An input method's text, or the system clipboard's (the
            // window reads it for the platform's paste key).
            let text = text.clone();
            self.type_text(ctx, &text);
            ctx.set_handled();
            return;
        }
        if let TextEvent::WindowFocusChange(true) = event {
            ctx.submit_action::<DocAction>(DocAction::WindowFocused);
            return;
        }
        let TextEvent::Keyboard(k) = event else {
            return;
        };
        if k.state != KeyState::Down {
            return;
        }
        if self.editing && !k.is_composing && self.edit_key(ctx, k) {
            ctx.set_handled();
            return;
        }
        // The caret keys are the platform's; copying and every other
        // command go on to the keymap (whose copy puts the app's selection,
        // which follows the view's, on the clipboard).
        if let Some(chord) = keys::chord(k, self.platform)
            && !self.yielded.contains(&chord)
            && self.caret_key(ctx, &chord)
        {
            ctx.set_handled();
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match event.action {
            Action::SetTextSelection => {
                if let Some(ActionData::SetTextSelection(sel)) = &event.data
                    && let (Some(a), Some(f)) = (
                        self.doc_position(&sel.anchor),
                        self.doc_position(&sel.focus),
                    )
                {
                    self.state.anchor = None;
                    self.state.caret = a;
                    self.move_caret(ctx, f, a != f);
                    ctx.set_handled();
                }
            }
            // A screen reader's or dictation's edit (edit mode only).
            Action::ReplaceSelectedText if self.editing => {
                if let Some(ActionData::Value(text)) = &event.data {
                    let range = self
                        .selection()
                        .unwrap_or(CharRange::new(self.state.caret.0, self.state.caret.0));
                    ctx.submit_action::<DocAction>(DocAction::Replace {
                        range,
                        text: text.to_string(),
                    });
                    ctx.set_handled();
                }
            }
            Action::SetValue if self.editing => {
                if let Some(ActionData::Value(text)) = &event.data {
                    ctx.submit_action::<DocAction>(DocAction::Replace {
                        range: CharRange::new(0, self.model.doc_len),
                        text: text.to_string(),
                    });
                    ctx.set_handled();
                }
            }
            Action::Focus => ctx.request_focus(),
            Action::ScrollIntoView => {
                self.follow = true;
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross: Option<Length>,
    ) -> Length {
        // Fills what it is given; asks for the measure's column (an
        // estimate until the font is measured).
        let column = match (self.aids.measure, self.advance) {
            (0, _) => 1200.0,
            (m, Some(a)) => f64::from(m) * a,
            (m, None) => f64::from(m) * f64::from(self.font.size) * 0.5,
        };
        let want = match axis {
            Axis::Horizontal => column.max(MIN_COLUMN) + 2.0 * INSET,
            Axis::Vertical => 400.0,
        };
        match len_req {
            LenReq::MinContent => Length::px(200.0),
            LenReq::MaxContent => Length::px(want),
            LenReq::FitContent(space) => space,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (fcx, lcx) = ctx.text_contexts();
        let column = self.column_for(size.width, fcx, lcx);
        if (column - self.column).abs() > 0.5 {
            self.column = column;
            self.layouts.clear();
            self.line_starts.iter_mut().for_each(|l| *l = None);
            self.para_runs.iter_mut().for_each(|r| *r = None);
            self.dirty_paras.iter_mut().for_each(|d| *d = true);
            self.follow = true;
        }
        self.column_x = ((size.width - column) / 2.0).max(INSET).floor();
        self.size = size;
        // No document: the hint, in the reading font, wrapped to the column.
        self.hint_layout =
            (self.model.paragraphs.is_empty() && !self.empty_hint.is_empty()).then(|| {
                let size = self.font.size;
                plain_layout(
                    fcx,
                    lcx,
                    &self.empty_hint,
                    &self.font.family,
                    size,
                    Some(column),
                )
            });
        // Edit mode: the word in the corner, at the interface's size.
        self.badge_layout = (self.editing && !self.editing_word.is_empty()).then(|| {
            plain_layout(
                fcx,
                lcx,
                &self.editing_word,
                crate::fonts::DEFAULT_STACK,
                theme::UI_TEXT,
                None,
            )
        });
        self.layout_view(fcx, lcx);
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let p = self.palette.clone();
        let size = ctx.content_box().size();
        let card = RoundedRect::from_rect(size.to_rect(), theme::PANEL_RADIUS);
        painter.fill(card, theme::color(p.background)).draw();
        // The focus ring (design system D): drawn inside the view, which
        // fills its space, so from the edge in: the ring, a 1 px line in
        // the inner focus color, then the border, which stays.
        let ring = theme::ring_width(&p);
        let border_at = if self.focused {
            ring + theme::FOCUS_INNER
        } else {
            0.0
        };
        // In edit mode the border is dashed and heavier, beside the word
        // "Editing" in the corner, so the mode shows by shape and by word,
        // not by color.
        let border = if self.editing {
            Stroke::new(2.0).with_dashes(0.0, [8.0, 5.0])
        } else {
            Stroke::new(1.0)
        };
        painter
            .stroke(
                RoundedRect::from_rect(
                    size.to_rect().inset(-(border_at + border.width / 2.0)),
                    theme::PANEL_RADIUS,
                ),
                &border,
                theme::color(p.border),
            )
            .draw();
        // Text stays inside the ring.
        let clip = ring + theme::FOCUS_INNER + 2.0;
        let mut text_area = size.to_rect().inset(-clip);
        if self.top != (0, 0.0) {
            // Scrolled: the line above the first one on screen (the scroll
            // stops on a line's top) is not drawn half into the top
            // margin, so no heading shows cut through (W9b-n).
            text_area.y0 = text_area.y0.max(INSET - 2.0);
        }
        painter.push_fill_clip(RoundedRect::from_rect(text_area, theme::PANEL_RADIUS - 2.0));
        let brushes = self.brushes();
        let mut painted = Vec::new();
        let caret_pos = self.state.caret;
        let reading = self.state.reading;
        let (word, sentence) = match self.shown {
            Some(s) => (s.word, s.sentence),
            None => (self.state.spoken, self.state.sentence),
        };
        let spoken = word.filter(|_| reading);
        let sentence = sentence.filter(|_| reading);
        let selection = self
            .state
            .anchor
            .map(|a| CharRange::new(a.0.min(caret_pos.0), a.0.max(caret_pos.0)));

        // The reading ruler under the text: a band on the reading line,
        // with a bar at its start, and a paler band on the rows around it.
        let marks = self.ruler_marks();
        let band_x0 = (self.column_x - 12.0).max(2.0);
        let band_x1 = (self.column_x + self.column + 12.0).min(size.width - 2.0);
        for &(mark, y0, y1) in &marks {
            let r = Rect::new(band_x0, y0, band_x1, y1);
            // The band's rows get a bar too, thinner than the reading
            // line's, so the band shows by its shape and not by a pale
            // tint alone (GUI audit QW3), as the terminal draws it.
            let bar = match mark {
                RowMark::Focus => {
                    painter
                        .fill(RoundedRect::from_rect(r, 4.0), theme::color(p.ruler_focus))
                        .draw();
                    RULER_FOCUS_BAR
                }
                RowMark::Band => {
                    painter.fill(r, theme::color(p.ruler_band)).draw();
                    RULER_BAND_BAR
                }
                RowMark::Normal | RowMark::Masked => continue,
            };
            let bar = Rect::new(band_x0, y0, band_x0 + bar, y1);
            painter.fill(bar, theme::color(p.focus)).draw();
            painted.push(PaintStep::RulerBar(mark, bar));
        }

        for &(i, y) in &self.visible {
            let Some(pl) = self.layouts.get(&i) else {
                continue;
            };
            let para = &self.model.paragraphs[i];
            let origin = Vec2::new(self.column_x + pl.indent, y + pl.top_gap);
            let tf = Affine::translate(origin);
            let p_end = para.start.0 + para.len_chars();
            // The part of the layout on screen, in its own coordinates:
            // only those lines are drawn.
            let on_screen = (-origin.y, size.height - origin.y);
            let touches = |r: CharRange| r.end.0 > para.start.0 && r.start.0 < p_end;
            let band = |r: CharRange| -> Option<Vec<Rect>> {
                if r.end.0 <= para.start.0 || r.start.0 > p_end || r.is_empty() {
                    return None;
                }
                let a = pl
                    .bytes
                    .byte(&para.text, r.start.0.saturating_sub(para.start.0));
                let b = pl.bytes.byte(&para.text, r.end.0.min(p_end) - para.start.0);
                let sel = Selection::new(
                    Cursor::from_byte_index(&pl.layout, a, Affinity::Downstream),
                    Cursor::from_byte_index(&pl.layout, b, Affinity::Upstream),
                );
                Some(
                    sel.geometry(&pl.layout)
                        .into_iter()
                        .map(|(bb, _)| Rect::new(bb.x0, bb.y0, bb.x1, bb.y1) + origin)
                        .collect(),
                )
            };
            // Bands, from the faintest up: the spoken sentence, code, the
            // marks, the selection, then the word.
            let sentence_rects = sentence.and_then(band).unwrap_or_default();
            for &r in &sentence_rects {
                painter.fill(r, theme::color(p.spoken_sentence)).draw();
                painted.push(PaintStep::SentenceBand(r));
            }
            for s in self.spans_in(i) {
                if s.style == SpanStyle::Code
                    && let Some(rects) = band(s.range)
                {
                    for r in rects {
                        painter
                            .fill(
                                RoundedRect::from_rect(r.inflate(2.0, 0.0), 3.0),
                                theme::color(p.code_background),
                            )
                            .draw();
                    }
                }
            }
            // The reader's marks and the search matches: their bands now,
            // their shapes over the text below.
            let mark_rects: Vec<(DocMark, Vec<Rect>)> = self
                .marks
                .iter()
                .filter(|(r, _)| r.end.0 > para.start.0 && r.start.0 <= p_end)
                .map(|&(r, mark)| (mark, band(r).unwrap_or_default()))
                .collect();
            for (mark, rects) in &mark_rects {
                let fill = match mark {
                    DocMark::Highlight => p.user_highlight,
                    DocMark::Note => p.note,
                    DocMark::Bookmark => p.bookmark,
                    DocMark::FindHit => p.find_hit.1,
                    DocMark::CurrentFindHit => p.current_find_hit,
                };
                for rect in rects {
                    painter.fill(*rect, theme::color(fill)).draw();
                    painted.push(PaintStep::MarkBand(*mark, *rect));
                }
            }
            let selection_rects = selection.and_then(band).unwrap_or_default();
            for r in &selection_rects {
                painter.fill(*r, theme::color(p.selection.1)).draw();
            }
            let word_rects = spoken.and_then(band).unwrap_or_default();
            for r in &word_rects {
                painter
                    .fill(
                        RoundedRect::from_rect(r.inflate(3.0, 1.0), 4.0),
                        theme::color(p.spoken_word.1),
                    )
                    .draw();
                painted.push(PaintStep::WordBand(*r));
            }
            // The text, with its links' and difficult words' underlines.
            render_lines(painter, tf, &pl.layout, &brushes, on_screen);
            painted.push(PaintStep::Text(i));
            // A list item's bullet or number, in the hanging indent.
            if let (Some((m, at)), Some(item)) = (&pl.marker, &para.list) {
                render_text(painter, Affine::translate(origin + *at), m, &brushes, false);
                let r = Rect::new(0.0, 0.0, f64::from(m.full_width()), f64::from(m.height()))
                    + origin
                    + *at;
                painted.push(PaintStep::ListMarker(item.level, r));
            }
            for s in self.spans_in(i) {
                if s.style == SpanStyle::Difficult && touches(s.range) {
                    for r in band(s.range).unwrap_or_default() {
                        painted.push(PaintStep::Aid(AidMark::DifficultWord, r, p.difficult_word));
                    }
                }
            }
            // The spoken word's text again, in its own colour and with its
            // theme attribute, clipped to its band (no relayout per word:
            // bold thickens the glyphs where they stand).
            if !word_rects.is_empty() {
                let a = p.spoken_word_attrs;
                let bold = a.bold || !(a.italic || a.underline);
                let fg = theme::color(p.spoken_word.0);
                for r in &word_rects {
                    let clip = r.inflate(3.0, 1.0);
                    painter.push_fill_clip(clip);
                    let ys = (clip.y0 - origin.y, clip.y1 - origin.y);
                    render_emphasis(painter, tf, &pl.layout, &fg.into(), (bold, a.italic), ys);
                    painter.pop_clip();
                    if a.underline
                        && let Some(u) = underline_under(&pl.layout, origin, *r, self.font.size)
                    {
                        painter.fill(u, fg).draw();
                    }
                    painted.push(PaintStep::WordText {
                        clip,
                        bold,
                        italic: a.italic,
                        underline: a.underline,
                    });
                }
            }
            if !selection_rects.is_empty() {
                let fg: [masonry::peniko::Brush; BRUSHES] =
                    std::array::from_fn(|_| theme::color(p.selection.0).into());
                for r in &selection_rects {
                    painter.push_fill_clip(*r);
                    let ys = (r.y0 - origin.y, r.y1 - origin.y);
                    render_lines(painter, tf, &pl.layout, &fg, ys);
                    painter.pop_clip();
                }
            }
            // Every line, box, bar, and dot, over the text and the bands.
            let line = theme::color(p.text);
            for (mark, rects) in &mark_rects {
                for (k, rect) in rects.iter().enumerate() {
                    painted.push(PaintStep::MarkShape(*mark, *rect));
                    match mark {
                        DocMark::Highlight => {
                            painter
                                .fill(Rect::new(rect.x0, rect.y1 - 2.0, rect.x1, rect.y1), line)
                                .draw();
                        }
                        DocMark::Note => {
                            let mut x = rect.x0;
                            while x < rect.x1 {
                                let end = (x + 6.0).min(rect.x1);
                                painter
                                    .fill(Rect::new(x, rect.y1 - 2.0, end, rect.y1), line)
                                    .draw();
                                x += 10.0;
                            }
                        }
                        DocMark::Bookmark if k == 0 => {
                            painter
                                .fill(
                                    Rect::new(rect.x0 - 4.0, rect.y0, rect.x0 - 1.0, rect.y1),
                                    line,
                                )
                                .draw();
                        }
                        DocMark::Bookmark => {}
                        DocMark::FindHit | DocMark::CurrentFindHit => {
                            let w = if *mark == DocMark::CurrentFindHit {
                                2.5
                            } else {
                                1.0
                            };
                            painter
                                .stroke(rect.inflate(1.0, 0.0), &Stroke::new(w), line)
                                .draw();
                        }
                    }
                }
            }
            // Syllable separators, placed at layout, on the lines on
            // screen.
            if let Some(s) = &pl.seps {
                let (first, last) = line_span_within(&pl.layout, on_screen);
                let from = s.placed.partition_point(|&(n, _)| n < first);
                for &(_, at) in s.placed[from..].iter().take_while(|&&(n, _)| n < last) {
                    render_text(
                        painter,
                        Affine::translate(origin + at),
                        &s.layout,
                        &brushes,
                        false,
                    );
                    let r = Rect::new(0.0, 0.0, f64::from(s.width), 1.0) + origin + at;
                    painted.push(PaintStep::Aid(AidMark::Syllable, r, p.syllable_mark));
                }
            }
            // Writing suggestions: a double underline, two 1 px lines 2 px
            // apart at the text's foot.
            for &r in &self.lint {
                if !touches(r) {
                    continue;
                }
                let c = theme::color(p.lint);
                for rect in band(r).unwrap_or_default() {
                    let lines = Rect::new(rect.x0, rect.y1 - 4.0, rect.x1, rect.y1);
                    painter
                        .fill(Rect::new(rect.x0, rect.y1 - 4.0, rect.x1, rect.y1 - 3.0), c)
                        .draw();
                    painter
                        .fill(Rect::new(rect.x0, rect.y1 - 1.0, rect.x1, rect.y1), c)
                        .draw();
                    painted.push(PaintStep::Aid(AidMark::Lint, lines, p.lint));
                }
            }
            // The line under the spoken sentence: its cue in every palette,
            // and its only one under a Windows contrast theme, where the
            // band is the page.
            for r in &sentence_rects {
                if let Some(u) = underline_under(&pl.layout, origin, *r, self.font.size) {
                    painter.fill(u, theme::color(p.sentence_line)).draw();
                    painted.push(PaintStep::SentenceUnderline(u, p.sentence_line));
                }
            }
            // Misspelled words: 2 px dots every 4 px at the text's foot.
            for &r in &self.misspelled {
                if !touches(r) {
                    continue;
                }
                let c = theme::color(p.misspelling);
                for rect in band(r).unwrap_or_default() {
                    let y = rect.y1 - 2.5;
                    let mut x = rect.x0;
                    while x + 2.0 <= rect.x1 {
                        painter.fill(Rect::new(x, y, x + 2.0, y + 2.0), c).draw();
                        x += 4.0;
                    }
                    painted.push(PaintStep::Aid(
                        AidMark::Misspelling,
                        Rect::new(rect.x0, y, rect.x1, y + 2.0),
                        p.misspelling,
                    ));
                }
            }
            // The caret, when the view has focus.
            let para_end = CharPos(para.start.0 + para.span_chars());
            let in_para = caret_pos.0 >= para.start.0
                && (caret_pos.0 < para_end.0 || (i + 1 == self.model.paragraphs.len()));
            if in_para && !reading {
                let off = caret_pos
                    .0
                    .saturating_sub(para.start.0)
                    .min(para.len_chars());
                let b = pl.bytes.byte(&para.text, off);
                let bb = Cursor::from_byte_index(&pl.layout, b, Affinity::Downstream)
                    .geometry(&pl.layout, 2.0);
                let h = if para.text.is_empty() {
                    f64::from(self.font.size) * 1.2
                } else {
                    bb.y1 - bb.y0
                };
                let r = Rect::new(bb.x0, bb.y0, bb.x0 + 2.0, bb.y0 + h) + origin;
                if self.focused {
                    painter.fill(r, theme::color(p.caret)).draw();
                }
                // "Reading from here": a play-shaped triangle in the left
                // margin on the caret's line, where Play starts, shown with
                // or without the focus (reading mode only; the caret is the
                // node's selection, so a screen reader has it already).
                if !self.editing {
                    let x1 = self.column_x - 14.0;
                    let s = (h * 0.5).clamp(6.0, 12.0);
                    let cy = r.y0 + h / 2.0;
                    if x1 - s >= 2.0 {
                        let mut tri = BezPath::new();
                        tri.move_to((x1 - s, cy - s / 2.0 - 1.0));
                        tri.line_to((x1, cy));
                        tri.line_to((x1 - s, cy + s / 2.0 + 1.0));
                        tri.close_path();
                        painter.fill(tri, theme::color(p.text)).draw();
                        painted.push(PaintStep::ReadingFrom(Rect::new(
                            x1 - s,
                            cy - s / 2.0 - 1.0,
                            x1,
                            cy + s / 2.0 + 1.0,
                        )));
                    }
                }
            }
        }
        // No document: how to open one, where the text would start.
        if let Some(h) = self
            .hint_layout
            .as_ref()
            .filter(|_| self.model.paragraphs.is_empty())
        {
            let at = Vec2::new(self.column_x, INSET);
            render_text(painter, Affine::translate(at), h, &brushes, false);
            let r = Rect::new(0.0, 0.0, f64::from(h.width()), f64::from(h.height())) + at;
            painted.push(PaintStep::Hint(r));
        }
        self.painted = painted;

        // The ruler's mask (a typoscope): the rows outside the band dimmed.
        for &(mark, y0, y1) in &marks {
            if mark == RowMark::Masked {
                painter
                    .fill(
                        Rect::new(0.0, y0, size.width, y1),
                        theme::with_alpha(p.background, 0.72),
                    )
                    .draw();
            }
        }

        // Where the window is in the document: a slim thumb on the right.
        if self.model.doc_len > 0
            && let (Some(&(first, _)), Some(&(last, _))) =
                (self.visible.first(), self.visible.last())
        {
            let paras = &self.model.paragraphs;
            let a = paras[first].start.0 as f64 / self.model.doc_len as f64;
            let b =
                (paras[last].start.0 + paras[last].span_chars()) as f64 / self.model.doc_len as f64;
            let track_h = size.height - 16.0;
            let h = ((b - a) * track_h).max(28.0).min(track_h);
            let y = 8.0 + a * (track_h - h).max(0.0) / (1.0 - (b - a)).max(0.001);
            let x = size.width - 7.0 - clip + 2.0;
            painter
                .fill(
                    RoundedRect::new(
                        x,
                        y.min(size.height - 8.0 - h),
                        x + 4.0,
                        y.min(size.height - 8.0 - h) + h,
                        2.0,
                    ),
                    theme::with_alpha(p.dim_text, 0.55),
                )
                .draw();
        }

        painter.pop_clip();
        // Outside the text's clip, which starts lower once scrolled.
        // Edit mode: the word in a box in the top right corner.
        if let Some(b) = self.badge_layout.as_ref().filter(|_| self.editing) {
            let (w, h) = (f64::from(b.width()), f64::from(b.height()));
            let x1 = size.width - clip - 4.0;
            let r = Rect::new(x1 - w - 12.0, clip + 2.0, x1, clip + 2.0 + h + 4.0);
            painter
                .fill(RoundedRect::from_rect(r, 4.0), theme::color(p.background))
                .draw();
            painter
                .stroke(
                    RoundedRect::from_rect(r, 4.0),
                    &Stroke::new(1.0),
                    theme::color(p.border),
                )
                .draw();
            let at = Vec2::new(r.x0 + 6.0, r.y0 + 2.0);
            render_text(painter, Affine::translate(at), b, &brushes, false);
            self.painted.push(PaintStep::Badge(r));
        }

        if self.focused {
            painter
                .stroke(
                    RoundedRect::from_rect(size.to_rect().inset(-ring / 2.0), theme::PANEL_RADIUS),
                    &Stroke::new(ring),
                    theme::color(p.focus),
                )
                .draw();
            painter
                .stroke(
                    RoundedRect::from_rect(
                        size.to_rect().inset(-(ring + theme::FOCUS_INNER / 2.0)),
                        theme::PANEL_RADIUS - ring,
                    ),
                    &Stroke::new(theme::FOCUS_INNER),
                    theme::color(p.focus_inner),
                )
                .draw();
        }
    }

    fn accepts_text_input(&self) -> bool {
        // Cached when the widget is made, so always: typing and input
        // methods are taken only in edit mode.
        true
    }

    fn accessibility_role(&self) -> Role {
        // On macOS a Document is an AXGroup, which VoiceOver does not read
        // as text; a read-only multi-line text input is an AXTextArea, with
        // the text, the caret, and the reading commands.
        if self.edit_role || self.editing || self.platform == Platform::MacOs {
            Role::MultilineTextInput
        } else {
            Role::Document
        }
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let full = self.full_passes.get() != self.seen_full;
        self.seen_full = self.full_passes.get();
        if self.editing {
            // A multi-line edit: typing, and edits from a screen reader or
            // dictation, which the driver makes through the app.
            node.add_action(Action::ReplaceSelectedText);
            node.add_action(Action::SetValue);
        } else {
            node.set_read_only();
        }
        node.set_label(self.label.as_str());
        if !self.model.title.is_empty() {
            node.set_description(self.model.title.as_str());
        } else if self.model.paragraphs.is_empty() && !self.empty_hint.is_empty() {
            // What the empty window draws, read with the node's name when it
            // takes the focus.
            node.set_description(self.empty_hint.as_str());
        }
        node.add_action(Action::SetTextSelection);
        node.add_action(Action::ScrollIntoView);

        let n = self.para_count();
        if self.para_ids.len() != n {
            self.para_ids = vec![None; n];
        }
        let mut children = Vec::new();
        let mut sent = 0usize;
        let mut out: Vec<(NodeId, Node)> = Vec::new();
        let mut changed = std::mem::take(&mut self.changed_lines);
        for i in 0..n {
            let lines = changed.remove(&i);
            let unsent = full || self.dirty_paras[i] || self.para_ids[i].is_none();
            if !unsent && lines.is_none() {
                children.extend(self.para_ids[i].iter().flatten().flatten().copied());
                continue;
            }
            self.runs_of(i);
            // Borrowed out while its nodes are made; put back below.
            let runs = self.para_runs[i].take().unwrap_or_default();
            let line_count = runs.lines().len();
            let same_lines = self.para_ids[i]
                .as_ref()
                .is_some_and(|ids| ids.len() == line_count);
            let whole = unsent || !same_lines;
            if whole {
                // Every run of the paragraph.
                let old: Vec<NodeId> = self.para_ids[i]
                    .take()
                    .into_iter()
                    .flatten()
                    .flatten()
                    .collect();
                let mut ids = Vec::with_capacity(line_count);
                for line in runs.lines() {
                    ids.push(self.line_nodes(line, &mut out));
                    sent += line.len();
                }
                let new: Vec<NodeId> = ids.iter().flatten().copied().collect();
                self.forget_ids(&old, &new);
                self.para_ids[i] = Some(ids);
                self.dirty_paras[i] = false;
            } else if let Some(mut lines) = lines {
                // Only the lines whose runs changed (a moved highlight).
                lines.sort_unstable();
                lines.dedup();
                for g in lines.into_iter().filter(|&g| g < line_count) {
                    let line = &runs.lines()[g];
                    let ids = self.line_nodes(line, &mut out);
                    sent += line.len();
                    let old = self.para_ids[i]
                        .as_mut()
                        .map(|l| std::mem::replace(&mut l[g], ids.clone()))
                        .unwrap_or_default();
                    self.forget_ids(&old, &ids);
                }
            }
            self.para_runs[i] = Some(runs);
            children.extend(self.para_ids[i].iter().flatten().flatten().copied());
        }
        if n == 0 {
            // An empty document still has one (empty) run, so screen
            // readers find a text control.
            let empty = RunSet::build(&[], &[]).runs;
            let id = self.id_for(CharPos::ZERO);
            out.push((id, self.run_node(&empty[0], None, None)));
            children.push(id);
            sent += 1;
        }
        if std::mem::take(&mut self.prune_ids) {
            // After a slide: runs that left the window are gone from the
            // tree; forget their ids so they are not reused.
            let shown: std::collections::HashSet<NodeId> = children.iter().copied().collect();
            self.ids_to_pos.retain(|id, _| shown.contains(id));
            self.run_ids.retain(|_, id| shown.contains(id));
        }
        ctx.tree_update().nodes.extend(out);
        node.set_children(children);
        self.last_nodes_sent = sent;

        // The caret (or, while reading, the caret on the spoken word) as
        // the text selection.
        let (anchor, focus) = match (self.state.reading, self.state.spoken) {
            (true, Some(r)) if self.select_spoken => (r.start, r.end),
            (true, Some(r)) => (r.start, r.start),
            _ => (
                self.state.anchor.unwrap_or(self.state.caret),
                self.state.caret,
            ),
        };
        if let (Some(a), Some(f)) = (self.text_position(anchor), self.text_position(focus)) {
            node.set_text_selection(TextSelection {
                anchor: a,
                focus: f,
            });
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(format!("{} paragraphs", self.para_count()))
    }
}

/// The lines of `layout` that reach into `ys` (the layout's own
/// coordinates, top and bottom). Lines are in order down the layout, so
/// the walk stops at the first line below.
fn lines_within(
    layout: &Layout<BrushIndex>,
    ys: (f64, f64),
) -> impl Iterator<Item = masonry::parley::Line<'_, BrushIndex>> {
    layout
        .lines()
        .skip_while(move |l| f64::from(l.metrics().block_max_coord) < ys.0)
        .take_while(move |l| f64::from(l.metrics().block_min_coord) <= ys.1)
}

/// The visual lines of `layout` that reach into `ys`, as a range of
/// line indices (first, past the last).
fn line_span_within(layout: &Layout<BrushIndex>, ys: (f64, f64)) -> (usize, usize) {
    let mut first = None;
    let mut last = 0;
    for (n, l) in layout.lines().enumerate() {
        let m = l.metrics();
        if f64::from(m.block_max_coord) < ys.0 {
            continue;
        }
        if f64::from(m.block_min_coord) > ys.1 {
            break;
        }
        first.get_or_insert(n);
        last = n + 1;
    }
    (first.unwrap_or(0), last)
}

/// `text` laid out in one style in the text color, wrapped at `width` when
/// given: the empty window's hint and the edit-mode badge.
fn plain_layout(
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    text: &str,
    family: &str,
    size: f32,
    width: Option<f64>,
) -> Layout<BrushIndex> {
    let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
    b.push_default(StyleProperty::FontFamily(FontFamily::Source(
        family.to_owned().into(),
    )));
    b.push_default(StyleProperty::FontSize(size));
    b.push_default(StyleProperty::Brush(BrushIndex(B_TEXT)));
    let mut layout = b.build(text);
    layout.break_all_lines(width.map(|w| w as f32));
    layout
}

/// Draws the lines of `layout` that reach into `ys` (layout coordinates),
/// as Masonry's `render_text` draws them all: underlines, the glyphs, then
/// strikethroughs. A paragraph of a thousand lines (one very long line)
/// costs only the lines on screen.
fn render_lines(
    painter: &mut Painter<'_>,
    transform: Affine,
    layout: &Layout<BrushIndex>,
    brushes: &[masonry::peniko::Brush],
    ys: (f64, f64),
) {
    use masonry::imaging::record::Glyph;
    use masonry::kurbo::Line;
    use masonry::parley::PositionedLayoutItem;
    use masonry::peniko::{Fill, Style};
    for line in lines_within(layout, ys) {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let style = glyph_run.style();
            let run = glyph_run.run();
            let metrics = run.metrics();
            let x0 = f64::from(glyph_run.offset());
            let x1 = f64::from(glyph_run.offset() + glyph_run.advance());
            if let Some(underline) = &style.underline {
                let offset = underline.offset.unwrap_or(metrics.underline_offset);
                let width = underline.size.unwrap_or(metrics.underline_size);
                let y = f64::from(glyph_run.baseline() - offset + width / 2.);
                painter
                    .stroke(
                        Line::new((x0, y), (x1, y)),
                        &Stroke::new(width.into()),
                        &brushes[underline.brush.0],
                    )
                    .transform(transform)
                    .draw();
            }
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let glyph_xform = run
                .synthesis()
                .skew()
                .map(|angle| Affine::skew(f64::from(angle).to_radians().tan(), 0.0));
            let glyphs: Vec<Glyph> = glyph_run
                .glyphs()
                .map(|g| {
                    let out = Glyph {
                        id: g.id,
                        x: x + g.x,
                        y: y + g.y,
                    };
                    x += g.advance;
                    out
                })
                .collect();
            painter
                .glyphs(run.font(), &brushes[style.brush.0])
                .hint(false)
                .transform(transform)
                .glyph_transform(glyph_xform)
                .font_size(run.font_size())
                .normalized_coords(run.normalized_coords())
                .draw(&Style::Fill(Fill::NonZero), &glyphs);
            if let Some(strike) = &style.strikethrough {
                let offset = strike.offset.unwrap_or(metrics.strikethrough_offset);
                let width = strike.size.unwrap_or(metrics.strikethrough_size);
                let y = f64::from(glyph_run.baseline() - offset + metrics.strikethrough_size / 2.);
                painter
                    .stroke(
                        Line::new((x0, y), (x1, y)),
                        &Stroke::new(width.into()),
                        &brushes[strike.brush.0],
                    )
                    .transform(transform)
                    .draw();
            }
        }
    }
}

/// The thickness the spoken word's glyph outlines grow by when drawn bold:
/// one twenty-fourth of the font size, FreeType's synthetic bold amount.
/// The glyphs stay where the layout put them, so nothing on the line moves.
const SYNTHETIC_BOLD: f32 = 1.0 / 24.0;

/// The slant of a synthetic italic, in degrees.
const SYNTHETIC_ITALIC: f64 = 12.0;

/// Draws `layout`'s glyphs in `brush`, bold and slanted when asked, without
/// laying anything out again. Bold fills each glyph and strokes its outline
/// in the same color (the renderers ignore `font_embolden`); italic skews
/// each glyph about its own origin. Only the lines reaching into `ys`
/// (layout coordinates: the word's band) are drawn.
fn render_emphasis(
    painter: &mut Painter<'_>,
    transform: Affine,
    layout: &Layout<BrushIndex>,
    brush: &masonry::peniko::Brush,
    (bold, italic): (bool, bool),
    ys: (f64, f64),
) {
    use masonry::imaging::record::Glyph;
    use masonry::parley::PositionedLayoutItem;
    use masonry::peniko::{Fill, Style};
    let fill = Style::Fill(Fill::NonZero);
    for line in lines_within(layout, ys) {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let run = glyph_run.run();
            let font_size = run.font_size();
            let skew = run.synthesis().skew().map_or(0.0, f64::from)
                + if italic { SYNTHETIC_ITALIC } else { 0.0 };
            let glyph_xform = (skew != 0.0).then(|| Affine::skew(skew.to_radians().tan(), 0.0));
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let glyphs: Vec<Glyph> = glyph_run
                .glyphs()
                .map(|g| {
                    let out = Glyph {
                        id: g.id,
                        x: x + g.x,
                        y: y + g.y,
                    };
                    x += g.advance;
                    out
                })
                .collect();
            painter
                .glyphs(run.font(), brush)
                .transform(transform)
                .glyph_transform(glyph_xform)
                .font_size(font_size)
                .normalized_coords(run.normalized_coords())
                .draw(&fill, &glyphs);
            if bold {
                let stroke = Style::Stroke(Stroke::new(f64::from(font_size * SYNTHETIC_BOLD)));
                painter
                    .glyphs(run.font(), brush)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .font_size(font_size)
                    .normalized_coords(run.normalized_coords())
                    .draw(&stroke, &glyphs);
            }
        }
    }
}

/// The rectangle of a line under `band` (one visual line of a range, in
/// view coordinates), at the font's underline position on that line and
/// at least 1.5 px or 0.06 em thick; `None` if no line holds it.
fn underline_under(
    layout: &Layout<BrushIndex>,
    origin: Vec2,
    band: Rect,
    font_size: f32,
) -> Option<Rect> {
    use masonry::parley::PositionedLayoutItem;
    let mid = ((band.y0 + band.y1) / 2.0 - origin.y) as f32;
    let line = layout.lines().find(|l| {
        let m = l.metrics();
        m.block_min_coord <= mid && mid <= m.block_max_coord
    })?;
    let baseline = line.metrics().baseline;
    let (offset, size) = line
        .items()
        .find_map(|item| match item {
            PositionedLayoutItem::GlyphRun(g) => {
                let m = g.run().metrics();
                Some((m.underline_offset, m.underline_size))
            }
            PositionedLayoutItem::InlineBox(_) => None,
        })
        .unwrap_or((-0.1 * font_size, 0.05 * font_size));
    let thick = f64::from(size.max(0.06 * font_size)).max(1.5);
    let top = origin.y + f64::from(baseline - offset);
    Some(Rect::new(band.x0, top, band.x1, top + thick))
}
