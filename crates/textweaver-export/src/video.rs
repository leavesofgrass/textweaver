//! Karaoke video: the text shown as it is read, the spoken word in bold
//! and underlined, with the captions as a soft subtitle track and the
//! chapters, in an MP4 made by ffmpeg.
//!
//! textweaver draws the frames itself (the `video` feature: a small
//! renderer with the bundled reading font) and pipes them to ffmpeg as raw
//! RGBA at a low, constant frame rate. A frame is drawn only when the
//! spoken word changes; between changes the same bytes are sent again, which
//! the H.264 encoder stores in almost no space. ffmpeg is found exactly as
//! for M4B ([`crate::ffmpeg::find`]) and is never downloaded or bundled.

use std::ffi::OsString;
use std::path::Path;

use crate::timeline::Timeline;

#[cfg(feature = "video")]
pub mod font;
#[cfg(feature = "video")]
pub mod frame;

/// Frames per second: words are shown to a tenth of a second.
pub const FPS: u32 = 10;

/// How the video looks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoOptions {
    /// The page color (the theme's background), as RGB.
    pub page: [u8; 3],
    /// The text color (the theme's foreground), as RGB.
    pub text: [u8; 3],
}

impl Default for VideoOptions {
    /// Light text on a dark page.
    fn default() -> Self {
        VideoOptions {
            page: [0x12, 0x14, 0x1c],
            text: [0xf2, 0xf2, 0xf2],
        }
    }
}

/// What is on screen during one frame: the sentence and the spoken word
/// (indexes into [`Timeline::sentences`] and its words).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameState {
    /// The sentence shown, if any has started.
    pub sentence: Option<usize>,
    /// The word marked as spoken, if one is being said.
    pub word: Option<usize>,
}

/// The frames of a video, as runs: each state, and how many frames in a
/// row show it. One run is one drawn frame; the counts add up to the
/// video's length at [`FPS`].
pub fn schedule(timeline: &Timeline, fps: u32) -> Vec<(FrameState, u64)> {
    let fps = u64::from(fps.max(1));
    let total = (timeline.duration_ms * fps).div_ceil(1000).max(1);
    let mut runs: Vec<(FrameState, u64)> = Vec::new();
    let mut s = 0usize;
    for i in 0..total {
        let t = i * 1000 / fps;
        while s + 1 < timeline.sentences.len() && timeline.sentences[s + 1].start_ms <= t {
            s += 1;
        }
        let state = match timeline.sentences.get(s) {
            Some(sentence) if sentence.start_ms <= t => FrameState {
                sentence: Some(s),
                word: sentence
                    .words
                    .iter()
                    .position(|w| w.start_ms <= t && t < w.end_ms),
            },
            _ => FrameState {
                sentence: None,
                word: None,
            },
        };
        match runs.last_mut() {
            Some((last, n)) if *last == state => *n += 1,
            _ => runs.push((state, 1)),
        }
    }
    runs
}

/// The H.264 encoders tried, best first: libx264, then the platform's own
/// (Media Foundation, VideoToolbox), then OpenH264.
const H264: [&str; 4] = ["libx264", "h264_mf", "h264_videotoolbox", "libopenh264"];

/// The video encoder to use from `ffmpeg -encoders` output: the first
/// H.264 encoder offered, else MPEG-4 Part 2, which every ffmpeg has.
pub fn pick_encoder(encoders: &str) -> &'static str {
    let offered = |name: &str| {
        encoders
            .lines()
            .any(|l| l.split_whitespace().nth(1) == Some(name))
    };
    H264.into_iter().find(|e| offered(e)).unwrap_or("mpeg4")
}

/// The ffmpeg arguments that make `out` from raw frames on standard input,
/// the audio in `wav`, the WebVTT captions in `vtt`, and the title and
/// chapters in `metadata`, with `encoder` for the picture.
pub fn args(wav: &Path, vtt: &Path, metadata: &Path, out: &Path, encoder: &str) -> Vec<OsString> {
    let size = format!("{}x{}", frame_size().0, frame_size().1);
    let fps = FPS.to_string();
    let mut a: Vec<OsString> = Vec::new();
    let mut push = |s: &[&str]| a.extend(s.iter().map(OsString::from));
    push(&["-y", "-hide_banner", "-loglevel", "error"]);
    push(&[
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-s",
        &size,
        "-framerate",
        &fps,
        "-i",
        "pipe:0",
    ]);
    a.push("-i".into());
    a.push(wav.into());
    a.push("-i".into());
    a.push(vtt.into());
    a.push("-i".into());
    a.push(metadata.into());
    let mut push = |s: &[&str]| a.extend(s.iter().map(OsString::from));
    push(&["-map", "0:v", "-map", "1:a", "-map", "2:s"]);
    push(&["-map_metadata", "3", "-map_chapters", "3"]);
    push(&["-codec:v", encoder]);
    match encoder {
        "libx264" => push(&["-preset", "veryfast", "-tune", "stillimage", "-crf", "23"]),
        "mpeg4" => push(&["-qscale:v", "4"]),
        _ => push(&["-b:v", "1M"]),
    }
    push(&["-pix_fmt", "yuv420p"]);
    push(&["-codec:a", "aac", "-b:a", "96k"]);
    push(&[
        "-codec:s",
        "mov_text",
        "-metadata:s:s:0",
        "handler_name=Captions",
    ]);
    push(&["-f", "mp4", "-movflags", "+faststart"]);
    a.push(out.into());
    a
}

/// The frame size in pixels.
pub fn frame_size() -> (u32, u32) {
    (1280, 720)
}

/// What the video encoder did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct VideoStats {
    /// Frames sent to ffmpeg (the video's length at [`FPS`]).
    pub frames: u64,
    /// Frames drawn: one for each change of the spoken word.
    pub drawn: u64,
    /// Frames per second.
    pub fps: u32,
}

#[cfg(feature = "video")]
pub use encode::encode_with_stop;

#[cfg(feature = "video")]
mod encode {
    use std::io::Write;
    use std::path::Path;
    use std::process::Stdio;

    use super::frame::FrameRenderer;
    use super::{FPS, VideoOptions, VideoStats, args, pick_encoder, schedule};
    use crate::ExportError;
    use crate::timeline::Timeline;

    /// Asks ffmpeg which encoders it has, for [`pick_encoder`].
    fn encoders(ffmpeg: &Path) -> String {
        textweaver_core::process::command(ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map(|o| textweaver_core::process::decode_output(&o.stdout))
            .unwrap_or_default()
    }

    /// Draws the frames for `timeline` and has `ffmpeg` encode them with
    /// the audio in `wav`, the captions in `vtt` and the chapters in
    /// `metadata` into `out`. `stop` is asked before every frame and while
    /// ffmpeg finishes; a stop kills ffmpeg, removes `out`, and ends in
    /// [`ExportError::Cancelled`]. Any failure removes `out`.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_with_stop(
        ffmpeg: &Path,
        timeline: &Timeline,
        wav: &Path,
        vtt: &Path,
        metadata: &Path,
        out: &Path,
        opts: &VideoOptions,
        stop: &dyn Fn() -> bool,
    ) -> Result<VideoStats, ExportError> {
        let result = encode(ffmpeg, timeline, wav, vtt, metadata, out, opts, stop);
        if result.is_err() {
            let _ = std::fs::remove_file(out);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn encode(
        ffmpeg: &Path,
        timeline: &Timeline,
        wav: &Path,
        vtt: &Path,
        metadata: &Path,
        out: &Path,
        opts: &VideoOptions,
        stop: &dyn Fn() -> bool,
    ) -> Result<VideoStats, ExportError> {
        let mut renderer = FrameRenderer::new(opts).map_err(|e| ExportError::Video(e.0.clone()))?;
        let encoder = pick_encoder(&encoders(ffmpeg));
        let mut child = textweaver_core::process::command(ffmpeg)
            .args(args(wav, vtt, metadata, out, encoder))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ExportError::io(ffmpeg, e))?;
        let reader = crate::ffmpeg::drain(child.stderr.take());
        let mut stdin = child.stdin.take();
        let mut stats = VideoStats {
            fps: FPS,
            ..VideoStats::default()
        };
        let mut broken = false;
        'frames: for (state, count) in schedule(timeline, FPS) {
            if stop() {
                return Err(crate::ffmpeg::kill(child, reader));
            }
            let sentence = state.sentence.and_then(|i| timeline.sentences.get(i));
            let spoken = sentence
                .zip(state.word)
                .and_then(|(s, w)| s.words.get(w))
                .map(|w| w.caption.clone());
            let bytes = renderer.draw(sentence.map(|s| s.text.as_str()), spoken);
            stats.drawn += 1;
            let Some(pipe) = stdin.as_mut() else {
                break;
            };
            for _ in 0..count {
                if stop() {
                    return Err(crate::ffmpeg::kill(child, reader));
                }
                if pipe.write_all(bytes).is_err() {
                    // ffmpeg stopped reading: its own message says why.
                    broken = true;
                    break 'frames;
                }
                stats.frames += 1;
            }
        }
        drop(stdin);
        let waited = crate::ffmpeg::wait_with_stop(child, reader, ffmpeg, stop);
        match waited {
            Ok(()) if broken => Err(ExportError::Ffmpeg(
                "ffmpeg stopped reading the video frames".to_owned(),
            )),
            Ok(()) => Ok(stats),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::{TimedSentence, TimedWord};

    fn word(start_ms: u64, end_ms: u64, caption: std::ops::Range<usize>) -> TimedWord {
        TimedWord {
            start_ms,
            end_ms,
            source: None,
            caption,
            text: String::new(),
        }
    }

    fn timeline() -> Timeline {
        Timeline {
            sentences: vec![
                TimedSentence {
                    start_ms: 0,
                    end_ms: 1000,
                    source: None,
                    text: "One two.".into(),
                    spoken: "One two.".into(),
                    words: vec![word(0, 500, 0..3), word(500, 1000, 4..8)],
                },
                TimedSentence {
                    start_ms: 1200,
                    end_ms: 2000,
                    source: None,
                    text: "Three.".into(),
                    spoken: "Three.".into(),
                    words: vec![word(1200, 2000, 0..6)],
                },
            ],
            duration_ms: 2000,
            ..Timeline::default()
        }
    }

    #[test]
    fn a_frame_is_drawn_per_word_change() {
        let runs = schedule(&timeline(), FPS);
        let states: Vec<(Option<usize>, Option<usize>)> =
            runs.iter().map(|(s, _)| (s.sentence, s.word)).collect();
        assert_eq!(
            states,
            vec![
                (Some(0), Some(0)),
                (Some(0), Some(1)),
                // The pause between sentences: the sentence, nothing spoken.
                (Some(0), None),
                (Some(1), Some(0)),
            ]
        );
        let counts: Vec<u64> = runs.iter().map(|r| r.1).collect();
        assert_eq!(counts, vec![5, 5, 2, 8]);
        assert_eq!(
            counts.iter().sum::<u64>(),
            20,
            "two seconds at ten a second"
        );
    }

    #[test]
    fn an_hour_draws_words_not_refreshes() {
        // 265 words a minute for an hour, in sentences of ten words.
        let mut sentences = Vec::new();
        let ms_per_word = 60_000 / 265;
        let mut t = 0;
        for _ in 0..(265 * 60 / 10) {
            let start = t;
            let words: Vec<TimedWord> = (0..10)
                .map(|i| {
                    let w = word(t, t + ms_per_word, i * 5..i * 5 + 4);
                    t += ms_per_word;
                    w
                })
                .collect();
            sentences.push(TimedSentence {
                start_ms: start,
                end_ms: t,
                source: None,
                text: String::new(),
                spoken: String::new(),
                words,
            });
        }
        let tl = Timeline {
            sentences,
            duration_ms: t,
            ..Timeline::default()
        };
        let runs = schedule(&tl, FPS);
        assert_eq!(runs.len(), 265 * 60, "one drawn frame per word");
    }

    #[test]
    fn the_encoder_prefers_h264() {
        let list = " V....D libx264   H.264\n V....D h264_mf  H264 via MediaFoundation\n";
        assert_eq!(pick_encoder(list), "libx264");
        assert_eq!(pick_encoder(" V....D h264_mf  x\n"), "h264_mf");
        assert_eq!(pick_encoder(""), "mpeg4");
    }

    #[test]
    fn the_arguments_carry_captions_and_chapters() {
        let a = args(
            Path::new("a.wav"),
            Path::new("c.vtt"),
            Path::new("m.txt"),
            Path::new("o.mp4"),
            "libx264",
        );
        let j = a
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(j.contains("-f rawvideo -pix_fmt rgba -s 1280x720 -framerate 10 -i pipe:0"));
        assert!(j.contains("-i a.wav -i c.vtt -i m.txt"));
        assert!(j.contains("-map 0:v -map 1:a -map 2:s -map_metadata 3 -map_chapters 3"));
        assert!(j.contains("-codec:v libx264"));
        assert!(j.contains("-codec:s mov_text"));
        assert!(j.ends_with("-movflags +faststart o.mp4"), "{j}");
    }
}
