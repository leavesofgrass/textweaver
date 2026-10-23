//! CW (Morse code) practice: Koch-method lessons, random character and
//! word drills, and checking what the learner copied.
//!
//! - **Koch practice** (`cw_practice_koch`) sends groups of the characters
//!   of the learner's Koch lesson (`textweaver_cw::training`); **random
//!   character practice** and **random word practice** send groups from
//!   every character the course teaches, or common words and contact
//!   abbreviations.
//! - **A round** is `[speech.cw] practice_groups` groups of
//!   `practice_group` characters. For each group a prompt opens ("Group 2
//!   of 10."); the group is sent, and the learner types what they heard
//!   and presses Enter. textweaver says the score in words, starting with
//!   the count ("9 of 10. Missed: K as R. Sent: ..."), and opens the
//!   prompt for the next group. Enter with nothing typed sends the group
//!   again; Escape stops. After the last group the round's score is said,
//!   and in Koch practice a round at `practice_threshold` percent or more
//!   adds the next character.
//! - **Nothing is spoiled.** The group is never shown before the answer:
//!   not as dots and dashes (the owner's decision), not on the status line,
//!   and not by RSVP, which follow readings of the document only. What was
//!   sent is said and shown after the answer, in the score.
//! - **Sending.** With CW as the speech engine and textweaver speaking
//!   its own messages, the group is queued after the prompt's message on
//!   that engine; with a screen reader speaking them, it is timed as
//!   below. Otherwise a CW engine of its own
//!   is started on a helper thread with the frontend's speech starter, and
//!   the group goes out once the message has had time to be spoken: its
//!   expected length at the speech rate, plus `practice_delay_ms`. The
//!   two engines would otherwise talk over each other.
//! - **Progress** (the Koch lesson and the latest scores) is kept in
//!   `cw-practice.json` in the data folder ([`textweaver_store::cw_practice`]),
//!   written through the background writer.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use textweaver_cw::training::{
    self, Check, KOCH_LESSONS, Miss, Rng, koch_characters, koch_new_character,
};
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_speech::{SayMode, SpeechService};
use textweaver_store::cw_practice::{CwPractice, Drill, PracticeScore};

use crate::app::{App, Mode};
use crate::command::{Effect, PromptPurpose};

/// The engine id of CW.
const CW: &str = textweaver_engines::cw::BACKEND_ID;

/// Practice state: the progress read from disk, the round under way, and
/// the CW engine practice started for itself.
#[derive(Default)]
pub(crate) struct CwPracticeState {
    /// `cw-practice.json`, read on first use.
    progress: Option<CwPractice>,
    /// The round under way.
    round: Option<Round>,
    /// The CW engine started for practice (when CW is not the speech
    /// engine).
    speech: Option<SpeechService>,
    /// That engine, while it starts on a helper thread.
    starting: Option<Receiver<Option<SpeechService>>>,
    /// The drill generator, seeded from the clock on first use.
    rng: Option<Rng>,
}

impl std::fmt::Debug for CwPracticeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CwPracticeState")
            .field("round", &self.round)
            .field("speech", &self.speech.is_some())
            .field("starting", &self.starting.is_some())
            .finish_non_exhaustive()
    }
}

/// A round of practice.
#[derive(Debug)]
struct Round {
    drill: Drill,
    /// The Koch lesson practiced (Koch rounds).
    lesson: u8,
    /// The group being copied.
    group: String,
    /// Its number, from 1.
    n: u32,
    /// The groups in the round.
    groups: u32,
    correct: u32,
    total: u32,
    /// When to send the group on practice's own engine; `None` once sent
    /// (or when CW is the speech engine, which queues it at once).
    send_at: Option<Instant>,
}

/// Roughly how long `text` takes to say at `wpm` words per minute (six
/// characters to a word).
fn speaking_time(text: &str, wpm: u16) -> Duration {
    let chars = u64::try_from(text.chars().count()).unwrap_or(u64::MAX);
    let wpm = u64::from(wpm.max(1));
    Duration::from_millis(chars.saturating_mul(10_000) / wpm)
}

impl App {
    /// The practice commands.
    pub(crate) fn cw_practice_action(&mut self, a: ActionId) -> Vec<Effect> {
        match a {
            ActionId::CwPracticeKoch => self.start_cw_practice(Drill::Koch),
            ActionId::CwPracticeCharacters => self.start_cw_practice(Drill::Characters),
            ActionId::CwPracticeWords => self.start_cw_practice(Drill::Words),
            ActionId::CwPracticeProgress => {
                let msg = self.cw_practice_progress();
                self.tell(&msg);
                vec![Effect::Redraw]
            }
            _ => vec![Effect::Redraw],
        }
    }

    /// Gives practice a CW engine of its own, instead of starting one with
    /// the frontend's speech starter: tests pass one on the null audio
    /// output.
    #[doc(hidden)]
    pub fn set_cw_practice_speech(&mut self, speech: SpeechService) {
        self.cw_practice.speech = Some(speech);
    }

    /// The Koch lesson reached, from `cw-practice.json`.
    pub fn cw_koch_lesson(&mut self) -> u8 {
        self.practice_progress().koch_lesson
    }

    /// The progress, read on first use.
    fn practice_progress(&mut self) -> &mut CwPractice {
        let paths = self.paths.clone();
        self.cw_practice.progress.get_or_insert_with(|| {
            paths
                .as_ref()
                .and_then(|p| {
                    CwPractice::load(p)
                        .map_err(|e| log::warn!("cannot read CW practice progress: {e}"))
                        .ok()
                })
                .unwrap_or_default()
        })
    }

    /// What `cw_practice_progress` says.
    fn cw_practice_progress(&mut self) -> String {
        let p = self.practice_progress().clone();
        let lesson = p.koch_lesson.clamp(1, KOCH_LESSONS);
        let recent = if p.recent.is_empty() {
            self.msg("cw-practice-no-rounds")
        } else {
            let scores: Vec<String> = p
                .recent
                .iter()
                .rev()
                .take(3)
                .map(|s| training::percent(s.correct, s.total).to_string())
                .collect();
            self.msg_args("cw-practice-recent", &args!["scores" => scores.join(", ")])
        };
        let new = self.char_name(koch_new_character(lesson));
        self.msg_args(
            "cw-practice-progress",
            &args!["lesson" => lesson, "new" => new, "recent" => recent],
        )
    }

    /// The drill generator.
    fn practice_rng(&mut self) -> &mut Rng {
        self.cw_practice.rng.get_or_insert_with(|| {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64);
            Rng::new(seed)
        })
    }

    /// Seeds the drill generator, so a test knows the groups.
    #[doc(hidden)]
    pub fn seed_cw_practice(&mut self, seed: u64) {
        self.cw_practice.rng = Some(Rng::new(seed));
    }

    /// A new group for `drill` at `lesson`.
    fn practice_group(&mut self, drill: Drill, lesson: u8) -> String {
        let size = usize::from(self.settings.speech.cw.practice_group);
        let rng = self.practice_rng();
        match drill {
            Drill::Koch => training::koch_group(lesson, size, rng),
            Drill::Characters => training::character_group(size, rng),
            Drill::Words => training::word_group(size, rng),
        }
    }

    /// Starts a round: says what it practices and opens the first group.
    fn start_cw_practice(&mut self, drill: Drill) -> Vec<Effect> {
        self.stop_speech();
        self.stop_practice_sending();
        let lesson = self.practice_progress().koch_lesson.clamp(1, KOCH_LESSONS);
        let groups = u32::from(self.settings.speech.cw.practice_groups.max(1));
        let intro = match drill {
            Drill::Koch => {
                let new = self.char_name(koch_new_character(lesson));
                self.msg_args(
                    "cw-practice-koch-start",
                    &args![
                        "lesson" => lesson,
                        "count" => koch_characters(lesson).len(),
                        "new" => new
                    ],
                )
            }
            Drill::Characters => {
                self.msg_args("cw-practice-characters-start", &args!["groups" => groups])
            }
            Drill::Words => self.msg_args("cw-practice-words-start", &args!["groups" => groups]),
        };
        let group = self.practice_group(drill, lesson);
        self.cw_practice.round = Some(Round {
            drill,
            lesson,
            group,
            n: 1,
            groups,
            correct: 0,
            total: 0,
            send_at: None,
        });
        if self.speech.backend_id() != CW && self.cw_practice.speech.is_none() {
            self.start_practice_engine();
            if self.cw_practice.starting.is_none() {
                // No CW engine and no way to start one.
                return self.practice_unavailable();
            }
        }
        self.open_copy_prompt(&intro, false)
    }

    /// Starts a CW engine for practice on a helper thread, with the
    /// frontend's speech starter; [`App::cw_practice_tick`] picks it up.
    fn start_practice_engine(&mut self) {
        if self.cw_practice.starting.is_some() {
            return;
        }
        let Some(starter) = self.speech_starter() else {
            return;
        };
        let mut settings = self.settings.clone();
        settings.speech.backend = CW.to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.waker_slot();
        let spawned = std::thread::Builder::new()
            .name("textweaver-cw-practice".into())
            .spawn(move || {
                let (service, id, _) = starter(&settings);
                let _ = tx.send((id == CW).then_some(service));
                wake.wake();
            });
        if spawned.is_ok() {
            self.cw_practice.starting = Some(rx);
        }
    }

    /// Opens the copy prompt for the round's group, saying `before` first
    /// (what the round practices, or the last group's score); `again`
    /// says the group is sent again. Then sends the group (see the module
    /// documentation for when).
    fn open_copy_prompt(&mut self, before: &str, again: bool) -> Vec<Effect> {
        let Some(round) = self.cw_practice.round.as_ref() else {
            return vec![Effect::Redraw];
        };
        let (n, groups) = (round.n, round.groups);
        let id = if again {
            "cw-practice-again"
        } else {
            "cw-practice-group"
        };
        let label = self.msg_args(id, &args!["n" => n, "groups" => groups]);
        let said = if before.is_empty() {
            label.clone()
        } else {
            format!("{before} {label}")
        };
        if !self.mode.is_prompt() {
            self.return_mode = self.mode;
        }
        self.mode = Mode::for_prompt(PromptPurpose::CwCopy);
        self.prompt_purpose = PromptPurpose::CwCopy;
        self.tell(&said);
        self.send_practice_group(&said);
        vec![Effect::Prompt {
            label,
            purpose: PromptPurpose::CwCopy,
        }]
    }

    /// Sends the round's group: queued after `said` when CW is the speech
    /// engine and says it too, else timed (on CW, or on practice's own
    /// engine) to follow `said` spoken by speech or a screen reader.
    fn send_practice_group(&mut self, said: &str) {
        let queue = self.speech.backend_id() == CW && self.self_voicing;
        let wait = speaking_time(said, self.settings.speech.rate.wpm());
        let delay = Duration::from_millis(u64::from(self.settings.speech.cw.practice_delay_ms));
        let Some(round) = self.cw_practice.round.as_mut() else {
            return;
        };
        if queue {
            round.send_at = None;
            self.speech.say(round.group.clone(), SayMode::Queue);
        } else {
            round.send_at = Some(Instant::now() + wait + delay);
        }
    }

    /// Practice cannot send: says so and ends the round (a copy prompt
    /// already open then answers nothing).
    fn practice_unavailable(&mut self) -> Vec<Effect> {
        self.cw_practice.round = None;
        let msg = self.msg("cw-practice-unavailable");
        self.error(&msg);
        vec![Effect::Redraw]
    }

    /// True while a group waits to be sent: the frontend ticks often.
    pub(crate) fn cw_practice_pending(&self) -> bool {
        self.cw_practice.starting.is_some()
            || self
                .cw_practice
                .round
                .as_ref()
                .is_some_and(|r| r.send_at.is_some())
    }

    /// Picks up practice's engine when it has started, and sends the group
    /// when its time comes (called from [`App::tick`]).
    pub(crate) fn cw_practice_tick(&mut self, now: Instant) -> Vec<Effect> {
        if let Some(rx) = &self.cw_practice.starting {
            match rx.try_recv() {
                Ok(service) => {
                    self.cw_practice.starting = None;
                    match service {
                        Some(s) => self.cw_practice.speech = Some(s),
                        None if self.cw_practice.round.is_some() => {
                            return self.practice_unavailable();
                        }
                        None => {}
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.cw_practice.starting = None,
            }
        }
        if let Some(s) = &self.cw_practice.speech {
            // Its statuses mean nothing to the reader; keep the channel empty.
            while s.try_status().is_some() {}
        }
        let Some(round) = self.cw_practice.round.as_mut() else {
            return Vec::new();
        };
        let s = if self.speech.backend_id() == CW {
            Some(&self.speech)
        } else {
            self.cw_practice.speech.as_ref()
        };
        let (Some(at), Some(s)) = (round.send_at, s) else {
            return Vec::new();
        };
        if now >= at {
            round.send_at = None;
            s.say(round.group.clone(), SayMode::Interrupt);
        }
        Vec::new()
    }

    /// Stops a group being sent.
    fn stop_practice_sending(&mut self) {
        if let Some(r) = self.cw_practice.round.as_mut() {
            r.send_at = None;
        }
        if let Some(s) = &self.cw_practice.speech {
            s.stop();
        }
    }

    /// Escape in the copy prompt: stops the round. Nothing is kept from an
    /// unfinished round.
    pub(crate) fn cw_practice_stop(&mut self) -> Vec<Effect> {
        self.stop_practice_sending();
        if self.speech.backend_id() == CW {
            self.speech.stop();
        }
        self.cw_practice.round = None;
        self.release_practice_engine();
        let msg = self.msg("cw-practice-stopped");
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// Lets practice's own engine go (on a helper thread, as stopping it
    /// waits for its thread), so it does not hold the audio device.
    fn release_practice_engine(&mut self) {
        if let Some(s) = self.cw_practice.speech.take() {
            let _ = std::thread::Builder::new()
                .name("textweaver-cw-practice-end".into())
                .spawn(move || drop(s));
        }
    }

    /// The copy prompt's answer.
    pub(crate) fn answer_cw_copy(&mut self, text: &str) -> Vec<Effect> {
        let Some(round) = self.cw_practice.round.as_ref() else {
            return vec![Effect::Redraw];
        };
        if text.trim().is_empty() {
            return self.open_copy_prompt("", true);
        }
        let group = round.group.clone();
        let check = training::check(&group, text);
        let score = self.check_said(&check, &group, round.drill);
        let Some(round) = self.cw_practice.round.as_mut() else {
            return vec![Effect::Redraw];
        };
        round.correct += check.correct;
        round.total += check.total;
        if round.n < round.groups {
            round.n += 1;
            let (drill, lesson) = (round.drill, round.lesson);
            let next = self.practice_group(drill, lesson);
            if let Some(r) = self.cw_practice.round.as_mut() {
                r.group = next;
            }
            return self.open_copy_prompt(&score, false);
        }
        self.finish_round(&score)
    }

    /// The last group was copied: says the round's score, adds a Koch
    /// character when it reached the pass mark, and keeps the score.
    fn finish_round(&mut self, last: &str) -> Vec<Effect> {
        let Some(round) = self.cw_practice.round.take() else {
            return vec![Effect::Redraw];
        };
        self.release_practice_engine();
        let percent = training::percent(round.correct, round.total);
        let mut said = format!(
            "{last} {}",
            self.msg_args(
                "cw-practice-round",
                &args![
                    "correct" => round.correct,
                    "total" => round.total,
                    "percent" => percent
                ],
            )
        );
        let threshold = self.settings.speech.cw.practice_threshold;
        if round.drill == Drill::Koch {
            let passed = training::passes(round.correct, round.total, threshold);
            let next = if passed && round.lesson < KOCH_LESSONS {
                let lesson = round.lesson + 1;
                self.practice_progress().koch_lesson = lesson;
                let new = self.char_name(koch_new_character(lesson));
                self.msg_args(
                    "cw-practice-added",
                    &args!["lesson" => lesson, "new" => new],
                )
            } else if passed {
                self.msg("cw-practice-all-learned")
            } else {
                self.msg_args("cw-practice-not-yet", &args!["threshold" => threshold])
            };
            said = format!("{said} {next}");
        }
        let score = PracticeScore {
            at: textweaver_store::now_ts(),
            drill: round.drill,
            lesson: (round.drill == Drill::Koch).then_some(round.lesson),
            correct: round.correct,
            total: round.total,
        };
        self.practice_progress().record(score);
        self.save_practice_progress();
        self.tell(&said);
        vec![Effect::Redraw]
    }

    /// Queues `cw-practice.json` for writing.
    fn save_practice_progress(&mut self) {
        let (Some(paths), Some(progress)) = (self.paths.clone(), self.cw_practice.progress.clone())
        else {
            return;
        };
        self.writer.send(crate::writer::Job::CwPractice {
            paths,
            progress: Box::new(progress),
        });
    }

    /// A character as said and shown: letters and figures as themselves,
    /// punctuation by name ("period"), so it is heard.
    fn char_name(&self, c: char) -> String {
        let id = match c {
            '.' => "cw-char-period",
            ',' => "cw-char-comma",
            '=' => "cw-char-equals",
            '/' => "cw-char-slash",
            '?' => "cw-char-question",
            _ => return c.to_string(),
        };
        self.msg(id)
    }

    /// The score of one group in words: "9 of 10. Missed: K as R. Sent:
    /// ...", or "5 of 5, all correct."
    fn check_said(&self, check: &Check, group: &str, drill: Drill) -> String {
        if check.perfect() {
            return self.msg_args(
                "cw-practice-perfect",
                &args!["correct" => check.correct, "total" => check.total],
            );
        }
        let missed: Vec<String> = check
            .misses
            .iter()
            .map(|m| match *m {
                Miss::Wrong { sent, typed } => self.msg_args(
                    "cw-practice-wrong",
                    &args!["sent" => self.char_name(sent), "typed" => self.char_name(typed)],
                ),
                Miss::Missing(sent) => self.msg_args(
                    "cw-practice-missing",
                    &args!["sent" => self.char_name(sent)],
                ),
                Miss::Extra(typed) => self.msg_args(
                    "cw-practice-extra",
                    &args!["typed" => self.char_name(typed)],
                ),
            })
            .collect();
        // Words are said as words; groups of characters one by one.
        let sent = if drill == Drill::Words {
            group.to_owned()
        } else {
            group
                .chars()
                .map(|c| self.char_name(c))
                .collect::<Vec<_>>()
                .join(" ")
        };
        self.msg_args(
            "cw-practice-score",
            &args![
                "correct" => check.correct,
                "total" => check.total,
                "missed" => missed.join(", "),
                "sent" => sent
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaking_time_follows_the_rate() {
        // 60 characters, ten words, at 200 words per minute: 3 seconds.
        assert_eq!(
            speaking_time(&"x".repeat(60), 200),
            Duration::from_millis(3000)
        );
        assert_eq!(speaking_time("", 200), Duration::ZERO);
    }
}
