//! CW keying: Morse code from a key or paddle, typed at the cursor in
//! edit mode.
//!
//! **A mode, turned on and off** by the CW keying command (Alt+Shift+F11),
//! so ordinary typing is never taken by surprise; it ends when edit mode
//! ends. While it is on:
//!
//! - **MIDI key adapters** (the N6ARA TinyMIDI, the CTR2 MIDI) are read
//!   through crate::cw_midi in both frontends, with the notes and
//!   channel of `[speech.cw]`.
//! - **Keyboard key adapters** (VBand and Vail: Left and Right Control;
//!   the TinyMIDI in keyboard mode: `[` and `]`) work in the textweaver
//!   app, which hands their presses and releases to
//!   [`App::cw_adapter_key`] and gives them to nothing else. The terminal
//!   reader keys from MIDI only, since most terminals never report a
//!   key's release.
//! - **textweaver does the keying** on its own thread, on the clock: the
//!   iambic keyer of `[speech.cw] keyer` (mode B by default, mode A, or
//!   straight) at the character speed, and the decoder, which adapts to
//!   the operator's speed (textweaver_cw::keyer, textweaver_cw::decode).
//! - **The sidetone**, the CW tone at the `[speech.cw]` pitch and level,
//!   follows the key when `[speech.cw] sidetone` is on and the speech
//!   engine plays its own audio (it uses the same output; never in
//!   tests).
//! - **Decoded characters are typed** at the cursor, letters in lower
//!   case, with a space when the silence after a character is as long as
//!   a word gap. Prosigns without a character of their own are typed as
//!   `<AR>`, `<AS>`, `<CT>`, `<SK>` and `<SN>`, the form the CW engine
//!   sends back as the prosign; BT starts a new paragraph; HH, the error
//!   sign, erases the word just keyed.
//! - **The status line** carries the characters keyed, newest last, in at
//!   most 40 cells ("CW: cq de"), shown and not said, so a screen reader
//!   puts each one on the Braille display as it is decoded. RSVP
//!   (Alt+Shift+R, which edit mode otherwise refuses) shows the word
//!   being keyed.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use textweaver_a11y::{Announcer, Priority};
use textweaver_aids::{Rsvp, WordTrack};
use textweaver_core::CharRange;
use textweaver_cw::{
    AdapterKey, Decoded, Decoder, Input, Keyed, Keyer, KeyerMode, KeyerOptions, MidiMap,
};
use textweaver_engines::cw::{CwOptions, Sidetone};
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_speech::Caps;
use textweaver_store::{CwKeyer, Settings};

use crate::app::App;
use crate::command::Effect;
use crate::cw_midi::{MidiListener, MidiStatus};

/// A key a keyboard key adapter presses (see [`App::cw_adapter_key`]).
pub type CwAdapterKey = AdapterKey;

/// The cells of a Braille display line the status line fits in.
const STATUS_CELLS: usize = 40;

/// Characters of keyed text kept for the status line and RSVP.
const KEPT_CHARS: usize = 200;

/// To the keying thread.
enum ToKeyer {
    /// A contact closed (true) or opened, at a time.
    Contact(Input, bool, Instant),
    /// New keyer options (the settings changed).
    Options(KeyerOptions),
    /// Keying ends.
    Stop,
}

/// From the MIDI threads and the keying thread.
enum FromKeyer {
    Decoded(Decoded),
    Midi(MidiStatus),
}

/// Keying while it is on.
pub(crate) struct Keying {
    to_keyer: Sender<ToKeyer>,
    from_keyer: Receiver<FromKeyer>,
    thread: Option<JoinHandle<()>>,
    /// The notes and channel the MIDI inputs map, shared with them.
    midi_map: Arc<Mutex<MidiMap>>,
    _midi: MidiListener,
    /// The adapter keys held down (repeats are ignored).
    held: Vec<AdapterKey>,
    /// The text keyed, for the status line and RSVP (the newest
    /// [`KEPT_CHARS`]).
    shown: String,
    /// The word being keyed, as typed, and the one before it with the
    /// space after it, for the error sign.
    word: String,
    previous: String,
    /// The status line was given the newest text.
    shown_dirty: bool,
    /// RSVP shows the words keyed.
    rsvp: Option<Rsvp>,
}

impl std::fmt::Debug for Keying {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keying")
            .field("shown", &self.shown)
            .finish_non_exhaustive()
    }
}

impl Drop for Keying {
    fn drop(&mut self) {
        // The thread ends and lets the sidetone go. The MIDI inputs, which
        // hold senders of their own, close after it.
        let _ = self.to_keyer.send(ToKeyer::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The keyer's options from `settings`.
fn keyer_options(settings: &Settings) -> KeyerOptions {
    let cw = &settings.speech.cw;
    let mode = match cw.keyer {
        CwKeyer::IambicB => KeyerMode::IambicB,
        CwKeyer::IambicA => KeyerMode::IambicA,
        CwKeyer::Straight => KeyerMode::Straight,
    };
    KeyerOptions::at_wpm(mode, cw.wpm)
}

/// The MIDI notes and channel from `settings`.
fn midi_map(settings: &Settings) -> MidiMap {
    let cw = &settings.speech.cw;
    MidiMap {
        dit: cw.dit_note,
        dah: cw.dah_note,
        straight: cw.straight_note,
        channel: cw.midi_channel,
        swap: cw.swap_paddles,
    }
}

/// The keying thread: contacts in, the keyer and the decoder on the
/// clock, the sidetone set at each tone edge, decoded characters out
/// (with the waker rung).
fn run(
    rx: Receiver<ToKeyer>,
    out: Sender<FromKeyer>,
    wake: crate::wake::WakeSlot,
    options: KeyerOptions,
    sidetone: Option<Sidetone>,
) {
    let origin = Instant::now();
    let ms = |t: Instant| t.saturating_duration_since(origin).as_secs_f64() * 1000.0;
    let mut keyer = Keyer::new(options);
    let wpm = (1200.0 / options.unit_ms).round();
    // Between 5 and 50 words per minute: exact as u16.
    let mut decoder = Decoder::new(wpm as u16);
    let mut last = 0.0f64;
    loop {
        let deadline = match (keyer.next_deadline(), decoder.next_deadline()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let got = match deadline {
            Some(d) => {
                let at = origin + Duration::from_secs_f64(d.max(0.0) / 1000.0);
                rx.recv_timeout(at.saturating_duration_since(Instant::now()))
            }
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let first = match got {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        // Everything already queued is applied before the clock moves
        // on, so no silence is heard between contacts that have arrived.
        let mut stop = false;
        for m in first
            .into_iter()
            .chain(std::iter::from_fn(|| rx.try_recv().ok()))
        {
            match m {
                ToKeyer::Contact(input, down, at) => {
                    // Two sources may arrive a little out of order.
                    last = last.max(ms(at));
                    keyer.input(input, down, last);
                }
                ToKeyer::Options(o) => keyer.set_options(o),
                ToKeyer::Stop => stop = true,
            }
        }
        if stop {
            break;
        }
        let now = ms(Instant::now()).max(last);
        keyer.advance(now);
        for edge in keyer.take_edges() {
            if let Some(s) = &sidetone {
                s.set(edge.on);
            }
            decoder.tone(edge.on, edge.at_ms);
        }
        decoder.advance(now);
        let decoded = decoder.take();
        if !decoded.is_empty() {
            for d in decoded {
                let _ = out.send(FromKeyer::Decoded(d));
            }
            wake.wake();
        }
    }
    if let Some(s) = &sidetone {
        s.set(false);
    }
}

/// The last `cells` characters of `text`.
fn newest(text: &str, cells: usize) -> &str {
    let n = text.chars().count();
    if n <= cells {
        return text;
    }
    let cut = text
        .char_indices()
        .nth(n - cells)
        .map_or(text.len(), |(i, _)| i);
    &text[cut..]
}

impl App {
    /// True while CW keying is on.
    pub fn cw_keying_active(&self) -> bool {
        self.cw_keying.is_some()
    }

    /// The CW keying command: turns keying on (in edit mode only) or off.
    pub(crate) fn cw_keying_toggle(&mut self) -> Vec<Effect> {
        if self.cw_keying.take().is_some() {
            let msg = self.msg("cw-keying-off");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        if !self.is_editing() {
            let key = self.key(ActionId::ToggleEditMode);
            let msg = self.msg_args("cw-keying-needs-edit", &args!["key" => key]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        self.stop_speech();
        let options = keyer_options(&self.settings);
        // The sidetone shares the speech engine's output: none when the
        // engine plays no audio of its own (a screen reader's, the silent
        // engine, the tests' recorder).
        let tones = self.speech.capabilities().contains(Caps::TONES);
        let mut sidetone_failed = None;
        let sidetone = if self.settings.speech.cw.sidetone && tones {
            Sidetone::start(&CwOptions::from_settings(&self.settings))
                .map_err(|e| sidetone_failed = Some(e.to_string()))
                .ok()
        } else {
            None
        };
        let (to_keyer, rx) = channel();
        let (out, from_keyer) = channel();
        let wake = self.wake.clone();
        let thread = {
            let out = out.clone();
            std::thread::Builder::new()
                .name("cw-keying".into())
                .spawn(move || run(rx, out, wake, options, sidetone))
                .map_err(|e| log::warn!("cannot start CW keying: {e}"))
                .ok()
        };
        let midi_map = Arc::new(Mutex::new(midi_map(&self.settings)));
        let (map, contacts, wake) = (Arc::clone(&midi_map), to_keyer.clone(), self.wake.clone());
        let (midi, opening) = crate::cw_midi::listen(
            None,
            move |_name, bytes| {
                let at = Instant::now();
                let m = *map.lock().unwrap_or_else(|e| e.into_inner());
                if let Some((input, down)) = m.contact(bytes) {
                    let _ = contacts.send(ToKeyer::Contact(input, down, at));
                }
            },
            move |s| {
                let _ = out.send(FromKeyer::Midi(s));
                wake.wake();
            },
        )
        .unwrap_or_else(|e| {
            log::warn!("CW keying: MIDI: {e}");
            (MidiListener::none(), 0)
        });
        self.cw_keying = Some(Keying {
            to_keyer,
            from_keyer,
            thread,
            midi_map,
            _midi: midi,
            held: Vec::new(),
            shown: String::new(),
            word: String::new(),
            previous: String::new(),
            shown_dirty: false,
            rsvp: None,
        });
        let key = self.key(ActionId::CwKeying);
        let mut msg = self.msg_args("cw-keying-on", &args!["key" => key]);
        if !crate::cw_midi::AVAILABLE {
            msg = format!("{msg} {}", self.msg("cw-keying-midi-not-in-build"));
        } else if opening == 0 {
            msg = format!("{msg} {}", self.msg("cw-keying-no-midi"));
        }
        self.tell(&msg);
        if let Some(e) = sidetone_failed {
            let msg = self.msg_args("cw-keying-no-sidetone", &args!["error" => e]);
            self.warn(&msg);
        }
        vec![Effect::Redraw]
    }

    /// A key of a keyboard key adapter went down (`down`) or up, in the
    /// textweaver app: Left or Right Control, `[` or `]`. While CW keying
    /// is on, it keys, and the frontend gives it to nothing else (true);
    /// otherwise it is an ordinary key (false). Repeats of a held key are
    /// ignored.
    pub fn cw_adapter_key(&mut self, key: CwAdapterKey, down: bool) -> bool {
        self.cw_adapter_key_at(key, down, Instant::now())
    }

    /// [`cw_adapter_key`](Self::cw_adapter_key) at a given time (tests).
    pub fn cw_adapter_key_at(&mut self, key: CwAdapterKey, down: bool, at: Instant) -> bool {
        let swap = self.settings.speech.cw.swap_paddles;
        let Some(k) = self.cw_keying.as_mut() else {
            return false;
        };
        let was = k.held.contains(&key);
        if down == was {
            return true;
        }
        if down {
            k.held.push(key);
        } else {
            k.held.retain(|h| *h != key);
        }
        let _ = k
            .to_keyer
            .send(ToKeyer::Contact(key.contact(swap), down, at));
        true
    }

    /// `[speech.cw]` changed: keying follows it (the speed, keyer, notes,
    /// channel and paddle swap at once; the sidetone's pitch and level the
    /// next time keying turns on).
    pub(crate) fn cw_keying_settings_changed(&mut self) {
        let options = keyer_options(&self.settings);
        let map = midi_map(&self.settings);
        if let Some(k) = self.cw_keying.as_ref() {
            let _ = k.to_keyer.send(ToKeyer::Options(options));
            *k.midi_map.lock().unwrap_or_else(|e| e.into_inner()) = map;
        }
    }

    /// RSVP's key in edit mode while keying: RSVP shows the words keyed,
    /// or stops. False when keying is off.
    pub(crate) fn cw_keying_rsvp_toggle(&mut self) -> bool {
        let settings = (&self.settings.reading_aids.rsvp).into();
        let Some(k) = self.cw_keying.as_mut() else {
            return false;
        };
        let on = k.rsvp.is_none();
        k.rsvp = on.then(|| keyed_rsvp(&k.shown, settings));
        let msg = self.msg(if on {
            "cw-keying-rsvp-on"
        } else {
            "aids-rsvp-off"
        });
        self.tell(&msg);
        true
    }

    /// RSVP of the words keyed, while it shows.
    pub(crate) fn cw_keying_rsvp(&self) -> Option<&Rsvp> {
        self.cw_keying.as_ref()?.rsvp.as_ref()
    }

    /// Applies what keying decoded since the last tick (called from
    /// [`App::tick`]); ends keying when edit mode ended.
    pub(crate) fn cw_keying_tick(&mut self) -> Vec<Effect> {
        if self.cw_keying.is_none() {
            return Vec::new();
        }
        if !self.is_editing() {
            self.cw_keying = None;
            let msg = self.msg("cw-keying-ended");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let got: Vec<FromKeyer> = self
            .cw_keying
            .as_ref()
            .map(|k| k.from_keyer.try_iter().collect())
            .unwrap_or_default();
        if got.is_empty() {
            return Vec::new();
        }
        for g in got {
            match g {
                FromKeyer::Decoded(d) => self.cw_keyed(d),
                FromKeyer::Midi(MidiStatus::Listening(name)) => {
                    let msg = self.msg_args("cw-keying-listening", &args!["name" => name]);
                    self.tell(&msg);
                }
                FromKeyer::Midi(MidiStatus::Failed(name, error)) => {
                    let msg = self.msg_args(
                        "cw-keying-midi-failed",
                        &args!["name" => name, "error" => error],
                    );
                    self.warn(&msg);
                }
            }
        }
        self.show_keyed_line();
        vec![Effect::Redraw]
    }

    /// One decoded character or word space: typed, and kept for the
    /// status line.
    fn cw_keyed(&mut self, d: Decoded) {
        let typed: String = match d {
            Decoded::Char(Keyed::Text(t)) => t.to_owned(),
            Decoded::Char(Keyed::Paragraph) => "\n\n".to_owned(),
            Decoded::WordSpace => {
                let after_paragraph = self
                    .cw_keying
                    .as_ref()
                    .is_some_and(|k| k.word.is_empty() || k.word.ends_with('\n'));
                if after_paragraph {
                    return;
                }
                " ".to_owned()
            }
            Decoded::Char(Keyed::Error) => return self.cw_erase_word(),
            Decoded::Char(Keyed::Unknown) => {
                let msg = self.msg("cw-keying-unknown");
                self.tell(&msg);
                return;
            }
        };
        if !self.cw_type(&typed) {
            return;
        }
        let Some(k) = self.cw_keying.as_mut() else {
            return;
        };
        let shown = match d {
            Decoded::Char(Keyed::Paragraph) => " <BT> ",
            _ => typed.as_str(),
        };
        k.shown.push_str(shown);
        let n = k.shown.chars().count();
        if n > KEPT_CHARS {
            let cut = k.shown.len() - newest(&k.shown, KEPT_CHARS).len();
            k.shown.drain(..cut);
        }
        if typed == " " || typed.ends_with('\n') {
            k.word.push_str(&typed);
            k.previous = std::mem::take(&mut k.word);
        } else {
            k.word.push_str(&typed);
        }
        k.shown_dirty = true;
    }

    /// Types `text` at the cursor, as typing does (one undo step with the
    /// typing around it). False when there is no editor.
    fn cw_type(&mut self, text: &str) -> bool {
        let Some(range) = self.edit.as_ref().and_then(|e| {
            let sel = e.session.editor()?.selection();
            Some(if sel.anchor <= sel.head {
                CharRange::new(sel.anchor, sel.head)
            } else {
                CharRange::new(sel.head, sel.anchor)
            })
        }) else {
            return false;
        };
        self.replace_range(range, text);
        true
    }

    /// The error sign: the word being keyed is erased, or, between
    /// words, the word before and its space; only text keying typed, and
    /// only while it is still just before the cursor.
    fn cw_erase_word(&mut self) {
        let Some(k) = self.cw_keying.as_mut() else {
            return;
        };
        let gone = if k.word.is_empty() {
            std::mem::take(&mut k.previous)
        } else {
            std::mem::take(&mut k.word)
        };
        let n = gone.chars().count();
        let at_caret = self.edit.as_ref().and_then(|e| {
            let ed = e.session.editor()?;
            let caret = ed.selection().head;
            let start = caret.0.checked_sub(n)?;
            let text = ed.text().slice(start..caret.0).to_string();
            (text == gone).then_some(CharRange::new(textweaver_core::CharPos(start), caret))
        });
        let shown = gone.trim().to_owned();
        match at_caret {
            Some(range) if n > 0 => {
                self.replace_range(range, "");
                if let Some(k) = self.cw_keying.as_mut() {
                    if let Some(rest) = k.shown.trim_end().strip_suffix(shown.as_str()) {
                        let keep = rest.len();
                        k.shown.truncate(keep);
                    }
                    k.shown_dirty = true;
                }
                let msg = self.msg_args("cw-keying-erased", &args!["text" => shown]);
                self.tell(&msg);
            }
            _ => {
                let msg = self.msg("cw-keying-nothing-erased");
                self.tell(&msg);
            }
        }
    }

    /// The status line while keying: "CW:" and the newest characters
    /// keyed, in at most 40 cells. Shown, not said, for the Braille
    /// display. RSVP follows the word being keyed.
    fn show_keyed_line(&mut self) {
        let settings = (&self.settings.reading_aids.rsvp).into();
        let Some(k) = self.cw_keying.as_mut() else {
            return;
        };
        if !std::mem::take(&mut k.shown_dirty) {
            return;
        }
        if k.rsvp.is_some() {
            k.rsvp = Some(keyed_rsvp(&k.shown, settings));
        }
        let shown = k.shown.clone();
        let label = self.msg_args("cw-sending", &args!["text" => ""]);
        let room = STATUS_CELLS.saturating_sub(label.trim_end().chars().count() + 1);
        let line = self.msg_args("cw-sending", &args!["text" => newest(&shown, room.max(8))]);
        let line = self.screen_text(&line);
        self.status.announce(&line, Priority::Polite);
    }
}

/// RSVP showing the last word of `keyed`.
fn keyed_rsvp(keyed: &str, settings: textweaver_aids::RsvpSettings) -> Rsvp {
    let track = WordTrack::from_text(keyed);
    let last = track.len().saturating_sub(1);
    let mut rsvp = Rsvp::new(track, settings);
    rsvp.seek_word(last, 0);
    rsvp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyer_options_follow_the_settings() {
        let mut s = Settings::default();
        let o = keyer_options(&s);
        assert_eq!(o.mode, KeyerMode::IambicB);
        assert!((o.unit_ms - 60.0).abs() < 1e-9);
        assert!(o.memory);
        s.speech.cw.keyer = CwKeyer::Straight;
        s.speech.cw.wpm = 30;
        let o = keyer_options(&s);
        assert_eq!(o.mode, KeyerMode::Straight);
        assert!((o.unit_ms - 40.0).abs() < 1e-9);
        let m = midi_map(&s);
        assert_eq!((m.dit, m.dah, m.straight, m.channel), (20, 21, 30, 0));
    }

    #[test]
    fn newest_keeps_the_end() {
        assert_eq!(newest("cq de", 30), "cq de");
        assert_eq!(newest("abcdef", 3), "def");
    }
}
