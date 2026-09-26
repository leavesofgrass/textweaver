//! Times each reading aid on a generated 1 MB document.
//!
//! ```text
//! cargo run -p textweaver-aids --release --example aids_bench
//! ```

use std::time::{Duration, Instant};

use textweaver_aids::rsvp::{Area, TuiBoxOptions, tui_box};
use textweaver_aids::{
    BionicOptions, DifficultOptions, FontSettings, FrequencyList, Rsvp, RsvpSettings,
    RulerSettings, ScowlList, SyllableOptions, TextSpacing, ViewRow, WordTrack, bionic_range,
    difficult_range, reading_level, ruler_rows, split_range,
};
use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;

const WORDS: &[&str] = &[
    "the",
    "reader",
    "follows",
    "each",
    "word",
    "carefully,",
    "and",
    "extraordinary",
    "documents",
    "become",
    "accessible.",
    "Students",
    "with",
    "print",
    "disabilities",
    "deserve",
    "tools;",
    "readability",
    "matters",
    "(always)",
    "for",
    "everyone",
    "who",
    "reads",
    "text.",
    "Dr.",
    "Smith",
    "reviewed",
    "https://example.org/page",
    "in",
    "1990,",
    "noting",
    "understanding",
];

fn corpus(target: usize) -> String {
    let mut s = String::with_capacity(target + 256);
    let mut i = 0usize;
    while s.len() < target {
        for _ in 0..12 {
            s.push_str(WORDS[i % WORDS.len()]);
            s.push(' ');
            i = i.wrapping_mul(31).wrapping_add(7) % 1_000_003;
        }
        s.push_str("end.");
        if i.is_multiple_of(5) {
            s.push_str("\n\n");
        } else {
            s.push(' ');
        }
    }
    s
}

fn time<T>(label: &str, f: impl FnOnce() -> T) -> (T, Duration) {
    let t = Instant::now();
    let v = f();
    let d = t.elapsed();
    println!("{label:<48} {:>9.2} ms", d.as_secs_f64() * 1000.0);
    (v, d)
}

fn main() {
    let text = corpus(1 << 20);
    println!("document: {} bytes", text.len());
    let (doc, _) = time("load Document::from_plain_text", || {
        Document::from_plain_text(&text)
    });
    let all = doc.full_range();

    let (track, _) = time("RSVP WordTrack::from_document", || {
        WordTrack::from_document(&doc)
    });
    println!(
        "  words: {}, sentences: {}, paragraphs: {}",
        track.len(),
        track.sentence_count(),
        track.paragraph_count()
    );
    let n = track.len();
    let mut rsvp = Rsvp::new(track, RsvpSettings::default());
    time("RSVP play + tick through every word", || {
        let mut now = 0u64;
        rsvp.play(now);
        while let Some(d) = rsvp.deadline() {
            now = d;
            rsvp.tick(now);
        }
        assert_eq!(rsvp.index() + 1, n);
    });
    time("RSVP frame + TUI box for every word", || {
        let area = Area::new(0, 1, 80, 22);
        let opts = TuiBoxOptions::default();
        let mut cells = 0usize;
        for i in 0..n {
            rsvp.seek_word(i, 0);
            if let Some(f) = rsvp.frame()
                && let Some(b) = tui_box(&f, area, &opts)
            {
                cells += b.rows.len();
            }
        }
        cells
    });
    time("RSVP sentence steps through the document", || {
        rsvp.seek_word(0, 0);
        let mut steps = 0;
        while rsvp.next_sentence(0) == textweaver_aids::RsvpEvent::Moved {
            steps += 1;
        }
        steps
    });

    let (b, _) = time("bionic, whole document", || {
        bionic_range(&doc, all, &BionicOptions::star())
    });
    println!("  ranges: {}", b.len());
    time("bionic, one 4 KB screen", || {
        bionic_range(
            &doc,
            CharRange::new(500_000, 504_096),
            &BionicOptions::star(),
        )
    });

    time("reading level, whole document", || reading_level(&doc, all));

    let (s, _) = time("syllables, whole document", || {
        split_range(&doc, all, &SyllableOptions::default())
    });
    println!("  display bytes: {}", s.text.len());
    time("syllables, one 4 KB screen", || {
        split_range(
            &doc,
            CharRange::new(500_000, 504_096),
            &SyllableOptions::default(),
        )
    });

    let list = FrequencyList::from_pairs(
        WORDS
            .iter()
            .map(|w| (*w, 5.0))
            .chain([("extraordinary", 3.0), ("disabilities", 3.5)]),
    );
    time("difficult words, whole document", || {
        difficult_range(&doc, all, &list, &DifficultOptions::default())
    });

    // SCOWL, built in: unpacking and indexing happens once, on first use.
    let (scowl, _) = time("SCOWL list: unpack and index (first use)", || {
        ScowlList::builtin().expect("built-in SCOWL list")
    });
    println!("  words: {}", scowl.len());
    let (marked, _) = time("difficult words (SCOWL), whole document", || {
        difficult_range(&doc, all, scowl, &DifficultOptions::default())
    });
    println!("  marked: {}", marked.len());
    // A varied 1 MB text: words drawn from the whole list, most of them
    // common, a few made up (a fixed seed, so every run is the same).
    let vocab: Vec<(&str, u8)> = scowl.words().filter(|(w, _)| w.len() > 1).collect();
    let common: Vec<&str> = vocab
        .iter()
        .filter(|(_, l)| *l <= 40)
        .map(|(w, _)| *w)
        .collect();
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut varied = String::with_capacity(1_050_000);
    let mut n = 0usize;
    while varied.len() < 1_000_000 {
        let r = next();
        let w = match r % 100 {
            0..=84 => common[(r >> 8) as usize % common.len()],
            85..=97 => vocab[(r >> 8) as usize % vocab.len()].0,
            _ => "zorblaxity",
        };
        varied.push_str(w);
        n += 1;
        varied.push_str(if n.is_multiple_of(15) { ".\n\n" } else { " " });
    }
    let varied_doc = Document::from_plain_text(&varied);
    let varied_all = varied_doc.full_range();
    println!("  varied text: {} bytes, {} words", varied.len(), n);
    let (marked, _) = time("difficult words (SCOWL), varied 1 MB", || {
        difficult_range(&varied_doc, varied_all, scowl, &DifficultOptions::default())
    });
    println!("  marked: {}", marked.len());

    time("ruler marks, 40 rows", || {
        let rows: Vec<ViewRow> = (0..40)
            .map(|i| ViewRow {
                range: CharRange::new(i * 80, i * 80 + 79),
                line: i / 2,
            })
            .collect();
        ruler_rows(&rows, CharPos(1234), &RulerSettings::default())
    });
    time("font + spacing CSS", || {
        FontSettings::default().to_css(&TextSpacing::wcag(), "main")
    });
}
