//! Fuzz target: Obsidian's Markdown in the reader (ADR-0044): callouts,
//! embeds (read as links, since the input has no folder), tags,
//! highlights, comments, and block ids, through the Markdown loader; and
//! the parts of a note an embed can name, found by heading and by block
//! id. The first line of the input names the part; the rest is the note.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_formats::{callout, obsidian};

fuzz_target!(|data: &[u8]| {
    textweaver_fuzz::load_checked(data, "md");
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let (part, note) = text.split_once('\n').unwrap_or((text, ""));
    if let Some(section) = obsidian::heading_section(note, part) {
        assert!(note.contains(section));
    }
    let _ = obsidian::block_section(note, part.trim_start_matches('^'));
    for line in note.lines().take(64) {
        if let Some(head) = callout::head(line, true) {
            let _ = (head.canonical(), head.title());
        }
    }
});
