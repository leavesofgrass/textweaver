//! The keyer: paddle contacts in, tone on and off out.
//!
//! A paddle has two contacts, dit and dah. Pressed alone, each sends its
//! element again and again while it is held; pressed together (a
//! squeeze), an iambic keyer sends dits and dahs in turn, starting with
//! the contact closed first. Each element is followed by one unit of
//! silence, so the elements of a character keep their spacing whatever
//! the hand does. Spacing between characters and words is the
//! operator's.
//!
//! - **Mode A** finishes the element sounding when the squeeze is
//!   released, and stops.
//! - **Mode B** also sends one opposite element after it, if both contacts
//!   were closed at any time during the element (squeeze, release during
//!   the dah of A: di-dah-dit, an R in mode B, an A in mode A).
//! - **Dot and dash memory**: a contact pressed while an element sounds,
//!   or in the silence after it, is remembered and its element sent next,
//!   even if it was released before then.
//! - **Straight**: no timing at all; the tone follows the contact, as for
//!   a straight key (a straight key on either contact of a paddle
//!   adapter). The [`Input::Straight`] contact is always straight, in
//!   every mode.
//!
//! Time is a virtual clock in milliseconds, given with every call, so the
//! keyer is tested without waiting and driven by a real clock in the app:
//! [`Keyer::next_deadline`] says when it next needs [`Keyer::advance`].
//! Events must be given in time order.

/// A contact of a key or paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Input {
    /// The dit contact (the left paddle, unless swapped).
    Dit,
    /// The dah contact.
    Dah,
    /// A straight key: the tone follows it.
    Straight,
}

/// An element the keyer sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    /// One unit of tone.
    Dit,
    /// Three units of tone.
    Dah,
}

impl Element {
    fn other(self) -> Element {
        match self {
            Element::Dit => Element::Dah,
            Element::Dah => Element::Dit,
        }
    }

    fn index(self) -> usize {
        match self {
            Element::Dit => 0,
            Element::Dah => 1,
        }
    }
}

/// How the paddle contacts become elements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyerMode {
    /// Iambic mode A.
    IambicA,
    /// Iambic mode B (the default).
    #[default]
    IambicB,
    /// No keyer: either contact keys the tone directly.
    Straight,
}

/// The keyer's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyerOptions {
    /// The mode.
    pub mode: KeyerMode,
    /// One unit (a dit, and the silence after each element), in
    /// milliseconds: `1200 / wpm`.
    pub unit_ms: f64,
    /// Dot and dash memory.
    pub memory: bool,
}

impl Default for KeyerOptions {
    fn default() -> Self {
        KeyerOptions {
            mode: KeyerMode::IambicB,
            unit_ms: 1200.0 / f64::from(crate::timing::DEFAULT_CHAR_WPM),
            memory: true,
        }
    }
}

impl KeyerOptions {
    /// Options for `wpm` words per minute (kept within
    /// [`crate::MIN_WPM`] to [`crate::MAX_WPM`]).
    pub fn at_wpm(mode: KeyerMode, wpm: u16) -> Self {
        let wpm = wpm.clamp(crate::MIN_WPM, crate::MAX_WPM);
        KeyerOptions {
            mode,
            unit_ms: 1200.0 / f64::from(wpm),
            memory: true,
        }
    }
}

/// The tone turned on or off at a time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToneEdge {
    /// When, in milliseconds on the caller's clock.
    pub at_ms: f64,
    /// On (true) or off.
    pub on: bool,
}

/// An element being sent.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sending {
    element: Element,
    start_ms: f64,
    /// The tone's end was reported.
    ended: bool,
    /// Both contacts were closed at some time during it (mode B).
    squeezed: bool,
}

/// The keyer (see the module docs).
#[derive(Clone, Debug)]
pub struct Keyer {
    options: KeyerOptions,
    /// Dit and dah contacts closed.
    down: [bool; 2],
    /// Dit and dah remembered.
    memory: [bool; 2],
    /// The straight contact closed.
    straight: bool,
    sending: Option<Sending>,
    /// The tone is on (as reported).
    tone: bool,
    edges: Vec<ToneEdge>,
}

impl Keyer {
    /// A keyer with nothing pressed.
    pub fn new(options: KeyerOptions) -> Self {
        Keyer {
            options,
            down: [false; 2],
            memory: [false; 2],
            straight: false,
            sending: None,
            tone: false,
            edges: Vec::new(),
        }
    }

    /// The options.
    pub fn options(&self) -> KeyerOptions {
        self.options
    }

    /// New options, from the next element on.
    pub fn set_options(&mut self, options: KeyerOptions) {
        self.options = options;
    }

    fn element_ms(&self, e: Element) -> f64 {
        match e {
            Element::Dit => self.options.unit_ms,
            Element::Dah => 3.0 * self.options.unit_ms,
        }
    }

    fn set_tone(&mut self, on: bool, at_ms: f64) {
        if self.tone != on {
            self.tone = on;
            self.edges.push(ToneEdge { at_ms, on });
        }
    }

    /// A contact closed (`down`) or opened at `at_ms`.
    pub fn input(&mut self, input: Input, down: bool, at_ms: f64) {
        self.advance(at_ms);
        let paddle = match input {
            Input::Straight => None,
            Input::Dit => Some(Element::Dit),
            Input::Dah => Some(Element::Dah),
        };
        let Some(e) = paddle.filter(|_| self.options.mode != KeyerMode::Straight) else {
            // The tone follows the contact; the paddle contacts too in
            // straight mode. Any contact closed keeps the tone on.
            match input {
                Input::Straight => self.straight = down,
                Input::Dit => self.down[0] = down,
                Input::Dah => self.down[1] = down,
            }
            if self.sending.is_none() {
                let on = self.straight
                    || (self.options.mode == KeyerMode::Straight && self.down.iter().any(|d| *d));
                self.set_tone(on, at_ms);
            }
            return;
        };
        let was = self.down[e.index()];
        self.down[e.index()] = down;
        if down && !was {
            match self.sending.as_mut() {
                // Remembered while an element sounds or in its silence.
                Some(_) if self.options.memory => self.memory[e.index()] = true,
                Some(_) => {}
                None if self.straight => {}
                None => self.start(e, at_ms),
            }
        }
        if self.down[0]
            && self.down[1]
            && let Some(s) = self.sending.as_mut()
        {
            s.squeezed = true;
        }
    }

    fn start(&mut self, e: Element, at_ms: f64) {
        self.memory[e.index()] = false;
        self.sending = Some(Sending {
            element: e,
            start_ms: at_ms,
            ended: false,
            squeezed: self.down[0] && self.down[1],
        });
        self.set_tone(true, at_ms);
    }

    /// Runs the keyer up to `now_ms`: tones end and the next elements
    /// start at their exact times.
    pub fn advance(&mut self, now_ms: f64) {
        while let Some(s) = self.sending {
            let tone_end = s.start_ms + self.element_ms(s.element);
            if !s.ended && now_ms >= tone_end {
                self.set_tone(false, tone_end);
                if let Some(s) = self.sending.as_mut() {
                    s.ended = true;
                }
            }
            let space_end = tone_end + self.options.unit_ms;
            if now_ms < space_end {
                return;
            }
            self.sending = None;
            let last = s.element;
            let other = last.other();
            let squeeze_b = self.options.mode == KeyerMode::IambicB && s.squeezed;
            let next = if self.down[other.index()] || self.memory[other.index()] || squeeze_b {
                Some(other)
            } else if self.down[last.index()] || self.memory[last.index()] {
                Some(last)
            } else {
                None
            };
            match next {
                Some(e) => self.start(e, space_end),
                None => {
                    self.memory = [false; 2];
                    if self.straight {
                        self.set_tone(true, space_end);
                    }
                }
            }
        }
    }

    /// When the keyer next changes on its own (a tone ends, or the
    /// silence after an element ends), if it will.
    pub fn next_deadline(&self) -> Option<f64> {
        let s = self.sending?;
        let tone_end = s.start_ms + self.element_ms(s.element);
        Some(if s.ended {
            tone_end + self.options.unit_ms
        } else {
            tone_end
        })
    }

    /// True while the tone is on.
    pub fn tone(&self) -> bool {
        self.tone
    }

    /// The tone's edges since the last call, in time order.
    pub fn take_edges(&mut self) -> Vec<ToneEdge> {
        std::mem::take(&mut self.edges)
    }

    /// Lets go of every contact and stops at once: no element is
    /// finished, nothing is remembered.
    pub fn reset(&mut self, at_ms: f64) {
        self.down = [false; 2];
        self.memory = [false; 2];
        self.straight = false;
        self.sending = None;
        self.set_tone(false, at_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const U: f64 = 60.0;

    fn keyer(mode: KeyerMode) -> Keyer {
        Keyer::new(KeyerOptions {
            mode,
            unit_ms: U,
            memory: true,
        })
    }

    /// Runs `events` (contact, down, time in units), then the keyer to the
    /// end, and gives the elements sent as dots and dashes, with each
    /// tone's start in units.
    fn run(mode: KeyerMode, events: &[(Input, bool, f64)]) -> (String, Vec<f64>) {
        let mut k = keyer(mode);
        for (input, down, at) in events {
            k.input(*input, *down, at * U);
        }
        k.advance(1000.0 * U);
        let edges = k.take_edges();
        let mut code = String::new();
        let mut starts = Vec::new();
        for pair in edges.chunks(2) {
            assert!(pair[0].on && !pair[1].on, "{edges:?}");
            let len = (pair[1].at_ms - pair[0].at_ms) / U;
            code.push(if (len - 1.0).abs() < 1e-9 { '.' } else { '-' });
            starts.push(pair[0].at_ms / U);
        }
        (code, starts)
    }

    use Input::{Dah, Dit};
    use KeyerMode::{IambicA, IambicB};

    #[test]
    fn one_contact_repeats_its_element_while_held() {
        // Dit held for 5.5 units: dits at 0, 2, 4 (the third started while
        // held), then released.
        let (code, starts) = run(IambicB, &[(Dit, true, 0.0), (Dit, false, 5.5)]);
        assert_eq!(code, "...");
        assert_eq!(starts, [0.0, 2.0, 4.0]);
        let (code, starts) = run(IambicA, &[(Dah, true, 0.0), (Dah, false, 4.5)]);
        assert_eq!(code, "--");
        assert_eq!(starts, [0.0, 4.0]);
    }

    #[test]
    fn a_tap_sends_one_whole_element() {
        let (code, _) = run(IambicB, &[(Dah, true, 0.0), (Dah, false, 0.2)]);
        assert_eq!(code, "-");
    }

    #[test]
    fn squeeze_alternates_from_the_first_contact() {
        // Dit first, squeeze held for 7.5 units: dit 0, dah 2, dit 6.
        let events = [
            (Dit, true, 0.0),
            (Dah, true, 0.1),
            (Dit, false, 7.5),
            (Dah, false, 7.5),
        ];
        let (code, starts) = run(IambicA, &events);
        assert_eq!(code, ".-.");
        assert_eq!(starts, [0.0, 2.0, 6.0]);
    }

    #[test]
    fn squeeze_released_during_the_dah_gives_a_in_mode_a_and_r_in_mode_b() {
        // Squeeze for A, release both during the dah (units 2 to 5).
        let events = [
            (Dit, true, 0.0),
            (Dah, true, 0.2),
            (Dit, false, 3.0),
            (Dah, false, 3.0),
        ];
        assert_eq!(run(IambicA, &events).0, ".-");
        let (code, starts) = run(IambicB, &events);
        assert_eq!(code, ".-.");
        assert_eq!(starts, [0.0, 2.0, 6.0]);
    }

    #[test]
    fn squeeze_released_during_a_dit_in_mode_b_adds_the_dah() {
        // Dah first, then dit; released during the dit (units 4 to 5):
        // mode A gives N, mode B gives K.
        let events = [
            (Dah, true, 0.0),
            (Dit, true, 1.0),
            (Dah, false, 4.5),
            (Dit, false, 4.5),
        ];
        assert_eq!(run(IambicA, &events).0, "-.");
        assert_eq!(run(IambicB, &events).0, "-.-");
    }

    #[test]
    fn dash_memory_sends_a_dah_tapped_during_a_dit() {
        // Dit held for half a unit; dah tapped and released within the
        // dit. Mode A, memory on: the dah follows (A).
        let events = [
            (Dit, true, 0.0),
            (Dah, true, 0.3),
            (Dah, false, 0.5),
            (Dit, false, 0.6),
        ];
        let (code, starts) = run(IambicA, &events);
        assert_eq!(code, ".-");
        assert_eq!(starts, [0.0, 2.0]);
        // Memory off: only the dit.
        let mut k = Keyer::new(KeyerOptions {
            mode: IambicA,
            unit_ms: U,
            memory: false,
        });
        for (i, d, at) in events {
            k.input(i, d, at * U);
        }
        k.advance(100.0 * U);
        assert_eq!(k.take_edges().len(), 2);
    }

    #[test]
    fn dot_memory_works_in_the_silence_after_a_dah() {
        // Dah tapped; dit tapped in the silence after it (unit 3.5).
        let events = [
            (Dah, true, 0.0),
            (Dah, false, 1.0),
            (Dit, true, 3.5),
            (Dit, false, 3.7),
        ];
        let (code, starts) = run(IambicA, &events);
        assert_eq!(code, "-.");
        assert_eq!(starts, [0.0, 4.0]);
        assert_eq!(run(IambicB, &events).0, "-.");
    }

    #[test]
    fn the_slow_memory_test_of_the_keyer_manuals() {
        // Press dah and release it, then press dit before the dah ends and
        // release it too, all within the dah: dah then dit (N), in both
        // modes, because the dit was remembered.
        let events = [
            (Dah, true, 0.0),
            (Dah, false, 0.5),
            (Dit, true, 1.0),
            (Dit, false, 1.5),
        ];
        assert_eq!(run(IambicA, &events).0, "-.");
        assert_eq!(run(IambicB, &events).0, "-.");
    }

    #[test]
    fn deadlines_follow_the_element_and_its_silence() {
        let mut k = keyer(IambicB);
        assert_eq!(k.next_deadline(), None);
        k.input(Dah, true, 0.0);
        assert!(k.tone());
        assert_eq!(k.next_deadline(), Some(3.0 * U));
        k.input(Dah, false, 1.0 * U);
        k.advance(3.0 * U);
        assert!(!k.tone());
        assert_eq!(k.next_deadline(), Some(4.0 * U));
        k.advance(4.0 * U);
        assert_eq!(k.next_deadline(), None);
    }

    #[test]
    fn straight_mode_follows_the_contact() {
        let mut k = keyer(KeyerMode::Straight);
        k.input(Dit, true, 10.0);
        k.input(Dit, false, 250.0);
        k.input(Dah, true, 400.0);
        k.input(Dah, false, 470.0);
        let edges = k.take_edges();
        let times: Vec<(f64, bool)> = edges.iter().map(|e| (e.at_ms, e.on)).collect();
        assert_eq!(
            times,
            [(10.0, true), (250.0, false), (400.0, true), (470.0, false)]
        );
        assert_eq!(k.next_deadline(), None);
    }

    #[test]
    fn the_straight_contact_is_straight_in_iambic_modes() {
        let mut k = keyer(IambicB);
        k.input(Input::Straight, true, 0.0);
        k.input(Input::Straight, false, 333.0);
        let edges = k.take_edges();
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[1].at_ms, 333.0);
    }

    #[test]
    fn reset_stops_at_once() {
        let mut k = keyer(IambicB);
        k.input(Dit, true, 0.0);
        k.input(Dah, true, 0.0);
        k.reset(10.0);
        k.advance(1000.0);
        let edges = k.take_edges();
        assert_eq!(edges.last().map(|e| (e.at_ms, e.on)), Some((10.0, false)));
        assert!(!k.tone());
    }
}
