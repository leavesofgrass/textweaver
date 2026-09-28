//! The document view and announcer in Masonry's test harness, checked
//! through the AccessKit tree a screen reader would get (the consumer crate
//! the platform adapters use).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use masonry::accesskit::Role;
use masonry::core::keyboard::{Key, NamedKey};
use masonry::core::{NewWidget, TextEvent, WidgetTag};
use masonry_testing::{TestHarness, TestHarnessParams};
use textweaver_app::DocWindow;
use textweaver_app::a11y::Priority;
use textweaver_app::core::{CharPos, CharRange};
use textweaver_app::text::Document;
use textweaver_xilem::document::{DocAction, DocFont, DocModel, DocState, DocumentView};
use textweaver_xilem::theme::{self, Palette};
use textweaver_xilem::widgets::{Announcer, Message};
use textweaver_xilem::window::{self, WINDOW_UNITS};

const DOC: WidgetTag<DocumentView> = WidgetTag::named("doc");

fn harness_with(doc: &Document, focus: CharPos) -> (TestHarness<DocumentView>, DocWindow) {
    let p = Palette::galaxy();
    let view = DocumentView::new(p.clone(), DocFont::default(), Rc::new(Cell::new(0)));
    let mut params = TestHarnessParams::default();
    params.window_size = (900, 600).into();
    let mut h = TestHarness::create_with(
        theme::default_properties(&p),
        NewWidget::new(view).with_tag(DOC),
        params,
    );
    for b in textweaver_xilem::fonts::bundled_blobs() {
        h.register_fonts(b);
    }
    let w = DocWindow::with_budget(doc, focus, WINDOW_UNITS);
    let model = DocModel {
        paragraphs: window::window_paragraphs(doc, w.range()),
        spans: window::window_spans(doc, w.range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    (h, w)
}

fn doc_text(h: &TestHarness<DocumentView>) -> String {
    let node = h.access_node(h.root_id()).expect("document node");
    node.document_range().text()
}

#[test]
fn the_document_is_one_readonly_node_with_its_text() {
    let doc = Document::from_plain_text("First paragraph here.\n\nSecond one, a little longer.");
    let (h, _) = harness_with(&doc, CharPos::ZERO);
    let node = h.access_node(h.root_id()).unwrap();
    assert_eq!(node.role(), Role::Document);
    assert!(node.is_read_only());
    assert_eq!(node.label().as_deref(), Some("Document"));
    assert_eq!(node.description().as_deref(), Some("Test"));
    assert!(node.supports_text_ranges());
    assert_eq!(doc_text(&h), doc.text().to_string());
    // Every child is a text run of at most 255 characters.
    for c in node.children() {
        assert_eq!(c.role(), Role::TextRun);
        assert!(c.data().character_lengths().len() <= 255);
    }
}

#[test]
fn the_caret_is_the_selection_and_follows_the_state() {
    let doc = Document::from_plain_text("One two three.\nFour five.");
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    h.edit_root_widget(|mut d| {
        DocumentView::set_state(
            &mut d,
            DocState {
                caret: CharPos(8),
                ..DocState::default()
            },
        )
    });
    let _ = h.redraw();
    let node = h.access_node(h.root_id()).unwrap();
    let focus = node.text_selection_focus().expect("caret");
    assert_eq!(focus.to_global_usv_index(), 8);
}

#[test]
fn the_spoken_word_has_a_background_colour_and_carries_the_caret() {
    let doc = Document::from_plain_text("One two three.\nFour five.");
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    let state = DocState {
        caret: CharPos(4),
        anchor: None,
        spoken: Some(CharRange::new(4, 7)),
        sentence: Some(CharRange::new(0, 14)),
        reading: true,
    };
    h.edit_root_widget(|mut d| DocumentView::set_state(&mut d, state));
    let _ = h.redraw();
    let node = h.access_node(h.root_id()).unwrap();
    let marked: Vec<String> = node
        .children()
        .filter(|c| c.data().background_color().is_some())
        .map(|c| c.data().value().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(marked, vec!["two".to_owned()]);
    let focus = node.text_selection_focus().unwrap();
    assert_eq!(focus.to_global_usv_index(), 4);
    // Stopping clears it.
    h.edit_root_widget(|mut d| {
        DocumentView::set_state(
            &mut d,
            DocState {
                caret: CharPos(4),
                ..DocState::default()
            },
        )
    });
    let _ = h.redraw();
    let node = h.access_node(h.root_id()).unwrap();
    assert!(
        node.children()
            .all(|c| c.data().background_color().is_none())
    );
    assert_eq!(doc_text(&h), doc.text().to_string());
}

#[test]
fn arrow_keys_move_the_caret_and_tell_the_driver() {
    let doc = Document::from_plain_text("ab cd\nef");
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    h.focus_on(Some(h.root_id()));
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowRight)));
    let (action, _) = h.pop_action::<DocAction>().expect("caret moved");
    assert!(matches!(
        action,
        DocAction::CaretMoved {
            caret: CharPos(1),
            ..
        }
    ));
    h.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowDown)));
    let (action, _) = h.pop_action::<DocAction>().expect("line down");
    let DocAction::CaretMoved { caret, .. } = action;
    assert_eq!(
        caret,
        CharPos(7),
        "down from column 1 lands on column 1 of the next line"
    );
    // A key the view does not handle is left for the keymap.
    h.process_text_event(TextEvent::key_down(Key::Character("p".into())));
    assert!(h.pop_action::<DocAction>().is_none());
}

#[test]
fn announcements_are_new_live_nodes_even_when_repeated() {
    let p = Palette::galaxy();
    let full = Rc::new(Cell::new(0));
    let tag: WidgetTag<Announcer> = WidgetTag::named("ann");
    let mut h = TestHarness::create(
        theme::default_properties(&p),
        NewWidget::new(Announcer::new(full)).with_tag(tag),
    );
    let say = |h: &mut TestHarness<Announcer>, text: &str| {
        h.edit_root_widget(|mut a| {
            Announcer::say(
                &mut a,
                [Message {
                    text: text.into(),
                    priority: Priority::Polite,
                }],
            )
        });
        let _ = h.redraw();
        let node = h.access_node(h.root_id()).unwrap();
        let last = node.children().last().expect("a message node");
        assert_eq!(last.value().as_deref(), Some(text));
        assert_ne!(last.live(), masonry::accesskit::Live::Off);
        last.id()
    };
    let a = say(&mut h, "Paused.");
    let b = say(&mut h, "Paused.");
    assert_ne!(a, b, "a repeated message is a new node");
}

/// Large documents: the time to build the view for a 10-million-character
/// document's window and to move the highlight. Printed for the ADR's
/// measurements; the limits here are loose (debug builds are slow).
#[test]
fn large_documents_open_and_highlight_quickly() {
    let para = "The quick brown fox jumps over the lazy dog, and the reader keeps going. ";
    let mut text = String::with_capacity(10_100_000);
    let mut i = 0;
    while text.len() < 10_000_000 {
        text.push_str(para);
        i += 1;
        if i % 6 == 0 {
            text.push('\n');
        }
    }
    let doc = Document::from_plain_text(&text);
    let focus = CharPos(doc.len_chars() / 2);
    let started = Instant::now();
    let (mut h, w) = harness_with(&doc, focus);
    let open_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut moves = Vec::new();
    let mut pos = w.range().start.0 + 1000;
    for _ in 0..20 {
        let r = CharRange::new(pos, pos + 3);
        let t = Instant::now();
        h.edit_root_widget(|mut d| {
            DocumentView::set_state(
                &mut d,
                DocState {
                    caret: r.start,
                    anchor: None,
                    spoken: Some(r),
                    sentence: None,
                    reading: true,
                },
            )
        });
        let _ = h.redraw();
        moves.push(t.elapsed().as_secs_f64() * 1000.0);
        pos += 4;
    }
    moves.sort_by(f64::total_cmp);
    let median = moves[moves.len() / 2];
    let worst = moves[moves.len() - 1];
    println!(
        "10M chars: window {} chars; open (model, layout, runs, tree) {open_ms:.1} ms; highlight median {median:.2} ms, worst {worst:.2} ms",
        w.range().len()
    );
    assert!(open_ms < 20_000.0, "{open_ms} ms");
    assert!(median < 1_000.0, "{median} ms");
}

/// The runs' ids and text, in the document node's order.
fn runs(h: &TestHarness<DocumentView>) -> Vec<(String, String)> {
    let node = h.access_node(h.root_id()).unwrap();
    node.children()
        .map(|c| {
            (
                format!("{:?}", c.id()),
                c.data().value().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// Reading past the window's edge: the window slides, and the screen
/// reader keeps its place. The runs that stay keep their nodes and text,
/// the caret (the collapsed selection on the spoken word) is sent again on
/// a node that is in the tree, and it points at the spoken word.
#[test]
fn a_window_slide_while_reading_keeps_the_screen_readers_place() {
    let para = "Reading on and on, sentence after sentence, past the edge. ";
    let mut text = String::new();
    let mut i = 0;
    while text.len() < WINDOW_UNITS * 3 {
        text.push_str(para);
        i += 1;
        if i % 4 == 0 {
            text.push('\n');
        }
    }
    let doc = Document::from_plain_text(&text);
    let (mut h, mut w) = harness_with(&doc, CharPos::ZERO);
    // The text is ASCII, so bytes and chars agree.
    let word_at = |pos: usize| {
        let start = text[pos..]
            .find(|c: char| c.is_ascii_alphabetic())
            .map_or(pos, |b| pos + b);
        CharRange::new(start, start + 4)
    };
    let reading = |r: CharRange| DocState {
        caret: r.start,
        anchor: None,
        spoken: Some(r),
        sentence: None,
        reading: true,
    };
    // Reading near the window's end, before it slides.
    let before_edge = word_at(w.range().end.0 - WINDOW_UNITS / 6);
    assert_eq!(
        w.follow(&doc, before_edge.start),
        textweaver_app::WindowChange::Unchanged
    );
    h.edit_root_widget(|mut d| DocumentView::set_state(&mut d, reading(before_edge)));
    let _ = h.redraw();
    let before = runs(&h);
    // The next word is past the slide point: the window slides forward.
    let old = w.range();
    let spoken = word_at(w.range().end.0 - WINDOW_UNITS / 16);
    let change = w.follow(&doc, spoken.start);
    assert!(
        matches!(change, textweaver_app::WindowChange::Forward { .. }),
        "{change:?}"
    );
    assert!(w.range().start > old.start);
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, w.range()),
        spans: window::window_spans(&doc, w.range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
    };
    h.edit_root_widget(|mut d| {
        DocumentView::slide_model(&mut d, model);
        DocumentView::set_state(&mut d, reading(spoken));
    });
    let _ = h.redraw();
    let after = runs(&h);
    // The text is the new window's.
    assert_eq!(doc_text(&h), doc.slice(w.range()));
    // The runs that stayed kept their nodes and their text: all but those
    // around the spoken word, before and after, and the paragraphs now on
    // screen, which split at their visual lines once laid out, as they do
    // when scrolled to. Before the slide kept ids, none stayed.
    let stayed: Vec<_> = before
        .iter()
        .filter(|(id, _)| after.iter().any(|(a, _)| a == id))
        .collect();
    let kept = stayed.iter().filter(|r| after.contains(r)).count();
    assert!(stayed.len() > 100, "{} runs stayed", stayed.len());
    assert!(
        kept * 100 >= stayed.len() * 85,
        "{kept} of {} runs that stayed kept their text",
        stayed.len()
    );
    // No node that left the window is still there.
    let ids: std::collections::HashSet<_> = after.iter().map(|(id, _)| id).collect();
    assert_eq!(ids.len(), after.len(), "every run has its own node");
    // The caret is on the spoken word, on a node in the tree.
    let node = h.access_node(h.root_id()).unwrap();
    let focus = node
        .text_selection_focus()
        .expect("the caret is sent again");
    assert_eq!(
        w.range().start.0 + focus.to_global_usv_index(),
        spoken.start.0,
        "the caret is on the spoken word"
    );
    // A jump replaces every run.
    let far = word_at(doc.len_chars() - 1000);
    assert_eq!(
        w.follow(&doc, far.start),
        textweaver_app::WindowChange::Recentred
    );
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, w.range()),
        spans: window::window_spans(&doc, w.range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
    };
    h.edit_root_widget(|mut d| {
        DocumentView::set_model(&mut d, model);
        DocumentView::set_state(&mut d, reading(far));
    });
    let _ = h.redraw();
    let jumped = runs(&h);
    assert!(jumped.iter().all(|r| !after.contains(r)));
    let node = h.access_node(h.root_id()).unwrap();
    let focus = node.text_selection_focus().unwrap();
    assert_eq!(w.range().start.0 + focus.to_global_usv_index(), far.start.0);
}

#[test]
fn the_edit_role_experiment_is_a_readonly_multiline_edit() {
    let p = Palette::galaxy();
    let view = DocumentView::new(p.clone(), DocFont::default(), Rc::new(Cell::new(0)))
        .with_edit_role(true);
    let mut h = TestHarness::create(theme::default_properties(&p), NewWidget::new(view));
    let doc = Document::from_plain_text("Some text.");
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, doc.full_range()),
        spans: Vec::new(),
        doc_len: doc.len_chars(),
        title: String::new(),
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    let node = h.access_node(h.root_id()).unwrap();
    assert_eq!(node.role(), Role::MultilineTextInput);
    assert!(node.is_read_only());
    assert!(node.supports_text_ranges());
    assert_eq!(node.document_range().text(), "Some text.");
}
