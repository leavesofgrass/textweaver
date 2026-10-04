//! Audio and subtitle export (ADR-0011).
//!
//! A document is read into audio sentence by sentence through any speech
//! backend with [`Caps::SYNTH_TO_FILE`]:
//!
//! 1. [`textweaver_text::plan`] cuts the document into utterances, as for
//!    reading aloud (structure announcements included, per the narration
//!    policy), and each is normalized on its own with the same pipeline the
//!    speech service uses (skipping what the engine normalizes itself).
//! 2. [`SpeechBackend::synthesize_utterance`] writes each utterance to its
//!    own WAV file (in a private temporary folder) and reports word timings
//!    when the engine knows them.
//! 3. The pieces are joined into one WAV ([`wav::WavWriter`]), and the
//!    running length gives every sentence its exact start and end, and
//!    every reported word its time: the [`Timeline`].
//! 4. Chapters come from headings and section breaks ([`chapters`]).
//! 5. For `.flac`, the WAV is encoded in process, with the title and
//!    chapters as Vorbis comments (`flac`, the `flac` feature). For `.mp3`,
//!    LAME encodes it in process and the title and chapters go in an ID3v2
//!    tag (`mp3`, the `mp3` feature). For `.opus`, libopus encodes it
//!    in process as Ogg Opus, mono at a speech bit rate, with the title
//!    and chapters as Vorbis comments (`opus`, the `opus` feature). For
//!    `.m4b` (and MP3, FLAC or Opus in a build without those features),
//!    ffmpeg converts the WAV, with the
//!    title and chapters in its metadata ([`ffmpeg`]); without ffmpeg
//!    those fail with a clear message and the others still work. A `.wav`
//!    gets the title and chapters as an ID3 tag (`id3tags`, the `id3`
//!    feature).
//! 6. Subtitles ([`cues`]) are SRT or WebVTT cues from the timeline, by
//!    caption line (Star's grouping) or by word.
//!
//! Progress is reported per sentence, and the caller can cancel between
//! sentences, and during encoding: the progress callback is asked again,
//! with the last count, between encoded blocks (and every 100 ms while
//! ffmpeg runs, which is then stopped). A stop at any step ends in
//! [`ExportError::Cancelled`] with no output file left behind.

pub mod chapters;
pub mod cues;
pub mod ffmpeg;
#[cfg(feature = "flac")]
pub mod flac;
#[cfg(feature = "id3")]
pub mod id3tags;
#[cfg(feature = "mp3")]
pub mod mp3;
#[cfg(feature = "opus")]
pub mod opus;
#[cfg(any(feature = "flac", feature = "mp3", feature = "opus"))]
pub(crate) mod pcm;
pub mod timeline;
pub mod vorbis;
pub mod wav;

use std::cell::{Cell, RefCell};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use serde::Serialize;
use textweaver_core::{CharRange, PunctuationLevel};
use textweaver_speech::{Caps, NormalizeConfig, Pipeline, SpeechBackend, SpeechError};
use textweaver_text::{Document, NarrationPolicy, plan};

pub use chapters::{ChapterNames, ChapterOptions};
pub use cues::{CaptionLine, CaptionMeta, Cue, CueOptions, Karaoke, SubtitleFormat};
pub use ffmpeg::AudioFormat;
pub use timeline::{Chapter, TimedSentence, TimedWord, Timeline};

/// Export failures. Every message reads as a sentence.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The backend cannot write audio files.
    #[error("The {0} voice cannot write audio files.")]
    NoFileSynthesis(String),
    /// The output name has no supported extension.
    #[error("Cannot write {0}: use a .wav, .flac, .mp3, .opus, or .m4b file name.")]
    UnsupportedFormat(PathBuf),
    /// The FLAC encoder failed.
    #[error("The FLAC encoder failed: {0}")]
    Flac(String),
    /// The MP3 encoder (LAME) failed.
    #[error("The MP3 encoder failed: {0}")]
    Mp3(String),
    /// The Opus encoder failed.
    #[error("The Opus encoder failed: {0}")]
    Opus(String),
    /// The tags could not be written into the file.
    #[error("Cannot write the title and chapters into {path}: {message}")]
    Tags {
        /// The audio file.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// The subtitle name has no supported extension.
    #[error("Cannot write subtitles to {0}: use a .srt, .vtt, or .ass file name.")]
    UnsupportedSubtitles(PathBuf),
    /// ffmpeg is needed and was not found.
    #[error(
        "Writing {0} needs ffmpeg, which was not found. Install ffmpeg, or set TEXTWEAVER_FFMPEG to its path, or export to .flac, .mp3, .opus, or .wav."
    )]
    NoFfmpeg(&'static str),
    /// ffmpeg failed.
    #[error("ffmpeg failed: {0}")]
    Ffmpeg(String),
    /// The engine failed on one sentence.
    #[error("The voice failed on sentence {sentence}: {source}")]
    Speech {
        /// 1-based sentence number.
        sentence: usize,
        /// What the engine said.
        source: SpeechError,
    },
    /// An engine wrote something that is not a WAV file export can read.
    #[error("Cannot read the audio in {path}: {reason}.")]
    BadWav {
        /// The file.
        path: PathBuf,
        /// What is wrong with it.
        reason: String,
    },
    /// Two sentences came back in different audio formats.
    #[error("The voice changed audio format in {path}: expected {expected}, found {found}.")]
    FormatMismatch {
        /// The file with the new format.
        path: PathBuf,
        /// The format of the first sentence.
        expected: String,
        /// The format found.
        found: String,
    },
    /// The audio would not fit in one WAV file (4 GB).
    #[error("The audio is too long for one WAV file; export a smaller part.")]
    TooLong,
    /// The document has nothing to read in the range.
    #[error("There is nothing to read aloud.")]
    Empty,
    /// The caller cancelled.
    #[error("Export canceled.")]
    Cancelled,
    /// A file operation failed.
    #[error("Cannot use {path}: {message}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The system's message.
        message: String,
    },
}

impl ExportError {
    pub(crate) fn io(path: &Path, e: std::io::Error) -> Self {
        ExportError::Io {
            path: path.to_owned(),
            message: e.to_string(),
        }
    }
}

/// How a document is read into audio.
#[derive(Clone, Debug, Default)]
pub struct ExportOptions {
    /// The part of the document to export; `None` for all of it.
    pub range: Option<CharRange>,
    /// How the document is cut into utterances and structure announced.
    pub narration: NarrationPolicy,
    /// Normalization settings (as for reading aloud).
    pub normalize: NormalizeConfig,
    /// Punctuation verbosity.
    pub punctuation: PunctuationLevel,
    /// Speak capitals within words separately.
    pub split_caps: bool,
    /// Silence added between utterances, in ms (0: the engine's own pauses).
    pub gap_ms: u64,
    /// Which markers start chapters.
    pub chapters: ChapterOptions,
    /// Names for untitled chapters, in the interface language.
    pub chapter_names: ChapterNames,
    /// What the subtitle file says about itself (a WebVTT `NOTE`, the ASS
    /// title); the title defaults to the document's.
    pub captions: CaptionMeta,
}

/// Progress through an export, reported before each sentence and once at
/// the end (`done == total`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Sentences done.
    pub done: usize,
    /// Sentences in all.
    pub total: usize,
}

impl Progress {
    /// Percent done, 0 to 100.
    pub fn percent(&self) -> usize {
        (self.done * 100).checked_div(self.total).unwrap_or(100)
    }
}

/// Reads `doc` into one WAV file at `out` through `backend` and returns the
/// timeline. `progress` is called before each sentence and at the end;
/// returning `ControlFlow::Break` cancels (the partial file is removed).
pub fn synthesize_wav(
    doc: &Document,
    backend: &mut dyn SpeechBackend,
    out: &Path,
    opts: &ExportOptions,
    progress: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<Timeline, ExportError> {
    let result = synthesize_inner(doc, backend, out, opts, progress);
    if result.is_err() {
        let _ = std::fs::remove_file(out);
    }
    result
}

fn synthesize_inner(
    doc: &Document,
    backend: &mut dyn SpeechBackend,
    out: &Path,
    opts: &ExportOptions,
    progress: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<Timeline, ExportError> {
    let caps = backend.capabilities();
    if !caps.contains(Caps::SYNTH_TO_FILE) {
        return Err(ExportError::NoFileSynthesis(backend.id().to_owned()));
    }
    let range = opts.range.unwrap_or_else(|| doc.full_range());
    let planned = plan(doc, range, &opts.narration);
    if planned.iter().all(|u| u.text.trim().is_empty()) {
        return Err(ExportError::Empty);
    }
    let pipeline = Pipeline::for_settings(
        &opts.normalize,
        opts.punctuation,
        opts.split_caps,
        caps.contains(Caps::NATIVE_NORMALIZATION),
    );
    let tmp = tempfile::Builder::new()
        .prefix("textweaver-export-")
        .tempdir()
        .map_err(|e| ExportError::io(&std::env::temp_dir(), e))?;
    let mut writer = wav::WavWriter::create(out)?;
    let total = planned.len();
    let mut sentences: Vec<TimedSentence> = Vec::with_capacity(total);
    for (i, u) in planned.into_iter().enumerate() {
        if progress(Progress { done: i, total }).is_break() {
            return Err(ExportError::Cancelled);
        }
        let u = pipeline.apply(u);
        if u.text.trim().is_empty() {
            continue;
        }
        let piece = tmp.path().join(format!("{i:06}.wav"));
        let synth =
            backend
                .synthesize_utterance(&u, &piece)
                .map_err(|source| ExportError::Speech {
                    sentence: i + 1,
                    source,
                })?;
        let data = wav::WavData::read(&piece)?;
        let _ = std::fs::remove_file(&piece);
        if !sentences.is_empty() {
            writer.silence(opts.gap_ms)?;
        }
        let start = writer.frames();
        writer.append(&data, &piece)?;
        let end = writer.frames();
        let format = writer.format().cloned().unwrap_or(data.format);
        sentences.push(timeline::timed_sentence(
            doc,
            &u,
            format.ms(start),
            format.ms(end),
            &synth.words,
        ));
    }
    let duration_ms = writer.format().map_or(0, |f| f.ms(writer.frames()));
    writer.finish()?;
    // A stop asked for during the last sentence is a stop: the file is
    // removed by `synthesize_wav`, never reported as written.
    if progress(Progress { done: total, total }).is_break() {
        return Err(ExportError::Cancelled);
    }
    let chapters = chapters::place_named(
        doc,
        &sentences,
        duration_ms,
        &opts.chapters,
        &opts.chapter_names,
    );
    Ok(Timeline {
        sentences,
        duration_ms,
        chapters,
        title: doc.meta.title.clone(),
        author: doc.meta.author.clone(),
    })
}

/// Renders subtitles for a timeline, with no note (see
/// [`cues::render_file`]).
pub fn subtitles(timeline: &Timeline, format: SubtitleFormat, opts: &CueOptions) -> String {
    cues::render_file(timeline, format, opts, None)
}

/// What [`export`] wrote.
#[derive(Clone, Debug, Serialize)]
pub struct ExportReport {
    /// The audio file.
    pub out: PathBuf,
    /// Its format.
    pub format: AudioFormat,
    /// The subtitle file, if one was asked for.
    pub subtitles: Option<PathBuf>,
    /// The ffmpeg used for conversion, if any.
    pub ffmpeg: Option<PathBuf>,
    /// Sentences, words, and chapters with their times.
    pub timeline: Timeline,
}

/// Where the subtitles go and how they are built.
#[derive(Clone, Debug)]
pub struct SubtitleRequest {
    /// The `.srt` or `.vtt` file.
    pub path: PathBuf,
    /// Cue options.
    pub cues: CueOptions,
}

/// Exports `doc` to `out` (`.wav`, `.flac`, `.mp3`, `.opus`, or `.m4b`, by
/// extension), with optional subtitles and the title and chapters in the
/// file's own tag format. `ffmpeg` is the converter to use for MP3 and M4B
/// (usually [`ffmpeg::find`]); `None` makes those formats fail with a clear
/// message before any synthesis starts.
pub fn export(
    doc: &Document,
    backend: &mut dyn SpeechBackend,
    out: &Path,
    subtitles_to: Option<&SubtitleRequest>,
    ffmpeg: Option<&Path>,
    opts: &ExportOptions,
    progress: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<ExportReport, ExportError> {
    let format =
        AudioFormat::from_path(out).ok_or_else(|| ExportError::UnsupportedFormat(out.into()))?;
    let sub_format = subtitles_to
        .map(|s| {
            SubtitleFormat::from_path(&s.path)
                .ok_or_else(|| ExportError::UnsupportedSubtitles(s.path.clone()))
        })
        .transpose()?;
    let ffmpeg = match (format.needs_ffmpeg(), ffmpeg) {
        (false, _) => None,
        (true, Some(ff)) => Some(ff),
        (true, None) => return Err(ExportError::NoFfmpeg(format.name())),
    };
    // The progress callback also answers "stop?" after synthesis, asked
    // again with the last count it was given.
    let last = Cell::new(Progress { done: 0, total: 0 });
    let progress = RefCell::new(progress);
    let mut tracked = |p: Progress| {
        last.set(p);
        (*progress.borrow_mut())(p)
    };
    let stop = || (*progress.borrow_mut())(last.get()).is_break();
    let result = export_inner(doc, backend, out, format, ffmpeg, opts, &mut tracked, &stop)
        .and_then(|r| {
            if stop() {
                Err(ExportError::Cancelled)
            } else {
                Ok(r)
            }
        });
    let (timeline, ffmpeg_used) = match result {
        Ok(r) => r,
        Err(e) => {
            if matches!(e, ExportError::Cancelled) {
                let _ = std::fs::remove_file(out);
            }
            return Err(e);
        }
    };
    let subtitles_path = match (subtitles_to, sub_format) {
        (Some(req), Some(f)) => {
            let mut meta = opts.captions.clone();
            if meta.title.is_none() {
                meta.title = timeline.title.clone();
            }
            let text = cues::render_file(&timeline, f, &req.cues, Some(&meta));
            std::fs::write(&req.path, text).map_err(|e| ExportError::io(&req.path, e))?;
            Some(req.path.clone())
        }
        _ => None,
    };
    Ok(ExportReport {
        out: out.to_owned(),
        format,
        subtitles: subtitles_path,
        ffmpeg: ffmpeg_used,
        timeline,
    })
}

/// The audio part of [`export`]: synthesis, then encoding, which `stop`
/// can interrupt.
#[allow(clippy::too_many_arguments)]
fn export_inner(
    doc: &Document,
    backend: &mut dyn SpeechBackend,
    out: &Path,
    format: AudioFormat,
    ffmpeg: Option<&Path>,
    opts: &ExportOptions,
    progress: &mut dyn FnMut(Progress) -> ControlFlow<()>,
    stop: &dyn Fn() -> bool,
) -> Result<(Timeline, Option<PathBuf>), ExportError> {
    // Every format but WAV is made from a WAV in a private folder beside
    // the output (removed when done).
    let work = |out: &Path| {
        let dir = out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        tempfile::Builder::new()
            .prefix(".textweaver-export-")
            .tempdir_in(dir)
            .map_err(|e| ExportError::io(dir, e))
    };
    let (timeline, ffmpeg_used) = match (format, ffmpeg) {
        (_, Some(ff)) => {
            let work = work(out)?;
            let wav_path = work.path().join("audio.wav");
            let meta_path = work.path().join("metadata.txt");
            let timeline = synthesize_wav(doc, backend, &wav_path, opts, progress)?;
            let meta = chapters::ffmetadata(
                timeline.title.as_deref(),
                timeline.author.as_deref(),
                &timeline.chapters,
            );
            std::fs::write(&meta_path, meta).map_err(|e| ExportError::io(&meta_path, e))?;
            if let Err(e) =
                ffmpeg::run_with_stop(ff, &ffmpeg::args(&wav_path, &meta_path, out, format), stop)
            {
                let _ = std::fs::remove_file(out);
                return Err(e);
            }
            (timeline, Some(ff.to_owned()))
        }
        #[cfg(feature = "flac")]
        (AudioFormat::Flac, None) => {
            let work = work(out)?;
            let wav_path = work.path().join("audio.wav");
            let timeline = synthesize_wav(doc, backend, &wav_path, opts, progress)?;
            let comments = flac::comments(
                timeline.title.as_deref(),
                timeline.author.as_deref(),
                &timeline.chapters,
            );
            if let Err(e) = flac::encode_with_stop(&wav_path, out, &comments, stop) {
                let _ = std::fs::remove_file(out);
                return Err(e);
            }
            (timeline, None)
        }
        #[cfg(feature = "opus")]
        (AudioFormat::Opus, None) => {
            let work = work(out)?;
            let wav_path = work.path().join("audio.wav");
            let timeline = synthesize_wav(doc, backend, &wav_path, opts, progress)?;
            let comments = vorbis::comments(
                timeline.title.as_deref(),
                timeline.author.as_deref(),
                &timeline.chapters,
            );
            if let Err(e) = opus::encode_with_stop(&wav_path, out, &comments, stop) {
                let _ = std::fs::remove_file(out);
                return Err(e);
            }
            (timeline, None)
        }
        #[cfg(feature = "mp3")]
        (AudioFormat::Mp3, None) => {
            let work = work(out)?;
            let wav_path = work.path().join("audio.wav");
            let timeline = synthesize_wav(doc, backend, &wav_path, opts, progress)?;
            let written = mp3::encode_with_stop(&wav_path, out, stop).and_then(|()| {
                id3tags::write(
                    out,
                    timeline.title.as_deref(),
                    timeline.author.as_deref(),
                    &timeline.chapters,
                )
            });
            if let Err(e) = written {
                let _ = std::fs::remove_file(out);
                return Err(e);
            }
            (timeline, None)
        }
        _ => {
            let timeline = synthesize_wav(doc, backend, out, opts, progress)?;
            #[cfg(feature = "id3")]
            id3tags::write(
                out,
                timeline.title.as_deref(),
                timeline.author.as_deref(),
                &timeline.chapters,
            )?;
            (timeline, None)
        }
    };
    Ok((timeline, ffmpeg_used))
}
