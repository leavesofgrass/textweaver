//! Math exploration: moving through a formula term by term.
//!
//! Alt+Shift+X (`explore_math`) starts on the math at the cursor (a
//! `Math` marker, or `$…$`, `\(…\)`, `\[…\]`, or backtick ASCIIMath found
//! in the line) and says the whole expression. Then, until Escape:
//!
//! - Right and Left move to the next and previous term at the same level;
//! - Down goes into the part (a fraction's numerator, a script, a root);
//! - Up comes back out;
//! - Home and End go to the first and last term;
//! - Space or Enter says the part again.
//!
//! Each step says [`NavStep::announcement`](textweaver_math::NavStep::announcement)
//! ("numerator, a plus b") and highlights the part's source chars, and the
//! cursor moves there so screen readers and magnifiers follow. At a
//! boundary nothing moves, and textweaver says so. Any other key leaves
//! exploration and does what it usually does; the frontend sends
//! [`Command::MathStep`](crate::Command::MathStep) while
//! [`App::math_exploring`] is true.

use textweaver_a11y::Channel;
use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_lexicon::args;
use textweaver_math::{DetectOptions, Math, NavStep, Navigator, Notation, SpeechOptions};
use textweaver_speech::Earcon;

use crate::app::App;
use crate::command::Effect;
use crate::text_util;

/// One move in math exploration ([`Command::MathStep`](crate::Command::MathStep)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MathMove {
    /// The next term at this level (Right).
    Next,
    /// The previous term at this level (Left).
    Previous,
    /// Into the part (Down).
    Enter,
    /// Out to the enclosing part (Up).
    Exit,
    /// The first term at this level (Home).
    First,
    /// The last term at this level (End).
    Last,
    /// Say the part again (Space, Enter).
    Repeat,
    /// Leave exploration (Escape).
    Leave,
}

/// The formula being explored and the place in it.
#[derive(Clone, Debug)]
pub(crate) struct MathExplore {
    math: Math,
    /// Where [`Math::source`] starts in the document.
    content_start: CharPos,
    /// Child indices from the root, as the navigator lists them.
    path: Vec<usize>,
    /// The part's chars in the document, highlighted.
    pub(crate) span: CharRange,
}

impl MathExplore {
    /// A navigator at the saved place.
    fn navigator(&self, opts: SpeechOptions) -> Navigator<'_> {
        let mut nav = Navigator::new(&self.math, opts);
        for &k in &self.path {
            if nav.enter().is_none() {
                break;
            }
            for _ in 0..k {
                if nav.next_part().is_none() {
                    break;
                }
            }
        }
        nav
    }
}

/// The math at `pos` in `doc`: its whole range, the content (without
/// delimiters), and its notation.
fn math_at(
    doc: &textweaver_text::Document,
    pos: CharPos,
    asciimath: Option<char>,
) -> Option<(CharRange, Notation)> {
    let index = doc.marker_index();
    let marked = index
        .enclosing(MarkerKind::Math, pos)
        .or_else(|| index.enclosing(MarkerKind::Math, pos.saturating_sub(1)));
    let search = match marked {
        Some(m) => m.range,
        None => text_util::line_range(doc, text_util::line_of(doc, pos)),
    };
    let text = doc.slice(search);
    let opts = DetectOptions {
        asciimath,
        ..DetectOptions::default()
    };
    let regions = textweaver_math::find_math(&text, &opts);
    let rel = pos.0.saturating_sub(search.start.0);
    let region = regions
        .iter()
        .find(|r| r.range.start.0 <= rel && rel < r.range.end.0.max(r.range.start.0 + 1))
        .or_else(|| marked.and(regions.first()))
        .or_else(|| {
            // The cursor just before the math on its line.
            regions.iter().find(|r| r.range.start.0 >= rel)
        })?;
    let content = CharRange::new(
        search.start.saturating_add(region.content.start.0),
        search.start.saturating_add(region.content.end.0),
    );
    Some((content, region.notation))
}

impl App {
    /// True while math exploration is on: the frontend sends arrow keys,
    /// Home, End, Space, Enter, and Escape as
    /// [`Command::MathStep`](crate::Command::MathStep).
    pub fn math_exploring(&self) -> bool {
        self.math_explore.is_some()
    }

    /// Leaves math exploration without a word (another key was pressed).
    pub fn stop_math_exploring(&mut self) {
        self.math_explore = None;
    }

    /// The part of the formula being explored, highlighted.
    pub fn math_explore_span(&self) -> Option<CharRange> {
        self.math_explore.as_ref().map(|m| m.span)
    }

    fn math_speech_options(&self) -> SpeechOptions {
        SpeechOptions::new(self.settings.normalization.math_verbosity)
    }

    /// `explore_math`: starts on the math at the cursor.
    pub(crate) fn explore_math(&mut self) {
        let asciimath = self.settings.normalization.asciimath_delimiter;
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let pos = s.cursor;
        let Some((content, notation)) = math_at(&s.doc, pos, asciimath) else {
            self.speech.earcon(Earcon::Boundary);
            let msg = self.msg("mathx-no-math");
            self.tell(&msg);
            return;
        };
        let source = s.doc.slice(content);
        let math = textweaver_math::parse(&source, notation);
        self.stop_speech();
        let mut explore = MathExplore {
            math,
            content_start: content.start,
            path: Vec::new(),
            span: content,
        };
        let step = explore.navigator(self.math_speech_options()).current();
        explore.span = self.doc_span(&explore, &step);
        self.math_explore = Some(explore);
        let msg = self.msg_args(
            "mathx-exploring",
            &args![
                "math" => step.announcement(),
                "parts" => if step.has_children { "yes" } else { "no" }
            ],
        );
        self.math_say(&msg);
        self.math_follow();
    }

    /// A step of math exploration.
    pub(crate) fn math_step(&mut self, mv: MathMove) -> Vec<Effect> {
        let Some(mut explore) = self.math_explore.take() else {
            return vec![Effect::Redraw];
        };
        if mv == MathMove::Leave {
            self.stop_speech();
            let msg = self.msg("mathx-left");
            self.math_say(&msg);
            return vec![Effect::Redraw];
        }
        let opts = self.math_speech_options();
        let (step, path) = {
            let mut nav = explore.navigator(opts);
            let step = match mv {
                MathMove::Next => nav.next_part(),
                MathMove::Previous => nav.previous_part(),
                MathMove::Enter => nav.enter(),
                MathMove::Exit => nav.exit(),
                MathMove::First => nav.first_part(),
                MathMove::Last => nav.last_part(),
                MathMove::Repeat | MathMove::Leave => Some(nav.current()),
            };
            (step, nav.path().to_vec())
        };
        match step {
            Some(step) => {
                explore.path = path;
                explore.span = self.doc_span(&explore, &step);
                self.math_explore = Some(explore);
                self.math_say(&step.announcement());
                self.math_follow();
            }
            None => {
                self.math_explore = Some(explore);
                self.speech.earcon(Earcon::Boundary);
                let msg = self.msg(match mv {
                    MathMove::Next => "mathx-last-term",
                    MathMove::Previous => "mathx-first-term",
                    MathMove::Enter => "mathx-no-parts",
                    MathMove::Exit => "mathx-whole-expression",
                    _ => "mathx-nothing-here",
                });
                self.math_say(&msg);
            }
        }
        vec![Effect::Redraw]
    }

    /// The document chars of a navigation step.
    fn doc_span(&self, explore: &MathExplore, step: &NavStep) -> CharRange {
        let start = explore.content_start.saturating_add(step.span.start.0);
        let end = explore.content_start.saturating_add(step.span.end.0);
        CharRange::new(start, end.max(start))
    }

    /// Says `text` as a caret move does (spoken, or on the status line with
    /// a screen reader).
    fn math_say(&mut self, text: &str) {
        self.speak_content(Channel::Caret, text);
    }

    /// Moves the cursor to the part being explored.
    fn math_follow(&mut self) {
        let Some(span) = self.math_explore.as_ref().map(|m| m.span) else {
            return;
        };
        if let Some(s) = self.session.as_mut() {
            s.cursor = span.start.clamp_to(s.doc.len_chars());
        }
        self.scroll_to_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppConfig, Command};

    fn app(text: &str) -> App {
        let mut app = App::new(AppConfig::for_tests());
        let dir = std::env::temp_dir().join(format!("tw-math-explore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("m{}.md", text.len()));
        std::fs::write(&file, text).unwrap();
        app.open(&file).unwrap();
        app
    }

    fn step(app: &mut App, mv: MathMove) -> String {
        app.dispatch(Command::MathStep(mv));
        app.status_text().to_owned()
    }

    #[test]
    fn explores_a_fraction_term_by_term() {
        let text = "The area is $\\frac{a+b}{2} + c$ here.\n";
        let mut app = app(text);
        let at = text.find("frac").unwrap();
        app.set_cursor(CharPos(at));
        app.dispatch(Command::Action(textweaver_keymap::ActionId::ExploreMath));
        assert!(app.math_exploring());
        assert!(
            app.status_text().starts_with("Exploring math:"),
            "{}",
            app.status_text()
        );
        let whole = app.math_explore_span().unwrap();
        let doc = app.session().unwrap().doc.slice(whole);
        assert_eq!(doc, "\\frac{a+b}{2} + c");

        let first = step(&mut app, MathMove::Enter);
        let span = app.math_explore_span().unwrap();
        assert_eq!(
            app.session().unwrap().doc.slice(span),
            "\\frac{a+b}{2}",
            "{first}"
        );
        assert_eq!(app.session().unwrap().cursor, span.start);
        let inside = step(&mut app, MathMove::Enter);
        assert!(inside.starts_with("numerator"), "{inside}");
        let part = |app: &App| {
            let span = app.math_explore_span().unwrap();
            let text = app.session().unwrap().doc.slice(span);
            text.trim_matches(|c| c == '{' || c == '}').to_owned()
        };
        assert_eq!(part(&app), "a+b");
        let den = step(&mut app, MathMove::Next);
        assert!(den.starts_with("denominator"), "{den}");
        assert_eq!(part(&app), "2");
        // A boundary: nothing moves.
        let before = app.math_explore_span();
        let edge = step(&mut app, MathMove::Next);
        assert_eq!(edge, "Last term.");
        assert_eq!(app.math_explore_span(), before);
        step(&mut app, MathMove::Exit);
        let up = step(&mut app, MathMove::Exit);
        assert!(up.starts_with("expression"), "{up}");
        // The highlight follows.
        let hl = app.highlights(app.session().unwrap().doc.full_range());
        assert!(
            hl.iter()
                .any(|h| h.range == app.math_explore_span().unwrap())
        );
        let left = step(&mut app, MathMove::Leave);
        assert_eq!(left, "Left math.");
        assert!(!app.math_exploring());
    }

    #[test]
    fn no_math_here_says_so() {
        let mut app = app("Just words.\n");
        app.dispatch(Command::Action(textweaver_keymap::ActionId::ExploreMath));
        assert!(!app.math_exploring());
        assert!(app.status_text().starts_with("No math here"));
    }
}
