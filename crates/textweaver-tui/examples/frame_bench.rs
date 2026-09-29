//! Times the terminal reader's frames on a 1 MB Markdown document with
//! math, code, bionic reading, and difficult words on, and counts how many
//! frames draw while idle (W6u).
//!
//! ```text
//! cargo run -p textweaver-tui --release --example frame_bench
//! ```
//!
//! Nothing is spoken (the silent backend); the screen is ratatui's
//! `TestBackend`, 100 by 40 cells.

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use textweaver_app::formats::{Registry, Source};
use textweaver_app::store::{DocKey, MathDisplay, Settings};
use textweaver_app::theme::ColorSupport;
use textweaver_app::{App, AppConfig, load_options};
use textweaver_tui::{Tui, frame};

/// A section of Markdown with a heading, prose, inline math, and a code
/// block, repeated to about `target` bytes.
fn corpus(target: usize) -> String {
    let section = "## Section heading\n\n\
        The quadratic formula $x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$ gives the roots, \
        and extraordinary readability matters for everyone who reads text. \
        Students with print disabilities deserve tools; $a^2 + b^2 = c^2$ always.\n\n\
        ```rust\nfn main() {\n    let x = 42; // the answer\n    println!(\"{x}\");\n}\n```\n\n";
    let mut s = String::with_capacity(target + section.len());
    s.push_str("# Frame bench\n\n");
    while s.len() < target {
        s.push_str(section);
    }
    s
}

fn app(aids: bool, text: &str) -> App {
    let mut settings = Settings::default();
    if aids {
        settings.reading.math_display = MathDisplay::Unicode;
        settings.reading_aids.bionic = true;
        settings.reading_aids.difficult_words = true;
    }
    let doc = Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: text.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &load_options(&settings),
        )
        .expect("the corpus loads");
    let mut app = App::new(AppConfig {
        settings,
        ..AppConfig::for_tests()
    });
    app.open_document(doc, DocKey::untitled(1), "bench".into());
    app
}

fn main() {
    let text = corpus(1 << 20);
    println!("document: {} bytes", text.len());
    for aids in [false, true] {
        if aids {
            // What the first frame paid for the difficult-word list while
            // the aid was off, before W6u checked the setting first.
            let t = Instant::now();
            let list = textweaver_app::aids::ScowlList::builtin();
            println!(
                "difficult-word list, first use: {:.1} ms (loaded: {})",
                t.elapsed().as_secs_f64() * 1e3,
                list.is_some()
            );
        }
        let what = if aids { "with reading aids" } else { "plain" };
        let mut tui = Tui::with_color_support(app(aids, &text), ColorSupport::TrueColor);
        let mut term = Terminal::new(TestBackend::new(100, 40)).expect("a test terminal");
        // The first draw loads what is loaded once (the difficult-word list,
        // the syntax set); time it on its own.
        let t = Instant::now();
        term.draw(|f| tui.draw(f)).expect("draw");
        println!(
            "{what}: first draw {:.2} ms",
            t.elapsed().as_secs_f64() * 1e3
        );
        let n = 200;
        let t = Instant::now();
        for _ in 0..n {
            term.draw(|f| tui.draw(f)).expect("draw");
        }
        let per = t.elapsed().as_secs_f64() * 1e3 / f64::from(n);
        println!("{what}: a draw of the same view {per:.3} ms");
        // Ten seconds of idle frames, as the event loop runs them every
        // 40 ms: how many draw, and the time they take.
        let frames = 250u32;
        let before = tui.draws();
        let t = Instant::now();
        for _ in 0..frames {
            frame(&mut term, &mut tui).expect("frame");
        }
        let spent = t.elapsed();
        println!(
            "{what}: {frames} idle frames drew {} times in {:.1} ms",
            tui.draws() - before,
            spent.as_secs_f64() * 1e3
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
