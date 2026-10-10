//! The clip player for recorded audio (a DAISY book's narration; see
//! `textweaver_speech::recorded`): MP3 and WAV clips decoded with symphonia
//! and played through [`Playback`], so their events come from the same
//! audio clock as speech.
//!
//! - **Exact positions.** Clip times become sample numbers once, in
//!   integers at the file's rate. Within a file the decoder runs on from
//!   clip to clip with no seek (a book's phrases follow one another), and a
//!   clip is trimmed by counting decoded samples, never by trusting the
//!   container. A seek happens only on a jump (reading from somewhere
//!   else); it is symphonia's accurate seek, which counts MP3 frames from
//!   the start of the file instead of guessing from the bit rate, then
//!   trims to the sample.
//! - **MP3 start delay.** Decoded MP3 starts [`MP3_DELAY`] samples late
//!   (the encoder's and the decoder's delay), and the book's clip times
//!   count from the recording before it was encoded; positions in MP3 files
//!   are moved by that much.
//! - **Overlaps and gaps.** A clip that starts a little before the end of
//!   the one before (at most [`OVERLAP`]) plays on from where the audio
//!   is, so nothing is heard twice; a larger overlap seeks back. A gap is
//!   skipped by decoding past it.
//! - **One rate.** The output opens at the first file's rate, mixed to
//!   mono (books are almost always one rate throughout). A file at another
//!   rate is resampled to it with one continuous linear resampler per
//!   clip, so no length is lost between packets.
//! - **Highlight.** Each phrase reports its first word as its audio starts;
//!   when its length is known, the other words follow at an even pace over
//!   the phrase (an estimate: only each phrase's start is exact), and every
//!   few seconds the current word is reported again, so a long recording
//!   is never taken for a stalled engine.
//! - **Streaming.** Audio is decoded a little ahead of the output, so an
//!   hour-long file costs a few seconds of memory.

use std::collections::VecDeque;
use std::io::Cursor;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_MP3, Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use textweaver_core::{UtteranceId, Volume};
use textweaver_speech::{ClipPlayer, EventSink, PlayerFactory, RecordedClip};

use crate::audio::AudioOutput;
use crate::playback::Playback;
use crate::protocol::EndStatus;

/// Samples decoded MP3 runs late: LAME's encoder delay (576) plus the
/// decoder's (529), the usual case.
pub const MP3_DELAY: u64 = 1105;

/// The longest overlap between two clips that is played through rather
/// than sought back.
pub const OVERLAP: Duration = Duration::from_millis(500);

/// The longest gap decoded past rather than sought over.
const SKIP_AHEAD: Duration = Duration::from_secs(30);

/// How far decoding runs ahead of the output.
const AHEAD: Duration = Duration::from_secs(2);

/// How often the current word is reported again.
const KEEPALIVE: Duration = Duration::from_secs(4);

/// Reads a file of the book (on disk, or an archive member).
pub type ReadFile = Arc<dyn Fn(&Path) -> std::io::Result<Vec<u8>> + Send + Sync>;

/// The player factory for `textweaver_speech::RecordedPlan`: plays to
/// `output`, reading files with `read`.
pub fn player_factory(output: AudioOutput, read: ReadFile) -> PlayerFactory {
    Arc::new(
        move || Ok(Box::new(BookPlayer::new(output, Arc::clone(&read))) as Box<dyn ClipPlayer>),
    )
}

/// `d` as samples at `rate`, rounded down, in integers.
pub fn samples_at(d: Duration, rate: u32) -> u64 {
    u64::try_from(d.as_nanos() * u128::from(rate) / 1_000_000_000).unwrap_or(u64::MAX)
}

/// One audio file being decoded.
pub(crate) struct Source {
    file: PathBuf,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: u32,
    /// Its sample rate.
    pub(crate) rate: u32,
    /// Samples by which a clip time is moved ([`MP3_DELAY`] for MP3).
    pub(crate) delay: u64,
    /// Decoded mono samples not yet taken.
    pending: VecDeque<i16>,
    /// The stream position of `pending[0]`.
    at: u64,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("file", &self.file)
            .field("rate", &self.rate)
            .field("at", &self.at)
            .finish_non_exhaustive()
    }
}

impl Source {
    /// Opens `file` (read with `read` unless it is on disk) at its start.
    pub(crate) fn open(file: &Path, read: &ReadFile) -> Result<Source, String> {
        let media: Box<dyn MediaSource> = match std::fs::File::open(file) {
            Ok(f) => Box::new(f),
            Err(_) => Box::new(Cursor::new(read(file).map_err(|e| e.to_string())?)),
        };
        let mss = MediaSourceStream::new(media, Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        // Gapless trimming off: positions count every decoded sample, and
        // MP3's delay is allowed for with `MP3_DELAY`.
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| e.to_string())?;
        let format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| "no audio track".to_owned())?;
        let rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| "no sample rate".to_owned())?;
        let delay = if track.codec_params.codec == CODEC_TYPE_MP3 {
            MP3_DELAY
        } else {
            0
        };
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| e.to_string())?;
        let track = track.id;
        Ok(Source {
            file: file.to_owned(),
            format,
            decoder,
            track,
            rate,
            delay,
            pending: VecDeque::new(),
            at: 0,
        })
    }

    /// Where the next sample taken comes from in the stream.
    pub(crate) fn position(&self) -> u64 {
        self.at
    }

    /// Moves to stream position `to` with an accurate seek (decoding then
    /// trims to the sample). False when the file cannot seek.
    fn seek(&mut self, to: u64) -> bool {
        let seek = SeekTo::TimeStamp {
            ts: to,
            track_id: self.track,
        };
        match self.format.seek(SeekMode::Accurate, seek) {
            Ok(done) => {
                self.decoder.reset();
                self.pending.clear();
                self.at = done.actual_ts.min(to);
                true
            }
            Err(e) => {
                log::warn!(
                    "recorded audio: cannot seek in {}: {e}",
                    self.file.display()
                );
                false
            }
        }
    }

    /// Decodes the next packet into `pending`. False at the end.
    fn decode_more(&mut self) -> bool {
        use symphonia::core::errors::Error;
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(Error::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(_) => return false,
            };
            if packet.track_id() != self.track {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                // A damaged packet is skipped; the next one resyncs.
                Err(Error::DecodeError(_)) => continue,
                Err(_) => return false,
            };
            let spec = *decoded.spec();
            let frames = decoded.frames();
            if frames == 0 {
                continue;
            }
            let mut buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
            buf.copy_interleaved_ref(decoded);
            if self.pending.is_empty() {
                self.at = packet.ts();
            }
            let channels = spec.channels.count().max(1);
            for frame in buf.samples().chunks(channels).take(frames) {
                let sum: i32 = frame.iter().map(|&s| i32::from(s)).sum();
                let n = i32::try_from(frame.len()).unwrap_or(1);
                self.pending
                    .push_back(i16::try_from(sum / n).unwrap_or_default());
            }
            return true;
        }
    }

    /// Takes the next samples from stream position `from` (skipping what
    /// comes before it) up to `end`, at most `max`. `None` at the end of
    /// the clip or the file.
    pub(crate) fn take(&mut self, from: u64, end: Option<u64>, max: usize) -> Option<Vec<i16>> {
        loop {
            if self.at < from {
                let skip = usize::try_from(from - self.at)
                    .unwrap_or(usize::MAX)
                    .min(self.pending.len());
                self.pending.drain(..skip);
                self.at += skip as u64;
            }
            if self.at >= from && !self.pending.is_empty() {
                let left = end.map_or(usize::MAX, |e| {
                    usize::try_from(e.saturating_sub(self.at)).unwrap_or(usize::MAX)
                });
                let n = left.min(max).min(self.pending.len());
                if n == 0 {
                    return None;
                }
                self.at += n as u64;
                return Some(self.pending.drain(..n).collect());
            }
            if end.is_some_and(|e| self.at >= e) || !self.decode_more() {
                return None;
            }
        }
    }

    /// Gets ready to play from stream position `begin`: decodes on from
    /// here (no seek) when `begin` is a little ahead, or plays on from here
    /// when it overlaps by at most [`OVERLAP`]; otherwise seeks. Returns
    /// where the clip's audio starts.
    pub(crate) fn prepare(&mut self, begin: u64) -> u64 {
        let here = self.at;
        let overlap = samples_at(OVERLAP, self.rate);
        let ahead = samples_at(SKIP_AHEAD, self.rate);
        if begin >= here && begin - here <= ahead {
            return begin;
        }
        if begin < here && here - begin <= overlap {
            return here;
        }
        if self.seek(begin) {
            begin
        } else {
            here.max(begin)
        }
    }

    /// The file this decodes.
    pub(crate) fn file(&self) -> &Path {
        &self.file
    }
}

/// Linear resampling of one continuous clip from `from` Hz to `to` Hz.
/// Output sample `j` is taken at input position `j * from / to`, computed
/// in integers from the clip's start, so chunk boundaries never add or
/// lose a sample.
#[derive(Debug)]
pub(crate) struct Resample {
    from: u64,
    to: u64,
    /// Input samples seen before the current chunk.
    base: u64,
    /// The last input sample of the previous chunk.
    prev: Option<i16>,
    /// The next output sample.
    next: u64,
}

impl Resample {
    pub(crate) fn new(from: u32, to: u32) -> Self {
        Resample {
            from: u64::from(from.max(1)),
            to: u64::from(to.max(1)),
            base: 0,
            prev: None,
            next: 0,
        }
    }

    /// Resamples the next chunk of input.
    pub(crate) fn push(&mut self, input: &[i16]) -> Vec<i16> {
        let len = input.len() as u64;
        let get = |i: u64| -> Option<i16> {
            if i + 1 == self.base {
                self.prev
            } else if i >= self.base && i < self.base + len {
                usize::try_from(i - self.base).ok().map(|k| input[k])
            } else {
                None
            }
        };
        let mut out = Vec::with_capacity(input.len() * self.to as usize / self.from as usize + 1);
        loop {
            let num = u128::from(self.next) * u128::from(self.from);
            let idx = u64::try_from(num / u128::from(self.to)).unwrap_or(u64::MAX);
            let frac = u64::try_from(num % u128::from(self.to)).unwrap_or(0);
            let Some(a) = get(idx) else { break };
            let b = if frac == 0 {
                a
            } else {
                match get(idx + 1) {
                    Some(b) => b,
                    None => break,
                }
            };
            let v = i64::from(a)
                + (i64::from(b) - i64::from(a)) * i64::try_from(frac).unwrap_or(0)
                    / i64::try_from(self.to).unwrap_or(1);
            out.push(i16::try_from(v).unwrap_or_default());
            self.next += 1;
        }
        if let Some(&last) = input.last() {
            self.prev = Some(last);
        }
        self.base += len;
        out
    }
}

/// One utterance: its clips and how far they have been decoded.
#[derive(Debug)]
struct Job {
    token: u64,
    clips: Vec<RecordedClip>,
    /// The clip being decoded.
    clip: usize,
    /// Its stream position and end, once it is open.
    span: Option<(u64, Option<u64>)>,
    resample: Option<Resample>,
    /// Word byte ranges and where each starts in the text.
    words: Vec<(Range<u32>, u32)>,
    text_len: u32,
    /// Output samples the whole utterance lasts, when every clip has an end.
    total: Option<u64>,
    /// Output samples pushed so far.
    pushed: u64,
    /// The next word to report, and the next keepalive sample.
    next_word: usize,
    next_keepalive: u64,
    failed: bool,
}

/// Plays recorded clips (see the module docs).
pub struct BookPlayer {
    playback: Playback<()>,
    read: ReadFile,
    jobs: VecDeque<Job>,
    source: Option<Source>,
    /// The output's rate, once the first clip chose it.
    rate: Option<u32>,
}

impl std::fmt::Debug for BookPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookPlayer")
            .field("jobs", &self.jobs.len())
            .field("source", &self.source)
            .field("rate", &self.rate)
            .finish_non_exhaustive()
    }
}

/// The words of `text`: byte ranges between whitespace, with their start.
fn words_of(text: &str) -> Vec<(Range<u32>, u32)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        let i = u32::try_from(i).unwrap_or(u32::MAX);
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push((s..i, s));
                start = None;
            }
            _ => {}
        }
    }
    out
}

impl BookPlayer {
    /// A player for `output`, reading files with `read`.
    pub fn new(output: AudioOutput, read: ReadFile) -> Self {
        BookPlayer {
            playback: Playback::new("recorded", output, 22_050),
            read,
            jobs: VecDeque::new(),
            source: None,
            rate: None,
        }
    }

    /// The playback client (its feed counts every sample played).
    pub fn playback(&self) -> &Playback<()> {
        &self.playback
    }

    /// Opens the job's next clip: the right file, at the right place.
    /// False when the file cannot be played (the clip is skipped).
    fn open_clip(&mut self) -> bool {
        let Some(job) = self.jobs.front() else {
            return false;
        };
        let clip = job.clips[job.clip].clone();
        if self
            .source
            .as_ref()
            .is_none_or(|s| s.file() != clip.file.as_path())
        {
            match Source::open(&clip.file, &self.read) {
                Ok(s) => self.source = Some(s),
                Err(e) => {
                    log::warn!("recorded audio: cannot play {}: {e}", clip.file.display());
                    self.source = None;
                    if let Some(job) = self.jobs.front_mut()
                        && !std::mem::replace(&mut job.failed, true)
                    {
                        let file = clip.file.file_name().unwrap_or_default().to_string_lossy();
                        self.playback
                            .on_error(job.token, format!("cannot play the audio file {file}"));
                    }
                    return false;
                }
            }
        }
        let Some(source) = self.source.as_mut() else {
            return false;
        };
        let rate = source.rate;
        let begin = samples_at(clip.begin, rate) + source.delay;
        let end = clip.end.map(|e| samples_at(e, rate) + source.delay);
        let start = source.prepare(begin);
        // The output keeps the first file's rate; other files resample.
        if self.rate.is_none() {
            self.rate = Some(rate);
            self.playback.set_sample_rate(rate);
        }
        let out_rate = self.rate.unwrap_or(rate);
        let Some(job) = self.jobs.front_mut() else {
            return false;
        };
        job.span = Some((start, end));
        job.resample = (out_rate != rate).then(|| Resample::new(rate, out_rate));
        if job.clip == 0 && job.total.is_none() {
            job.total = job.clips.iter().try_fold(0u64, |sum, c| {
                let end = c.end?;
                Some(sum + samples_at(end.saturating_sub(c.begin), out_rate))
            });
        }
        true
    }

    /// Reports the words (and keepalives) the pushed audio has reached.
    fn mark_words(&mut self) {
        let rate = u64::from(self.rate.unwrap_or(22_050));
        let Some(job) = self.jobs.front_mut() else {
            return;
        };
        let keep = samples_at(KEEPALIVE, u32::try_from(rate).unwrap_or(22_050)).max(1);
        loop {
            let word_at = job
                .words
                .get(job.next_word)
                .map(|(_, offset)| match job.total {
                    Some(total) if job.next_word > 0 => {
                        total * u64::from(*offset) / u64::from(job.text_len.max(1))
                    }
                    _ if job.next_word == 0 => 0,
                    // The length is unknown: the other words wait for the end.
                    _ => u64::MAX,
                });
            let next = word_at.unwrap_or(u64::MAX).min(job.next_keepalive);
            if next > job.pushed || job.words.is_empty() {
                break;
            }
            let range = if word_at == Some(next) {
                let r = job.words[job.next_word].0.clone();
                job.next_word += 1;
                if job.next_word == 1 {
                    job.next_keepalive = next + keep;
                }
                r
            } else {
                job.next_keepalive += keep;
                let current = job.next_word.saturating_sub(1);
                job.words[current].0.clone()
            };
            self.playback.on_word(job.token, next, |_| Some(range));
        }
    }

    /// Decodes until the output has [`AHEAD`] waiting or nothing is left.
    fn pump(&mut self) {
        loop {
            let feed = self.playback.feed();
            let waiting = feed.pushed().saturating_sub(feed.consumed());
            let rate = self.rate.unwrap_or(22_050);
            if waiting >= samples_at(AHEAD, rate) {
                return;
            }
            let Some(job) = self.jobs.front() else {
                return;
            };
            if job.clip >= job.clips.len() {
                let token = job.token;
                self.playback.on_end(token, EndStatus::Done);
                self.jobs.pop_front();
                continue;
            }
            if job.span.is_none() && !self.open_clip() {
                if let Some(job) = self.jobs.front_mut() {
                    job.clip += 1;
                    job.span = None;
                }
                continue;
            }
            let Some(job) = self.jobs.front_mut() else {
                return;
            };
            let Some((from, end)) = job.span else {
                continue;
            };
            let chunk = self
                .source
                .as_mut()
                .and_then(|s| s.take(from, end, 4096).map(|c| (c, s.position())));
            match chunk {
                Some((samples, at)) => {
                    job.span = Some((at, end));
                    let samples = match job.resample.as_mut() {
                        Some(r) => r.push(&samples),
                        None => samples,
                    };
                    job.pushed += samples.len() as u64;
                    let token = job.token;
                    if self.playback.ensure_player().is_err() {
                        log::warn!("recorded audio: the output did not open");
                    }
                    self.playback.on_audio(token, &samples);
                    self.mark_words();
                }
                None => {
                    job.clip += 1;
                    job.span = None;
                }
            }
        }
    }
}

impl ClipPlayer for BookPlayer {
    fn play(&mut self, id: UtteranceId, text: &str, clips: &[RecordedClip]) {
        let token = self.playback.next_token();
        self.playback.enqueue(id, token, 0, ());
        self.jobs.push_back(Job {
            token,
            clips: clips.to_vec(),
            clip: 0,
            span: None,
            resample: None,
            words: words_of(text),
            text_len: u32::try_from(text.len()).unwrap_or(u32::MAX),
            total: None,
            pushed: 0,
            next_word: 0,
            next_keepalive: 0,
            failed: false,
        });
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.pump();
        self.playback.emit(sink);
    }

    fn stop(&mut self) {
        self.playback.stop();
        self.jobs.clear();
    }

    fn pause(&mut self) {
        self.playback.pause();
    }

    fn resume(&mut self) {
        self.playback.resume();
    }

    fn set_volume(&mut self, volume: Volume) {
        self.playback.set_gain(volume.fraction());
    }
}

#[cfg(test)]
mod tests;
