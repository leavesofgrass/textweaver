//! The decoder: tone on and off in time, out as characters and word
//! spaces.
//!
//! Each tone is a dit or a dah by its length; a silence of about two
//! units ends a character, and about five units a word (the midpoints
//! between the one-unit gap inside a character and the three-unit gap
//! between characters, and between three and seven units).
//!
//! **It adapts to the operator's speed.** A straight key's elements are
//! as long as the hand makes them, so the decoder keeps the lengths of
//! the last [`HISTORY`] tones and splits them where two neighbors, in
//! order of length, differ most (at least by [`SPLIT_RATIO`]): the
//! shorter group are dits, the longer dahs, and the unit is the dits'
//! mean length (or a third of the dahs' when there are no dits). A
//! character's tones are classified when it ends, with what was learned
//! from its own tones included, so the first mixed character at a new
//! speed already decodes. It starts from the speed it is given, so the
//! first characters decode at that speed. An iambic keyer's elements are
//! exact, and decode the same way.
//!
//! Time is in milliseconds on the caller's clock, as in [`crate::keyer`];
//! [`Decoder::next_deadline`] says when silence next ends a character or
//! a word.
//!
//! **What characters become** ([`decode`]): the letters, figures and
//! punctuation of the Morse table; the accented E; prosigns that have no
//! character of their own in angle brackets (`<AR>`, `<AS>`, `<CT>`,
//! `<SK>`, `<SN>`), the form textweaver sends back as the prosign; BT a
//! new paragraph; HH (eight dits or more) the error sign. KN shares its
//! code with the open bracket, and becomes `(`.

use std::collections::VecDeque;

use crate::table::code;

/// How many recent tones the speed is learned from.
pub const HISTORY: usize = 24;

/// How much longer the shortest dah must be than the longest dit for a
/// split between them to be believed.
pub const SPLIT_RATIO: f64 = 1.6;

/// The shortest unit the decoder believes, in milliseconds (60 words per
/// minute).
const MIN_UNIT_MS: f64 = 20.0;

/// The longest unit the decoder believes, in milliseconds (4 words per
/// minute).
const MAX_UNIT_MS: f64 = 300.0;

/// What a character decodes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyed {
    /// Text to type: a character, or a prosign in angle brackets.
    Text(&'static str),
    /// BT: a new paragraph.
    Paragraph,
    /// HH, eight dits or more: the error sign.
    Error,
    /// A code Morse code does not have.
    Unknown,
}

/// The decoder's output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoded {
    /// A character ended.
    Char(Keyed),
    /// A word ended (once, after its last character).
    WordSpace,
}

/// What the code `pattern` (dots and dashes) is (see the module docs).
pub fn decode(pattern: &str) -> Keyed {
    if pattern.len() >= 8 && pattern.bytes().all(|b| b == b'.') {
        return Keyed::Error;
    }
    match pattern {
        "-...-" => return Keyed::Paragraph,
        ".-.-." => return Keyed::Text("<AR>"),
        ".-..." => return Keyed::Text("<AS>"),
        "-.-.-" => return Keyed::Text("<CT>"),
        "...-.-" => return Keyed::Text("<SK>"),
        "...-." => return Keyed::Text("<SN>"),
        _ => {}
    }
    const CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789.,:?'-/()\"=+@!&;_$é";
    for (i, c) in CHARS.char_indices() {
        if code(c) == Some(pattern) {
            return Keyed::Text(&CHARS[i..i + c.len_utf8()]);
        }
    }
    Keyed::Unknown
}

/// The decoder (see the module docs).
#[derive(Clone, Debug)]
pub struct Decoder {
    /// Recent tone lengths, newest last.
    recent: VecDeque<f64>,
    /// The unit and the dit/dah boundary, learned.
    unit_ms: f64,
    boundary_ms: f64,
    /// The tones of the character under way.
    pending: Vec<f64>,
    /// The tone's start, while it is on.
    on_since: Option<f64>,
    /// The last tone's end.
    off_since: Option<f64>,
    /// A character was given since the last word space.
    in_word: bool,
    out: Vec<Decoded>,
}

impl Decoder {
    /// A decoder expecting `wpm` words per minute at first.
    pub fn new(wpm: u16) -> Self {
        let unit = 1200.0 / f64::from(wpm.clamp(crate::MIN_WPM, crate::MAX_WPM));
        Decoder {
            recent: VecDeque::with_capacity(HISTORY),
            unit_ms: unit,
            boundary_ms: 2.0 * unit,
            pending: Vec::new(),
            on_since: None,
            off_since: None,
            in_word: false,
            out: Vec::new(),
        }
    }

    /// The unit learned so far, in milliseconds.
    pub fn unit_ms(&self) -> f64 {
        self.unit_ms
    }

    /// The speed learned so far, in words per minute.
    pub fn wpm(&self) -> f64 {
        1200.0 / self.unit_ms
    }

    /// The tone went on or off at `at_ms`.
    pub fn tone(&mut self, on: bool, at_ms: f64) {
        self.advance(at_ms);
        match (on, self.on_since) {
            (true, None) => {
                self.on_since = Some(at_ms);
                self.off_since = None;
            }
            (false, Some(start)) => {
                self.on_since = None;
                self.off_since = Some(at_ms);
                let len = (at_ms - start).max(0.0);
                self.pending.push(len);
                if self.recent.len() == HISTORY {
                    self.recent.pop_front();
                }
                self.recent.push_back(len);
                self.learn();
            }
            _ => {}
        }
    }

    /// Learns the boundary and the unit from the recent tones.
    fn learn(&mut self) {
        let mut sorted: Vec<f64> = self.recent.iter().copied().collect();
        sorted.sort_by(f64::total_cmp);
        let split = sorted
            .windows(2)
            .filter(|w| w[0] > 0.0)
            .map(|w| (w[1] / w[0], w[0], w[1]))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((ratio, short, long)) = split
            && ratio >= SPLIT_RATIO
        {
            self.boundary_ms = (short * long).sqrt();
        }
        let dits: Vec<f64> = sorted
            .iter()
            .copied()
            .filter(|l| *l < self.boundary_ms)
            .collect();
        let unit = if dits.is_empty() {
            let dahs = &sorted;
            dahs.iter().sum::<f64>() / dahs.len().max(1) as f64 / 3.0
        } else {
            dits.iter().sum::<f64>() / dits.len() as f64
        };
        self.unit_ms = unit.clamp(MIN_UNIT_MS, MAX_UNIT_MS);
        // With one kind of tone only, the boundary follows the unit.
        if split.is_none_or(|(r, _, _)| r < SPLIT_RATIO) {
            self.boundary_ms = 2.0 * self.unit_ms;
        }
    }

    /// Ends characters and words whose silence has lasted long enough by
    /// `now_ms`.
    pub fn advance(&mut self, now_ms: f64) {
        let Some(off) = self.off_since else {
            return;
        };
        let silent = now_ms - off;
        if !self.pending.is_empty() && silent >= 2.0 * self.unit_ms {
            let pattern: String = self
                .pending
                .drain(..)
                .map(|l| if l < self.boundary_ms { '.' } else { '-' })
                .collect();
            self.out.push(Decoded::Char(decode(&pattern)));
            self.in_word = true;
        }
        if self.pending.is_empty() && self.in_word && silent >= 5.0 * self.unit_ms {
            self.in_word = false;
            self.out.push(Decoded::WordSpace);
        }
    }

    /// When silence next ends a character or a word, if it will.
    pub fn next_deadline(&self) -> Option<f64> {
        let off = self.off_since?;
        if !self.pending.is_empty() {
            Some(off + 2.0 * self.unit_ms)
        } else if self.in_word {
            Some(off + 5.0 * self.unit_ms)
        } else {
            None
        }
    }

    /// What was decoded since the last call, in order.
    pub fn take(&mut self) -> Vec<Decoded> {
        std::mem::take(&mut self.out)
    }

    /// Forgets the character under way (keying stopped); the speed
    /// learned stays.
    pub fn clear(&mut self) {
        self.pending.clear();
        self.on_since = None;
        self.off_since = None;
        self.in_word = false;
        self.out.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyer::{Input, Keyer, KeyerMode, KeyerOptions};

    #[test]
    fn codes_decode_to_their_text() {
        assert_eq!(decode(".-"), Keyed::Text("a"));
        assert_eq!(decode("--.."), Keyed::Text("z"));
        assert_eq!(decode("-----"), Keyed::Text("0"));
        assert_eq!(decode(".-.-.-"), Keyed::Text("."));
        assert_eq!(decode("..--.."), Keyed::Text("?"));
        assert_eq!(decode("..-.."), Keyed::Text("é"));
        assert_eq!(decode("-.--."), Keyed::Text("("));
        assert_eq!(decode(".-.-."), Keyed::Text("<AR>"));
        assert_eq!(decode("...-.-"), Keyed::Text("<SK>"));
        assert_eq!(decode(".-..."), Keyed::Text("<AS>"));
        assert_eq!(decode("-...-"), Keyed::Paragraph);
        assert_eq!(decode("........"), Keyed::Error);
        assert_eq!(decode("..........."), Keyed::Error);
        assert_eq!(decode("......"), Keyed::Unknown);
        assert_eq!(decode("--..-"), Keyed::Unknown);
    }

    /// A tiny random number generator, so jitter is the same every run.
    struct Lcg(u64);

    impl Lcg {
        /// A number in `[-1, 1)`.
        fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
        }
    }

    /// Keys `text` by hand at `wpm`, every length off by up to `jitter`
    /// (a fraction), and decodes it starting from `start_wpm`.
    fn hand_keyed(text: &str, wpm: f64, jitter: f64, start_wpm: u16, seed: u64) -> String {
        let unit = 1200.0 / wpm;
        let mut rng = Lcg(seed);
        let mut d = Decoder::new(start_wpm);
        let mut t = 1000.0;
        let mut len = |units: f64| units * unit * (1.0 + jitter * rng.next());
        for word in text.split(' ') {
            for (ci, c) in word.chars().enumerate() {
                if ci > 0 {
                    t += len(3.0);
                }
                let pattern = code(c).unwrap_or("");
                for (ei, e) in pattern.chars().enumerate() {
                    if ei > 0 {
                        t += len(1.0);
                    }
                    d.tone(true, t);
                    t += len(if e == '-' { 3.0 } else { 1.0 });
                    d.tone(false, t);
                }
            }
            t += len(7.0);
            d.advance(t);
        }
        d.advance(t + 100.0 * unit);
        render(&d.take())
    }

    fn render(out: &[Decoded]) -> String {
        out.iter()
            .map(|o| match o {
                Decoded::Char(Keyed::Text(s)) => (*s).to_owned(),
                Decoded::Char(Keyed::Paragraph) => "\n".to_owned(),
                Decoded::Char(Keyed::Error) => "#".to_owned(),
                Decoded::Char(Keyed::Unknown) => "*".to_owned(),
                Decoded::WordSpace => " ".to_owned(),
            })
            .collect()
    }

    #[test]
    fn exact_keying_decodes_at_the_expected_speed() {
        assert_eq!(hand_keyed("PARIS PARIS", 20.0, 0.0, 20, 1), "paris paris ");
    }

    #[test]
    fn straight_key_decodes_at_several_speeds_with_jitter() {
        let text = "CQ CQ DE TEST 73";
        let want = "cq cq de test 73 ";
        for (wpm, seed) in [
            (5.0, 2),
            (10.0, 3),
            (15.0, 4),
            (20.0, 5),
            (25.0, 6),
            (30.0, 7),
        ] {
            assert_eq!(
                hand_keyed(text, wpm, 0.2, 20, seed),
                want,
                "{wpm} words per minute"
            );
        }
    }

    #[test]
    fn the_decoder_adapts_from_a_wrong_starting_speed() {
        // Starting at 20, the operator keys at 35 and at 8. The first
        // character may decode wrong; from there on it follows.
        let fast = hand_keyed("PARIS PARIS QRL", 35.0, 0.15, 20, 8);
        assert!(fast.ends_with("paris qrl "), "{fast}");
        let slow = hand_keyed("PARIS PARIS QRL", 8.0, 0.15, 20, 9);
        assert!(slow.ends_with("paris qrl "), "{slow}");
    }

    #[test]
    fn the_speed_is_learned() {
        let mut d = Decoder::new(20);
        let unit = 1200.0 / 30.0;
        let mut t = 0.0;
        for e in [1.0, 3.0, 1.0, 3.0] {
            d.tone(true, t);
            t += e * unit;
            d.tone(false, t);
            t += unit;
        }
        assert!((d.wpm() - 30.0).abs() < 0.01, "{}", d.wpm());
    }

    #[test]
    fn prosigns_and_the_error_sign_come_through() {
        assert_eq!(hand_keyed("= +", 20.0, 0.1, 20, 10), "\n <AR> ");
        let mut d = Decoder::new(20);
        let mut t = 0.0;
        for _ in 0..8 {
            d.tone(true, t);
            t += 60.0;
            d.tone(false, t);
            t += 60.0;
        }
        d.advance(t + 1000.0);
        assert_eq!(d.take(), [Decoded::Char(Keyed::Error), Decoded::WordSpace]);
    }

    #[test]
    fn deadlines_say_when_silence_ends_a_character_and_a_word() {
        let mut d = Decoder::new(20);
        assert_eq!(d.next_deadline(), None);
        d.tone(true, 0.0);
        assert_eq!(d.next_deadline(), None);
        d.tone(false, 60.0);
        assert_eq!(d.next_deadline(), Some(180.0));
        d.advance(180.0);
        assert_eq!(d.take(), [Decoded::Char(Keyed::Text("e"))]);
        assert_eq!(d.next_deadline(), Some(360.0));
        d.advance(360.0);
        assert_eq!(d.take(), [Decoded::WordSpace]);
        assert_eq!(d.next_deadline(), None);
    }

    #[test]
    fn an_iambic_keyer_decodes_exactly() {
        // CQ on a paddle in mode B at 25 words per minute.
        let opts = KeyerOptions::at_wpm(KeyerMode::IambicB, 25);
        let u = opts.unit_ms;
        let mut k = Keyer::new(opts);
        let mut d = Decoder::new(25);
        let feed = |k: &mut Keyer, d: &mut Decoder, to: f64| {
            k.advance(to);
            for e in k.take_edges() {
                d.tone(e.on, e.at_ms);
            }
            d.advance(to);
        };
        // C: squeeze, dah first. Dah 0 to 3, dit 4 to 5; dah released at
        // 5.5, but the dit was squeezed, so mode B adds the dah (6 to 9);
        // dit still held, so a dit (10 to 11), released during it.
        k.input(Input::Dah, true, 0.0);
        k.input(Input::Dit, true, 0.5 * u);
        feed(&mut k, &mut d, 5.5 * u);
        k.input(Input::Dah, false, 5.5 * u);
        feed(&mut k, &mut d, 10.5 * u);
        k.input(Input::Dit, false, 10.5 * u);
        feed(&mut k, &mut d, 14.0 * u);
        // Q, from 14: dah held through two dahs; dit tapped during the
        // second, sent from memory; dah still held, so a dah; released
        // during it.
        let t0 = 14.0 * u;
        k.input(Input::Dah, true, t0);
        feed(&mut k, &mut d, t0 + 4.5 * u);
        k.input(Input::Dit, true, t0 + 4.5 * u);
        k.input(Input::Dit, false, t0 + 4.8 * u);
        feed(&mut k, &mut d, t0 + 8.5 * u);
        k.input(Input::Dah, false, t0 + 10.5 * u);
        feed(&mut k, &mut d, t0 + 40.0 * u);
        assert_eq!(render(&d.take()), "cq ");
    }
}
