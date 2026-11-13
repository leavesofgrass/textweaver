//! Caption files (SubRip and WebVTT) read as transcripts: paragraphs, the
//! speakers' names, a heading every five minutes, and no cue numbers,
//! times or markup in the text.

use std::path::PathBuf;

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::Document;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/captions")
        .join(name);
    std::fs::read_to_string(path).expect("caption fixture")
}

fn load_bytes(data: Vec<u8>, hint: &str, options: &LoadOptions) -> Document {
    Registry::with_builtins()
        .load(
            &Source::Bytes {
                data,
                hint: hint.into(),
            },
            options,
        )
        .unwrap()
}

fn headings(doc: &Document) -> Vec<String> {
    doc.marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| doc.slice(m.range))
        .collect()
}

/// No time, arrow, cue number or tag is left in the text.
fn assert_clean(text: &str) {
    for bad in ["-->", "00:", "<", ">", "{", "&amp;", "WEBVTT", "NOTE"] {
        assert!(!text.contains(bad), "{bad:?} in {text:?}");
    }
    assert!(
        !text.lines().any(|l| l.trim().parse::<u32>().is_ok()),
        "a cue number in {text:?}"
    );
}

const LECTURE: &str = "Good morning. Today we look at cells. A cell is the smallest unit of life.\n\n\
Is that true for viruses?\n\n\
Good question. No.\n\n\
Minute 5\n\n\
After the break, membranes. This cue has a damaged time.\n\n\
Minute 10\n\n\
That is all.";

#[test]
fn srt_reads_as_paragraphs_with_minute_headings() {
    let doc = Registry::with_builtins()
        .load(
            &Source::Path(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures/captions/lecture.srt"),
            ),
            &LoadOptions::default(),
        )
        .unwrap();
    let text = doc.text().to_string();
    assert_eq!(doc.meta.format, "captions");
    assert_eq!(doc.meta.title.as_deref(), Some("lecture.srt"));
    assert_eq!(text, LECTURE);
    assert_eq!(headings(&doc), ["Minute 5", "Minute 10"]);
    assert_clean(&text);
}

#[test]
fn a_byte_order_mark_and_crlf_change_nothing() {
    let crlf = format!("\u{feff}{}", fixture("lecture.srt").replace('\n', "\r\n"));
    let doc = load_bytes(crlf.into_bytes(), "srt", &LoadOptions::default());
    assert_eq!(doc.text().to_string(), LECTURE);
}

#[test]
fn vtt_keeps_voices_and_drops_its_header_notes_and_roll_up() {
    let doc = load_bytes(
        fixture("lab.vtt").into_bytes(),
        "vtt",
        &LoadOptions::default(),
    );
    let text = doc.text().to_string();
    assert_eq!(
        text,
        "Ada Example: Welcome to the lab. Today we measure & compare.\n\n\
         Bo Example: Can we start now? I have a question. About the samples."
    );
    assert_clean(&text);
    assert!(headings(&doc).is_empty());
}

#[test]
fn the_minute_heading_comes_from_the_options() {
    let options = LoadOptions {
        minute_heading: "Minuto {n}".into(),
        ..LoadOptions::default()
    };
    let doc = load_bytes(fixture("lecture.srt").into_bytes(), "srt", &options);
    assert_eq!(headings(&doc), ["Minuto 5", "Minuto 10"]);
}

#[test]
fn a_file_without_cues_is_plain_text_and_says_so() {
    let doc = load_bytes(
        b"Just some words.\nNo times here.".to_vec(),
        "srt",
        &LoadOptions::default(),
    );
    assert_eq!(doc.text().to_string(), "Just some words.\nNo times here.");
    assert_eq!(
        textweaver_formats::warnings(&doc.meta),
        [textweaver_formats::captions::NO_CUES_WARNING]
    );
}

#[test]
fn hostile_captions_do_not_panic() {
    let long_name = "N".repeat(10_000);
    let many_lines = "line\n".repeat(10_000);
    for text in [
        String::new(),
        "-->".into(),
        "WEBVTT".into(),
        "1\n-->\n".into(),
        "00:00:00,000 --> 99999999999:00:00,000\n<v".into(),
        "00:00.000 --> 00:01.000\n<v ".to_owned() + &long_name + ">hi",
        "00:00.000 --> 00:01.000\n".to_owned() + &many_lines,
        "00:00.000 --> 00:01.000\n&#xFFFFFFFF; &#0; &; {\\".into(),
        "\u{feff}\u{feff}\r\r\n\n-->\u{0}\u{7}".into(),
    ] {
        let doc = load_bytes(text.into_bytes(), "vtt", &LoadOptions::default());
        let t = doc.text().to_string();
        assert!(!t.contains(&long_name), "the name is cut");
        assert!(t.matches("line").count() <= textweaver_formats::captions::MAX_CUE_LINES);
    }
}

#[test]
fn caption_files_are_documents_in_the_format_lists() {
    let r = Registry::with_builtins();
    assert!(r.extensions().contains(&"srt"));
    assert!(r.extensions().contains(&"vtt"));
}
