//! Selecting with the mouse in the document view: a click places the
//! caret, a drag selects, Shift with a click extends, a double click
//! selects a word and a triple click a paragraph, and a drag at an edge
//! scrolls. The selection is the keyboard's (`DocState`), and a drag is
//! reported once, when the button comes up.

use std::cell::Cell;
use std::rc::Rc;

use masonry::core::pointer::PointerButtons;
use masonry::core::{
    NewWidget, PointerButton, PointerButtonEvent, PointerEvent, PointerState, PointerUpdate,
};
use masonry::dpi::PhysicalPosition;
use masonry::kurbo::Point;
use masonry_testing::{PRIMARY_MOUSE, TestHarness, TestHarnessParams};
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::text::Document;
use textweaver_xilem::document::{CaretEcho, DocAction, DocFont, DocModel, DocState, DocumentView};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::window::{self, WINDOW_UNITS};

const TEXT: &str = "Alpha beta gamma.\nDelta epsilon.\nZeta eta theta.";

fn harness(text: &str, height: u32) -> TestHarness<DocumentView> {
    let p = Palette::galaxy();
    let view = DocumentView::new(p.clone(), DocFont::default(), Rc::new(Cell::new(0)));
    let mut params = TestHarnessParams::default();
    params.window_size = (900, height).into();
    let mut h =
        TestHarness::create_with(theme::default_properties(&p), NewWidget::new(view), params);
    for b in textweaver_xilem::fonts::bundled_blobs() {
        h.register_fonts(b);
    }
    let doc = Document::from_plain_text(text);
    let w = textweaver_app::DocWindow::with_budget(&doc, CharPos::ZERO, WINDOW_UNITS);
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, w.range()),
        spans: window::window_spans(&doc, w.range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
        ..DocModel::default()
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    h.focus_on(Some(h.root_id()));
    h
}

/// Where the view draws the caret for `pos`: a point just inside the
/// char after it, so a click there puts the caret at `pos`.
fn point_of(h: &mut TestHarness<DocumentView>, pos: CharPos) -> Point {
    let keep = h.root_widget().state();
    h.edit_root_widget(|mut d| {
        DocumentView::set_state(
            &mut d,
            DocState {
                caret: pos,
                ..DocState::default()
            },
        )
    });
    let _ = h.redraw();
    let foot = h.root_widget().caret_point().expect("the caret was drawn");
    h.edit_root_widget(|mut d| DocumentView::set_state(&mut d, keep));
    let _ = h.redraw();
    Point::new(foot.x + 2.0, foot.y - 6.0)
}

fn state(at: Point, count: u8, shift: bool, down: bool) -> PointerState {
    let mut s = PointerState {
        position: PhysicalPosition::new(at.x, at.y),
        count,
        ..PointerState::default()
    };
    if down {
        let mut b = PointerButtons::default();
        b.insert(PointerButton::Primary);
        s.buttons = b;
    }
    if shift {
        s.modifiers = masonry::core::Modifiers::SHIFT;
    }
    s
}

fn press(h: &mut TestHarness<DocumentView>, at: Point, count: u8, shift: bool) {
    h.process_pointer_event(PointerEvent::Down(PointerButtonEvent {
        pointer: PRIMARY_MOUSE,
        button: Some(PointerButton::Primary),
        state: state(at, count, shift, true),
    }));
}

fn drag(h: &mut TestHarness<DocumentView>, at: Point) {
    h.process_pointer_event(PointerEvent::Move(PointerUpdate {
        pointer: PRIMARY_MOUSE,
        current: state(at, 1, false, true),
        coalesced: vec![],
        predicted: vec![],
    }));
}

fn release(h: &mut TestHarness<DocumentView>, at: Point) {
    h.process_pointer_event(PointerEvent::Up(PointerButtonEvent {
        pointer: PRIMARY_MOUSE,
        button: Some(PointerButton::Primary),
        state: state(at, 1, false, false),
    }));
}

/// Every caret move the view reported since the last call.
fn moves(
    h: &mut TestHarness<DocumentView>,
) -> Vec<(CharPos, Option<CharRange>, Option<CaretEcho>)> {
    let mut out = Vec::new();
    while let Some((a, _)) = h.pop_action::<DocAction>() {
        if let DocAction::CaretMoved {
            caret,
            selection,
            echo,
        } = a
        {
            out.push((caret, selection, echo));
        }
    }
    out
}

fn click(h: &mut TestHarness<DocumentView>, at: Point, count: u8, shift: bool) {
    press(h, at, count, shift);
    release(h, at);
}

/// The node's selection as the screen reader has it: anchor and focus.
fn told(h: &TestHarness<DocumentView>) -> (usize, usize) {
    let node = h.access_node(h.root_id()).expect("the document");
    let a = node.text_selection_anchor().expect("anchor");
    let f = node.text_selection_focus().expect("focus");
    (a.to_global_usv_index(), f.to_global_usv_index())
}

#[test]
fn a_click_puts_the_caret_where_the_pointer_is() {
    let mut h = harness(TEXT, 600);
    for pos in [6, 11, 18, 24, 33, 38] {
        let at = point_of(&mut h, CharPos(pos));
        let _ = moves(&mut h);
        click(&mut h, at, 1, false);
        let _ = h.redraw();
        assert_eq!(
            moves(&mut h),
            vec![(CharPos(pos), None, None)],
            "a click at {pos} places the caret there, quietly"
        );
        assert_eq!(h.root_widget().state().caret, CharPos(pos));
        assert_eq!(told(&h), (pos, pos));
    }
}

#[test]
fn a_drag_selects_and_is_reported_once_when_it_ends() {
    let mut h = harness(TEXT, 600);
    let from = point_of(&mut h, CharPos(6));
    let path: Vec<Point> = [8, 11, 14, 20, 24]
        .into_iter()
        .map(|p| point_of(&mut h, CharPos(p)))
        .collect();
    let _ = moves(&mut h);
    press(&mut h, from, 1, false);
    for at in &path {
        drag(&mut h, *at);
        let _ = h.redraw();
        // Drawn as it grows, but nothing reported and the screen reader's
        // selection left alone until the button comes up.
        assert!(moves(&mut h).is_empty(), "no report during the drag");
        assert_eq!(told(&h), (0, 0));
    }
    assert_eq!(h.root_widget().state().anchor, Some(CharPos(6)));
    assert_eq!(h.root_widget().state().caret, CharPos(24));
    release(&mut h, *path.last().unwrap());
    let _ = h.redraw();
    let reported = moves(&mut h);
    assert_eq!(reported.len(), 1, "one report: {reported:?}");
    let (caret, selection, echo) = reported[0].clone();
    assert_eq!(caret, CharPos(24));
    assert_eq!(selection, Some(CharRange::new(6, 24)));
    assert_eq!(
        echo,
        Some(CaretEcho::Selection {
            text: "beta gamma.\nDelta ".into(),
            selected: true,
        }),
        "the self-voicing echo says the selected text, as Shift with a caret key does"
    );
    assert_eq!(told(&h), (6, 24));
    // A drag back over its start selects the other way.
    press(&mut h, path[3], 1, false);
    drag(&mut h, from);
    release(&mut h, from);
    let reported = moves(&mut h);
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].1, Some(CharRange::new(6, 20)));
    assert_eq!(h.root_widget().state().caret, CharPos(6));
}

#[test]
fn shift_with_a_click_extends_the_selection() {
    let mut h = harness(TEXT, 600);
    let a = point_of(&mut h, CharPos(6));
    let b = point_of(&mut h, CharPos(24));
    let c = point_of(&mut h, CharPos(11));
    click(&mut h, a, 1, false);
    let _ = moves(&mut h);
    click(&mut h, b, 1, true);
    let reported = moves(&mut h);
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].1, Some(CharRange::new(6, 24)));
    // Again, from the same fixed end.
    click(&mut h, c, 1, true);
    let reported = moves(&mut h);
    assert_eq!(reported[0].1, Some(CharRange::new(6, 11)));
    // A plain click drops the selection, and says nothing.
    click(&mut h, b, 1, false);
    assert_eq!(moves(&mut h), vec![(CharPos(24), None, None)]);
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_a_paragraph() {
    let mut h = harness(TEXT, 600);
    // Inside "epsilon" (chars 24 to 31 of the second paragraph).
    let at = point_of(&mut h, CharPos(27));
    let _ = moves(&mut h);
    click(&mut h, at, 1, false);
    click(&mut h, at, 2, false);
    let reported = moves(&mut h);
    assert_eq!(reported.len(), 2, "the click, then the word: {reported:?}");
    assert_eq!(
        reported[1].1,
        Some(CharRange::new(24, 31)),
        "the word alone"
    );
    assert!(matches!(
        &reported[1].2,
        Some(CaretEcho::Selection { text, selected: true }) if text == "epsilon"
    ));
    click(&mut h, at, 3, false);
    let reported = moves(&mut h);
    assert_eq!(
        reported[0].1,
        Some(CharRange::new(18, 33)),
        "the paragraph with its line break"
    );
    // A double click and drag selects by words.
    let further = point_of(&mut h, CharPos(36));
    press(&mut h, at, 2, false);
    drag(&mut h, further);
    release(&mut h, further);
    let reported = moves(&mut h);
    assert_eq!(reported.len(), 1);
    assert_eq!(
        reported[0].1,
        Some(CharRange::new(24, 37)),
        "to the end of \"Zeta\""
    );
}

#[test]
fn a_drag_at_the_bottom_edge_scrolls_and_keeps_selecting() {
    let text: String = (1..=80)
        .map(|n| format!("Line number {n} of the long document."))
        .collect::<Vec<_>>()
        .join("\n");
    let mut h = harness(&text, 300);
    let top_before = h.root_widget().visible_paragraphs()[0].0;
    let from = point_of(&mut h, CharPos(0));
    let _ = moves(&mut h);
    press(&mut h, from, 1, false);
    let edge = Point::new(from.x, 298.0);
    drag(&mut h, edge);
    let caret_at_edge = h.root_widget().state().caret;
    for _ in 0..20 {
        h.animate_ms(50);
    }
    let _ = h.redraw();
    let top_after = h.root_widget().visible_paragraphs()[0].0;
    assert!(
        top_after > top_before,
        "the view scrolled: {top_before} to {top_after}"
    );
    let caret = h.root_widget().state().caret;
    assert!(
        caret.0 > caret_at_edge.0,
        "the selection followed: {caret:?}"
    );
    assert!(moves(&mut h).is_empty(), "still nothing reported");
    // Back inside the view, the scrolling stops.
    let inside = Point::new(from.x, 150.0);
    drag(&mut h, inside);
    h.animate_ms(50);
    let _ = h.redraw();
    let still = h.root_widget().visible_paragraphs()[0].0;
    for _ in 0..5 {
        h.animate_ms(50);
    }
    let _ = h.redraw();
    assert_eq!(h.root_widget().visible_paragraphs()[0].0, still);
    release(&mut h, inside);
    let reported = moves(&mut h);
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].1.map(|r| r.start), Some(CharPos(0)));
}

#[test]
fn a_right_click_still_opens_the_menu_on_the_selection() {
    let mut h = harness(TEXT, 600);
    let a = point_of(&mut h, CharPos(6));
    let b = point_of(&mut h, CharPos(24));
    let inside = point_of(&mut h, CharPos(11));
    press(&mut h, a, 1, false);
    drag(&mut h, b);
    release(&mut h, b);
    let _ = moves(&mut h);
    h.process_pointer_event(PointerEvent::Down(PointerButtonEvent {
        pointer: PRIMARY_MOUSE,
        button: Some(PointerButton::Secondary),
        state: state(inside, 1, false, false),
    }));
    let mut menu = false;
    while let Some((a, _)) = h.pop_action::<DocAction>() {
        match a {
            DocAction::ContextMenu { .. } => menu = true,
            DocAction::CaretMoved { .. } => panic!("the selection is kept for the menu"),
            _ => {}
        }
    }
    assert!(menu);
    assert_eq!(h.root_widget().state().anchor, Some(CharPos(6)));
}
