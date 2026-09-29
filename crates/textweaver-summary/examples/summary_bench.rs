//! Times a summary of generated documents, 40 runs each (ADR-0037).
//!
//! ```text
//! cargo run -p textweaver-summary --release --example summary_bench
//! ```
//!
//! Prints the median and the worst run for 100 KB, 1 MB, and 10 MB of
//! text, and how much of each was sampled. Loading the document is not
//! timed; the reader has it open already.

use std::time::{Duration, Instant};

use textweaver_summary::{Options, summarize_with};
use textweaver_text::Document;

const WORDS: &[&str] = &[
    "the",
    "reader",
    "speech",
    "highlight",
    "document",
    "students",
    "accessible",
    "voice",
    "sentence",
    "paragraph",
    "heading",
    "keyboard",
    "screen",
    "quickly",
    "reliable",
    "position",
    "braille",
    "display",
    "chapter",
    "summary",
    "library",
    "notes",
    "photosynthesis",
    "energy",
    "plants",
    "sunlight",
    "history",
    "river",
    "mountain",
    "equation",
    "theorem",
    "proof",
    "citation",
    "journal",
    "research",
    "method",
    "results",
    "and",
    "with",
    "from",
    "of",
    "in",
    "a",
    "is",
    "was",
    "for",
    "that",
    "on",
];

/// A small deterministic random source.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or(0) % n.max(1)
    }
}

fn corpus(bytes: usize) -> String {
    let mut r = Lcg(7);
    let mut s = String::with_capacity(bytes + 1024);
    while s.len() < bytes {
        for _ in 0..(2 + r.below(6)) {
            let n = 6 + r.below(18);
            for i in 0..n {
                let w = WORDS[r.below(WORDS.len())];
                if i == 0 {
                    let mut c = w.chars();
                    if let Some(f) = c.next() {
                        s.extend(f.to_uppercase());
                        s.push_str(c.as_str());
                    }
                } else {
                    s.push(' ');
                    s.push_str(w);
                }
            }
            s.push_str(". ");
        }
        s.push_str("\n\n");
    }
    s
}

fn main() {
    let options = Options::default();
    let sizes = [("100 KB", 100_000), ("1 MB", 1 << 20), ("10 MB", 10 << 20)];
    let mut docs: Vec<(String, Document)> = sizes
        .into_iter()
        .map(|(label, bytes)| (label.to_owned(), Document::from_plain_text(&corpus(bytes))))
        .collect();
    for path in std::env::args().skip(1) {
        match textweaver_formats::load_path(&path) {
            Ok(d) => docs.push((path, d)),
            Err(e) => eprintln!("cannot open {path}: {e}"),
        }
    }
    for (label, doc) in docs {
        // The first run also fills the document's own caches (blank lines,
        // marker tables), which the reader has filled already.
        let t = Instant::now();
        let _ = summarize_with(&doc, &options);
        let first = t.elapsed();
        let mut runs: Vec<Duration> = Vec::with_capacity(40);
        let mut last = None;
        for _ in 0..40 {
            let t = Instant::now();
            let s = summarize_with(&doc, &options);
            runs.push(t.elapsed());
            last = Some(s);
        }
        runs.sort();
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        if let Some(s) = last {
            println!(
                "{label}: first {:.1} ms, then median {:.1} ms, worst {:.1} ms; read {} of {} chars, ranked {} of {} sentences",
                ms(first),
                ms(runs[runs.len() / 2]),
                ms(runs[runs.len() - 1]),
                s.chars_read,
                s.chars,
                s.ranked,
                s.candidates,
            );
        }
    }
}
