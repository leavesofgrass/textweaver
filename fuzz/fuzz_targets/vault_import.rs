//! Fuzz target: the Obsidian vault importer (`textweaver-vault`'s
//! `import.rs`). Any note text parses without panicking, and its front
//! matter, written back, reads again as the same front matter. The bytes are also read as a small vault on
//! disk, two notes split at the first NUL, in graph mode, so links between
//! the notes are resolved.
//!
//! The vault is a folder of its own under the system's temporary folder,
//! named with the process id. The two notes are rewritten for each input
//! and nothing else is written; nothing is removed.

#![no_main]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use textweaver_vault::{
    ImportMode, ImportOptions, extract_links, first_line, inline_tags, parse_note, read_vault,
    split_front_matter, strip_link_syntax,
};

fn vault() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("textweaver-fuzz-vault-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok().map(|()| dir)
    })
    .as_deref()
}

fn check_note(path: &Path, text: &str) {
    let note = parse_note(path, text);
    let _ = serde_json::to_string(&note);
    let (fm, body) = split_front_matter(text);
    if !fm.is_empty() {
        let rendered = fm.render();
        let (again, rest) = split_front_matter(&rendered);
        assert_eq!(again, fm, "front matter reads back the same:\n{rendered}");
        assert_eq!(rest, "", "rendered front matter has no body");
    }
    let _ = extract_links(body);
    let _ = inline_tags(body);
    let _ = first_line(body);
    let _ = strip_link_syntax(body);
}

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let (a, b) = text.split_once('\0').unwrap_or((&text, ""));
    check_note(Path::new("vault/Note A.md"), a);
    check_note(Path::new("vault/sub/Note B.md"), b);

    let Some(dir) = vault() else {
        return;
    };
    if std::fs::write(dir.join("Note A.md"), a).is_err()
        || std::fs::write(dir.join("Note B.md"), b).is_err()
    {
        return;
    }
    for mode in [ImportMode::Graph, ImportMode::Library] {
        let options = ImportOptions {
            mode,
            ..ImportOptions::default()
        };
        if let Ok(read) = read_vault(dir, &options) {
            assert_eq!(read.notes.len(), 2, "both notes are read");
        }
    }
});
