use std::path::PathBuf;
use std::time::Duration;

use textweaver_core::UtteranceKind;
use textweaver_formats::{AudioClip, AudioPar, LoadOptions, Registry, Source};

use super::*;

const TEXT: &str = "Chapter One\n\nThe first words.\n\nNo audio here.\n\nLast words.";

fn doc() -> Document {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data: TEXT.as_bytes().to_vec(),
                hint: "txt".into(),
            },
            &LoadOptions::default(),
        )
        .unwrap()
}

fn par(from: usize, to: usize, file: &str) -> AudioPar {
    AudioPar {
        range: CharRange::new(from, to),
        clips: vec![AudioClip {
            file: PathBuf::from(file),
            begin: Duration::ZERO,
            end: None,
        }],
    }
}

fn audio() -> BookAudio {
    BookAudio {
        pars: vec![
            par(0, 13, "a.mp3"),
            par(13, 31, "b.mp3"),
            par(47, 58, "c.mp3"),
        ],
        has_text: true,
    }
}

#[test]
fn phrases_play_and_speech_reads_the_text_between() {
    let d = doc();
    assert_eq!(d.text().to_string(), TEXT);
    let policy = NarrationPolicy::default();
    let (u, _, pars) = plan(&d, CharRange::new(0, 58), &policy, &[], &audio());
    let texts: Vec<_> = u.iter().map(|u| u.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Chapter One",
            "The first words.",
            "No audio here.",
            "Last words."
        ]
    );
    assert!(u.iter().all(|u| u.kind == UtteranceKind::Text));
    let ranges: Vec<_> = pars.iter().map(|p| p.range).collect();
    assert_eq!(
        ranges,
        [
            CharRange::new(0, 13),
            CharRange::new(13, 31),
            CharRange::new(47, 58)
        ]
    );
    assert_eq!(pars[2].clips[0].file, PathBuf::from("c.mp3"));
    // Chunks are numbered in order.
    let chunks: Vec<_> = u.iter().map(|u| u.id.chunk).collect();
    assert_eq!(chunks, [0, 1, 2, 3]);
}

#[test]
fn reading_from_inside_a_phrase_plays_that_phrase() {
    let d = doc();
    let policy = NarrationPolicy::default();
    let (u, _, pars) = plan(&d, CharRange::new(17, 40), &policy, &[], &audio());
    assert_eq!(u[0].text, "first words.");
    assert_eq!(u[0].source_range().map(|r| r.start), Some(CharPos(17)));
    assert_eq!(pars.len(), 1);
    assert_eq!(pars[0].range, CharRange::new(17, 31));
    // The rest is speech, up to the end of the range.
    assert!(u[1].text.starts_with("No audio"), "{:?}", u[1].text);
    assert_eq!(u.len(), 2);
}
