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
    harness_on(doc, focus, textweaver_app::keymap::Platform::current())
}

/// The view following `platform`'s caret keys.
fn harness_on(
    doc: &Document,
    focus: CharPos,
    platform: textweaver_app::keymap::Platform,
) -> (TestHarness<DocumentView>, DocWindow) {
    let p = Palette::galaxy();
    let view = DocumentView::new(p.clone(), DocFont::default(), Rc::new(Cell::new(0)))
        .with_platform(platform);
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
        ..DocModel::default()
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
    let (h, _) = harness_on(
        &doc,
        CharPos::ZERO,
        textweaver_app::keymap::Platform::Windows,
    );
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
    let DocAction::CaretMoved { caret, .. } = action else {
        panic!("not a caret move: {action:?}");
    };
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
        ..DocModel::default()
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
        ..DocModel::default()
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
        ..DocModel::default()
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    let node = h.access_node(h.root_id()).unwrap();
    assert_eq!(node.role(), Role::MultilineTextInput);
    assert!(node.is_read_only());
    assert!(node.supports_text_ranges());
    assert_eq!(node.document_range().text(), "Some text.");
}

/// Caret keys carry what textweaver's own voice says in the self-voicing
/// mode, as the terminal's caret keys do: the char, the word, the line as
/// drawn, the end of a line, and a selection growing or shrinking. Each
/// platform's keys are pressed, built from its table (`keys::caret_keys`),
/// on every system: the macOS keys are checked on Windows too.
#[test]
fn caret_keys_carry_what_the_self_voicing_mode_says() {
    use textweaver_app::keymap::{KeyChord, Modifiers, Platform};
    use textweaver_xilem::document::CaretEcho;
    use textweaver_xilem::keys::{self, CaretMove, CaretStep};
    for platform in Platform::ALL {
        let doc = Document::from_plain_text("ab cd\nef");
        let (mut h, _) = harness_on(&doc, CharPos::ZERO, platform);
        h.focus_on(Some(h.root_id()));
        let key = |step, forward, shift: bool| -> TextEvent {
            let (chord, _) = keys::caret_keys(platform)
                .into_iter()
                .find(|(_, m)| *m == CaretMove { step, forward })
                .expect("every move has a key");
            let chord = if shift {
                KeyChord::new(chord.key, chord.mods | Modifiers::SHIFT)
            } else {
                chord
            };
            TextEvent::Keyboard(keys::press(&chord, platform))
        };
        let echo = |h: &mut TestHarness<DocumentView>, e: TextEvent| {
            h.process_text_event(e);
            match h.pop_action::<DocAction>() {
                Some((DocAction::CaretMoved { echo, .. }, _)) => echo,
                other => panic!("{platform:?}: not a caret move: {other:?}"),
            }
        };
        assert_eq!(
            echo(&mut h, key(CaretStep::Char, true, false)),
            Some(CaretEcho::Char('b'))
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::Word, true, false)),
            Some(CaretEcho::Word("cd".into())),
            "{platform:?}"
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::LineEdge, true, false)),
            Some(CaretEcho::LineEnd),
            "{platform:?}"
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::Char, false, true)),
            Some(CaretEcho::Selection {
                text: "d".into(),
                selected: true
            })
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::Char, true, true)),
            Some(CaretEcho::Selection {
                text: "d".into(),
                selected: false
            })
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::Line, true, false)),
            Some(CaretEcho::Line("ef".into()))
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::DocumentEdge, true, false)),
            Some(CaretEcho::Line("ef".into())),
            "{platform:?}"
        );
        assert_eq!(
            echo(&mut h, key(CaretStep::DocumentEdge, false, false)),
            Some(CaretEcho::Line("ab cd".into())),
            "{platform:?}"
        );
    }
}

/// Down onto an empty line says the end of a line. An empty paragraph is
/// laid out with a stand-in char, so its line reaches past its text; the
/// echo once sliced the text by that line and panicked.
#[test]
fn a_caret_key_onto_an_empty_line_says_line_end() {
    use textweaver_app::keymap::{Key as TwKey, KeyChord, Platform};
    use textweaver_xilem::document::CaretEcho;
    let doc = Document::from_plain_text("Title\n\nText after.");
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    h.focus_on(Some(h.root_id()));
    let down = KeyChord::plain(TwKey::Down);
    h.process_text_event(TextEvent::Keyboard(textweaver_xilem::keys::press(
        &down,
        Platform::current(),
    )));
    match h.pop_action::<DocAction>() {
        Some((DocAction::CaretMoved { echo, .. }, _)) => {
            assert_eq!(echo, Some(CaretEcho::LineEnd));
        }
        other => panic!("not a caret move: {other:?}"),
    }
}

/// The window taking the focus with the document focused is told to the
/// driver, which says the names in textweaver's own voice.
#[test]
fn the_window_taking_focus_is_reported() {
    let doc = Document::from_plain_text("Text.");
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    h.focus_on(Some(h.root_id()));
    h.process_text_event(TextEvent::WindowFocusChange(true));
    assert_eq!(
        h.pop_action::<DocAction>().map(|(a, _)| a),
        Some(DocAction::WindowFocused)
    );
    h.process_text_event(TextEvent::WindowFocusChange(false));
    assert!(h.pop_action::<DocAction>().is_none());
}

/// On macOS the document is a read-only text area (AXTextArea), not a
/// Document, which VoiceOver sees as an AXGroup; elsewhere a Document.
#[test]
fn on_macos_the_document_is_a_read_only_text_area() {
    use textweaver_app::keymap::Platform;
    let doc = Document::from_plain_text("Some text.");
    let (h, _) = harness_on(&doc, CharPos::ZERO, Platform::MacOs);
    let node = h.access_node(h.root_id()).expect("the document");
    assert_eq!(node.role(), Role::MultilineTextInput);
    assert!(node.is_read_only());
    assert_eq!(node.document_range().text(), "Some text.");
    for p in [Platform::Windows, Platform::Linux] {
        let (h, _) = harness_on(&doc, CharPos::ZERO, p);
        let node = h.access_node(h.root_id()).expect("the document");
        assert_eq!(node.role(), Role::Document, "{p:?}");
    }
}

/// One very long line (W8b-g): Parley keeps every cluster's place past
/// 64 KB of text in one style. Its cluster offset was a `u16`, so a line
/// started inside a character there, and a cursor past it found no
/// cluster at all (the frame-time probe's panic, and a misplaced word
/// band).
#[test]
fn parley_keeps_its_place_past_64_kb_of_one_style() {
    use masonry::core::{BrushIndex, StyleProperty};
    use masonry::parley::style::FontFamily;
    use masonry::parley::{Affinity, Cursor, FontContext, LayoutContext};
    let text = "Caf\u{e9} cr\u{e8}me na\u{ef}ve, \u{e9}t\u{e9} words read along. ".repeat(3_000);
    assert!(text.len() > 100_000);
    let mut fcx = FontContext::new();
    for b in textweaver_xilem::fonts::bundled_blobs() {
        fcx.collection.register_fonts(b, None);
    }
    let mut lcx = LayoutContext::<BrushIndex>::new();
    let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(FontFamily::Source(
        textweaver_xilem::fonts::DEFAULT_STACK.into(),
    )));
    builder.push_default(StyleProperty::FontSize(18.0));
    let mut layout = builder.build(&text);
    layout.break_all_lines(Some(600.0));
    let mut end = 0;
    for line in layout.lines() {
        let r = line.text_range();
        assert_eq!(r.start, end, "lines follow each other");
        assert!(text.is_char_boundary(r.start) && text.is_char_boundary(r.end));
        end = r.end;
    }
    assert_eq!(end, text.len());
    for b in [70_000, 100_003, text.len() - 40] {
        let b = (b..).find(|&b| text.is_char_boundary(b)).unwrap();
        let c = Cursor::from_byte_index(&layout, b, Affinity::Downstream);
        assert_eq!(c.index(), b, "the cursor at byte {b}");
    }
}

/// One very long line: a highlight move rebuilds and sends only the runs
/// of the lines the word left and reached, the rest keep their nodes, the
/// tree's text never changes, and the word band is drawn on screen where
/// the word is (W8b-g).
#[test]
fn a_move_on_one_long_line_sends_only_its_lines() {
    let text = "Caf\u{e9} cr\u{e8}me na\u{ef}ve, \u{e9}t\u{e9} words read along. ".repeat(6_000);
    let doc = Document::from_plain_text(&text);
    let start = CharPos(80_000);
    let (mut h, w) = harness_with(&doc, start);
    let window_text: String = doc.slice(w.range()).to_string();
    assert!(w.range().len() > 60_000, "one long paragraph in the window");
    let mut pos = start.0;
    let mut before = runs(&h);
    for k in 0..12 {
        // The next word.
        let off = pos - w.range().start.0;
        let rest: String = window_text.chars().skip(off).collect();
        let len = rest
            .find(' ')
            .map_or(3, |b| rest[..b].chars().count())
            .max(1);
        let r = CharRange::new(pos, pos + len);
        h.edit_root_widget(|mut d| {
            d.widget.last_nodes_sent = 0;
            DocumentView::set_state(
                &mut d,
                DocState {
                    caret: r.start,
                    anchor: None,
                    spoken: Some(r),
                    sentence: Some(CharRange::new(r.start.0, r.end.0 + 30)),
                    reading: true,
                },
            )
        });
        let _ = h.redraw();
        let sent = h.root_widget().last_nodes_sent;
        assert!(sent <= 8, "move {k}: {sent} nodes sent");
        let after = runs(&h);
        assert_eq!(
            after.iter().map(|r| r.1.as_str()).collect::<String>(),
            window_text
        );
        // Nearly every run keeps its node and text.
        let kept = after.iter().filter(|r| before.contains(r)).count();
        assert!(
            after.len() - kept <= 8,
            "move {k}: {} runs changed",
            after.len() - kept
        );
        before = after;
        let node = h.access_node(h.root_id()).unwrap();
        let marked: Vec<String> = node
            .children()
            .filter(|c| c.data().background_color().is_some())
            .map(|c| c.data().value().unwrap_or_default().to_owned())
            .collect();
        let word: String = window_text.chars().skip(off).take(len).collect();
        assert_eq!(marked, vec![word], "move {k}");
        let focus = node.text_selection_focus().unwrap();
        assert_eq!(focus.to_global_usv_index(), off, "move {k}");
        // The word's band is on screen.
        let bands: Vec<_> = h
            .root_widget()
            .painted()
            .iter()
            .filter_map(|s| match s {
                textweaver_xilem::document::PaintStep::WordBand(r) => Some(*r),
                _ => None,
            })
            .collect();
        assert!(!bands.is_empty(), "move {k}: no word band");
        for b in bands {
            assert!(
                b.y0 >= 0.0 && b.y1 <= 600.0 && b.width() > 1.0,
                "move {k}: {b:?}"
            );
        }
        pos += len + 1;
    }
}

/// A view of `text` in a window `width` logical pixels wide, at `size`
/// pixels and `measure` characters.
fn measured(text: &str, width: u32, size: f32, measure: u16) -> TestHarness<DocumentView> {
    let p = Palette::galaxy();
    let font = DocFont {
        size,
        ..DocFont::default()
    };
    let view = DocumentView::new(p.clone(), font, Rc::new(Cell::new(0)));
    let mut params = TestHarnessParams::default();
    params.window_size = (width, 700).into();
    let mut h = TestHarness::create_with(
        theme::default_properties(&p),
        NewWidget::new(view).with_tag(DOC),
        params,
    );
    for b in textweaver_xilem::fonts::bundled_blobs() {
        h.register_fonts(b);
    }
    let doc = Document::from_plain_text(text);
    let w = DocWindow::with_budget(&doc, CharPos::ZERO, WINDOW_UNITS);
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, w.range()),
        spans: window::window_spans(&doc, w.range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
        ..DocModel::default()
    };
    h.edit_root_widget(|mut d| {
        DocumentView::set_model(&mut d, model);
        let aids = textweaver_xilem::document::DocAids {
            measure,
            ..Default::default()
        };
        DocumentView::set_aids(&mut d, aids);
    });
    let _ = h.redraw();
    h
}

/// Plain English prose, long enough to wrap many times.
const PROSE: &str = "Reading on a screen is easier when a line is neither too long nor too \
short. The eye has to find the start of the next line, and a long line makes that hard, \
while a short one breaks every phrase. Typographers have long said that a line of about \
sixty six characters is a comfortable measure for continuous reading, and this paragraph \
is here to be wrapped at that measure in the window.";

/// `[display] measure`: at 14 and 24 points (18.67 and 32 px) the first
/// line holds about the measure's characters, the column grows with the
/// font, and 0 fills the window (design system C4).
#[test]
fn the_measure_sets_the_line_length_at_any_size() {
    let mut columns = Vec::new();
    for size in [18.67_f32, 32.0] {
        let h = measured(PROSE, 1400, size, 66);
        let lines = h.root_widget().line_lengths(0).expect("laid out");
        let first = lines[0];
        assert!(
            (55..=72).contains(&first),
            "at {size} px the first line holds {first} chars: {lines:?}"
        );
        columns.push(h.root_widget().column_width());
    }
    assert!(columns[1] > columns[0] * 1.5, "{columns:?}");
    // 0 fills the window, less its insets.
    let h = measured(PROSE, 1400, 18.67, 0);
    assert!(h.root_widget().column_width() > 1300.0);
}

/// A 25-character measure wraps near 25 characters.
#[test]
fn a_twenty_five_character_column_wraps_near_twenty_five() {
    let h = measured(PROSE, 1000, 18.67, 25);
    let lines = h.root_widget().line_lengths(0).expect("laid out");
    let body = &lines[..lines.len() - 1];
    assert!(body.len() > 10, "{lines:?}");
    for &n in body {
        // A line of narrow letters holds a few more; its trailing space counts.
        assert!((14..=30).contains(&n), "a line of {n} chars: {lines:?}");
    }
    let avg = body.iter().sum::<usize>() as f64 / body.len() as f64;
    assert!((19.0..=26.0).contains(&avg), "average {avg}: {lines:?}");
}

/// A list three deep: a bullet item, a numbered item inside it, and a
/// bullet inside that, then a paragraph.
fn nested_list() -> Document {
    use textweaver_app::core::MarkerKind;
    use textweaver_app::text::{DocumentMeta, Marker};
    let text = "Fruit\nApples\nGreen ones\nTart\nAfter the list.";
    let item =
        |a, b, level| Marker::new(MarkerKind::ListItem, CharRange::new(a, b)).with_level(level);
    let markers = vec![
        item(6, 28, 1),
        item(13, 28, 2).with_label("2."),
        item(24, 28, 3),
    ];
    Document::new(DocumentMeta::default(), text.into(), markers)
}

#[test]
fn list_items_draw_their_bullets_numbers_and_nesting() {
    use textweaver_xilem::document::{DocMark, PaintStep};
    let doc = nested_list();
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    // A highlight on each item's first letter shows where its text starts.
    let firsts = [6, 13, 24, 29];
    let marks = firsts
        .iter()
        .map(|&a| (CharRange::new(a, a + 1), DocMark::Highlight))
        .collect();
    h.edit_root_widget(|mut d| DocumentView::set_marks(&mut d, marks));
    let _ = h.redraw();
    let steps = h.root_widget().painted().to_vec();
    let markers: Vec<(u8, masonry::kurbo::Rect)> = steps
        .iter()
        .filter_map(|s| match s {
            PaintStep::ListMarker(level, r) => Some((*level, *r)),
            _ => None,
        })
        .collect();
    let starts: Vec<f64> = steps
        .iter()
        .filter_map(|s| match s {
            PaintStep::MarkBand(DocMark::Highlight, r) => Some(r.x0),
            _ => None,
        })
        .collect();
    assert_eq!(markers.iter().map(|m| m.0).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(starts.len(), 4);
    // Each level is indented further, and its marker hangs left of its
    // text; the paragraph after the list is back at the column's edge.
    assert!(starts[0] < starts[1] && starts[1] < starts[2], "{starts:?}");
    assert!(starts[3] < starts[0], "{starts:?}");
    for (k, (_, r)) in markers.iter().enumerate() {
        assert!(r.x1 < starts[k], "marker {k} hangs left of its text");
        assert!(r.x0 > starts[3] - 1.0, "marker {k} stays in the column");
    }
    // Drawn only: the screen reader's text is the document's, unchanged.
    assert_eq!(doc_text(&h), doc.text().to_string());
    // Bullets are shapes, not glyphs the font may lack: a disc at depth
    // 1 and a square at depth 3, each inside its marker's box.
    use textweaver_xilem::runs::Bullet;
    let bullets: Vec<(Bullet, masonry::kurbo::Rect)> = steps
        .iter()
        .filter_map(|s| match s {
            PaintStep::Bullet(b, r) => Some((*b, *r)),
            _ => None,
        })
        .collect();
    assert_eq!(
        bullets.iter().map(|b| b.0).collect::<Vec<_>>(),
        [Bullet::Disc, Bullet::Square]
    );
    for ((_, dot), (_, r)) in bullets.iter().zip([markers[0], markers[2]]) {
        assert!(dot.width() > 2.0 && dot.x0 >= r.x0 - 1.0 && dot.x1 <= r.x1 + 1.0);
        assert!(dot.y0 >= r.y0 && dot.y1 <= r.y1, "{dot:?} in {r:?}");
    }
}

#[test]
fn edit_mode_draws_no_list_markers() {
    use textweaver_xilem::document::PaintStep;
    let doc = nested_list();
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    let has_markers = |h: &TestHarness<DocumentView>| {
        h.root_widget()
            .painted()
            .iter()
            .any(|s| matches!(s, PaintStep::ListMarker(..) | PaintStep::Bullet(..)))
    };
    assert!(has_markers(&h));
    // The source's own dashes and numbers show while editing.
    h.edit_root_widget(|mut d| DocumentView::set_editing(&mut d, true));
    let _ = h.redraw();
    assert!(!has_markers(&h));
    h.edit_root_widget(|mut d| DocumentView::set_editing(&mut d, false));
    let _ = h.redraw();
    assert!(has_markers(&h));
    assert_eq!(doc_text(&h), doc.text().to_string());
}

#[test]
fn the_empty_window_draws_its_hint_and_describes_it() {
    use textweaver_xilem::document::PaintStep;
    let p = Palette::galaxy();
    let hint = "No document is open. Press Ctrl+O to open one.";
    let view = DocumentView::new(p.clone(), DocFont::default(), Rc::new(Cell::new(0)))
        .with_empty_hint(hint);
    let mut h = TestHarness::create(theme::default_properties(&p), NewWidget::new(view));
    let _ = h.redraw();
    assert_eq!(h.root_widget().hint_shown(), Some(hint));
    assert!(
        h.root_widget()
            .painted()
            .iter()
            .any(|s| matches!(s, PaintStep::Hint(r) if r.width() > 0.0))
    );
    let node = h.access_node(h.root_id()).unwrap();
    assert_eq!(node.description().as_deref(), Some(hint));
    // A document opens: the hint goes.
    let doc = Document::from_plain_text("Some text.");
    let model = DocModel {
        paragraphs: window::window_paragraphs(&doc, doc.full_range()),
        doc_len: doc.len_chars(),
        title: "Test".into(),
        ..DocModel::default()
    };
    h.edit_root_widget(|mut d| DocumentView::set_model(&mut d, model));
    let _ = h.redraw();
    assert_eq!(h.root_widget().hint_shown(), None);
    assert!(
        !h.root_widget()
            .painted()
            .iter()
            .any(|s| matches!(s, PaintStep::Hint(_)))
    );
}

#[test]
fn edit_mode_draws_the_word_editing_and_reading_mode_marks_where_play_starts() {
    use textweaver_xilem::document::PaintStep;
    let doc = Document::from_plain_text("One two three.\nFour five.");
    let (mut h, _) = harness_with(&doc, CharPos::ZERO);
    h.edit_root_widget(|mut d| DocumentView::set_editing_word(&mut d, "Editing"));
    let _ = h.redraw();
    let steps = h.root_widget().painted().to_vec();
    // Reading mode, not reading: the "reading from here" triangle, no badge.
    assert!(steps.iter().any(|s| matches!(s, PaintStep::ReadingFrom(_))));
    assert!(!steps.iter().any(|s| matches!(s, PaintStep::Badge(_))));
    assert_eq!(h.root_widget().badge_shown(), None);
    h.edit_root_widget(|mut d| DocumentView::set_editing(&mut d, true));
    let _ = h.redraw();
    let steps = h.root_widget().painted().to_vec();
    assert_eq!(h.root_widget().badge_shown(), Some("Editing"));
    assert!(
        steps
            .iter()
            .any(|s| matches!(s, PaintStep::Badge(r) if r.width() > 0.0))
    );
    assert!(!steps.iter().any(|s| matches!(s, PaintStep::ReadingFrom(_))));
    // The mode is in the node's role too, as before.
    let node = h.access_node(h.root_id()).unwrap();
    assert_eq!(node.role(), Role::MultilineTextInput);
}
