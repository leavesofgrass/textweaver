//! The streaming dictation spike (Agent W5d): measures whether live
//! partial results are possible with Whisper `base.en` on RTen.
//!
//! It never opens a microphone and never plays audio: the speech comes
//! from WAV files written by `tw export-audio` (in `fixtures/d/`), with
//! word-level SRT subtitles as the reference word times.
//!
//! Commands:
//!
//! - `bench [--runs N] [--audio WAV]`: the spectrogram, encoder and
//!   decoder times for 1, 2, 4 and 8 seconds of speech, N runs each
//!   (default 40), median and worst, with the process's memory.
//! - `encoder-lengths`: whether the encoder accepts less than 30 seconds.
//! - `stream [--step-ms MS] [--trim] [--repeat N] WAV...`: LocalAgreement-2
//!   inside earshot utterances, with Whisper; commit latency from word end
//!   to commit, rewrites avoided, and the final text against the batch
//!   transcript and the reference.
//! - `fake [--step-ms MS] [--encode-ms MS] [--word-ms MS] WAV...`: the same
//!   loop with a fake recognizer driven by the reference word times, so
//!   the agreement, commit and announce path can be checked without a
//!   model.
//!
//! The model folder is `--model DIR`, or `TEXTWEAVER_WHISPER_RTEN`. Run:
//!
//! ```text
//! cargo run -p textweaver-dictation --features rten --release \
//!     --example stream_probe -- --model <dir> bench
//! ```
//!
//! Time in the stream simulation is virtual: audio "arrives" in real
//! time, a run starts when the previous one has finished and at least one
//! step of new audio is there, and each run takes the wall time it really
//! took on this machine. Owner: Agent W5d.

// Without a feature only the agreement logic and its tests are built.
#![cfg_attr(not(any(feature = "rten", feature = "mic")), allow(dead_code))]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// LocalAgreement-2 and the streaming simulation, independent of any
/// model (so the tests run without the `rten` feature).
mod la {
    /// Samples per second.
    pub const RATE: f64 = 16_000.0;

    /// A piece of recognized text, times in seconds from the start of the
    /// audio given to the recognizer.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Seg {
        pub start: f64,
        pub end: f64,
        pub text: String,
    }

    /// Something that turns a stretch of audio into text.
    pub trait Recognizer {
        /// Recognizes samples `start..end`; returns the segments and the
        /// seconds the work took.
        fn recognize(&mut self, start: usize, end: usize) -> (Vec<Seg>, f64);

        /// The seconds of the last run that could not have been cancelled
        /// (the spectrogram and the encoder; the decoder stops between
        /// tokens).
        fn last_fixed(&self) -> f64 {
            0.0
        }
    }

    /// A reference word: its text and times in seconds.
    #[derive(Clone, Debug, PartialEq)]
    pub struct RefWord {
        pub text: String,
        pub start: f64,
        pub end: f64,
    }

    /// Lower case, letters, digits and apostrophes only.
    pub fn norm(w: &str) -> String {
        w.chars()
            .filter(|c| c.is_alphanumeric() || *c == '\'')
            .flat_map(char::to_lowercase)
            .collect()
    }

    /// The words of some segments.
    pub fn words(segs: &[Seg]) -> Vec<String> {
        segs.iter()
            .flat_map(|s| s.text.split_whitespace().map(str::to_owned))
            .filter(|w| !norm(w).is_empty())
            .collect()
    }

    /// How many leading words two hypotheses agree on (normalized).
    pub fn common_prefix(a: &[String], b: &[String]) -> usize {
        a.iter()
            .zip(b)
            .take_while(|(x, y)| norm(x) == norm(y))
            .count()
    }

    /// Where a hypothesis goes on after the committed words `cw`: after
    /// them when it starts with them, otherwise after the last word the
    /// two share (Whisper sometimes rewrites an early word; the committed
    /// text is never changed).
    pub fn continuation(cw: &[String], hyp: &[String]) -> usize {
        if common_prefix(cw, hyp) == cw.len() {
            return cw.len().min(hyp.len());
        }
        // The prefix of the hypothesis closest to the committed words (the
        // edit distance of all of `cw` against `hyp[..p]`, least `p` on a
        // tie is the one nearest `cw.len()`).
        let c: Vec<String> = cw.iter().map(|w| norm(w)).collect();
        let h: Vec<String> = hyp.iter().map(|w| norm(w)).collect();
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

    /// A committed word and when it was committed (virtual seconds).
    #[derive(Clone, Debug, PartialEq)]
    pub struct Commit {
        pub word: String,
        pub at: f64,
        /// Committed at the pause (the utterance's final run), not by
        /// agreement.
        pub at_pause: bool,
    }

    /// What one simulated session produced.
    #[derive(Clone, Debug, Default)]
    pub struct StreamResult {
        pub commits: Vec<Commit>,
        /// Announcements: one per commit event, the words committed then.
        pub announcements: Vec<(f64, String)>,
        /// Recognizer runs.
        pub runs: usize,
        /// Seconds of recognizer work per run.
        pub run_costs: Vec<f64>,
        /// Runs that returned no words after an earlier run had some
        /// (skipped for agreement).
        pub empty_runs: usize,
        /// Runs cancelled because the pause came while they ran.
        pub cancelled_runs: usize,
        /// Words a naive "show every hypothesis" display would have shown
        /// and then changed.
        pub rewrites_avoided: usize,
        /// Committed words the utterance's final run disagreed with (they
        /// stay; this counts commits a batch run would have written
        /// differently).
        pub late_disagreements: usize,
        /// Per utterance: seconds from the pause being detected to the
        /// last word committed.
        pub pause_to_final: Vec<f64>,
    }

    /// Streaming settings.
    #[derive(Clone, Copy, Debug)]
    pub struct StreamConfig {
        /// New audio needed before the next run.
        pub step: f64,
        /// Trim the buffer at a committed sentence end.
        pub trim: bool,
        /// Seconds after an utterance's (padded) end at which the pause is
        /// known: the detector's 600 ms minimum silence less its 200 ms
        /// padding.
        pub pause_after_end: f64,
        /// Cancel a partial run still going when the pause is found.
        pub cancel_at_pause: bool,
    }

    impl Default for StreamConfig {
        fn default() -> Self {
            StreamConfig {
                step: 0.7,
                trim: false,
                pause_after_end: 0.4,
                cancel_at_pause: true,
            }
        }
    }

    fn secs(samples: usize) -> f64 {
        samples as f64 / RATE
    }

    fn at_sample(t: f64) -> usize {
        (t * RATE).round().max(0.0) as usize
    }

    fn sentence_end(w: &str) -> bool {
        w.ends_with(['.', '?', '!'])
    }

    /// Runs LocalAgreement-2 over `utterances` (sample ranges) of audio
    /// `total` samples long.
    pub fn simulate(
        rec: &mut dyn Recognizer,
        utterances: &[std::ops::Range<usize>],
        total: usize,
        config: &StreamConfig,
    ) -> StreamResult {
        let mut out = StreamResult::default();
        let mut clock = 0.0f64;
        for u in utterances {
            let u_start = secs(u.start);
            let u_end = secs(u.end);
            let pause_known = (u_end + config.pause_after_end).min(secs(total).max(u_end));
            let mut buf_start = u.start;
            let mut prev: Vec<String> = Vec::new();
            // The current buffer's words already committed.
            let mut cw: Vec<String> = Vec::new();
            let mut last_audio_end = u_start;
            clock = clock.max(u_start);
            loop {
                let next = last_audio_end + config.step;
                clock = clock.max(next);
                if clock >= pause_known || next >= u_end {
                    break;
                }
                let avail = clock.min(u_end);
                last_audio_end = avail;
                let (segs, cost) = rec.recognize(buf_start, at_sample(avail));
                out.runs += 1;
                out.run_costs.push(cost);
                let started = clock;
                clock += cost;
                if config.cancel_at_pause && clock > pause_known {
                    // The pause came during this run: it is cancelled once
                    // its encoder is done, and the final run starts.
                    clock = pause_known.max(started + rec.last_fixed());
                    out.cancelled_runs += 1;
                    break;
                }
                let hyp = words(&segs);
                if hyp.is_empty() && !prev.is_empty() {
                    // Whisper sometimes returns nothing for a buffer cut
                    // mid-word; that is no evidence against the last run.
                    out.empty_runs += 1;
                    continue;
                }
                // Words a naive display showed and now changes.
                out.rewrites_avoided += prev.len() - common_prefix(&prev, &hyp);
                // LocalAgreement-2: commit what this run and the last one
                // agree on after the committed words.
                let (sp, sh) = (continuation(&cw, &prev), continuation(&cw, &hyp));
                let agreed = common_prefix(&prev[sp..], &hyp[sh..]);
                if agreed > 0 {
                    let new: Vec<String> = hyp[sh..sh + agreed].to_vec();
                    for w in &new {
                        out.commits.push(Commit {
                            word: w.clone(),
                            at: clock,
                            at_pause: false,
                        });
                    }
                    out.announcements.push((clock, new.join(" ")));
                    cw.extend(new);
                }
                prev = hyp;
                if config.trim && common_prefix(&cw, &prev) == cw.len() {
                    // Cut at the end of the last segment that is wholly
                    // committed and ends a sentence.
                    let mut n = 0usize;
                    let mut cut: Option<(f64, usize)> = None;
                    for s in &segs {
                        let k = words(std::slice::from_ref(s)).len();
                        if n + k > cw.len() {
                            break;
                        }
                        n += k;
                        if s.text.split_whitespace().last().is_some_and(sentence_end) {
                            cut = Some((s.end, n));
                        }
                    }
                    if let Some((end, n)) = cut
                        && end > 0.0
                    {
                        buf_start = (buf_start + at_sample(end)).min(u.end);
                        cw.drain(..n);
                        prev.drain(..n.min(prev.len()));
                    }
                }
            }
            // The pause: one last run over the whole buffer, everything
            // committed.
            clock = clock.max(pause_known);
            let (segs, cost) = rec.recognize(buf_start, u.end);
            out.runs += 1;
            out.run_costs.push(cost);
            clock += cost;
            let hyp = words(&segs);
            out.late_disagreements += cw.len() - common_prefix(&cw, &hyp).min(cw.len());
            out.rewrites_avoided += prev.len() - common_prefix(&prev, &hyp);
            let sh = continuation(&cw, &hyp);
            if hyp.len() > sh {
                let new: Vec<String> = hyp[sh..].to_vec();
                for w in &new {
                    out.commits.push(Commit {
                        word: w.clone(),
                        at: clock,
                        at_pause: true,
                    });
                }
                out.announcements.push((clock, new.join(" ")));
            }
            out.pause_to_final.push(clock - pause_known);
        }
        out
    }

    /// The batch path textweaver has today: each utterance transcribed
    /// once after its pause. Returns the words and, per utterance, the
    /// seconds from the pause to the text.
    pub fn batch(
        rec: &mut dyn Recognizer,
        utterances: &[std::ops::Range<usize>],
        total: usize,
        config: &StreamConfig,
    ) -> (Vec<String>, Vec<f64>) {
        let mut all = Vec::new();
        let mut waits = Vec::new();
        let mut clock = 0.0f64;
        for u in utterances {
            let pause_known = (secs(u.end) + config.pause_after_end).min(secs(total).max(secs(u.end)));
            clock = clock.max(pause_known);
            let (segs, cost) = rec.recognize(u.start, u.end);
            clock += cost;
            waits.push(clock - pause_known);
            all.extend(words(&segs));
        }
        (all, waits)
    }

    /// Word-level edit distance, and the pairs of equal words
    /// (hypothesis index, reference index).
    pub fn align(hyp: &[String], reference: &[String]) -> (usize, Vec<(usize, usize)>) {
        let h: Vec<String> = hyp.iter().map(|w| norm(w)).collect();
        let r: Vec<String> = reference.iter().map(|w| norm(w)).collect();
        let (n, m) = (h.len(), r.len());
        let mut d = vec![vec![0usize; m + 1]; n + 1];
        for (i, row) in d.iter_mut().enumerate() {
            row[0] = i;
        }
        for j in 0..=m {
            d[0][j] = j;
        }
        for i in 1..=n {
            for j in 1..=m {
                let sub = d[i - 1][j - 1] + usize::from(h[i - 1] != r[j - 1]);
                d[i][j] = sub.min(d[i - 1][j] + 1).min(d[i][j - 1] + 1);
            }
        }
        let mut pairs = Vec::new();
        let (mut i, mut j) = (n, m);
        while i > 0 && j > 0 {
            // Matches first, then insertions and deletions, then
            // substitutions, so equal words pair up when costs tie.
            if h[i - 1] == r[j - 1] && d[i][j] == d[i - 1][j - 1] {
                pairs.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            } else if d[i][j] == d[i - 1][j] + 1 {
                i -= 1;
            } else if d[i][j] == d[i][j - 1] + 1 {
                j -= 1;
            } else {
                i -= 1;
                j -= 1;
            }
        }
        pairs.reverse();
        (d[n][m], pairs)
    }

    /// Word error rate of `hyp` against `reference`, in percent.
    pub fn wer(hyp: &[String], reference: &[String]) -> f64 {
        if reference.is_empty() {
            return 0.0;
        }
        align(hyp, reference).0 as f64 * 100.0 / reference.len() as f64
    }

    /// Seconds from each reference word's end to its commit, for the
    /// committed words that match the reference.
    pub fn commit_latencies(commits: &[Commit], reference: &[RefWord]) -> Vec<(f64, bool)> {
        let hyp: Vec<String> = commits.iter().map(|c| c.word.clone()).collect();
        let r: Vec<String> = reference.iter().map(|w| w.text.clone()).collect();
        align(&hyp, &r)
            .1
            .into_iter()
            .map(|(i, j)| (commits[i].at - reference[j].end, commits[i].at_pause))
            .collect()
    }

    /// A recognizer that knows the answer: it returns the reference words
    /// that end inside the audio it is given, and the word being spoken at
    /// the buffer's end cut short (so the tail is unstable, as Whisper's
    /// is). Its cost is `encode + word * words`.
    pub struct FakeRecognizer {
        pub words: Vec<RefWord>,
        pub encode: f64,
        pub word: f64,
    }

    impl Recognizer for FakeRecognizer {
        fn recognize(&mut self, start: usize, end: usize) -> (Vec<Seg>, f64) {
            let (a, b) = (secs(start), secs(end));
            let mut segs = Vec::new();
            for w in &self.words {
                if w.start + 1e-9 < a || w.start >= b {
                    continue;
                }
                let text = if w.end <= b + 1e-9 {
                    w.text.clone()
                } else {
                    // Half heard: the first half of the letters.
                    let n = w.text.chars().count().div_ceil(2);
                    w.text.chars().take(n).collect()
                };
                segs.push(Seg {
                    start: w.start - a,
                    end: w.end.min(b) - a,
                    text,
                });
            }
            let cost = self.encode + self.word * segs.len() as f64;
            (segs, cost)
        }

        fn last_fixed(&self) -> f64 {
            self.encode
        }
    }

    /// Median and worst of some values.
    pub fn median_worst(v: &[f64]) -> (f64, f64) {
        if v.is_empty() {
            return (0.0, 0.0);
        }
        let mut s = v.to_vec();
        s.sort_by(f64::total_cmp);
        let mid = s.len() / 2;
        let median = if s.len() % 2 == 0 {
            (s[mid - 1] + s[mid]) / 2.0
        } else {
            s[mid]
        };
        (median, s[s.len() - 1])
    }

    /// The value below which `p` percent of `v` falls (nearest rank).
    pub fn percentile(v: &[f64], p: f64) -> f64 {
        if v.is_empty() {
            return 0.0;
        }
        let mut s = v.to_vec();
        s.sort_by(f64::total_cmp);
        let rank = ((p / 100.0) * s.len() as f64).ceil().max(1.0) as usize;
        s[rank.min(s.len()) - 1]
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn w(text: &str, start: f64, end: f64) -> RefWord {
            RefWord {
                text: text.into(),
                start,
                end,
            }
        }

        fn sentence() -> Vec<RefWord> {
            vec![
                w("Open", 0.2, 0.6),
                w("the", 0.6, 0.8),
                w("second", 0.8, 1.3),
                w("chapter.", 1.3, 2.0),
            ]
        }

        #[test]
        fn normalizes_words() {
            assert_eq!(norm("Chapter."), "chapter");
            assert_eq!(norm("don't,"), "don't");
            assert_eq!(common_prefix(&["a".into(), "B.".into()], &["A".into(), "b".into(), "c".into()]), 2);
        }

        #[test]
        fn commits_only_what_two_runs_agree_on() {
            let mut rec = FakeRecognizer {
                words: sentence(),
                encode: 0.1,
                word: 0.0,
            };
            let u = [0..at_sample(2.2)];
            let r = simulate(&mut rec, &u, at_sample(4.0), &StreamConfig::default());
            let text: Vec<&str> = r.commits.iter().map(|c| c.word.as_str()).collect();
            assert_eq!(text, ["Open", "the", "second", "chapter."]);
            // Commit times never go backwards, and every word is committed
            // after it was spoken.
            assert!(r.commits.windows(2).all(|p| p[0].at <= p[1].at));
            for (c, rw) in r.commits.iter().zip(sentence()) {
                assert!(c.at >= rw.end, "{c:?} before {rw:?}");
            }
            // The half-heard tail ("seco") was never committed or announced.
            assert!(r.announcements.iter().all(|(_, a)| !a.contains("seco ")));
            assert_eq!(r.late_disagreements, 0);
            assert!(r.rewrites_avoided > 0);
        }

        /// Returns nothing on its second run, as Whisper sometimes does.
        struct Flaky(FakeRecognizer, usize);

        impl Recognizer for Flaky {
            fn recognize(&mut self, start: usize, end: usize) -> (Vec<Seg>, f64) {
                self.1 += 1;
                let (segs, cost) = self.0.recognize(start, end);
                (if self.1 == 2 { Vec::new() } else { segs }, cost)
            }
        }

        #[test]
        fn a_changed_early_word_keeps_the_rest() {
            let split = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
            let cw = split("open until 9 in the evening and to");
            let hyp = split("open until 9am in the evening and to add more lamps where people liked to sit");
            assert_eq!(continuation(&cw, &hyp), 8);
            assert_eq!(continuation(&cw[..2], &hyp), 2);
            assert_eq!(continuation(&[], &hyp), 0);
            assert_eq!(continuation(&cw, &[]), 0);
        }

        #[test]
        fn an_empty_run_is_skipped() {
            let mut rec = Flaky(
                FakeRecognizer {
                    words: sentence(),
                    encode: 0.1,
                    word: 0.0,
                },
                0,
            );
            let u = [0..at_sample(2.2)];
            let r = simulate(&mut rec, &u, at_sample(4.0), &StreamConfig::default());
            assert_eq!(r.empty_runs, 1);
            let text: Vec<&str> = r.commits.iter().map(|c| c.word.as_str()).collect();
            assert_eq!(text, ["Open", "the", "second", "chapter."]);
        }

        #[test]
        fn a_run_going_at_the_pause_is_cancelled() {
            let run = |cancel_at_pause: bool| {
                let mut rec = FakeRecognizer {
                    words: sentence(),
                    encode: 0.3,
                    word: 0.5,
                };
                let config = StreamConfig {
                    cancel_at_pause,
                    ..StreamConfig::default()
                };
                simulate(&mut rec, &[0..at_sample(2.2)], at_sample(4.0), &config)
            };
            let (with, without) = (run(true), run(false));
            assert_eq!(with.cancelled_runs, 1);
            assert_eq!(without.cancelled_runs, 0);
            assert!(with.pause_to_final[0] < without.pause_to_final[0]);
            let text = |r: &StreamResult| r.commits.iter().map(|c| norm(&c.word)).collect::<Vec<_>>();
            assert_eq!(text(&with), text(&without));
        }

        #[test]
        fn trimming_keeps_the_text() {
            let mut words = sentence();
            words.push(w("Read", 2.1, 2.5));
            words.push(w("aloud.", 2.5, 3.2));
            let mut rec = FakeRecognizer {
                words,
                encode: 0.05,
                word: 0.0,
            };
            let u = [0..at_sample(3.4)];
            let config = StreamConfig {
                trim: true,
                ..StreamConfig::default()
            };
            let r = simulate(&mut rec, &u, at_sample(5.0), &config);
            let text: Vec<String> = r.commits.iter().map(|c| norm(&c.word)).collect();
            assert_eq!(text, ["open", "the", "second", "chapter", "read", "aloud"]);
        }

        #[test]
        fn batch_waits_for_the_pause() {
            let mut rec = FakeRecognizer {
                words: sentence(),
                encode: 0.5,
                word: 0.0,
            };
            let u = [0..at_sample(2.2)];
            let (text, waits) = batch(&mut rec, &u, at_sample(4.0), &StreamConfig::default());
            assert_eq!(text.len(), 4);
            assert!((waits[0] - 0.5).abs() < 1e-9);
        }

        #[test]
        fn alignment_and_wer() {
            let r: Vec<String> = ["the", "quick", "brown", "fox"].map(String::from).to_vec();
            let h: Vec<String> = ["The", "quick", "fox", "jumps"].map(String::from).to_vec();
            let (d, pairs) = align(&h, &r);
            assert_eq!(d, 2);
            assert_eq!(pairs, vec![(0, 0), (1, 1), (2, 3)]);
            assert!((wer(&h, &r) - 50.0).abs() < 1e-9);
        }

        #[test]
        fn statistics() {
            assert_eq!(median_worst(&[3.0, 1.0, 2.0]), (2.0, 3.0));
            assert_eq!(median_worst(&[1.0, 2.0, 3.0, 4.0]), (2.5, 4.0));
            assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0], 90.0), 9.0);
        }
    }
}

/// Reads an SRT file of one cue per word.
fn read_srt(path: &Path) -> Result<Vec<la::RefWord>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let time = |t: &str| -> Option<f64> {
        let (hms, ms) = t.trim().split_once(',')?;
        let mut p = hms.split(':').map(|x| x.parse::<f64>().ok());
        let (h, m, s) = (p.next()??, p.next()??, p.next()??);
        Some(h * 3600.0 + m * 60.0 + s + ms.parse::<f64>().ok()? / 1000.0)
    };
    let mut out = Vec::new();
    for block in text.replace("\r\n", "\n").split("\n\n") {
        let mut lines = block.lines().filter(|l| !l.trim().is_empty());
        let (Some(_), Some(times), Some(words)) = (lines.next(), lines.next(), lines.next()) else {
            continue;
        };
        let Some((a, b)) = times.split_once("-->") else {
            continue;
        };
        let (Some(start), Some(end)) = (time(a), time(b)) else {
            continue;
        };
        out.push(la::RefWord {
            text: words.trim().to_owned(),
            start,
            end,
        });
    }
    Ok(out)
}

/// The process's memory in MB: (working set, peak working set, private
/// bytes). Asks PowerShell on Windows and `/proc` on Linux.
fn memory() -> Option<(f64, f64, f64)> {
    let mb = |b: f64| b / 1_048_576.0;
    if cfg!(windows) {
        let v = windows_process(
            "\"$($p.WorkingSet64) $($p.PeakWorkingSet64) $($p.PrivateMemorySize64)\"",
        )?;
        (v.len() == 3).then(|| (mb(v[0]), mb(v[1]), mb(v[2])))
    } else {
        let s = std::fs::read_to_string("/proc/self/status").ok()?;
        let field = |name: &str| {
            s.lines()
                .find(|l| l.starts_with(name))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|x| x.parse::<f64>().ok())
                .map(|kb| kb / 1024.0)
        };
        Some((field("VmRSS:")?, field("VmHWM:")?, field("RssAnon:")?))
    }
}

/// Numbers about this process from PowerShell's `Get-Process` (`$p`).
fn windows_process(expr: &str) -> Option<Vec<f64>> {
    let script = format!("$p = Get-Process -Id {}; {expr}", std::process::id());
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    Some(s.split_whitespace().filter_map(|x| x.parse().ok()).collect())
}

/// Seconds of processor time this process has used, on all threads.
fn cpu_seconds() -> Option<f64> {
    if cfg!(windows) {
        windows_process("$p.TotalProcessorTime.TotalSeconds")?
            .first()
            .copied()
    } else {
        // utime and stime, fields 14 and 15, in clock ticks (100 a second
        // on Linux).
        let s = std::fs::read_to_string("/proc/self/stat").ok()?;
        let rest = s.rsplit_once(')')?.1;
        let f: Vec<&str> = rest.split_whitespace().collect();
        let tick = |i: usize| f.get(i).and_then(|x| x.parse::<f64>().ok());
        Some((tick(11)? + tick(12)?) / 100.0)
    }
}

fn print_memory(label: &str) {
    match memory() {
        Some((ws, peak, private)) => println!(
            "Memory {label}: working set {ws:.0} MB, peak {peak:.0} MB, private {private:.0} MB"
        ),
        None => println!("Memory {label}: not available"),
    }
}

/// Command-line options.
struct Opts {
    model: Option<PathBuf>,
    command: String,
    runs: usize,
    audio: Option<PathBuf>,
    step_ms: u32,
    trim: bool,
    no_cancel: bool,
    repeat: usize,
    encode_ms: f64,
    word_ms: f64,
    files: Vec<PathBuf>,
}

impl Opts {
    fn stream_config(&self) -> la::StreamConfig {
        la::StreamConfig {
            step: f64::from(self.step_ms) / 1000.0,
            trim: self.trim,
            cancel_at_pause: !self.no_cancel,
            ..la::StreamConfig::default()
        }
    }
}

fn parse_args() -> Result<Opts, String> {
    let mut o = Opts {
        model: std::env::var_os("TEXTWEAVER_WHISPER_RTEN").map(PathBuf::from),
        command: String::new(),
        runs: 40,
        audio: None,
        step_ms: 700,
        trim: false,
        no_cancel: false,
        repeat: 1,
        encode_ms: 700.0,
        word_ms: 25.0,
        files: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    let value = |args: &mut dyn Iterator<Item = String>, name: &str| {
        args.next().ok_or_else(|| format!("{name} needs a value"))
    };
    let num = |s: String, name: &str| s.parse::<f64>().map_err(|_| format!("{name}: not a number"));
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => o.model = Some(value(&mut args, &a)?.into()),
            "--runs" => o.runs = num(value(&mut args, &a)?, &a)? as usize,
            "--audio" => o.audio = Some(value(&mut args, &a)?.into()),
            "--step-ms" => o.step_ms = num(value(&mut args, &a)?, &a)? as u32,
            "--trim" => o.trim = true,
            "--no-cancel" => o.no_cancel = true,
            "--repeat" => o.repeat = (num(value(&mut args, &a)?, &a)? as usize).max(1),
            "--encode-ms" => o.encode_ms = num(value(&mut args, &a)?, &a)?,
            "--word-ms" => o.word_ms = num(value(&mut args, &a)?, &a)?,
            _ if o.command.is_empty() && !a.starts_with('-') => o.command = a,
            _ if !a.starts_with('-') => o.files.push(a.into()),
            _ => return Err(format!("Unknown option {a}")),
        }
    }
    if o.command.is_empty() {
        return Err(
            "Usage: stream_probe [--model DIR] bench|encoder-lengths|stream|fake [options] [WAV...]"
                .into(),
        );
    }
    Ok(o)
}

/// Prints one fixture's streaming result.
fn report_stream(
    name: &str,
    r: &la::StreamResult,
    batch_words: &[String],
    batch_waits: &[f64],
    reference: &[la::RefWord],
) {
    let committed: Vec<String> = r.commits.iter().map(|c| c.word.clone()).collect();
    let ref_words: Vec<String> = reference.iter().map(|w| w.text.clone()).collect();
    let lat = la::commit_latencies(&r.commits, reference);
    let agreed: Vec<f64> = lat.iter().filter(|l| !l.1).map(|l| l.0).collect();
    let all: Vec<f64> = lat.iter().map(|l| l.0).collect();
    let (costs_med, costs_worst) = la::median_worst(&r.run_costs);
    let (all_med, all_worst) = la::median_worst(&all);
    let (agr_med, agr_worst) = la::median_worst(&agreed);
    let (fin_med, fin_worst) = la::median_worst(&r.pause_to_final);
    let (bat_med, bat_worst) = la::median_worst(batch_waits);
    println!("File {name}");
    println!(
        "  Runs {} ({} empty, skipped; {} cancelled at the pause); run time median {costs_med:.2} s, worst {costs_worst:.2} s",
        r.runs, r.empty_runs, r.cancelled_runs
    );
    println!(
        "  Words committed {} ({} by agreement, {} at the pause)",
        r.commits.len(),
        r.commits.iter().filter(|c| !c.at_pause).count(),
        r.commits.iter().filter(|c| c.at_pause).count()
    );
    println!(
        "  Latency, word end to commit, all words: median {all_med:.2} s, 90th percentile {:.2} s, worst {all_worst:.2} s",
        la::percentile(&all, 90.0)
    );
    println!(
        "  Latency, words committed by agreement: median {agr_med:.2} s, worst {agr_worst:.2} s ({} words)",
        agreed.len()
    );
    println!("  Pause to last word, streaming: median {fin_med:.2} s, worst {fin_worst:.2} s");
    println!("  Pause to text, batch today: median {bat_med:.2} s, worst {bat_worst:.2} s");
    println!("  Rewrites avoided: {} words", r.rewrites_avoided);
    println!("  Committed words the final run disagreed with: {}", r.late_disagreements);
    println!(
        "  Word error rate: streaming {:.1} percent, batch {:.1} percent, streaming against batch {:.1} percent",
        la::wer(&committed, &ref_words),
        la::wer(batch_words, &ref_words),
        la::wer(&committed, batch_words)
    );
    println!("  Streamed text: {}", committed.join(" "));
    println!("  Batch text: {}", batch_words.join(" "));
    println!("  Announcements:");
    for (t, a) in &r.announcements {
        println!("    at {t:.2} s: {a}");
    }
}

/// Loads a fixture: 16 kHz audio, its utterances, and its reference words.
#[cfg(any(feature = "rten", feature = "mic"))]
fn load_fixture(
    wav: &Path,
) -> Result<(Vec<f32>, Vec<std::ops::Range<usize>>, Vec<la::RefWord>), String> {
    use textweaver_dictation::{audio, vad};
    let bytes = std::fs::read(wav).map_err(|e| format!("{}: {e}", wav.display()))?;
    let pcm = audio::to_whisper_rate(&audio::read_wav(&bytes)?)?;
    let config = vad::VadConfig::default();
    let spans = vad::utterances(&pcm, &config);
    let mut reference = read_srt(&wav.with_extension("srt")).unwrap_or_default();
    // The export's word cues are spread over each sentence's audio,
    // pauses included, so they drift from the speech. Each utterance's
    // words (those whose cue starts before the next utterance) are
    // stretched linearly onto the speech the detector found.
    let pad = f64::from(config.pad_ms) / 1000.0;
    let mut i = 0;
    for (k, s) in spans.iter().enumerate() {
        let next = spans.get(k + 1).map_or(f64::INFINITY, |n| n.start as f64 / la::RATE);
        let j = i + reference[i..].iter().take_while(|w| w.start < next).count();
        if j > i {
            let (a0, a1) = (reference[i].start, reference[j - 1].end);
            let b0 = s.start as f64 / la::RATE + if s.start == 0 { 0.0 } else { pad };
            let b1 = (s.end as f64 / la::RATE - pad).max(b0);
            let scale = if a1 > a0 { (b1 - b0) / (a1 - a0) } else { 0.0 };
            for w in &mut reference[i..j] {
                w.start = b0 + (w.start - a0) * scale;
                w.end = b0 + (w.end - a0) * scale;
            }
        }
        i = j;
    }
    Ok((pcm, spans, reference))
}

#[cfg(any(feature = "rten", feature = "mic"))]
fn run_fake(o: &Opts) -> Result<(), String> {
    for wav in &o.files {
        let (pcm, spans, reference) = load_fixture(wav)?;
        println!(
            "Utterances found: {}",
            spans
                .iter()
                .map(|s| format!("{:.2} to {:.2} s", s.start as f64 / la::RATE, s.end as f64 / la::RATE))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let config = o.stream_config();
        let mut rec = la::FakeRecognizer {
            words: reference.clone(),
            encode: o.encode_ms / 1000.0,
            word: o.word_ms / 1000.0,
        };
        let r = la::simulate(&mut rec, &spans, pcm.len(), &config);
        let (bw, bwait) = la::batch(&mut rec, &spans, pcm.len(), &config);
        report_stream(&wav.display().to_string(), &r, &bw, &bwait, &reference);
    }
    Ok(())
}

#[cfg(feature = "rten")]
mod whisper {
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    use textweaver_dictation::rten_whisper::{
        HOP, N_FFT, RtenWhisper, RtenWhisperFiles, hann_window, log_mel_spectrogram,
        mel_filters,
    };

    use super::{Opts, la, load_fixture, print_memory, report_stream};

    fn load(o: &Opts) -> Result<RtenWhisper, String> {
        let dir = o
            .model
            .as_deref()
            .ok_or("No model folder: give --model DIR or set TEXTWEAVER_WHISPER_RTEN")?;
        let files = RtenWhisperFiles::in_dir(dir).map_err(|e| e.to_string())?;
        print_memory("before loading");
        let t = Instant::now();
        let w = RtenWhisper::load(&files).map_err(|e| e.to_string())?;
        println!("Model loaded in {:.2} s", t.elapsed().as_secs_f64());
        print_memory("after loading");
        Ok(w)
    }

    /// Whisper over a whole fixture's audio.
    struct Real<'a> {
        w: &'a RtenWhisper,
        pcm: &'a [f32],
        fixed: f64,
    }

    impl la::Recognizer for Real<'_> {
        fn last_fixed(&self) -> f64 {
            self.fixed
        }

        fn recognize(&mut self, start: usize, end: usize) -> (Vec<la::Seg>, f64) {
            let cancel = AtomicBool::new(false);
            let t = Instant::now();
            self.fixed = 0.0;
            let segs = match self.w.transcribe(&self.pcm[start..end], None, &cancel, &mut |_| {}) {
                Ok((tr, tm)) => {
                    self.fixed = (tm.features + tm.encode).as_secs_f64();
                    tr
                }
                    .segments
                    .into_iter()
                    .map(|s| la::Seg {
                        start: s.start_ms as f64 / 1000.0,
                        end: s.end_ms as f64 / 1000.0,
                        text: s.text,
                    })
                    .collect(),
                Err(e) => {
                    eprintln!("Whisper failed: {e}");
                    Vec::new()
                }
            };
            (segs, t.elapsed().as_secs_f64())
        }
    }

    pub fn bench(o: &Opts) -> Result<(), String> {
        let w = load(o)?;
        let wav = o
            .audio
            .clone()
            .unwrap_or_else(|| "fixtures/d/stream-long.wav".into());
        let (pcm, _, _) = load_fixture(&wav)?;
        let cancel = AtomicBool::new(false);
        println!("Audio {}; {} runs per length", wav.display(), o.runs);
        for secs in [1usize, 2, 4, 8] {
            let n = (secs * 16_000).min(pcm.len());
            let clip = &pcm[..n];
            // One warm-up run, not counted.
            let _ = w.transcribe(clip, None, &cancel, &mut |_| {});
            let (mut feat, mut enc, mut dec, mut tot) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            let mut text = String::new();
            let cpu0 = super::cpu_seconds();
            for _ in 0..o.runs {
                let t = Instant::now();
                let (tr, tm) = w
                    .transcribe(clip, None, &cancel, &mut |_| {})
                    .map_err(|e| e.to_string())?;
                tot.push(t.elapsed().as_secs_f64());
                feat.push(tm.features.as_secs_f64());
                enc.push(tm.encode.as_secs_f64());
                dec.push(tm.decode.as_secs_f64());
                text = tr.text();
            }
            let f = |v: &[f64]| {
                let (m, x) = la::median_worst(v);
                format!("median {m:.3} s, worst {x:.3} s")
            };
            println!("Audio {secs} s:");
            println!("  Spectrogram {}", f(&feat));
            println!("  Encoder {}", f(&enc));
            println!("  Decoder {}", f(&dec));
            println!("  Whole run {}", f(&tot));
            if let (Some(a), Some(b)) = (cpu0, super::cpu_seconds()) {
                println!(
                    "  Processor time per run, all threads: {:.3} s",
                    (b - a) / o.runs.max(1) as f64
                );
            }
            println!("  Text: {text}");
            print_memory(&format!("after {secs} s"));
        }
        Ok(())
    }

    pub fn encoder_lengths(o: &Opts) -> Result<(), String> {
        let dir = o.model.as_deref().ok_or("No model folder")?;
        let files = RtenWhisperFiles::in_dir(dir).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(&files.encoder).map_err(|e| e.to_string())?;
        let model = rten::Model::load(bytes).map_err(|e| e.to_string())?;
        let input = model.node_id("input_features").map_err(|e| e.to_string())?;
        let output = model.node_id("last_hidden_state").map_err(|e| e.to_string())?;
        println!("Encoder input shape: {:?}", model.input_shape(0));
        let filters = mel_filters(80, N_FFT, 16_000);
        let window = hann_window(N_FFT);
        for secs in [5usize, 10, 30] {
            let samples = secs * 16_000;
            let audio = vec![0.0f32; samples];
            let mel = log_mel_spectrogram(&audio, samples, &filters, &window).with_new_axis(0);
            let t = Instant::now();
            let r = model.run_n(vec![(input, mel.view().into())], [output], None);
            match r {
                Ok(_) => println!(
                    "Encoder with {secs} s ({} frames): works, {:.3} s",
                    samples / HOP,
                    t.elapsed().as_secs_f64()
                ),
                Err(e) => println!("Encoder with {secs} s: fails: {e}"),
            }
        }
        Ok(())
    }

    // `with_new_axis` and `view` come from the tensor traits.
    use rten_tensor::prelude::*;

    pub fn stream(o: &Opts) -> Result<(), String> {
        let w = load(o)?;
        let config = o.stream_config();
        let on = |b: bool| if b { "on" } else { "off" };
        println!(
            "Step {} ms, trimming {}, cancel at the pause {}",
            o.step_ms,
            on(config.trim),
            on(config.cancel_at_pause)
        );
        for wav in &o.files {
            let (pcm, spans, reference) = load_fixture(wav)?;
            println!(
                "Utterances found: {}",
                spans
                    .iter()
                    .map(|s| format!("{:.2} to {:.2} s", s.start as f64 / la::RATE, s.end as f64 / la::RATE))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            for round in 0..o.repeat {
                let mut rec = Real {
                    w: &w,
                    pcm: &pcm,
                    fixed: 0.0,
                };
                let r = la::simulate(&mut rec, &spans, pcm.len(), &config);
                let (bw, bwait) = la::batch(&mut rec, &spans, pcm.len(), &config);
                report_stream(
                    &format!("{} (round {})", wav.display(), round + 1),
                    &r,
                    &bw,
                    &bwait,
                    &reference,
                );
                print_memory("after this file");
            }
        }
        Ok(())
    }
}

fn main() -> ExitCode {
    let o = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let result: Result<(), String> = match o.command.as_str() {
        #[cfg(any(feature = "rten", feature = "mic"))]
        "fake" => run_fake(&o),
        #[cfg(feature = "rten")]
        "bench" => whisper::bench(&o),
        #[cfg(feature = "rten")]
        "encoder-lengths" => whisper::encoder_lengths(&o),
        #[cfg(feature = "rten")]
        "stream" => whisper::stream(&o),
        other => Err(format!(
            "Unknown command {other}, or it needs --features rten"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
