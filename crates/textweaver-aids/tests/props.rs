//! Property tests: the aids never panic and never step out of bounds on
//! arbitrary text, and offset maps keep their invariants.

use proptest::prelude::*;
use textweaver_aids::html::wrap_ranges;
use textweaver_aids::rsvp::{Area, TuiBoxOptions, tui_box};
use textweaver_aids::{
    BionicOptions, DifficultOptions, FontFamily, FontSettings, FrequencyList, Rsvp, RsvpPosition,
    RsvpSettings, RulerMode, RulerScope, RulerSettings, SyllableOptions, TextSpacing, ViewRow,
    WordTrack, bionic_range, bionic_text, difficult_text, ruler_rows, split_range, split_text,
};
use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;

/// Text with words, numbers, punctuation, blank lines, accents, CJK, emoji,
/// combining marks, URLs, and backticks.
fn text() -> impl Strategy<Value = String> {
    let piece = prop_oneof![
        Just("word".to_owned()),
        Just("Reading".to_owned()),
        Just("readability".to_owned()),
        Just("extraordinarily".to_owned()),
        Just("don't".to_owned()),
        Just("well-known".to_owned()),
        Just("Dr.".to_owned()),
        Just("é".to_owned()),
        Just("e\u{301}".to_owned()),
        Just("日本語".to_owned()),
        Just("👍🏽".to_owned()),
        Just("1990s".to_owned()),
        Just("3.14".to_owned()),
        Just("https://example.org/a".to_owned()),
        Just("`code`".to_owned()),
        Just("snake_case".to_owned()),
        Just("—".to_owned()),
        Just(",".to_owned()),
        Just(".".to_owned()),
        Just("!".to_owned()),
        Just("(".to_owned()),
        Just(")".to_owned()),
        Just(" ".to_owned()),
        Just("  ".to_owned()),
        Just("\n".to_owned()),
        Just("\n\n".to_owned()),
        "[a-zA-Z]{1,12}",
    ];
    prop::collection::vec(piece, 0..60).prop_map(|v| v.concat())
}

#[derive(Clone, Debug)]
enum Op {
    Play,
    Pause,
    Toggle,
    Tick(u64),
    NextWord,
    PrevWord,
    NextSentence,
    PrevSentence,
    NextParagraph,
    PrevParagraph,
    SeekWord(usize),
    SeekPos(usize),
    Follow(usize),
    Wpm(u32),
    Lead(i32),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::Play),
        Just(Op::Pause),
        Just(Op::Toggle),
        (0u64..5_000).prop_map(Op::Tick),
        Just(Op::NextWord),
        Just(Op::PrevWord),
        Just(Op::NextSentence),
        Just(Op::PrevSentence),
        Just(Op::NextParagraph),
        Just(Op::PrevParagraph),
        (0usize..200).prop_map(Op::SeekWord),
        (0usize..2_000).prop_map(Op::SeekPos),
        (0usize..2_000).prop_map(Op::Follow),
        (0u32..5_000).prop_map(Op::Wpm),
        (-5i32..5).prop_map(Op::Lead),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn rsvp_stepping_stays_in_bounds(t in text(), ops in prop::collection::vec(op(), 0..80)) {
        let doc = Document::from_plain_text(&t);
        let len = doc.len_chars();
        let mut r = Rsvp::new(WordTrack::from_document(&doc), RsvpSettings::default());
        let mut now = 0u64;
        for op in ops {
            match op {
                Op::Play => { r.play(now); }
                Op::Pause => { r.pause(); }
                Op::Toggle => { r.toggle(now); }
                Op::Tick(dt) => {
                    now += dt;
                    let before = r.index();
                    let t = r.tick(now);
                    // Never more than one word per tick.
                    prop_assert!(r.index() <= before + 1);
                    if let Some(d) = t.next_deadline { prop_assert!(d > now || !t.advanced); }
                }
                Op::NextWord => { r.next_word(now); }
                Op::PrevWord => { r.previous_word(now); }
                Op::NextSentence => { r.next_sentence(now); }
                Op::PrevSentence => { r.previous_sentence(now); }
                Op::NextParagraph => { r.next_paragraph(now); }
                Op::PrevParagraph => { r.previous_paragraph(now); }
                Op::SeekWord(i) => { r.seek_word(i, now); }
                Op::SeekPos(p) => { r.seek_pos(CharPos(p), now); }
                Op::Follow(p) => { r.follow(CharPos(p)); }
                Op::Wpm(w) => { r.set_wpm(w); }
                Op::Lead(l) => {
                    let mut s = r.settings().clone();
                    s.lead_words = l;
                    r.set_settings(s);
                }
            }
            if r.is_empty() {
                prop_assert!(r.frame().is_none());
                continue;
            }
            prop_assert!(r.index() < r.len());
            let f = r.frame().unwrap();
            prop_assert_eq!(format!("{}{}{}", f.before, f.pivot, f.after), f.text);
            prop_assert!(!f.pivot.is_empty());
            prop_assert!(f.display.end.0 <= len);
            prop_assert!(f.display.contains_range(f.word));
            prop_assert_eq!(doc.slice(f.display), f.text);
            prop_assert!(f.duration > 0);
            prop_assert!(r.percent() <= 100);
            let _ = r.status();
        }
    }

    #[test]
    fn track_words_are_ordered_and_disjoint(t in text()) {
        let doc = Document::from_plain_text(&t);
        let track = WordTrack::from_document(&doc);
        let mut prev_end = 0usize;
        for i in 0..track.len() {
            let d = track.display_range(i).unwrap();
            prop_assert!(d.start.0 >= prev_end, "display ranges overlap at {}", i);
            prev_end = d.end.0;
        }
        prop_assert!(prev_end <= doc.len_chars());
    }

    #[test]
    fn tui_box_stays_inside_the_area(
        t in text(),
        index in 0usize..50,
        x in 0u16..20, y in 0u16..10, w in 0u16..100, h in 0u16..30,
        width in 0u16..80,
        pos in 0usize..9,
        prev in any::<bool>(), next in any::<bool>(),
        avoid in proptest::option::of(0u16..40),
    ) {
        let mut r = Rsvp::new(WordTrack::from_text(&t), RsvpSettings::default());
        r.seek_word(index, 0);
        let Some(f) = r.frame() else { return Ok(()); };
        let area = Area::new(x, y, w, h);
        let opts = TuiBoxOptions {
            position: RsvpPosition::ALL[pos],
            width,
            show_previous: prev,
            show_next: next,
            avoid_row: avoid,
        };
        let Some(b) = tui_box(&f, area, &opts) else {
            prop_assert!(w < 3 || h == 0);
            return Ok(());
        };
        prop_assert!(b.area.x >= x && b.area.y >= y);
        prop_assert!(b.area.x + b.area.width <= x + w);
        prop_assert!(b.area.y + b.area.height <= y + h);
        for (row, segs) in &b.rows {
            prop_assert!(*row >= b.area.y && *row < b.area.y + b.area.height);
            for s in segs {
                let cells = unicode_width::UnicodeWidthStr::width(s.text.as_str()) as u16;
                prop_assert!(s.col > b.area.x, "inside the left padding");
                prop_assert!(s.col + cells < b.area.x + b.area.width, "inside the right padding");
            }
        }
    }

    #[test]
    fn bionic_ranges_are_in_bounds_and_ordered(t in text(), ratio in 0.0f32..1.5, base in 0usize..100) {
        let opts = BionicOptions::with_ratio(ratio);
        let n = t.chars().count();
        let rs = bionic_text(&t, CharPos(base), &opts);
        let mut prev = base;
        for r in &rs {
            prop_assert!(!r.is_empty());
            prop_assert!(r.start.0 >= prev);
            prop_assert!(r.end.0 <= base + n);
            prev = r.end.0;
        }
        let doc = Document::from_plain_text(&t);
        let dr = bionic_range(&doc, CharRange::new(0, usize::MAX), &opts);
        for r in &dr {
            prop_assert!(r.end.0 <= doc.len_chars());
        }
    }

    #[test]
    fn syllable_maps_keep_their_invariants(t in text(), base in 0usize..50, a in 0usize..400, b in 0usize..400) {
        let opts = SyllableOptions::default();
        let s = split_text(&t, CharPos(base), &opts);
        prop_assert!(s.map.check_invariants(&s.text).is_ok());
        prop_assert_eq!(s.text.replace(textweaver_aids::syllables::MIDDOT, ""), t.replace(textweaver_aids::syllables::MIDDOT, ""));
        if !t.is_empty() {
            let ext = s.map.source_extent().unwrap();
            prop_assert_eq!(ext, CharRange::new(base, base + t.chars().count()));
        }
        // Any source range maps to display bytes on char boundaries, and back
        // inside the range.
        let r = CharRange::new(base + a.min(t.chars().count()), base + b.min(t.chars().count()));
        if let Some(d) = s.display_range(r) {
            prop_assert!(d.start <= d.end && d.end <= s.text.len());
            prop_assert!(s.text.is_char_boundary(d.start) && s.text.is_char_boundary(d.end));
            if !r.is_empty() && d.start < d.end {
                let back = s.to_source(d.clone()).unwrap();
                prop_assert!(r.contains_range(back), "{:?} -> {:?} -> {:?}", r, d, back);
            }
        }
        // Every byte offset maps somewhere or nowhere, without panicking.
        for i in 0..=s.text.len() {
            let _ = s.to_source(i..(i + 1).min(s.text.len()));
        }
        let doc = Document::from_plain_text(&t);
        let dr = split_range(&doc, doc.full_range(), &opts);
        prop_assert!(dr.map.check_invariants(&dr.text).is_ok());
    }

    #[test]
    fn wrap_ranges_never_panics(t in "[a-z é日]{0,40}", rs in prop::collection::vec((0usize..60, 0usize..60), 0..10), base in 0usize..20) {
        let ranges: Vec<CharRange> = rs.into_iter().map(|(a, b)| CharRange::new(a, b)).collect();
        let out = wrap_ranges(&t, CharPos(base), &ranges, "[", "]");
        prop_assert_eq!(out.replace(['[', ']'], ""), t);
    }

    #[test]
    fn ruler_marks_every_row(n in 0usize..30, pos in 0usize..400, above in 0u8..5, below in 0u8..5, mask in any::<bool>(), wrapped in any::<bool>()) {
        let rows: Vec<ViewRow> = (0..n)
            .map(|i| ViewRow { range: CharRange::new(i * 10, i * 10 + 9), line: if wrapped { i / 2 } else { i } })
            .collect();
        for mode in [RulerMode::Off, RulerMode::CurrentLine, RulerMode::Ruler] {
            for scope in [RulerScope::Row, RulerScope::Line] {
                let s = RulerSettings { mode, scope, rows_above: above, rows_below: below, mask_outside: mask };
                prop_assert_eq!(ruler_rows(&rows, CharPos(pos), &s).len(), n);
            }
        }
    }

    #[test]
    fn clamped_settings_always_validate(
        lh in any::<f32>(), ps in any::<f32>(), ls in any::<f32>(), ws in any::<f32>(),
        size in any::<f32>(), weight in any::<u16>(), name in "[ a-zA-Z\"{};]{0,12}",
    ) {
        let s = TextSpacing { line_height: lh, paragraph_spacing: ps, letter_spacing: ls, word_spacing: ws };
        prop_assert!(s.clamped().validate().is_ok());
        let _ = s.to_css("body");
        let f = FontSettings { family: FontFamily::from(name), size_pt: size, weight, fetch_missing: true };
        prop_assert!(f.clamped().validate().is_ok());
        let css = f.to_css(&s, "body");
        prop_assert!(!css.contains(";;"));
    }

    #[test]
    fn difficult_ranges_are_in_bounds(t in text()) {
        let list = FrequencyList::from_pairs([("readability", 3.0), ("reading", 3.0), ("word", 3.0)]);
        let opts = DifficultOptions { mark_unknown: true, ..DifficultOptions::default() };
        let n = t.chars().count();
        for r in difficult_text(&t, CharPos(0), &list, &opts) {
            prop_assert!(!r.is_empty() && r.end.0 <= n);
        }
    }
}
