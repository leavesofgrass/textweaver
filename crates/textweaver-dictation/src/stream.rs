//! Live dictation: words committed while the speaker is still talking
//! (ADR-0042).
//!
//! Whisper does not stream: it transcribes a stretch of audio at a time.
//! Live dictation re-runs it over the utterance being spoken as more
//! audio arrives, and commits the words two runs in a row agree on
//! (LocalAgreement-2, as in Whisper-Streaming). Committed words are never
//! withdrawn; the half-heard word at the end of the audio is never
//! committed, because the next run hears it differently. At the pause
//! that ends the utterance, one last run over all of it commits the rest.
//!
//! The pieces:
//!
//! - [`Agreement`]: the commit rule for one utterance, with no model and
//!   no clock.
//! - A listener thread finds utterances in the live audio
//!   ([`FindSpeech`]) and passes audio and
//!   speech events on, in order. When it finds a pause it cancels a
//!   partial run still going, so the utterance's last run starts sooner.
//! - The session loop, on the thread that owns the model: runs over the
//!   open utterance every [`StreamConfig::step`] of new audio after the
//!   last run finished, and the last run at each pause.
//! - The gate, so live dictation is never slower than transcribing each
//!   utterance at its pause: partial runs only once an utterance is past
//!   [`StreamConfig::min_utterance`] of speech (about 3 seconds; below
//!   that little is agreed on before the pause, and a run cancelled at the
//!   pause delays the last one), only while runs stay under
//!   [`StreamConfig::max_run`] (about 1.5 seconds), and not once the
//!   speaker has been quiet for 300 ms (the pause may be coming; its run
//!   will have those words). After a slow run the utterance waits for its
//!   pause.
//!
//! The loop is written against a feed and a recognizer, so the tests drive
//! it with a fake recognizer, in virtual time and in real time, without a
//! model or a microphone.

// The session loop serves the in-process backend (feature `rten`); without
// it, only the tests drive it.
#![cfg_attr(not(feature = "rten"), allow(dead_code))]

use std::collections::VecDeque;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::capture::{LiveAudio, WHISPER_SAMPLE_RATE};
use crate::transcript::{DictationEvent, Segment, Transcript};
use crate::vad::{FindSpeech, SpeechEvent};

/// How live dictation runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamConfig {
    /// New audio needed after the last run before the next partial run.
    pub step: Duration,
    /// Partial runs start only once the utterance has this much speech
    /// (from its start to the end of the latest speech heard; the silence
    /// after it does not count).
    pub min_utterance: Duration,
    /// A partial run slower than this ends partial runs for the
    /// utterance: the rest of its words come at its pause.
    pub max_run: Duration,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            step: Duration::from_millis(700),
            min_utterance: Duration::from_secs(3),
            max_run: Duration::from_millis(1500),
        }
    }
}

impl StreamConfig {
    /// No partial runs: each utterance is transcribed once, at its pause,
    /// while recording goes on. The fallback when partial runs cost too
    /// much.
    pub fn pauses_only() -> Self {
        StreamConfig {
            min_utterance: Duration::MAX,
            ..StreamConfig::default()
        }
    }

    fn samples(d: Duration) -> usize {
        usize::try_from(
            d.as_millis()
                .saturating_mul(u128::from(WHISPER_SAMPLE_RATE))
                / 1000,
        )
        .unwrap_or(usize::MAX)
    }
}

/// A word for comparing: lower case, letters, digits and apostrophes.
pub fn norm(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .flat_map(char::to_lowercase)
        .collect()
}

/// How many leading words two word lists agree on, compared with
/// [`norm`].
pub fn common_prefix(a: &[String], b: &[String]) -> usize {
    a.iter()
        .zip(b)
        .take_while(|(x, y)| norm(x) == norm(y))
        .count()
}

/// Where a hypothesis goes on after the `committed` words: after them
/// when it starts with them; otherwise after its prefix closest to them
/// (the least edit distance, then the length nearest theirs). Whisper
/// sometimes rewrites an early word ("until 9" becomes "until 9am"); the
/// committed words stay as they are, and matching a later word such as
/// "to" against the wrong "to" would drop everything between.
pub fn continuation(committed: &[String], hyp: &[String]) -> usize {
    if common_prefix(committed, hyp) == committed.len() {
        return committed.len().min(hyp.len());
    }
    let c: Vec<String> = committed.iter().map(|w| norm(w)).collect();
    let h: Vec<String> = hyp.iter().map(|w| norm(w)).collect();
    // prev[p]: edit distance of the committed words so far against
    // hyp[..p].
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    for (i, cword) in c.iter().enumerate() {
        let mut row = vec![i + 1; h.len() + 1];
        for j in 1..=h.len() {
            let sub = prev[j - 1] + usize::from(*cword != h[j - 1]);
            row[j] = sub.min(prev[j] + 1).min(row[j - 1] + 1);
        }
        prev = row;
    }
    (0..=h.len())
        .min_by_key(|&p| (prev[p], p.abs_diff(c.len())))
        .unwrap_or(0)
}

/// The words of some segments.
fn words(segments: &[Segment]) -> Vec<String> {
    segments
        .iter()
        .flat_map(|s| s.text.split_whitespace().map(str::to_owned))
        .collect()
}

/// LocalAgreement-2 for one utterance: which words are safe to commit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Agreement {
    committed: Vec<String>,
    last: Vec<String>,
}

impl Agreement {
    /// A new utterance, nothing committed.
    pub fn new() -> Self {
        Agreement::default()
    }

    /// The words committed so far.
    pub fn committed(&self) -> &[String] {
        &self.committed
    }

    /// A partial run's words (the utterance so far). Returns the words
    /// newly committed: those this run and the last one agree on after
    /// the committed words. An empty run is no information (Whisper
    /// sometimes returns nothing for audio cut mid-word): it commits
    /// nothing and is not compared with the next.
    pub fn partial(&mut self, hyp: Vec<String>) -> Vec<String> {
        if hyp.is_empty() {
            return Vec::new();
        }
        let from_last = continuation(&self.committed, &self.last);
        let from_hyp = continuation(&self.committed, &hyp);
        let agreed = common_prefix(&self.last[from_last..], &hyp[from_hyp..]);
        let new = hyp[from_hyp..from_hyp + agreed].to_vec();
        self.committed.extend(new.iter().cloned());
        self.last = hyp;
        new
    }

    /// The last run's words, over the whole utterance, at its pause.
    /// Returns the rest: the words after the committed ones, all
    /// committed now.
    pub fn finish(&mut self, hyp: Vec<String>) -> Vec<String> {
        let from = continuation(&self.committed, &hyp);
        let rest = hyp[from..].to_vec();
        self.committed.extend(rest.iter().cloned());
        self.last.clear();
        rest
    }
}

/// Turns a stretch of 16 kHz audio into text.
pub(crate) trait Recognizer {
    /// Transcribes `audio`. When `cancel` becomes true the result is no
    /// longer wanted: stop early and return anything.
    fn recognize(&mut self, audio: &[f32], cancel: &AtomicBool) -> Result<Vec<Segment>, String>;
}

/// What the listener and the owner of a session tell the session loop,
/// across threads: pauses found (to cancel a partial run), and the
/// session cancelled.
#[derive(Debug, Default)]
pub(crate) struct Signals {
    inner: Mutex<SignalState>,
}

#[derive(Debug, Default)]
struct SignalState {
    /// Pauses (and the end of the audio) found by the listener.
    pauses: u64,
    cancelled: bool,
    /// The running recognizer's cancel flag, and whether a pause
    /// cancels it (a partial run) or only cancelling the session does.
    run: Option<(Arc<AtomicBool>, bool)>,
}

impl Signals {
    fn lock(&self) -> std::sync::MutexGuard<'_, SignalState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The listener found a pause or the end of the audio. Called before
    /// the event is sent, so a run started before the event is taken is
    /// still cancelled.
    pub(crate) fn pause(&self) {
        let mut s = self.lock();
        s.pauses += 1;
        if let Some((flag, true)) = &s.run {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// The session is abandoned: stops the running recognizer and the
    /// listener.
    pub(crate) fn cancel(&self) {
        let mut s = self.lock();
        s.cancelled = true;
        if let Some((flag, _)) = &s.run {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// True once the session is cancelled.
    pub(crate) fn is_cancelled(&self) -> bool {
        self.lock().cancelled
    }

    /// A cancel flag for the next run. A partial run (`partial_after`
    /// is the number of pauses the loop has taken) is cancelled by any
    /// later pause, including one already found; every run is cancelled
    /// with the session.
    fn begin_run(&self, partial_after: Option<u64>) -> Arc<AtomicBool> {
        let mut s = self.lock();
        let paused = partial_after.is_some_and(|seen| s.pauses > seen);
        let flag = Arc::new(AtomicBool::new(s.cancelled || paused));
        s.run = Some((Arc::clone(&flag), partial_after.is_some()));
        flag
    }

    fn end_run(&self) {
        self.lock().run = None;
    }
}

/// Quiet after speech that holds off partial runs: half the pause that
/// ends an utterance.
const QUIET: Duration = Duration::from_millis(300);

/// What the listener sends the session loop, in order.
#[derive(Debug)]
pub(crate) enum Msg {
    /// More audio, after everything sent before, and where the latest
    /// speech heard so far ends.
    Audio {
        samples: Vec<f32>,
        speech_end: Option<usize>,
    },
    /// An utterance opened or closed, in the audio sent so far.
    Speech(SpeechEvent),
    /// The audio has ended; every utterance has been closed.
    End,
}

/// The next message from a feed.
#[derive(Debug)]
pub(crate) enum Next {
    Msg(Msg),
    /// Nothing yet.
    Empty,
    /// The feed has stopped for good.
    Gone,
}

/// Where the session loop gets its messages and its time.
pub(crate) trait Feed {
    /// The next message; with `wait`, waits a little for one.
    fn next(&mut self, wait: bool) -> Next;

    /// The time since the session started.
    fn now(&self) -> Duration;
}

/// The listener's messages, in real time.
pub(crate) struct ChannelFeed {
    rx: Receiver<Msg>,
    t0: Instant,
}

impl ChannelFeed {
    pub(crate) fn new(rx: Receiver<Msg>) -> Self {
        ChannelFeed {
            rx,
            t0: Instant::now(),
        }
    }
}

impl Feed for ChannelFeed {
    fn next(&mut self, wait: bool) -> Next {
        if wait {
            match self.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(m) => Next::Msg(m),
                Err(RecvTimeoutError::Timeout) => Next::Empty,
                Err(RecvTimeoutError::Disconnected) => Next::Gone,
            }
        } else {
            match self.rx.try_recv() {
                Ok(m) => Next::Msg(m),
                Err(TryRecvError::Empty) => Next::Empty,
                Err(TryRecvError::Disconnected) => Next::Gone,
            }
        }
    }

    fn now(&self) -> Duration {
        self.t0.elapsed()
    }
}

/// Starts the listener: takes `live` audio as it arrives, finds
/// utterances in it, and sends both on. It stops at the end of the audio
/// or when the session is cancelled.
pub(crate) fn spawn_listener(
    live: LiveAudio,
    mut finder: Box<dyn FindSpeech>,
    signals: Arc<Signals>,
) -> std::io::Result<(Receiver<Msg>, JoinHandle<()>)> {
    let (tx, rx) = channel();
    let handle = std::thread::Builder::new()
        .name("textweaver-dictation-listener".into())
        .spawn(move || {
            let send_speech = |e: SpeechEvent| {
                if matches!(e, SpeechEvent::Closed(_)) {
                    signals.pause();
                }
                tx.send(Msg::Speech(e)).is_ok()
            };
            loop {
                if signals.is_cancelled() {
                    return;
                }
                let (audio, ended) = live.take(Duration::from_millis(50));
                if !audio.is_empty() {
                    let events = finder.push(&audio);
                    let audio = Msg::Audio {
                        samples: audio,
                        speech_end: finder.speech_end(),
                    };
                    if tx.send(audio).is_err() {
                        return;
                    }
                    for e in events {
                        if !send_speech(e) {
                            return;
                        }
                    }
                }
                if ended {
                    for e in finder.finish() {
                        if !send_speech(e) {
                            return;
                        }
                    }
                    signals.pause();
                    let _ = tx.send(Msg::End);
                    return;
                }
            }
        })?;
    Ok((rx, handle))
}

/// What a live session did.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LiveReport {
    /// The session's text: each utterance's committed words.
    pub transcript: Transcript,
    /// Samples of audio heard.
    pub samples: usize,
    /// Utterances found.
    pub utterances: usize,
    /// Partial runs, cancelled ones included.
    pub partial_runs: usize,
    /// Partial runs cancelled because the pause came while they ran.
    pub cancelled_runs: usize,
    /// Words committed by agreement, before their utterance's pause.
    pub early_words: usize,
    /// Words committed in all.
    pub words: usize,
    /// Session time when the audio ended.
    pub ended_at: Option<Duration>,
    /// Session time when the last utterance was finished.
    pub finished_at: Duration,
    /// The most audio held at once, in samples.
    pub peak_audio: usize,
}

/// An utterance the loop is working on.
#[derive(Debug)]
struct Utterance {
    index: usize,
    /// Its first sample.
    start: usize,
    /// The end of the audio its last run heard (its start before any).
    heard: usize,
    /// A partial run was slow: no more until the pause.
    slow: bool,
    agreement: Agreement,
}

/// Audio kept while no utterance is open: an utterance opens with a
/// little audio from before it was recognized as speech (its padding,
/// and the speech before the minimum was reached).
const KEEP_IDLE: Duration = Duration::from_secs(3);

/// One live session: from the feed's first message to its end. Events go
/// to `emit` as they happen: `Committed` words, and a `Partial` segment
/// per utterance at its pause. Returns `Ok(None)` when the session was
/// cancelled, and an error when a final run failed.
pub(crate) fn run_live(
    rec: &mut dyn Recognizer,
    feed: &mut dyn Feed,
    signals: &Signals,
    config: &StreamConfig,
    emit: &mut dyn FnMut(DictationEvent),
) -> Result<Option<LiveReport>, String> {
    let mut s = Session {
        audio: Vec::new(),
        base: 0,
        speech_end: 0,
        open: None,
        closing: VecDeque::new(),
        ended: false,
        pauses: 0,
        next_index: 0,
        report: LiveReport::default(),
    };
    let step = StreamConfig::samples(config.step).max(1);
    let min_utterance = StreamConfig::samples(config.min_utterance);
    let quiet = StreamConfig::samples(QUIET);
    loop {
        if signals.is_cancelled() {
            return Ok(None);
        }
        // Everything waiting first, so a pause already found is known.
        loop {
            match feed.next(false) {
                Next::Msg(m) => s.take(m, feed.now()),
                Next::Empty => break,
                Next::Gone => {
                    s.gone(feed.now());
                    break;
                }
            }
        }
        if let Some((u, range)) = s.closing.pop_front() {
            if !s.final_run(rec, signals, u, range, emit)? {
                return Ok(None);
            }
            s.trim();
            continue;
        }
        if s.ended && s.open.is_none() {
            break;
        }
        let (seen, speech_end) = (s.seen(), s.speech_end);
        let due = s.open.as_ref().is_some_and(|u| {
            !u.slow
                && speech_end.saturating_sub(u.start) >= min_utterance
                && seen.saturating_sub(speech_end) < quiet
                && seen.saturating_sub(u.heard) >= step
        });
        if due {
            s.partial_run(rec, feed, signals, config, emit);
            continue;
        }
        s.trim();
        match feed.next(true) {
            Next::Msg(m) => s.take(m, feed.now()),
            Next::Empty => {}
            Next::Gone => s.gone(feed.now()),
        }
    }
    s.report.samples = s.seen();
    s.report.finished_at = feed.now();
    Ok(Some(s.report))
}

/// The session loop's state.
struct Session {
    /// Audio from sample `base` on.
    audio: Vec<f32>,
    base: usize,
    /// Where the latest speech heard ends.
    speech_end: usize,
    open: Option<Utterance>,
    /// Utterances closed and waiting for their last run, with their
    /// ranges.
    closing: VecDeque<(Utterance, Range<usize>)>,
    ended: bool,
    /// Pauses taken from the feed (closed utterances and the end).
    pauses: u64,
    next_index: usize,
    report: LiveReport,
}

impl Session {
    fn seen(&self) -> usize {
        self.base + self.audio.len()
    }

    fn slice(&self, range: Range<usize>) -> &[f32] {
        let from = range.start.saturating_sub(self.base).min(self.audio.len());
        let to = range.end.saturating_sub(self.base).min(self.audio.len());
        if range.start < self.base {
            log::warn!(
                "dictation: utterance from sample {} was trimmed to {}",
                range.start,
                self.base
            );
        }
        &self.audio[from..to.max(from)]
    }

    fn new_utterance(&mut self, start: usize) -> Utterance {
        let index = self.next_index;
        self.next_index += 1;
        self.report.utterances += 1;
        Utterance {
            index,
            start,
            heard: start,
            slow: false,
            agreement: Agreement::new(),
        }
    }

    fn take(&mut self, m: Msg, now: Duration) {
        match m {
            Msg::Audio {
                samples,
                speech_end,
            } => {
                self.audio.extend_from_slice(&samples);
                self.report.peak_audio = self.report.peak_audio.max(self.audio.len());
                if let Some(e) = speech_end {
                    self.speech_end = self.speech_end.max(e);
                }
            }
            Msg::Speech(SpeechEvent::Opened { start }) => {
                let u = self.new_utterance(start);
                if let Some(old) = self.open.replace(u) {
                    // Not expected: the finder closes before it opens.
                    let end = self.seen();
                    self.closing.push_back((old, 0..end));
                }
            }
            Msg::Speech(SpeechEvent::Closed(range)) => {
                self.pauses += 1;
                let u = match self.open.take() {
                    Some(u) => u,
                    None => self.new_utterance(range.start),
                };
                let range = u.start..range.end.max(u.start);
                self.closing.push_back((u, range));
            }
            Msg::End => {
                self.pauses += 1;
                self.gone(now);
            }
        }
    }

    /// The audio has ended (or the listener has gone): an utterance still
    /// open ends with the audio.
    fn gone(&mut self, now: Duration) {
        if self.ended {
            return;
        }
        self.ended = true;
        self.report.ended_at = Some(now);
        if let Some(u) = self.open.take() {
            let range = u.start..self.seen();
            self.closing.push_back((u, range));
        }
    }

    /// Drops audio nothing will need again.
    fn trim(&mut self) {
        // The oldest utterance still needing audio: one waiting for its
        // last run comes before the open one.
        let keep_from = self
            .closing
            .front()
            .map(|(u, _)| u.start)
            .or(self.open.as_ref().map(|u| u.start))
            .unwrap_or_else(|| self.seen().saturating_sub(StreamConfig::samples(KEEP_IDLE)));
        // Draining moves what is kept: only for a second or more.
        if keep_from >= self.base + WHISPER_SAMPLE_RATE as usize {
            let n = (keep_from - self.base).min(self.audio.len());
            self.audio.drain(..n);
            self.base += n;
        }
    }

    fn partial_run(
        &mut self,
        rec: &mut dyn Recognizer,
        feed: &mut dyn Feed,
        signals: &Signals,
        config: &StreamConfig,
        emit: &mut dyn FnMut(DictationEvent),
    ) {
        let end = self.seen();
        let Some(start) = self.open.as_ref().map(|u| u.start) else {
            return;
        };
        let flag = signals.begin_run(Some(self.pauses));
        let t = feed.now();
        let result = rec.recognize(self.slice(start..end), &flag);
        let cost = feed.now().saturating_sub(t);
        signals.end_run();
        self.report.partial_runs += 1;
        let Some(u) = self.open.as_mut() else {
            return;
        };
        u.heard = end;
        if cost > config.max_run {
            // Runs this slow would make the last run late: the rest of the
            // utterance waits for its pause.
            u.slow = true;
        }
        if flag.load(Ordering::SeqCst) {
            // The pause came while it ran: its last run follows at once.
            self.report.cancelled_runs += 1;
            return;
        }
        log::debug!(
            "dictation: partial run over {} ms took {} ms",
            (end - start) as u64 * 1000 / u64::from(WHISPER_SAMPLE_RATE),
            cost.as_millis()
        );
        let segments = match result {
            Ok(s) => s,
            Err(e) => {
                // The utterance's last run will say so if it persists.
                log::warn!("dictation: a partial run failed: {e}");
                return;
            }
        };
        let new = u.agreement.partial(words(&segments));
        if !new.is_empty() {
            self.report.early_words += new.len();
            self.report.words += new.len();
            emit(DictationEvent::Committed {
                text: new.join(" "),
                utterance: u.index,
                at_pause: false,
            });
        }
    }

    /// The utterance's last run: commits the rest. False when cancelled.
    fn final_run(
        &mut self,
        rec: &mut dyn Recognizer,
        signals: &Signals,
        mut u: Utterance,
        range: Range<usize>,
        emit: &mut dyn FnMut(DictationEvent),
    ) -> Result<bool, String> {
        let flag = signals.begin_run(None);
        let result = rec.recognize(self.slice(range.clone()), &flag);
        signals.end_run();
        if flag.load(Ordering::SeqCst) {
            return Ok(false);
        }
        let rest = u.agreement.finish(words(&result?));
        if !rest.is_empty() {
            self.report.words += rest.len();
            emit(DictationEvent::Committed {
                text: rest.join(" "),
                utterance: u.index,
                at_pause: true,
            });
        }
        let text = u.agreement.committed().join(" ");
        if text.is_empty() {
            // Speech with no words: said, rather than silence.
            emit(DictationEvent::NoWords { utterance: u.index });
        } else {
            let ms = |s: usize| s as u64 * 1000 / u64::from(WHISPER_SAMPLE_RATE);
            let segment = Segment {
                start_ms: ms(range.start),
                end_ms: ms(range.end),
                text,
            };
            emit(DictationEvent::Partial(segment.clone()));
            self.report.transcript.segments.push(segment);
        }
        Ok(true)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::vad::{FRAME, ScoredFinder, VadConfig};

    fn split(s: &str) -> Vec<String> {
        s.split(' ').map(String::from).collect()
    }

    #[test]
    fn words_compare_loosely() {
        assert_eq!(norm("Chapter."), "chapter");
        assert_eq!(norm("don't,"), "don't");
        assert_eq!(common_prefix(&split("a B."), &split("A b c")), 2);
    }

    /// The spike's fix: "until 9" committed early, then heard as "until
    /// 9am"; the committed "to" must not match the wrong "to".
    #[test]
    fn a_changed_early_word_keeps_the_rest() {
        let cw = split("open until 9 in the evening and to");
        let hyp =
            split("open until 9am in the evening and to add more lamps where people liked to sit");
        assert_eq!(continuation(&cw, &hyp), 8);
        assert_eq!(continuation(&cw[..2], &hyp), 2);
        assert_eq!(continuation(&[], &hyp), 0);
        assert_eq!(continuation(&cw, &[]), 0);
        let mut a = Agreement::new();
        a.committed = cw;
        assert_eq!(
            a.finish(hyp).join(" "),
            "add more lamps where people liked to sit"
        );
    }

    #[test]
    fn agreement_commits_what_two_runs_share() {
        let mut a = Agreement::new();
        assert!(a.partial(split("When the lib")).is_empty());
        assert_eq!(a.partial(split("When the Library of")), split("When the"));
        // An empty run is no information.
        assert!(a.partial(Vec::new()).is_empty());
        // "Library" and "library" agree; "of" and "opened" do not.
        assert_eq!(
            a.partial(split("When the library opened its")),
            split("library")
        );
        assert_eq!(a.committed(), split("When the library").as_slice());
        assert_eq!(
            a.partial(split("When the library opened its new")),
            split("opened its")
        );
        assert_eq!(
            a.finish(split("When the library opened its new room.")),
            split("new room.")
        );
        assert_eq!(
            a.committed().join(" "),
            "When the library opened its new room."
        );
    }

    // A fake speaker and a fake recognizer. A word is a burst of 20
    // frames (320 ms) at a level that names it, with 5 frames (80 ms) of
    // near silence after it; silence is zero. The recognizer reads the
    // levels back: a whole burst is its word, a burst cut short by the end
    // of the audio is the first half of its letters, as Whisper hears a
    // word cut mid-way.

    pub(crate) const VOCABULARY: [&str; 24] = [
        "The", "tram", "was", "late", "again", "this", "morning.", "Open", "the", "second",
        "chapter", "and", "read", "it", "aloud", "slowly,", "please.", "When", "library", "opened",
        "its", "new", "reading", "room.",
    ];
    const WORD_FRAMES: usize = 20;
    const GAP_FRAMES: usize = 5;

    fn level(word: usize) -> f32 {
        0.3 + 0.02 * word as f32
    }

    /// Audio for utterances of vocabulary words, each followed by a pause
    /// of the given milliseconds, after `lead_ms` of silence.
    pub(crate) fn speak(lead_ms: usize, utterances: &[(&[usize], usize)]) -> Vec<f32> {
        let ms = |m: usize| m * 16;
        let mut a = vec![0.0; ms(lead_ms)];
        for (words, pause) in utterances {
            for &w in *words {
                a.extend(std::iter::repeat_n(level(w), WORD_FRAMES * FRAME));
                a.extend(std::iter::repeat_n(0.01, GAP_FRAMES * FRAME));
            }
            a.extend(std::iter::repeat_n(0.0, ms(*pause)));
        }
        a
    }

    /// What the fake recognizer hears in `audio`: each run of samples at
    /// a word's level is that word, or the first half of its letters when
    /// the run is shorter than a word (cut by the end of the audio).
    pub(crate) fn hear(audio: &[f32]) -> Vec<Segment> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < audio.len() {
            let v = audio[i];
            let n = audio[i..].iter().take_while(|&&x| x == v).count();
            let word = ((v - 0.3) / 0.02).round();
            if v >= 0.25 && (0.0..VOCABULARY.len() as f32).contains(&word) {
                let word = VOCABULARY[word as usize];
                let text = if n >= WORD_FRAMES * FRAME {
                    word.to_owned()
                } else {
                    word.chars()
                        .take(word.chars().count().div_ceil(2))
                        .collect()
                };
                out.push(Segment {
                    start_ms: 0,
                    end_ms: 0,
                    text,
                });
            }
            i += n;
        }
        out
    }

    /// Speech is loud: the finder the fake speaker needs.
    pub(crate) fn loud_finder() -> ScoredFinder<fn(&[f32; FRAME]) -> f32> {
        fn loudness(f: &[f32; FRAME]) -> f32 {
            if f.iter().sum::<f32>() / FRAME as f32 >= 0.25 {
                1.0
            } else {
                0.0
            }
        }
        ScoredFinder::new(VadConfig::default(), loudness as fn(&[f32; FRAME]) -> f32)
    }

    #[test]
    fn the_fake_hears_whole_and_cut_words() {
        let a = speak(100, &[(&[7, 8, 9], 700)]);
        let text = |a: &[f32]| {
            hear(a)
                .into_iter()
                .map(|s| s.text)
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(text(&a), "Open the second");
        // Cut 10 frames into "second".
        let cut = 100 * 16 + (2 * (WORD_FRAMES + GAP_FRAMES) + 10) * FRAME + 7;
        assert_eq!(text(&a[..cut]), "Open the sec");
    }

    // Virtual time: the listener's messages, stamped with the time they
    // would arrive, and a recognizer whose runs take a set time.

    struct Timeline {
        now: f64,
        /// Time, message, and whether the listener has signaled it (for
        /// pauses).
        msgs: VecDeque<(f64, Msg, bool)>,
        signals: Arc<Signals>,
    }

    fn is_pause(m: &Msg) -> bool {
        matches!(m, Msg::End | Msg::Speech(SpeechEvent::Closed(_)))
    }

    impl Timeline {
        /// Messages for `audio` as a microphone and the listener would
        /// send them: 16 ms pieces, each followed by what the finder found.
        fn new(audio: &[f32], signals: Arc<Signals>) -> Rc<RefCell<Timeline>> {
            let mut finder = loud_finder();
            let mut msgs = VecDeque::new();
            let mut t = 0.0;
            for piece in audio.chunks(FRAME) {
                t += piece.len() as f64 / 16_000.0;
                let events = finder.push(piece);
                let audio = Msg::Audio {
                    samples: piece.to_vec(),
                    speech_end: finder.speech_end(),
                };
                msgs.push_back((t, audio, false));
                msgs.extend(events.into_iter().map(|e| (t, Msg::Speech(e), false)));
            }
            msgs.extend(
                finder
                    .finish()
                    .into_iter()
                    .map(|e| (t, Msg::Speech(e), false)),
            );
            msgs.push_back((t, Msg::End, false));
            Rc::new(RefCell::new(Timeline {
                now: 0.0,
                msgs,
                signals,
            }))
        }

        /// When each utterance's pause is found.
        fn pauses(&self) -> Vec<f64> {
            self.msgs
                .iter()
                .filter(|(_, m, _)| matches!(m, Msg::Speech(SpeechEvent::Closed(_))))
                .map(|(t, _, _)| *t)
                .collect()
        }

        fn pop(&mut self) -> Next {
            match self.msgs.pop_front() {
                Some((_, m, signaled)) => {
                    // The listener signals a pause as it finds it.
                    if is_pause(&m) && !signaled {
                        self.signals.pause();
                    }
                    Next::Msg(m)
                }
                None => Next::Gone,
            }
        }
    }

    struct VirtualFeed(Rc<RefCell<Timeline>>);

    impl Feed for VirtualFeed {
        fn next(&mut self, wait: bool) -> Next {
            let mut tl = self.0.borrow_mut();
            match tl.msgs.front() {
                None => Next::Gone,
                Some(&(t, _, _)) if t <= tl.now + 1e-9 => tl.pop(),
                Some(&(t, _, _)) if wait => {
                    tl.now = t;
                    tl.pop()
                }
                Some(_) => Next::Empty,
            }
        }

        fn now(&self) -> Duration {
            Duration::from_secs_f64(self.0.borrow().now)
        }
    }

    /// Runs cost `fixed` seconds (the part that cannot be cancelled, like
    /// Whisper's encoder) plus `per_word` for each word heard.
    struct VirtualRecognizer {
        tl: Rc<RefCell<Timeline>>,
        fixed: f64,
        per_word: f64,
        runs: usize,
        /// Runs (counted from 1) that return nothing.
        empty: Vec<usize>,
    }

    impl Recognizer for VirtualRecognizer {
        fn recognize(
            &mut self,
            audio: &[f32],
            cancel: &AtomicBool,
        ) -> Result<Vec<Segment>, String> {
            self.runs += 1;
            let segments = hear(audio);
            let mut tl = self.tl.borrow_mut();
            let t0 = tl.now;
            let end = t0 + self.fixed + self.per_word * segments.len() as f64;
            // The listener goes on while the run does: pauses it finds
            // meanwhile are signaled at once, which cancels a partial run.
            let signals = Arc::clone(&tl.signals);
            let mut first_pause = None;
            for (t, m, signaled) in tl.msgs.iter_mut() {
                if *t >= end {
                    break;
                }
                if is_pause(m) && !*signaled {
                    *signaled = true;
                    signals.pause();
                    first_pause.get_or_insert(*t);
                }
            }
            tl.now = match first_pause {
                // Stopped at the pause, or once the encoder is done.
                Some(tp) if cancel.load(Ordering::SeqCst) => tp.max(t0 + self.fixed),
                _ => end,
            };
            if self.empty.contains(&self.runs) {
                return Ok(Vec::new());
            }
            Ok(segments)
        }
    }

    /// A simulated session.
    pub(crate) struct Sim {
        /// Events with the virtual time they were emitted.
        pub events: Vec<(f64, DictationEvent)>,
        pub report: LiveReport,
        /// When each utterance's pause was found.
        pub pauses: Vec<f64>,
        /// Recognizer runs.
        pub runs: usize,
    }

    impl Sim {
        pub(crate) fn text(&self) -> String {
            self.report.transcript.text()
        }

        /// Committed events: time, text, utterance, at the pause.
        pub(crate) fn commits(&self) -> Vec<(f64, &str, usize, bool)> {
            self.events
                .iter()
                .filter_map(|(t, e)| match e {
                    DictationEvent::Committed {
                        text,
                        utterance,
                        at_pause,
                    } => Some((*t, text.as_str(), *utterance, *at_pause)),
                    _ => None,
                })
                .collect()
        }

        /// When each utterance's last words were committed.
        pub(crate) fn finals(&self) -> Vec<f64> {
            let mut out: Vec<f64> = Vec::new();
            for (t, _, u, _) in self.commits() {
                if u == out.len() {
                    out.push(t);
                } else if let Some(last) = out.get_mut(u) {
                    *last = t;
                }
            }
            out
        }
    }

    pub(crate) fn simulate(
        audio: &[f32],
        config: &StreamConfig,
        fixed: f64,
        per_word: f64,
        empty: Vec<usize>,
    ) -> Sim {
        let signals = Arc::new(Signals::default());
        let tl = Timeline::new(audio, Arc::clone(&signals));
        let pauses = tl.borrow().pauses();
        let mut rec = VirtualRecognizer {
            tl: Rc::clone(&tl),
            fixed,
            per_word,
            runs: 0,
            empty,
        };
        let mut feed = VirtualFeed(Rc::clone(&tl));
        let mut events = Vec::new();
        let report = run_live(&mut rec, &mut feed, &signals, config, &mut |e| {
            events.push((tl.borrow().now, e));
        })
        .unwrap()
        .unwrap();
        Sim {
            events,
            report,
            pauses,
            runs: rec.runs,
        }
    }

    /// Only the last run at each pause: today's behavior, per utterance.
    pub(crate) fn batch() -> StreamConfig {
        StreamConfig::pauses_only()
    }

    /// Twelve words, 4.8 seconds.
    pub(crate) const LONG: &[usize] = &[17, 8, 18, 19, 20, 21, 22, 23, 0, 1, 2, 3];
    pub(crate) const LONG_TEXT: &str =
        "When the library opened its new reading room. The tram was late";

    fn full_word(w: &str) -> bool {
        VOCABULARY.contains(&w)
    }

    #[test]
    fn words_are_committed_before_the_pause() {
        let audio = speak(300, &[(LONG, 1000)]);
        let live = simulate(&audio, &StreamConfig::default(), 0.1, 0.01, Vec::new());
        assert_eq!(live.text(), LONG_TEXT);
        assert_eq!(
            simulate(&audio, &batch(), 0.1, 0.01, Vec::new()).text(),
            LONG_TEXT
        );
        let commits = live.commits();
        let early: Vec<_> = commits.iter().filter(|c| !c.3).collect();
        assert!(!early.is_empty(), "{commits:?}");
        for (t, words, _, _) in &early {
            assert!(
                *t < live.pauses[0],
                "committed after the pause: {commits:?}"
            );
            // Never the half-heard word at the end.
            assert!(words.split(' ').all(full_word), "{commits:?}");
        }
        assert_eq!(live.report.words, 12);
        // Committed words add up to the text, in order.
        let joined: Vec<&str> = commits.iter().map(|c| c.1).collect();
        assert_eq!(joined.join(" "), LONG_TEXT);
    }

    #[test]
    fn an_empty_run_is_no_information() {
        let audio = speak(300, &[(LONG, 1000)]);
        let s = simulate(&audio, &StreamConfig::default(), 0.1, 0.01, vec![2, 3]);
        assert_eq!(s.text(), LONG_TEXT);
        assert!(s.commits().iter().all(|c| c.1.split(' ').all(full_word)));
    }

    #[test]
    fn a_run_going_at_the_pause_is_cancelled() {
        let audio = speak(300, &[(LONG, 1000)]);
        // Runs just under the gate's limit: the second is still going when
        // the pause is found.
        let (fixed, per_word) = (1.0, 0.04);
        let s = simulate(
            &audio,
            &StreamConfig::default(),
            fixed,
            per_word,
            Vec::new(),
        );
        assert!(s.report.cancelled_runs >= 1, "{:?}", s.report);
        assert_eq!(s.text(), LONG_TEXT);
        // The cancelled run stopped once its fixed part was done, then the
        // last run took its own time.
        let last = s.finals()[0];
        let bound = s.pauses[0] + fixed + fixed + per_word * 12.0 + 1e-6;
        assert!(last <= bound, "{last} > {bound}");
    }

    #[test]
    fn utterances_are_numbered_and_kept_apart() {
        let audio = speak(
            200,
            &[
                (&[7, 8, 9, 10], 900),
                (&[0, 1, 2, 3, 4], 900),
                (&[11, 12, 13], 900),
            ],
        );
        let s = simulate(&audio, &StreamConfig::default(), 0.1, 0.01, Vec::new());
        let segments: Vec<&str> = s
            .report
            .transcript
            .segments
            .iter()
            .map(|x| x.text.as_str())
            .collect();
        assert_eq!(
            segments,
            [
                "Open the second chapter",
                "The tram was late again",
                "and read it"
            ]
        );
        let order: Vec<usize> = s.commits().iter().map(|c| c.2).collect();
        assert!(order.windows(2).all(|w| w[0] <= w[1]), "{order:?}");
        assert_eq!(s.report.utterances, 3);
        // One Partial per utterance.
        let partials = s
            .events
            .iter()
            .filter(|(_, e)| matches!(e, DictationEvent::Partial(_)))
            .count();
        assert_eq!(partials, 3);
    }

    #[test]
    fn partial_runs_wait_for_three_seconds_of_utterance() {
        let audio = speak(300, &[(LONG, 1000)]);
        let s = simulate(&audio, &StreamConfig::default(), 0.1, 0.01, Vec::new());
        // The utterance starts 200 ms before its speech (its padding); two
        // runs agree at the earliest 3 seconds and one step in.
        let start = 0.3 - 0.2;
        let first = s.commits()[0].0;
        assert!(first >= start + 3.0 + 0.7, "first words at {first}");
        assert!(s.report.early_words > 0, "{:?}", s.report);
    }

    #[test]
    fn a_slow_run_waits_for_the_pause() {
        let words: Vec<usize> = LONG.iter().chain(LONG).copied().collect();
        let audio = speak(300, &[(&words, 1000)]);
        // Every run takes 1.6 seconds, over the 1.5-second limit.
        let s = simulate(&audio, &StreamConfig::default(), 1.6, 0.0, Vec::new());
        assert_eq!(s.report.partial_runs, 1, "{:?}", s.report);
        assert_eq!(s.report.early_words, 0);
        assert_eq!(s.text(), format!("{LONG_TEXT} {LONG_TEXT}"));
    }

    /// Short dictation (utterances under three seconds) is never later
    /// than transcribing each utterance at its pause, however slow the
    /// recognizer: no partial run happens at all.
    #[test]
    fn short_dictation_is_never_later_than_batch() {
        let script: [(&[usize], usize); 5] = [
            (&[0, 1, 2, 3], 800),
            (&[7, 8, 9, 10, 11, 12], 700),
            (&[4, 5], 1500),
            (&[13, 14, 15, 16], 650),
            (&[17, 8, 18], 900),
        ];
        let audio = speak(250, &script);
        for (fixed, per_word) in [(0.05, 0.0), (0.3, 0.02), (0.8, 0.1), (2.5, 0.3)] {
            let live = simulate(
                &audio,
                &StreamConfig::default(),
                fixed,
                per_word,
                Vec::new(),
            );
            let batch = simulate(&audio, &batch(), fixed, per_word, Vec::new());
            assert_eq!(live.text(), batch.text());
            assert_eq!(live.report.partial_runs, 0, "costs {fixed} + {per_word}");
            assert_eq!(live.runs, batch.runs);
            let (l, b) = (live.finals(), batch.finals());
            assert_eq!(l.len(), 5);
            for (u, (lt, bt)) in l.iter().zip(&b).enumerate() {
                assert!(lt <= bt, "utterance {u}: live {lt}, batch {bt}");
            }
        }
    }

    #[test]
    fn a_phrase_with_no_words_is_said() {
        // Speech at a level that names no word: heard, not recognized.
        let mut audio = vec![0.0f32; 16_000 / 4];
        audio.extend(std::iter::repeat_n(0.27, 16_000));
        audio.extend(std::iter::repeat_n(0.0, 16_000));
        let s = simulate(&audio, &StreamConfig::default(), 0.1, 0.0, Vec::new());
        let said: Vec<&DictationEvent> = s.events.iter().map(|(_, e)| e).collect();
        assert_eq!(said, [&DictationEvent::NoWords { utterance: 0 }]);
        assert!(s.report.transcript.is_empty());
    }

    #[test]
    fn silence_is_not_kept() {
        // A minute of silence, then a phrase: only a few seconds of audio
        // are ever held.
        let audio = speak(60_000, &[(&[7, 8, 9, 10], 900)]);
        let s = simulate(&audio, &StreamConfig::default(), 0.1, 0.01, Vec::new());
        assert_eq!(s.text(), "Open the second chapter");
        assert!(
            s.report.peak_audio < 16_000 * 6,
            "held {} samples",
            s.report.peak_audio
        );
    }

    #[test]
    fn a_cancelled_session_stops() {
        let audio = speak(100, &[(LONG, 800)]);
        let signals = Arc::new(Signals::default());
        let tl = Timeline::new(&audio, Arc::clone(&signals));
        signals.cancel();
        let mut rec = VirtualRecognizer {
            tl: Rc::clone(&tl),
            fixed: 0.1,
            per_word: 0.0,
            runs: 0,
            empty: Vec::new(),
        };
        let r = run_live(
            &mut rec,
            &mut VirtualFeed(Rc::clone(&tl)),
            &signals,
            &StreamConfig::default(),
            &mut |_| {},
        );
        assert_eq!(r, Ok(None));
        assert_eq!(rec.runs, 0);
    }

    #[test]
    fn silence_commits_nothing() {
        let audio = vec![0.0; 16_000 * 2];
        let s = simulate(&audio, &StreamConfig::default(), 0.1, 0.01, Vec::new());
        assert!(s.events.is_empty());
        assert!(s.report.transcript.is_empty());
        assert_eq!(s.report.samples, audio.len());
        assert_eq!(s.runs, 0);
    }

    /// The fake recognizer in real time: sleeps its cost, and stops early
    /// when cancelled.
    pub(crate) struct SleepyRecognizer {
        pub fixed: Duration,
        pub per_word: Duration,
    }

    impl Recognizer for SleepyRecognizer {
        fn recognize(
            &mut self,
            audio: &[f32],
            cancel: &AtomicBool,
        ) -> Result<Vec<Segment>, String> {
            let segments = hear(audio);
            let t0 = Instant::now();
            let fixed_end = t0 + self.fixed;
            let end = fixed_end + self.per_word * u32::try_from(segments.len()).unwrap_or(0);
            loop {
                let now = Instant::now();
                if now >= end || (now >= fixed_end && cancel.load(Ordering::SeqCst)) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(segments)
        }
    }

    /// The whole live path in real time: audio played at speaking pace,
    /// the listener thread, and the session loop, with the fake
    /// recognizer. Timing depends on the machine, so the checks are
    /// loose: the text is right, and words arrive before the audio ends.
    #[test]
    fn live_in_real_time() {
        use crate::capture::{AudioCapture, PacedCapture};
        let audio = speak(200, &[(LONG, 700)]);
        let mut capture = PacedCapture::new(audio);
        let live = capture.live().unwrap();
        let signals = Arc::new(Signals::default());
        let (rx, listener) =
            spawn_listener(live, Box::new(loud_finder()), Arc::clone(&signals)).unwrap();
        capture.start().unwrap();
        let mut feed = ChannelFeed::new(rx);
        let mut rec = SleepyRecognizer {
            fixed: Duration::from_millis(60),
            per_word: Duration::from_millis(5),
        };
        let mut events = Vec::new();
        let t0 = Instant::now();
        let report = run_live(
            &mut rec,
            &mut feed,
            &signals,
            &StreamConfig::default(),
            &mut |e| {
                events.push((t0.elapsed(), e));
            },
        )
        .unwrap()
        .unwrap();
        listener.join().unwrap();
        assert_eq!(report.transcript.text(), LONG_TEXT);
        let first = events
            .iter()
            .find(|(_, e)| matches!(e, DictationEvent::Committed { .. }))
            .map(|(t, _)| *t)
            .unwrap();
        let ended = report.ended_at.unwrap();
        assert!(
            first < ended,
            "first words at {first:?}, audio ended at {ended:?}"
        );
        assert!(report.early_words > 0, "{report:?}");
    }
}
