//! The karaoke video through a fake ffmpeg (`examples/fake_ffmpeg.rs`,
//! built by `cargo test`), which records its arguments, the caption and
//! chapter files, and the frames it is fed. A real ffmpeg run is a manual
//! check.

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use textweaver_core::{CharRange, MarkerKind};
use textweaver_export::video::{FPS, schedule};
use textweaver_export::{AudioFormat, ExportError, ExportOptions, Progress, export};
use textweaver_speech::RecordingBackend;
use textweaver_text::{Document, DocumentData, DocumentMeta, Marker};

fn book() -> Document {
    let text = "Intro\n\nHello world. Paid $5 today.\n\nNext\n\nThe end.";
    let at = |s: &str| {
        let b = text.find(s).unwrap();
        let start = text[..b].chars().count();
        CharRange::new(start, start + s.chars().count())
    };
    Document::from(DocumentData {
        meta: DocumentMeta {
            title: Some("Sample Book".into()),
            ..DocumentMeta::default()
        },
        text: text.into(),
        markers: vec![
            Marker::new(MarkerKind::Heading, at("Intro")).with_level(1),
            Marker::new(MarkerKind::Heading, at("Next")).with_level(2),
        ],
    })
}

/// The fake ffmpeg, copied into `dir` as `ffmpeg` (so its record lands
/// there).
fn fake_ffmpeg(dir: &Path) -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    // target/<profile>/deps/it-<hash> -> target/<profile>/examples/
    let examples = exe.parent().unwrap().parent().unwrap().join("examples");
    let name = format!("fake_ffmpeg{}", std::env::consts::EXE_SUFFIX);
    let built = examples.join(&name);
    assert!(
        built.is_file(),
        "the fake ffmpeg is built by `cargo test -p textweaver-export` (examples); not found at {}",
        built.display()
    );
    let to = dir.join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(&built, &to).unwrap();
    to
}

fn record(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("record.txt")).unwrap_or_default()
}

#[test]
fn the_video_has_a_frame_per_word_captions_and_chapters() {
    let tools = tempfile::tempdir().unwrap();
    let ff = fake_ffmpeg(tools.path());
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.mp4");
    let (mut backend, _rec) = RecordingBackend::new();
    let report = export(
        &book(),
        &mut backend,
        &out,
        None,
        Some(&ff),
        &ExportOptions::default(),
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(report.format, AudioFormat::Mp4);
    let stats = report.video.unwrap();
    let rec = record(tools.path());

    // Frames: the video's length at ten a second, all received whole.
    let duration = report.timeline.duration_ms;
    assert_eq!(stats.frames, (duration * u64::from(FPS)).div_ceil(1000));
    assert!(
        rec.contains(&format!("frames: {}\n", stats.frames)),
        "{rec}"
    );
    assert!(rec.contains("left over: 0\n"));

    // Drawn: one frame per spoken word, plus one for each pause.
    let runs = schedule(&report.timeline, FPS);
    assert_eq!(stats.drawn, runs.len() as u64);
    let words: usize = report
        .timeline
        .sentences
        .iter()
        .map(|s| s.words.len())
        .sum();
    let spoken = runs.iter().filter(|(s, _)| s.word.is_some()).count();
    assert_eq!(spoken, words, "a drawn frame for every word");
    assert!(stats.drawn < stats.frames, "repeats are not drawn again");

    // The caption track and the chapters went to ffmpeg.
    assert!(rec.contains("encoders asked"));
    assert!(rec.contains("arg: pipe:0\n"));
    assert!(rec.contains("arg: libx264\n"));
    assert!(rec.contains("arg: mov_text\n"));
    assert!(rec.contains("arg: 2:s\n"));
    assert!(rec.contains("arg: -map_chapters\n"));
    assert!(rec.contains("captions.vtt:\nWEBVTT"), "{rec}");
    assert!(rec.contains("Hello world."));
    assert!(rec.contains(";FFMETADATA1"));
    assert!(rec.contains("title=Intro"));
    assert!(rec.contains("title=Next"));
    assert_eq!(std::fs::read(&out).unwrap(), b"fake mp4");
}

#[test]
fn a_stop_during_the_encode_removes_the_partial_file() {
    let tools = tempfile::tempdir().unwrap();
    let ff = fake_ffmpeg(tools.path());
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.mp4");
    let (mut backend, _rec) = RecordingBackend::new();
    // Synthesis finishes; the stop comes a few frames into the encode.
    let mut asked_after = 0;
    let err = export(
        &book(),
        &mut backend,
        &out,
        None,
        Some(&ff),
        &ExportOptions::default(),
        &mut |p: Progress| {
            if p.total > 0 && p.done == p.total {
                asked_after += 1;
            }
            if asked_after > 4 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::Cancelled), "{err}");
    assert!(!out.exists(), "the partial video is removed");
}

#[test]
fn a_failing_ffmpeg_says_why_and_leaves_no_file() {
    let tools = tempfile::tempdir().unwrap();
    let ff = fake_ffmpeg(tools.path());
    std::fs::write(tools.path().join("fail"), b"").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.mp4");
    let (mut backend, _rec) = RecordingBackend::new();
    let err = export(
        &book(),
        &mut backend,
        &out,
        None,
        Some(&ff),
        &ExportOptions::default(),
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "ffmpeg failed: fake failure: no such encoder"
    );
    assert!(!out.exists());
}

#[test]
fn without_ffmpeg_the_video_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("book.mp4");
    let (mut backend, rec) = RecordingBackend::new();
    let err = export(
        &book(),
        &mut backend,
        &out,
        None,
        None,
        &ExportOptions::default(),
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::NoFfmpeg("MP4 video")), "{err}");
    assert!(
        err.to_string()
            .starts_with("Writing MP4 video needs ffmpeg")
    );
    assert!(rec.spoken_texts().is_empty(), "nothing was read aloud");
    assert!(!out.exists());
}
