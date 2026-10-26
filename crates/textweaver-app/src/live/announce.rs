//! Announcing a live document's new entries (beta 5, N5).
//!
//! A new or corrected entry goes out as one polite message holding the
//! whole entry, meaning first, when `[live] announce_new_entries` is on.
//! It goes through the app's normal message path, so the screen reader
//! shows it on the Braille display (and speaks it, unless its own speech
//! is off). textweaver's own voice says it only when `[live]
//! speak_new_entries` is on; in screen-reader mode textweaver never
//! speaks, as with every message.
//!
//! **Importance.** An entry is content the user chose to receive, like the
//! text of a line, not textweaver talking about itself: it is said as an
//! [`Importance::Answer`], which no
//! interface announcement level silences. `announce_new_entries` is the
//! switch for it. A new kind would only repeat that rule.
//!
//! **The throttle** ([`EntryThrottle`]):
//!
//! - **Never interrupt reading.** While textweaver reads aloud (or reads on
//!   the status line for a screen reader), nothing is said; what arrives
//!   waits and goes out as one digest when reading stops.
//! - **Bursts become one message.** The first entry after a quiet spell is
//!   said at once; entries arriving within [`BURST`] of the last
//!   announcement wait and are said together: the newest entry whole,
//!   meaning first, then how many ("3 new entries").
//! - **Fields have their own pace.** In the values view, each field's
//!   change waits at least its minimum interval (`[live]
//!   min_interval_seconds`) after that field was last said; only its latest
//!   value is said.
//! - Only a lost connection is assertive (crate::live::hub); nothing here
//!   is.
//!
//! The throttle keeps no clock of its own: every call takes `now`, so
//! tests drive it with a virtual clock.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use textweaver_a11y::{Importance, Priority, Verbosity};
use textweaver_lexicon::args;

use crate::app::App;
use crate::command::Effect;
use crate::playback::Playback;

/// The shortest time between two announcements of new entries: entries
/// arriving sooner are said together.
pub const BURST: Duration = Duration::from_secs(2);

/// What the throttle holds for one field of the values view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FieldSlot {
    /// When the field's change was last said.
    last: Option<Instant>,
    /// The latest change not yet said.
    pending: Option<String>,
    /// The fewest time between two of its announcements.
    interval: Duration,
    /// Its changes are spoken in textweaver's own voice.
    speak: bool,
}

/// Entries waiting to be said.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Held {
    /// How many entries arrived or changed.
    count: usize,
    /// The newest of them, as one line, meaning first.
    newest: String,
}

/// What the throttle releases: entries as a digest, and field changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Release {
    /// New or changed entries: how many, and the newest one whole.
    pub entries: Option<(usize, String)>,
    /// Field changes of the values view, in the order they first
    /// changed: each line, and whether textweaver's own voice says it.
    pub fields: Vec<(String, bool)>,
}

impl Release {
    /// True when there is nothing to say.
    pub fn is_empty(&self) -> bool {
        self.entries.is_none() && self.fields.is_empty()
    }
}

/// Holds new entries and field changes until they may be said (see the
/// module).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryThrottle {
    /// The shortest time between two announcements of entries ([`BURST`],
    /// unless changed for a test).
    burst: Option<Duration>,
    held: Option<Held>,
    /// When entries were last said.
    last_entries: Option<Instant>,
    fields: BTreeMap<String, FieldSlot>,
    /// Fields in the order their changes arrived first, for the release.
    order: Vec<String>,
}

impl EntryThrottle {
    /// Notes `count` new or changed entries, the newest being `newest`.
    pub fn entries(&mut self, count: usize, newest: String) {
        if count == 0 {
            return;
        }
        match self.held.as_mut() {
            Some(h) => {
                h.count += count;
                h.newest = newest;
            }
            None => self.held = Some(Held { count, newest }),
        }
    }

    /// Notes a change of the values view's field `key`, said as `text` at
    /// most once each `interval`, in textweaver's own voice when `speak`;
    /// a later change of the same field replaces one not yet said.
    pub fn field(&mut self, key: &str, text: String, interval: Duration, speak: bool) {
        if !self.fields.contains_key(key) {
            self.order.push(key.to_owned());
        }
        let slot = self.fields.entry(key.to_owned()).or_default();
        slot.pending = Some(text);
        slot.interval = interval;
        slot.speak = speak;
    }

    /// What may be said at `now`: nothing while `reading`; held entries
    /// once [`BURST`] has passed since the last ones were said; each
    /// field's change once its interval has passed since that field was
    /// last said.
    pub fn release(&mut self, now: Instant, reading: bool) -> Release {
        let mut out = Release::default();
        if reading {
            return out;
        }
        if self.entries_ready(now)
            && let Some(h) = self.held.take()
        {
            self.last_entries = Some(now);
            out.entries = Some((h.count, h.newest));
        }
        for key in &self.order {
            let Some(slot) = self.fields.get_mut(key) else {
                continue;
            };
            let ready = slot
                .last
                .is_none_or(|t| now.saturating_duration_since(t) >= slot.interval);
            if ready && let Some(text) = slot.pending.take() {
                slot.last = Some(now);
                out.fields.push((text, slot.speak));
            }
        }
        out
    }

    /// How long until something held may be said, when something is held
    /// and nothing blocks it but time; `None` when nothing waits, or while
    /// `reading` (the end of reading wakes the app).
    pub fn wait(&self, now: Instant, reading: bool) -> Option<Duration> {
        if reading {
            return None;
        }
        let left = |last: Option<Instant>, interval: Duration| {
            last.map_or(Duration::ZERO, |t| {
                interval.saturating_sub(now.saturating_duration_since(t))
            })
        };
        let entries = self
            .held
            .as_ref()
            .map(|_| left(self.last_entries, self.burst()));
        let fields = self
            .fields
            .values()
            .filter(|s| s.pending.is_some())
            .map(|s| left(s.last, s.interval))
            .min();
        entries.into_iter().chain(fields).min()
    }

    /// True when entries are waiting.
    pub fn holding(&self) -> bool {
        self.held.is_some() || self.fields.values().any(|s| s.pending.is_some())
    }

    /// Forgets everything held (the live document closed, or announcing
    /// was turned off); a changed burst time stays.
    pub fn clear(&mut self) {
        *self = EntryThrottle {
            burst: self.burst,
            ..EntryThrottle::default()
        };
    }

    fn entries_ready(&self, now: Instant) -> bool {
        self.last_entries
            .is_none_or(|t| now.saturating_duration_since(t) >= self.burst())
    }

    fn burst(&self) -> Duration {
        self.burst.unwrap_or(BURST)
    }

    /// Changes the time between two announcements of entries, for a test
    /// that sends entries faster than a person would.
    pub fn set_burst(&mut self, burst: Duration) {
        self.burst = Some(burst);
    }
}

impl App {
    /// New or changed entries arrived in the open live document: `count`
    /// of them, the newest being `newest` (one line, meaning first). They
    /// are said on the next tick, through the throttle, when `[live]
    /// announce_new_entries` is on.
    pub(crate) fn live_entries_arrived(&mut self, count: usize, newest: String) {
        if !self.settings.live.announce_new_entries || newest.is_empty() {
            return;
        }
        self.live_wire.throttle.entries(count, newest);
    }

    /// Takes the values view's field changes into the throttle: a field
    /// whose changes are shown or spoken (its `show` and `speak`) is said
    /// at most once each of its intervals (`[live] min_interval_seconds`),
    /// its latest value only; spoken in textweaver's own voice only when
    /// `speak`.
    fn live_take_fields(&mut self) {
        for c in self.live_take_field_announcements() {
            if (c.show || c.speak) && !c.line.is_empty() {
                let key = format!("{}{}", c.topic, c.path);
                self.live_wire
                    .throttle
                    .field(&key, c.line, c.min_interval, c.speak);
            }
        }
    }

    /// True while textweaver reads aloud, or reads on the status line for a
    /// screen reader: new entries wait.
    pub(crate) fn live_reading_now(&self) -> bool {
        self.playback == Playback::Reading || self.screen_say_all.is_some()
    }

    /// Says what the throttle releases at `now` (called from
    /// [`App::tick`]).
    pub(crate) fn live_announce_tick(&mut self, now: Instant) -> Vec<Effect> {
        self.live_take_fields();
        if !self.live_wire.throttle.holding() {
            return Vec::new();
        }
        let reading = self.live_reading_now();
        let release = self.live_wire.throttle.release(now, reading);
        if release.is_empty() {
            return Vec::new();
        }
        if let Some((count, newest)) = release.entries {
            let text = if count <= 1 {
                newest
            } else {
                self.msg_args(
                    "live-new-entries",
                    &args!["count" => count, "line" => newest.as_str()],
                )
            };
            let before = self.status.seq;
            self.say_live_entry(&text, self.settings.live.speak_new_entries);
            // The status line shows an entry: the terminal reader may show
            // it in its own form (App::live_status_is_entry).
            let seq = self.status.seq;
            if seq != before
                && let Some(h) = self.live_hub.as_mut()
            {
                h.entry_seq = Some(seq);
            }
        }
        for (line, speak) in release.fields {
            self.say_live_entry(&line, speak);
        }
        vec![Effect::Redraw]
    }

    /// Says new entries arriving within `burst` of the last announcement
    /// together ([`BURST`] by default): for a test that sends entries
    /// faster than a person would, and must see each one.
    #[doc(hidden)]
    pub fn set_live_announce_burst(&mut self, burst: Duration) {
        self.live_wire.throttle.set_burst(burst);
    }

    /// How long the frontend may wait before the throttle has something
    /// to say ([`App::tick_interval`]).
    pub(crate) fn live_announce_wait(&self, now: Instant) -> Option<Duration> {
        let reading = self.live_reading_now();
        self.live_wire.throttle.wait(now, reading)
    }

    /// Says a live entry or field change: politely, never silenced by the
    /// interface level, to the status line and the screen reader; in
    /// textweaver's own voice only when `voice` (`[live]
    /// speak_new_entries`, or a field's `speak`).
    pub(crate) fn say_live_entry(&mut self, text: &str, voice: bool) {
        self.say_kind_voiced(
            text,
            Verbosity::Low,
            Priority::Polite,
            Importance::Answer,
            Some(voice),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELD: Duration = Duration::from_secs(10);

    fn line(n: usize) -> String {
        format!("N{n}CALL, Ada Example.")
    }

    #[test]
    fn the_first_entry_is_said_at_once() {
        let t0 = Instant::now();
        let mut t = EntryThrottle::default();
        t.entries(1, line(1));
        let r = t.release(t0, false);
        assert_eq!(r.entries, Some((1, line(1))));
        assert!(!t.holding());
        assert_eq!(t.wait(t0, false), None);
    }

    #[test]
    fn a_burst_waits_and_goes_out_as_one_digest_with_the_newest_whole() {
        let t0 = Instant::now();
        let mut t = EntryThrottle::default();
        t.entries(1, line(1));
        assert_eq!(t.release(t0, false).entries, Some((1, line(1))));
        // Three more within the burst window: held.
        for (i, ms) in [(2, 300), (3, 700), (4, 1200)] {
            let now = t0 + Duration::from_millis(ms);
            t.entries(1, line(i));
            assert!(t.release(now, false).is_empty(), "held at {ms} ms");
        }
        let now = t0 + Duration::from_millis(1200);
        assert_eq!(t.wait(now, false), Some(Duration::from_millis(800)));
        let r = t.release(t0 + BURST, false);
        assert_eq!(r.entries, Some((3, line(4))));
        assert!(!t.holding());
    }

    #[test]
    fn nothing_is_said_while_reading_and_all_of_it_after() {
        let t0 = Instant::now();
        let mut t = EntryThrottle::default();
        t.entries(1, line(1));
        t.entries(2, line(3));
        assert!(t.release(t0, true).is_empty());
        assert_eq!(t.wait(t0, true), None, "reading's end wakes the app");
        let later = t0 + Duration::from_secs(60);
        assert!(t.release(later, true).is_empty());
        let r = t.release(later, false);
        assert_eq!(r.entries, Some((3, line(3))));
    }

    #[test]
    fn each_field_keeps_its_own_minimum_interval_and_only_its_latest_value() {
        let t0 = Instant::now();
        let mut t = EntryThrottle::default();
        t.field("temp", "Temperature: 21.4 C".into(), FIELD, true);
        t.field("door", "Door: closed".into(), FIELD, false);
        let r = t.release(t0, false);
        assert_eq!(
            r.fields,
            [
                ("Temperature: 21.4 C".to_owned(), true),
                ("Door: closed".to_owned(), false)
            ]
        );

        // Three seconds on, both change; neither may be said yet.
        let t3 = t0 + Duration::from_secs(3);
        t.field("temp", "Temperature: 21.5 C".into(), FIELD, false);
        t.field("temp", "Temperature: 21.6 C".into(), FIELD, false);
        assert!(t.release(t3, false).is_empty());
        assert_eq!(t.wait(t3, false), Some(Duration::from_secs(7)));

        // At ten seconds the temperature goes, with its latest value only.
        let t10 = t0 + FIELD;
        t.field("door", "Door: open".into(), FIELD, false);
        let r = t.release(t10, false);
        assert_eq!(
            r.fields,
            [
                ("Temperature: 21.6 C".to_owned(), false),
                ("Door: open".to_owned(), false)
            ]
        );

        // A field said at 10 s waits until 20 s; another is not held up.
        let t12 = t0 + Duration::from_secs(12);
        t.field("door", "Door: closed".into(), FIELD, false);
        assert!(t.release(t12, false).is_empty());
        let r = t.release(t0 + Duration::from_secs(20), false);
        assert_eq!(r.fields, [("Door: closed".to_owned(), false)]);
    }

    #[test]
    fn entries_and_fields_do_not_hold_each_other_up() {
        let t0 = Instant::now();
        let mut t = EntryThrottle::default();
        t.entries(1, line(1));
        t.field("temp", "Temperature: 20 C".into(), FIELD, false);
        let r = t.release(t0, false);
        assert_eq!(r.entries, Some((1, line(1))));
        assert_eq!(r.fields, [("Temperature: 20 C".to_owned(), false)]);
        t.entries(1, line(2));
        let r = t.release(t0 + BURST, false);
        assert_eq!(r.entries, Some((1, line(2))));
        t.clear();
        assert!(!t.holding());
    }
}
