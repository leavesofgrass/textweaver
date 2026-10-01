//! BRF formatting against reviewed snapshots in `fixtures/b`: the table
//! formats of BANA's Braille Formats: Principles of Print-to-Braille
//! Transcription (2016), section 11 (11.16 listed, 11.17 linear, 11.18
//! stairstep), and the typeform and capitals indicators of The Rules of
//! Unified English Braille (ICEB, second edition 2013), sections 8 and 9.
//!
//! Set `TW_BLESS=1` to write the snapshots again, then read the difference
//! before committing.

use std::path::PathBuf;

use textweaver_formats::{LoadOptions, Loader, MarkdownLoader, Source};
use textweaver_text::Document;
use textweaver_writers::{
    BrailleOptions, BrailleTableFormat, Format, WriteOptions, WriteReport, write_to_vec,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/b")
        .join(name)
}

fn load(name: &str) -> Document {
    MarkdownLoader
        .load(&Source::Path(fixture(name)), &LoadOptions::default())
        .expect("the fixture loads")
}

fn md(src: &str) -> Document {
    MarkdownLoader
        .load(
            &Source::Bytes {
                data: src.as_bytes().to_vec(),
                hint: "md".into(),
            },
            &LoadOptions::default(),
        )
        .expect("markdown loads")
}

fn brf(doc: &Document, tables: BrailleTableFormat) -> (String, WriteReport) {
    let options = WriteOptions {
        braille: BrailleOptions {
            table_format: tables,
            ..BrailleOptions::default()
        },
        ..WriteOptions::default()
    };
    let (bytes, report) = write_to_vec(doc, Format::Brf, &options).expect("writes");
    (String::from_utf8(bytes).expect("braille ASCII"), report)
}

/// Every line fits the 40-cell display and is braille ASCII.
fn assert_lines_fit(name: &str, text: &str) {
    for line in text.split("\r\n") {
        let line = line.trim_start_matches('\u{c}');
        assert!(line.len() <= 40, "{name}: {line:?} is over 40 cells");
        assert!(
            line.chars().all(|c| (' '..='_').contains(&c)),
            "{name}: {line:?}"
        );
    }
}

/// Compares with the snapshot, or writes it with `TW_BLESS=1`.
fn snapshot(name: &str, text: &str) {
    assert_lines_fit(name, text);
    let path = fixture(name);
    if std::env::var_os("TW_BLESS").is_some() {
        std::fs::write(&path, text).expect("write the snapshot");
        return;
    }
    let want = std::fs::read_to_string(&path).expect("the snapshot exists");
    assert_eq!(text, want, "{name} differs from its snapshot");
}

/// The text with its lines joined by spaces, to find a phrase that wraps.
fn joined(text: &str) -> String {
    text.split("\r\n")
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first page's lines, without the page number line.
fn page_one(text: &str) -> Vec<&str> {
    let page = text.split('\u{c}').next().unwrap_or_default();
    let mut lines: Vec<&str> = page.split("\r\n").collect();
    lines.truncate(24);
    lines
}

#[test]
fn table_fixture_in_each_format_matches_its_snapshot() {
    let doc = load("table.md");
    for (format, name) in [
        (BrailleTableFormat::Linear, "table.linear.brf"),
        (BrailleTableFormat::Listed, "table.listed.brf"),
        (BrailleTableFormat::Stairstep, "table.stairstep.brf"),
    ] {
        let (text, report) = brf(&doc, format);
        assert!(report.warnings.is_empty(), "{name}: {report:?}");
        snapshot(name, &text);
    }
}

/// 11.16: a blank line before each row; the first column heading and the
/// row heading as a cell-5 heading; each other entry after its column
/// heading and a colon in 1-3 margins; the note once.
#[test]
fn listed_rows_are_cell_five_headings_with_their_entries() {
    let doc = load("table.md");
    let (text, _) = brf(&doc, BrailleTableFormat::Listed);
    let lines = page_one(&text);
    let monday = lines
        .iter()
        .position(|l| *l == "    ,DAY3 ,MONDAY")
        .expect("Monday's row heading in cell 5");
    assert_eq!(lines[monday - 1], "", "a blank line before the row");
    assert_eq!(lines[monday + 1], ",SESSION3 ,CELL BIOLOGY");
    assert_eq!(lines[monday + 2], ",ROOM3 #BJD");
    // Tuesday's room is blank: three guide dots.
    assert!(lines.contains(&",ROOM3 \"\"\""), "{lines:#?}");
    // The transcriber's note, in 7-5 margins, once.
    assert_eq!(text.matches("@.<").count(), 1, "{text}");
    let note = lines
        .iter()
        .position(|l| l.starts_with("      @.<"))
        .expect("note");
    assert!(lines[note + 1].starts_with("    "), "runover in cell 5");
    assert!(!lines[note + 1].starts_with("     "), "runover in cell 5");
    // A second table in the same document has no second note.
    let doc = md("| A | B |\n| - | - |\n| 1 | 2 |\n\nText.\n\n| C | D |\n| - | - |\n| 3 | 4 |\n");
    let (text, _) = brf(&doc, BrailleTableFormat::Listed);
    assert_eq!(text.matches("@.<").count(), 1, "{text}");
    assert!(text.contains("    ,C3 #C\r\n,D3 #D\r\n"), "{text}");
}

/// 11.18: each row's entries in 1-1, 3-3, 5-5 margins; the headings at the
/// same steps inside the note; a runover at its entry's own margin.
#[test]
fn stairstep_entries_step_two_cells_right() {
    let doc = load("table.md");
    let (text, _) = brf(&doc, BrailleTableFormat::Stairstep);
    let lines = page_one(&text);
    let day = lines.iter().position(|l| *l == ",DAY").expect("heading 1");
    assert_eq!(lines[day + 1], "  ,SESSION");
    assert_eq!(lines[day + 2], "    ,ROOM@.>");
    let monday = lines.iter().position(|l| *l == ",MONDAY").expect("row");
    assert_eq!(lines[monday + 1], "  ,CELL BIOLOGY");
    assert_eq!(lines[monday + 2], "    #BJD");
    assert!(lines.contains(&"    \"\"\""), "{lines:#?}");
    // The long entry's runover stays in cell 3.
    let long = lines
        .iter()
        .position(|l| l.starts_with("  ,STATISTICS"))
        .expect("long entry");
    assert!(lines[long + 1].starts_with("  ") && !lines[long + 1].starts_with("   "));
}

/// 11.18: a table of five columns is listed, and the report says why.
#[test]
fn a_wide_table_is_listed_instead_of_stairstep() {
    let doc = md("| A | B | C | D | E |\n| - | - | - | - | - |\n| 1 | 2 | 3 | 4 | 5 |\n");
    let (text, report) = brf(&doc, BrailleTableFormat::Stairstep);
    assert!(text.contains("    ,A3 #A\r\n,B3 #B\r\n"), "{text}");
    assert!(
        report.warnings[0].contains("more than four columns"),
        "{report:?}"
    );
}

#[test]
fn a_table_without_a_header_row_lists_its_entries() {
    let doc = md("<table><tr><td>One</td><td>Two</td></tr></table>\n");
    let (text, _) = brf(&doc, BrailleTableFormat::Stairstep);
    assert_lines_fit("no header", &text);
    assert!(!text.contains("Column headings"), "{text}");
}

#[test]
fn typeforms_fixture_matches_its_snapshot() {
    let (text, _) = brf(&load("typeforms.md"), BrailleTableFormat::Linear);
    snapshot("typeforms.brf", &text);
    let text = joined(&text);
    // UEB Rules 9.4: four italic words are a passage.
    assert!(text.contains(".7,TO ,KILL A ,MOCKINGBIRD.'"), "{text}");
    // 9.3: one word takes the word indicator; 9.7.3: no terminator before
    // the question mark.
    assert!(text.contains("^1KEY"), "{text}");
    assert!(text.contains(".1,HAMLET8"), "{text}");
    // 9.4.4: the terminator inside a word is not needed at its end.
    assert!(text.contains("TEXT^1BOOK4"), "{text}");
}

#[test]
fn a_heading_wholly_in_one_typeform_leaves_it_out() {
    let (text, _) = brf(
        &md("# *Chapter One*\n\nText.\n"),
        BrailleTableFormat::Linear,
    );
    assert!(text.contains(",CHAPTER ,ONE"), "{text}");
    assert!(!text.contains(".7") && !text.contains(".1"), "{text}");
    // A typeform over part of a heading is kept.
    let (text, _) = brf(
        &md("# About *Hamlet*\n\nText.\n"),
        BrailleTableFormat::Linear,
    );
    assert!(text.contains(",ABOUT .1,HAMLET"), "{text}");
}

#[test]
fn capitals_fixture_matches_its_snapshot() {
    let (text, _) = brf(&load("capitals.md"), BrailleTableFormat::Linear);
    snapshot("capitals.brf", &text);
    let lines = page_one(&text);
    // The heading in capitals: the passage indicator once, and one
    // terminator after its last word (UEB Rules 8.5.7).
    assert_eq!(lines[0].trim(), ",,,CHAPTER ONE3 THE BEGINNING,'");
    assert_eq!(text.matches(",,,").count(), 2, "{text}");
    let text = joined(&text);
    assert!(text.contains(",,,KEEP OUT OF THE LAB,' AFTER"), "{text}");
    assert!(text.contains(",,NEW ,,YORK1"), "{text}");
}
