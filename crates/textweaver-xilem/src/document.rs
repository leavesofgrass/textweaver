//! `DocumentView`: the document as one focusable node for screen readers,
//! drawn with Parley and Vello, written against Masonry and AccessKit
//! directly (ADR-0023).
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
//! - **Keys.** Arrows, Home, End, Page Up, and Page Down (with Shift to
//!   select and Ctrl for words, paragraphs, and the document's ends) move
//!   the caret here, and the screen reader reads what the caret moved over.
//!   Every other key goes on to the keymap.
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
use textweaver_app::core::{CharPos, CharRange};

use crate::caret;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocAction {
    /// The user moved the caret (keys, pointer, or a screen reader).
    CaretMoved {
        /// The new caret.
        caret: CharPos,
        /// The selection, if any.
        selection: Option<CharRange>,
    },
}

/// A laid-out paragraph.
struct ParaLayout {
    layout: Layout<BrushIndex>,
    /// Height including the space after it.
    height: f64,
    /// Space above the text (headings get more).
    top_gap: f64,
}

/// The document view widget. See the module documentation.
pub struct DocumentView {
    palette: Palette,
    font: DocFont,
    model: DocModel,
    state: DocState,
    /// Speech-cursor style: select the spoken word instead of a caret.
    select_spoken: bool,
    focused: bool,

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
            model: DocModel::default(),
            state: DocState::default(),
            select_spoken: false,
            focused: false,
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

    /// Replaces the window's text (a new document or a moved window).
    pub fn set_model(this: &mut WidgetMut<'_, Self>, model: DocModel) {
        let w = &mut *this.widget;
        // Keep the scroll anchor on the same text when the window moves.
        let anchor_pos = w.model.paragraphs.get(w.top.0).map(|p| p.start);
        w.model = model;
        w.reset_caches();
        w.run_ids.clear();
        w.ids_to_pos.clear();
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

    /// The current state.
    pub fn state(&self) -> DocState {
        self.state
    }

    /// The paragraphs on screen (index and top y), for tests.
    pub fn visible_paragraphs(&self) -> &[(usize, f64)] {
        &self.visible
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
        let text = p.text.as_str();
        let mut b = lcx.ranged_builder(fcx, text, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(
            self.font.family.clone().into(),
        )));
        b.push_default(StyleProperty::FontSize(size));
        b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.5)));
        b.push_default(StyleProperty::Brush(BrushIndex(B_TEXT)));
        if self.font.bold {
            b.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
        }
        if let Some(level) = p.heading {
            b.push_default(StyleProperty::FontSize(size * Self::heading_scale(level)));
            b.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
            b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                1.25,
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
        let after = if text.is_empty() { 0.0 } else { em * 0.45 };
        ParaLayout {
            layout,
            height: top_gap + text_h + after,
            top_gap,
        }
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
                let per_line = (self.column / (em * 0.5)).max(10.0);
                let lines = (p.len_chars() as f64 / per_line).ceil().max(1.0);
                if p.text.is_empty() {
                    em * 0.6
                } else {
                    lines * em * 1.5 + em * 0.45
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
            (l.metrics().min_coord + l.metrics().max_coord) / 2.0
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
        let line = f64::from(self.font.size) * 1.5;
        let lines = ((view / line).floor() as isize - 1).max(1);
        let pos = self.move_lines(if down { lines } else { -lines }, fcx, lcx);
        // Scroll by the same page, so the caret keeps its place on screen.
        self.top.1 += if down { view } else { -view };
        pos
    }

    /// Handles a caret key. Returns true when the key was a caret key.
    fn caret_key(&mut self, ctx: &mut EventCtx<'_>, key: &Key, shift: bool, ctrl: bool) -> bool {
        let paras = &self.model.paragraphs;
        let pos = self.state.caret;
        let doc_end = CharPos(self.model.doc_len);
        let (fcx, lcx) = ctx.text_contexts();
        let new = match key {
            Key::Named(NamedKey::ArrowLeft) if ctrl => caret::prev_word(paras, pos),
            Key::Named(NamedKey::ArrowRight) if ctrl => caret::next_word(paras, pos),
            Key::Named(NamedKey::ArrowLeft) => caret::prev_char(paras, pos),
            Key::Named(NamedKey::ArrowRight) => caret::next_char(paras, pos),
            Key::Named(NamedKey::ArrowUp) if ctrl => caret::prev_paragraph(paras, pos),
            Key::Named(NamedKey::ArrowDown) if ctrl => caret::next_paragraph(paras, pos),
            Key::Named(NamedKey::ArrowUp) => self.move_lines(-1, fcx, lcx),
            Key::Named(NamedKey::ArrowDown) => self.move_lines(1, fcx, lcx),
            Key::Named(NamedKey::Home) if ctrl => CharPos::ZERO,
            Key::Named(NamedKey::End) if ctrl => doc_end,
            Key::Named(NamedKey::Home) => self.line_edge(false, fcx, lcx),
            Key::Named(NamedKey::End) => self.line_edge(true, fcx, lcx),
            Key::Named(NamedKey::PageUp) => self.page(false, fcx, lcx),
            Key::Named(NamedKey::PageDown) => self.page(true, fcx, lcx),
            _ => return false,
        };
        let vertical = matches!(
            key,
            Key::Named(
                NamedKey::ArrowUp | NamedKey::ArrowDown | NamedKey::PageUp | NamedKey::PageDown
            )
        ) && !ctrl;
        let goal = self.goal_x;
        self.move_caret(ctx, new, shift);
        if vertical {
            self.goal_x = goal;
        }
        true
    }

    fn move_caret(&mut self, ctx: &mut EventCtx<'_>, new: CharPos, extend: bool) {
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
        });
    }

    fn copy_selection(&self, ctx: &mut EventCtx<'_>) {
        if let Some(a) = self.state.anchor {
            let text = caret::text_between(&self.model.paragraphs, a, self.state.caret);
            if !text.is_empty() {
                ctx.set_clipboard(text);
            }
        }
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
        let TextEvent::Keyboard(k) = event else {
            return;
        };
        if k.state != KeyState::Down {
            return;
        }
        let m = k.modifiers;
        let ctrl = if cfg!(target_os = "macos") {
            m.meta()
        } else {
            m.ctrl()
        };
        if m.alt() {
            return;
        }
        if ctrl && matches!(&k.key, Key::Character(c) if c.eq_ignore_ascii_case("c")) {
            self.copy_selection(ctx);
            ctx.set_handled();
            return;
        }
        if self.caret_key(ctx, &k.key, m.shift(), ctrl) {
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

    fn accessibility_role(&self) -> Role {
        Role::Document
    }

    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let full = self.full_passes.get() != self.seen_full;
        self.seen_full = self.full_passes.get();
        node.set_read_only();
        node.set_label("Document");
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
