//! Prints spoken math at each verbosity for each argument (LaTeX, or
//! ASCIIMath when prefixed with `am:`), and checks the offset maps.
//!
//! ```text
//! cargo run -p textweaver-math --example speak -- '\frac{a}{b}' 'am:sum_(i=1)^n i'
//! ```

use textweaver_core::Verbosity;
use textweaver_math::{Notation, SpeechOptions, parse, speak};

fn main() {
    // `@FILE` reads one expression per line.
    let args: Vec<String> = std::env::args()
        .skip(1)
        .flat_map(|a| match a.strip_prefix('@') {
            Some(path) => std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_owned)
                .collect(),
            None => vec![a],
        })
        .collect();
    for arg in args {
        let (src, notation) = match arg.strip_prefix("am:") {
            Some(s) => (s.to_owned(), Notation::AsciiMath),
            None => (arg.clone(), Notation::Latex),
        };
        let math = parse(&src, notation);
        println!("{src}");
        for v in [Verbosity::Low, Verbosity::Normal, Verbosity::High] {
            let s = speak(&math, &SpeechOptions::new(v));
            let check = match s.map.check_invariants(&s.text) {
                Ok(()) => String::new(),
                Err(e) => format!("   <<{e}>>"),
            };
            println!("  {v:?}: {}{check}", s.text);
        }
        for d in &math.diagnostics {
            println!("  note: {} at {}", d.message, d.span);
        }
    }
}
