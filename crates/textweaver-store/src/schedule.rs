//! Spacing for study cards (B1-f2): SM-2, written in house.
//!
//! A card's schedule is worked out from its grades alone ([`Card::reviews`]),
//! oldest first, so nothing but the grades is stored or synced, and two
//! computers that hold the same grades agree on when a card is due.
//!
//! SM-2 (Wozniak, 1990), with the four grades read as its qualities:
//! Again 1, Hard 3, Good 4, Easy 5.
//!
//! - **Again** starts the card over: the next interval is one day and the
//!   repetitions count from zero again; the ease is left as it was.
//! - **Hard, Good, Easy** count a repetition. The first interval is one
//!   day, the second six, and each after that the last interval times the
//!   ease, rounded. The ease then changes by
//!   `0.1 - (5 - q) * (0.08 + (5 - q) * 0.02)`: Hard lowers it by 0.14,
//!   Good leaves it, Easy raises it by 0.1. It never falls below 1.3.
//!
//! A card falls due its interval after the grade that set it. It counts as
//! due from [`DUE_AHEAD_SECS`] before then, so a card graded one evening
//! is due the next morning.

use crate::cards::{Card, Grade, Review};

/// One day, in seconds.
pub const DAY_SECS: i64 = 86_400;

/// How long before its time a card counts as due: half a day.
// shortcut: due is measured by the clock, not by the local calendar day;
// a day cutoff in local time would need the time zone, if the owner asks.
pub const DUE_AHEAD_SECS: i64 = DAY_SECS / 2;

/// The ease a new card starts with.
pub const START_EASE: f64 = 2.5;

/// The lowest ease.
pub const MIN_EASE: f64 = 1.3;

/// The longest interval, in days (about a hundred years).
pub const MAX_INTERVAL_DAYS: u32 = 36_500;

/// A card's schedule, from its grades.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schedule {
    /// The ease factor.
    pub ease: f64,
    /// Repetitions recalled in a row since the last Again.
    pub repetitions: u32,
    /// The current interval, in days (zero for a new card).
    pub interval_days: u32,
    /// When the card is next due (Unix seconds, UTC); `None` for a card
    /// never graded.
    pub due: Option<i64>,
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule {
            ease: START_EASE,
            repetitions: 0,
            interval_days: 0,
            due: None,
        }
    }
}

/// The SM-2 quality of a grade.
fn quality(grade: Grade) -> f64 {
    match grade {
        Grade::Again => 1.0,
        Grade::Hard => 3.0,
        Grade::Good => 4.0,
        Grade::Easy => 5.0,
    }
}

impl Schedule {
    /// The schedule after `grade`, given at `ts`.
    pub fn after(self, grade: Grade, ts: i64) -> Schedule {
        let mut s = self;
        if grade == Grade::Again {
            s.repetitions = 0;
            s.interval_days = 1;
        } else {
            s.interval_days = match s.repetitions {
                0 => 1,
                1 => 6,
                _ => {
                    let next = (f64::from(s.interval_days) * s.ease)
                        .round()
                        .clamp(1.0, f64::from(MAX_INTERVAL_DAYS));
                    // In range: clamped to MAX_INTERVAL_DAYS just above.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let next = next as u32;
                    next
                }
            };
            s.repetitions = s.repetitions.saturating_add(1);
            let q = 5.0 - quality(grade);
            s.ease = (s.ease + (0.1 - q * (0.08 + q * 0.02))).max(MIN_EASE);
        }
        s.due = Some(ts.saturating_add(i64::from(s.interval_days) * DAY_SECS));
        s
    }

    /// The schedule from `reviews`, taken oldest first.
    pub fn from_reviews(reviews: &[Review]) -> Schedule {
        let mut sorted: Vec<&Review> = reviews.iter().collect();
        sorted.sort_by_key(|r| r.ts);
        sorted
            .into_iter()
            .fold(Schedule::default(), |s, r| s.after(r.grade, r.ts))
    }

    /// Never graded.
    pub fn is_new(&self) -> bool {
        self.due.is_none()
    }

    /// Graded, and due at `now` (Unix seconds).
    pub fn is_due(&self, now: i64) -> bool {
        self.due
            .is_some_and(|d| d.saturating_sub(DUE_AHEAD_SECS) <= now)
    }

    /// Whole days from `now` until the card is due, at least one, or zero
    /// for a card due now; `None` for a new card.
    pub fn days_until(&self, now: i64) -> Option<i64> {
        let due = self.due?;
        if self.is_due(now) {
            return Some(0);
        }
        let secs = due.saturating_sub(now);
        Some(((secs + DAY_SECS / 2) / DAY_SECS).max(1))
    }
}

impl Card {
    /// The card's schedule, from its grades.
    pub fn schedule(&self) -> Schedule {
        Schedule::from_reviews(&self.reviews)
    }
}

/// How a deck stands at a moment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DueCounts {
    /// Cards in all.
    pub cards: usize,
    /// Graded cards due now.
    pub due: usize,
    /// Cards never graded.
    pub new: usize,
    /// Days until the soonest card not yet due falls due.
    pub next_in_days: Option<i64>,
}

/// Counts `cards` at `now`.
pub fn due_counts(cards: &[Card], now: i64) -> DueCounts {
    let mut out = DueCounts {
        cards: cards.len(),
        ..DueCounts::default()
    };
    for c in cards {
        let s = c.schedule();
        if s.is_new() {
            out.new += 1;
        } else if s.is_due(now) {
            out.due += 1;
        } else if let Some(d) = s.days_until(now) {
            out.next_in_days = Some(out.next_in_days.map_or(d, |n| n.min(d)));
        }
    }
    out
}

/// The cards to study at `now`, as indexes into `cards`: the due ones
/// first, longest overdue first, then the new ones in document order.
/// Cards not yet due are left out.
pub fn study_order(cards: &[Card], now: i64) -> Vec<usize> {
    let mut due: Vec<(i64, usize)> = Vec::new();
    let mut new = Vec::new();
    for (i, c) in cards.iter().enumerate() {
        let s = c.schedule();
        match s.due {
            None => new.push(i),
            Some(d) if s.is_due(now) => due.push((d, i)),
            Some(_) => {}
        }
    }
    due.sort_unstable();
    due.into_iter().map(|(_, i)| i).chain(new).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = 1_000_000;

    fn run(grades: &[Grade]) -> Schedule {
        let mut ts = T;
        let reviews: Vec<Review> = grades
            .iter()
            .map(|g| {
                ts += 1;
                Review { grade: *g, ts }
            })
            .collect();
        Schedule::from_reviews(&reviews)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn good_steps_are_one_six_then_times_the_ease() {
        let s = run(&[Grade::Good]);
        assert_eq!((s.repetitions, s.interval_days), (1, 1));
        assert!(close(s.ease, 2.5));
        assert_eq!(s.due, Some(T + 1 + DAY_SECS));
        assert_eq!(run(&[Grade::Good; 2]).interval_days, 6);
        assert_eq!(run(&[Grade::Good; 3]).interval_days, 15);
        assert_eq!(run(&[Grade::Good; 4]).interval_days, 38);
    }

    #[test]
    fn hard_and_easy_change_the_ease() {
        assert!(close(run(&[Grade::Hard]).ease, 2.36));
        assert!(close(run(&[Grade::Easy]).ease, 2.6));
        // Easy three times: 1 day, 6 days, then 6 times the ease of 2.7
        // the first two left it at.
        assert_eq!(run(&[Grade::Easy; 3]).interval_days, 16);
        // The ease never falls below its floor.
        assert!(close(run(&[Grade::Hard; 12]).ease, MIN_EASE));
    }

    #[test]
    fn again_starts_over_and_keeps_the_ease() {
        let s = run(&[Grade::Easy, Grade::Good, Grade::Good, Grade::Again]);
        assert_eq!((s.repetitions, s.interval_days), (0, 1));
        assert!(close(s.ease, 2.6));
        let s = run(&[
            Grade::Easy,
            Grade::Good,
            Grade::Good,
            Grade::Again,
            Grade::Good,
        ]);
        assert_eq!((s.repetitions, s.interval_days), (1, 1));
    }

    #[test]
    fn grades_count_in_time_order() {
        let again = Review {
            grade: Grade::Again,
            ts: T + 10,
        };
        let good = Review {
            grade: Grade::Good,
            ts: T,
        };
        assert_eq!(
            Schedule::from_reviews(&[again.clone(), good.clone()]),
            Schedule::from_reviews(&[good, again])
        );
    }

    fn card(grades: &[(Grade, i64)]) -> Card {
        Card {
            id: "c".into(),
            kind: crate::cards::CardKind::Recall,
            source: crate::cards::CardSource::Heading,
            source_id: "h".into(),
            range: textweaver_core::CharRange::default(),
            question: "Q?".into(),
            answer: "A.".into(),
            reversed: false,
            created: 0,
            reviews: grades
                .iter()
                .map(|(grade, ts)| Review {
                    grade: *grade,
                    ts: *ts,
                })
                .collect(),
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn due_cards_come_first_then_new_ones() {
        let now = T + 30 * DAY_SECS;
        let cards = [
            card(&[]),
            // Due now, and due two days ago.
            card(&[(Grade::Good, now - DAY_SECS)]),
            card(&[(Grade::Good, now - 3 * DAY_SECS)]),
            // Due in six days.
            card(&[(Grade::Good, now - DAY_SECS), (Grade::Good, now)]),
            card(&[]),
        ];
        assert_eq!(study_order(&cards, now), vec![2, 1, 0, 4]);
        assert_eq!(
            due_counts(&cards, now),
            DueCounts {
                cards: 5,
                due: 2,
                new: 2,
                next_in_days: Some(6),
            }
        );
    }

    #[test]
    fn due_from_half_a_day_ahead() {
        let s = run(&[Grade::Good]);
        assert!(!s.is_due(T + DAY_SECS / 4));
        assert_eq!(s.days_until(T + DAY_SECS / 4), Some(1));
        assert!(s.is_due(T + DAY_SECS / 2 + 1));
        assert_eq!(s.days_until(T + DAY_SECS), Some(0));
        assert_eq!(run(&[Grade::Good; 2]).days_until(T), Some(6));
        assert!(Schedule::default().is_new());
        assert_eq!(Schedule::default().days_until(T), None);
    }
}
