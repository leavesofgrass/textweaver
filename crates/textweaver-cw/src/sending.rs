//! What is being sent now, for the status line.
//!
//! The speech backend that plays CW records each character as it starts
//! to sound ([`Sending::sent`]), and the reader's status line shows the
//! newest of them, so a screen reader puts them on the Braille display: a
//! deafblind learner reads what they feel or hear. One record serves the
//! process, as only one speech engine runs at a time.
//!
//! The record keeps a short tail of what was sent, words separated by
//! spaces, so a status line that is drawn less often than characters are
//! sent still shows every one of them.

use std::sync::{Mutex, OnceLock};

/// How many characters of the tail are kept.
pub const TAIL_CHARS: usize = 64;

/// A copy of the record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SendingSnapshot {
    /// Changes every time a character is recorded or the record is
    /// cleared.
    pub seq: u64,
    /// The newest characters sent, oldest first, words separated by one
    /// space (prosigns as `<BT>`); at most [`TAIL_CHARS`] characters.
    pub tail: String,
    /// The character sounding now, if one is.
    pub current: Option<String>,
    /// Characters skipped because Morse code does not have them, since the
    /// record was last cleared.
    pub skipped: u32,
}

/// The shared record (see the module docs).
#[derive(Debug, Default)]
pub struct Sending {
    state: Mutex<SendingSnapshot>,
}

impl Sending {
    fn with<T>(&self, f: impl FnOnce(&mut SendingSnapshot) -> T) -> T {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut s)
    }

    /// `label` starts to sound; `new_word` when it begins a word after
    /// another.
    pub fn sent(&self, label: &str, new_word: bool) {
        self.with(|s| {
            if new_word && !s.tail.is_empty() && !s.tail.ends_with(' ') {
                s.tail.push(' ');
            }
            s.tail.push_str(label);
            let n = s.tail.chars().count();
            if n > TAIL_CHARS {
                let cut = s
                    .tail
                    .char_indices()
                    .nth(n - TAIL_CHARS)
                    .map_or(0, |(i, _)| i);
                s.tail.drain(..cut);
            }
            s.current = Some(label.to_owned());
            s.seq = s.seq.wrapping_add(1);
        });
    }

    /// Nothing sounds now (a stop, the end of the audio). The tail stays.
    pub fn quiet(&self) {
        self.with(|s| {
            if s.current.take().is_some() {
                s.seq = s.seq.wrapping_add(1);
            }
        });
    }

    /// Adds `n` skipped characters to the count.
    pub fn skipped(&self, n: u32) {
        if n > 0 {
            self.with(|s| s.skipped = s.skipped.saturating_add(n));
        }
    }

    /// Starts afresh: an empty tail, nothing sounding, no skips (a new
    /// reading).
    pub fn clear(&self) {
        self.with(|s| {
            s.tail.clear();
            s.current = None;
            s.skipped = 0;
            s.seq = s.seq.wrapping_add(1);
        });
    }

    /// A copy of the record.
    pub fn snapshot(&self) -> SendingSnapshot {
        self.with(|s| s.clone())
    }
}

/// The process's record.
pub fn sending() -> &'static Sending {
    static SENDING: OnceLock<Sending> = OnceLock::new();
    SENDING.get_or_init(Sending::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tail_keeps_words_apart_and_stays_short() {
        let s = Sending::default();
        s.sent("C", false);
        s.sent("Q", false);
        s.sent("<BT>", true);
        let snap = s.snapshot();
        assert_eq!(snap.tail, "CQ <BT>");
        assert_eq!(snap.current.as_deref(), Some("<BT>"));
        let seq = snap.seq;
        s.quiet();
        assert!(s.snapshot().current.is_none());
        assert_ne!(s.snapshot().seq, seq);
        for _ in 0..100 {
            s.sent("E", false);
        }
        assert_eq!(s.snapshot().tail.chars().count(), TAIL_CHARS);
        s.skipped(2);
        s.skipped(1);
        assert_eq!(s.snapshot().skipped, 3);
        s.clear();
        assert_eq!(s.snapshot().tail, "");
        assert_eq!(s.snapshot().skipped, 0);
    }
}
