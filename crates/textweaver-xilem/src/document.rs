//! `DocumentView`: the document as one focusable node for screen readers,
//! drawn with Parley and Vello, written against Masonry and AccessKit
//! directly (ADR-0027).
//!
//! - **Accessibility.** One node, role `Document` and read-only, whose
//!   children are text runs of at most 255 characters ([`crate::runs`]).
//!   The caret is the node's text selection; the spoken word is its own run
//!   with a background colour; headings carry their level's size and
//!   weight. Only the runs of paragraphs that changed are sent again, so a
//!   highlight move costs a paragraph, not the document.
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
use masonry::kurbo::{Affine, Axis, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use masonry::layout::{LenReq, Length};
use masonry::parley::style::{FontFamily, FontStyle, FontWeight, LineHeight};
use masonry::parley::{Affinity, Cursor, FontContext, Layout, LayoutContext, Selection};
use textweaver_app::aids::{RowMark, RulerMode, RulerSettings, TextSpacing, ViewRow, ruler_rows};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::keymap::KeyChord;
use textweaver_app::keymap::Platform;

use crate::caret;
use crate::keys::{self, CaretStep};
use crate::runs::{Paragraph, Run, RunMark, RunSet};
use crate::theme::{self, Palette};
use crate::window::{SpanStyle, StyledSpan};

/// Widest text column, in logical pixels: about 70 characters at the
/// default size, a comfortable line for reading.
const MAX_COLUMN: f64 = 820.0;
/// Space around the text column.
const INSET: f64 = 28.0;
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
const BRUSHES: usize = 11;

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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DocAids {
    /// Line height, paragraph, letter, and word spacing, in multiples of
    /// the font size.
    pub spacing: TextSpacing,
    /// Off, the current line, or the ruler band (and the mask around it).
    pub ruler: RulerSettings,
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
        let mut out = vec![(Vec::new(), Vec::new()); paras.len()];
        if paras.is_empty() {
            return out;
        }
        for s in &self.spans {
            let a = caret::paragraph_at(paras, s.range.start);
            let b = caret::paragraph_at(paras, CharPos(s.range.end.0.saturating_sub(1)));
            for slot in out.iter_mut().take(b.max(a) + 1).skip(a) {
                slot.0.push(*s);
            }
        }
        for &k in &self.breaks {
            out[caret::paragraph_at(paras, k)].1.push(k);
        }
        out
    }
}

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
}

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
    /// Paragraphs on screen and their top y, from the last layout.
    visible: Vec<(usize, f64)>,
    follow: bool,
    goal_x: Option<f32>,

    // Accessibility.
    para_runs: Vec<Option<Vec<Run>>>,
    para_ids: Vec<Option<Vec<NodeId>>>,
    dirty_paras: Vec<bool>,
    run_ids: HashMap<CharPos, NodeId>,
    ids_to_pos: HashMap<NodeId, CharPos>,
    /// The window slid: forget the ids of runs no longer shown after the
    /// next accessibility pass.
    prune_ids: bool,
    full_passes: Rc<Cell<u64>>,
    seen_full: u64,
    /// Nodes sent in the last pass, when measuring.
    pub last_nodes_sent: usize,
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
            state: DocState::default(),
            select_spoken: false,
            edit_role: false,
            editing: false,
            platform: Platform::current(),
            yielded: Vec::new(),
            focused: false,
            label: "Document".to_owned(),
            misspelled: Vec::new(),
            layouts: HashMap::new(),
            line_starts: Vec::new(),
            column: 0.0,
            column_x: 0.0,
            size: Size::ZERO,
            top: (0, 0.0),
            visible: Vec::new(),
            follow: true,
            goal_x: None,
            para_runs: Vec::new(),
            para_ids: Vec::new(),
            dirty_paras: Vec::new(),
            run_ids: HashMap::new(),
            ids_to_pos: HashMap::new(),
            prune_ids: false,
            full_passes,
            seen_full: u64::MAX,
            last_nodes_sent: 0,
        }
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

    fn replace_model(this: &mut WidgetMut<'_, Self>, model: DocModel, keep_ids: bool) {
        let w = &mut *this.widget;
        // Keep the scroll anchor on the same text when the window moves.
        let anchor_pos = w.model.paragraphs.get(w.top.0).map(|p| p.start);
        let old = std::mem::replace(&mut w.model, model);
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
        let relayout = w.aids.spacing != aids.spacing;
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

    fn mark_dirty(&mut self, r: CharRange) {
        let paras = &self.model.paragraphs;
        if paras.is_empty() {
            return;
        }
        let a = caret::paragraph_at(paras, r.start);
        let b = caret::paragraph_at(paras, CharPos(r.end.0.saturating_sub(1).max(r.start.0)));
        for i in a..=b.min(paras.len() - 1) {
            self.dirty_paras[i] = true;
            self.para_runs[i] = None;
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
            sb.push_default(StyleProperty::Brush(BrushIndex(
                p.heading.map_or(B_TEXT, |l| B_H1 + usize::from(l - 1)),
            )));
            let mut layout = sb.build(sep);
            layout.break_all_lines(None);
            let width = layout.full_width();
            let baseline = layout.lines().next().map_or(0.0, |l| l.metrics().baseline);
            SepMarks {
                layout,
                width,
                baseline,
                at: breaks.iter().map(|&k| caret::byte_of(text, k)).collect(),
            }
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
        for s in &self.model.spans {
            if s.range.end.0 <= p.start.0 || s.range.start.0 >= p_end {
                continue;
            }
            let a = caret::byte_of(text, s.range.start.0.saturating_sub(p.start.0));
            let e = caret::byte_of(text, s.range.end.0.min(p_end) - p.start.0);
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
                    // A thicker underline than a link's, in the text's own
                    // colour: the shape carries the meaning, not a colour.
                    b.push(StyleProperty::Underline(true), a..e);
                    b.push(StyleProperty::UnderlineSize(Some(size * 0.1)), a..e);
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
                let a = caret::byte_of(text, k - 1);
                let e = caret::byte_of(text, k);
                b.push(StyleProperty::LetterSpacing(base + s.width), a..e);
            }
        }
        let mut layout = b.build(text);
        layout.break_all_lines(Some(self.column as f32));
        let em = f64::from(size);
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
        }
    }

    /// Paragraph `i`'s syllable breaks, as char offsets inside it (never at
    /// its start or end).
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
        let pl = self.build_layout(i, fcx, lcx);
        let p = &self.model.paragraphs[i];
        let starts: Vec<usize> = pl
            .layout
            .lines()
            .skip(1)
            .map(|l| caret::char_of(&p.text, l.text_range().start))
            .collect();
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
        let b = caret::byte_of(&p.text, off);
        let bb =
            Cursor::from_byte_index(&pl.layout, b, Affinity::Downstream).geometry(&pl.layout, 2.0);
        let h = if p.text.is_empty() {
            f64::from(self.font.size) * 1.2
        } else {
            bb.y1 - bb.y0
        };
        Some((
            i,
            Rect::new(bb.x0, pl.top_gap + bb.y0, bb.x1, pl.top_gap + bb.y0 + h),
        ))
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
    /// bottom in the view.
    fn rows_on_screen(&self) -> Vec<(ViewRow, f64, f64)> {
        let mut rows = Vec::new();
        for &(i, y) in &self.visible {
            let Some(pl) = self.layouts.get(&i) else {
                continue;
            };
            let p = &self.model.paragraphs[i];
            let top = y + pl.top_gap;
            let mut any = false;
            for line in pl.layout.lines() {
                let tr = line.text_range();
                let a = p.start.0 + caret::char_of(&p.text, tr.start);
                let b = p.start.0 + caret::char_of(&p.text, tr.end);
                let m = line.metrics();
                rows.push((
                    ViewRow {
                        range: CharRange::new(a, b),
                        line: i,
                    },
                    top + f64::from(m.block_min_coord),
                    top + f64::from(m.block_max_coord),
                ));
                any = true;
            }
            if !any {
                // A blank line: one empty row at its position.
                let h = f64::from(self.font.size) * 1.2;
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
            let b = caret::byte_of(&p.text, pos.0.saturating_sub(p.start.0).min(p.len_chars()));
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
        let c = Cursor::from_point(&pl.layout, x, y);
        CharPos(p.start.0 + caret::char_of(&p.text, c.index()))
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
        let b = caret::byte_of(&p.text, pos.0.saturating_sub(p.start.0).min(p.len_chars()));
        let sel = Selection::from_byte_index(&pl.layout, b, Affinity::Downstream);
        let moved = if end {
            sel.line_end(&pl.layout, false)
        } else {
            sel.line_start(&pl.layout, false)
        };
        CharPos(p.start.0 + caret::char_of(&p.text, moved.focus().index()))
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
                let b = caret::byte_of(&p.text, off);
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
                    (pos.x - self.column_x) as f32,
                    (pos.y - y - pl.top_gap) as f32,
                );
                return Some(CharPos(p.start.0 + caret::char_of(&p.text, c.index())));
            }
        }
        None
    }

    // --- Accessibility.

    fn runs_of(&mut self, i: usize) -> &[Run] {
        if self.para_runs[i].is_none() {
            let mut p = self.model.paragraphs[i].clone();
            p.line_starts = self.line_starts[i].clone();
            let mut marks = Vec::new();
            if let Some(r) = self.state.spoken {
                marks.push((r, RunMark::SpokenWord));
            }
            let set = RunSet::build(std::slice::from_ref(&p), &marks);
            let mut runs = set.runs;
            for r in &mut runs {
                r.paragraph = i;
            }
            self.para_runs[i] = Some(runs);
        }
        self.para_runs[i].as_deref().unwrap_or(&[])
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
        let runs = self.runs_of(i).to_vec();
        let set = RunSet { runs };
        let rp = set.position(pos);
        let run = set.runs.get(rp.run)?;
        let key = run.start;
        let id = self.id_for(key);
        Some(TextPosition {
            node: id,
            character_index: rp.index,
        })
    }

    /// The document position of an accessibility position.
    fn doc_position(&mut self, tp: &TextPosition) -> Option<CharPos> {
        let start = *self.ids_to_pos.get(&tp.node)?;
        let paras = &self.model.paragraphs;
        let i = caret::paragraph_at(paras, start);
        let runs = self.runs_of(i).to_vec();
        let set = RunSet { runs };
        let r = set.index_of(start)?;
        Some(set.char_pos(crate::runs::RunPos {
            run: r,
            index: tp.character_index,
        }))
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
        // Fills what it is given; asks for a comfortable column.
        let want = match axis {
            Axis::Horizontal => MAX_COLUMN + 2.0 * INSET,
            Axis::Vertical => 400.0,
        };
        match len_req {
            LenReq::MinContent => Length::px(200.0),
            LenReq::MaxContent => Length::px(want),
            LenReq::FitContent(space) => space,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let column = (size.width - 2.0 * INSET).clamp(80.0, MAX_COLUMN);
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
        let (fcx, lcx) = ctx.text_contexts();
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
        if !self.focused {
            painter
                .stroke(
                    RoundedRect::from_rect(size.to_rect().inset(-0.5), theme::PANEL_RADIUS),
                    &Stroke::new(1.0),
                    theme::color(p.border),
                )
                .draw();
        }
        // Text stays inside the ring.
        painter.push_fill_clip(RoundedRect::from_rect(
            size.to_rect().inset(-(theme::FOCUS_WIDTH + 1.0)),
            theme::PANEL_RADIUS - 2.0,
        ));
        let brushes = self.brushes();
        let caret_pos = self.state.caret;
        let reading = self.state.reading;
        let spoken = self.state.spoken.filter(|_| reading);
        let sentence = self.state.sentence.filter(|_| reading);
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
            match mark {
                RowMark::Focus => {
                    painter
                        .fill(RoundedRect::from_rect(r, 4.0), theme::color(p.ruler_focus))
                        .draw();
                    painter
                        .fill(
                            Rect::new(band_x0, y0, band_x0 + 4.0, y1),
                            theme::color(p.focus),
                        )
                        .draw();
                }
                RowMark::Band => painter.fill(r, theme::color(p.ruler_band)).draw(),
                RowMark::Normal | RowMark::Masked => {}
            }
        }

        for &(i, y) in &self.visible {
            let Some(pl) = self.layouts.get(&i) else {
                continue;
            };
            let para = &self.model.paragraphs[i];
            let origin = Vec2::new(self.column_x, y + pl.top_gap);
            let tf = Affine::translate(origin);
            let p_end = para.start.0 + para.len_chars();
            let band = |r: CharRange| -> Option<Vec<Rect>> {
                if r.end.0 <= para.start.0 || r.start.0 > p_end || r.is_empty() {
                    return None;
                }
                let a = caret::byte_of(&para.text, r.start.0.saturating_sub(para.start.0));
                let b = caret::byte_of(&para.text, r.end.0.min(p_end) - para.start.0);
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
            // Code backgrounds.
            for s in &self.model.spans {
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
            if let Some(rects) = sentence.and_then(band) {
                for r in rects {
                    painter.fill(r, theme::color(p.spoken_sentence)).draw();
                }
            }
            if let Some(rects) = selection.and_then(band) {
                for r in rects {
                    painter.fill(r, theme::color(p.selection.1)).draw();
                }
            }
            let word_rects = spoken.and_then(band).unwrap_or_default();
            for r in &word_rects {
                painter
                    .fill(
                        RoundedRect::from_rect(r.inflate(3.0, 1.0), 4.0),
                        theme::color(p.spoken_word.1),
                    )
                    .draw();
            }
            render_text(painter, tf, &pl.layout, &brushes, false);
            // Syllable separators, in the space left before each break,
            // on the line's baseline.
            if let Some(s) = &pl.seps {
                for &b in &s.at {
                    let Some(line) = pl.layout.lines().find(|l| l.text_range().contains(&b)) else {
                        continue;
                    };
                    let x = Cursor::from_byte_index(&pl.layout, b, Affinity::Downstream)
                        .geometry(&pl.layout, 1.0)
                        .x0;
                    let at = origin
                        + Vec2::new(
                            x - f64::from(s.width),
                            f64::from(line.metrics().baseline - s.baseline),
                        );
                    render_text(painter, Affine::translate(at), &s.layout, &brushes, false);
                }
            }
            // The spoken word's text again, in its own colour, clipped to
            // its band (no relayout per word).
            if !word_rects.is_empty() {
                let fg: [masonry::peniko::Brush; BRUSHES] =
                    std::array::from_fn(|_| theme::color(p.spoken_word.0).into());
                for r in &word_rects {
                    painter.push_fill_clip(r.inflate(3.0, 1.0));
                    render_text(painter, tf, &pl.layout, &fg, false);
                    painter.pop_clip();
                }
            }
            if let Some(rects) = selection.and_then(band) {
                let fg: [masonry::peniko::Brush; BRUSHES] =
                    std::array::from_fn(|_| theme::color(p.selection.0).into());
                for r in rects {
                    painter.push_fill_clip(r);
                    render_text(painter, tf, &pl.layout, &fg, false);
                    painter.pop_clip();
                }
            }
            // Misspelled words: a dotted underline at the text's foot.
            for &r in &self.misspelled {
                if r.end.0 <= para.start.0 || r.start.0 >= p_end {
                    continue;
                }
                for rect in band(r).unwrap_or_default() {
                    let y = rect.y1 - 2.5;
                    let mut x = rect.x0;
                    while x + 2.0 <= rect.x1 {
                        painter
                            .fill(Rect::new(x, y, x + 2.0, y + 2.0), theme::color(p.focus))
                            .draw();
                        x += 4.0;
                    }
                }
            }
            // The caret, when the view has focus.
            let para_end = CharPos(para.start.0 + para.span_chars());
            let in_para = caret_pos.0 >= para.start.0
                && (caret_pos.0 < para_end.0 || (i + 1 == self.model.paragraphs.len()));
            if self.focused && in_para && !reading {
                let off = caret_pos
                    .0
                    .saturating_sub(para.start.0)
                    .min(para.len_chars());
                let b = caret::byte_of(&para.text, off);
                let bb = Cursor::from_byte_index(&pl.layout, b, Affinity::Downstream)
                    .geometry(&pl.layout, 2.0);
                let h = if para.text.is_empty() {
                    f64::from(self.font.size) * 1.2
                } else {
                    bb.y1 - bb.y0
                };
                let r = Rect::new(bb.x0, bb.y0, bb.x0 + 2.0, bb.y0 + h) + origin;
                painter.fill(r, theme::color(p.caret)).draw();
            }
        }

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
            let x = size.width - 7.0;
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

        // The focus ring.
        if self.focused {
            let r = size.to_rect().inset(-1.0 - theme::FOCUS_WIDTH / 2.0);
            painter
                .stroke(
                    RoundedRect::from_rect(r, theme::PANEL_RADIUS),
                    &Stroke::new(theme::FOCUS_WIDTH),
                    theme::color(p.focus),
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
        if self.edit_role || self.editing {
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
        let mut forgotten = Vec::new();
        for i in 0..n {
            let rebuild = full || self.dirty_paras[i] || self.para_ids[i].is_none();
            if !rebuild {
                children.extend(self.para_ids[i].iter().flatten().copied());
                continue;
            }
            let runs = self.runs_of(i).to_vec();
            let ids: Vec<NodeId> = runs.iter().map(|r| self.id_for(r.start)).collect();
            // Runs of this paragraph that no longer exist.
            if let Some(old) = &self.para_ids[i] {
                forgotten.extend(old.iter().filter(|id| !ids.contains(id)).copied());
            }
            for (k, r) in runs.iter().enumerate() {
                let next = (r.continues_line && k + 1 < runs.len()).then(|| ids[k + 1]);
                let prev = (k > 0 && runs[k - 1].continues_line).then(|| ids[k - 1]);
                out.push((ids[k], self.run_node(r, next, prev)));
            }
            sent += runs.len();
            self.dirty_paras[i] = false;
            children.extend(ids.iter().copied());
            self.para_ids[i] = Some(ids);
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
        for id in forgotten {
            if let Some(k) = self.ids_to_pos.remove(&id) {
                self.run_ids.remove(&k);
            }
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
