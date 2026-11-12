//! No raw control characters on a terminal (security review, QW6). A
//! document can hold an escape sequence that writes the clipboard (OSC
//! 52) or sets the window title; `tw text`, `tw search` and `tw info`
//! drop control characters other than tab and line feed when standard
//! output is a terminal, and leave piped output as it is.
//!
//! A pseudo terminal is not needed: `TEXTWEAVER_ASSUME_TERMINAL=1` makes
//! `tw` treat its output as a terminal.

use std::process::{Command, Output, Stdio};

/// OSC 52, "write this to the clipboard", ended by BEL.
const OSC52: &str = "\u{1b}]52;c;aGVsbG8=\u{7}";

/// A plain text file whose title line and body hold [`OSC52`].
fn document(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("notes.txt");
    let text = format!("Title {OSC52}here\n\nA needle {OSC52}in the text.\tTabbed.\n");
    std::fs::write(&path, text).unwrap();
    path
}

/// Runs `tw` with `args`, its output a terminal when `terminal` is set.
fn tw(home: &std::path::Path, args: &[&str], terminal: bool) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tw"));
    cmd.args(args)
        .env("TEXTWEAVER_HOME", home)
        .env("TEXTWEAVER_ESPEAK_OUTPUT", "virtual")
        .env_remove("TEXTWEAVER_ASSUME_TERMINAL")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if terminal {
        cmd.env("TEXTWEAVER_ASSUME_TERMINAL", "1");
    }
    cmd.output().unwrap()
}

#[test]
fn a_terminal_gets_no_escape_characters() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let doc = document(dir.path());
    let doc = doc.to_str().unwrap();
    for args in [
        vec!["text", doc],
        vec!["search", doc, "needle"],
        vec!["info", doc],
    ] {
        let out = tw(&home, &args, true);
        assert!(out.status.success(), "{args:?}: {out:?}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(!text.contains('\u{1b}'), "{args:?} printed ESC: {text:?}");
        assert!(!text.contains('\u{7}'), "{args:?} printed BEL: {text:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(!stderr.contains('\u{1b}'), "{args:?}: {stderr:?}");
    }
    let out = tw(&home, &["text", doc], true);
    let text = String::from_utf8(out.stdout).unwrap();
    // The rest of the sequence prints as harmless text; tabs and line
    // feeds stay.
    assert!(
        text.contains("A needle ]52;c;aGVsbG8=in the text.\tTabbed.\n"),
        "{text:?}"
    );
}

#[test]
fn piped_output_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let doc = document(dir.path());
    let out = tw(&home, &["text", doc.to_str().unwrap()], false);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains(OSC52), "{text:?}");
}

#[test]
fn an_error_line_on_a_terminal_has_no_escape_characters() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    // A name that is never created: only quoted back in the error.
    let missing = dir.path().join(format!("gone{OSC52}.txt"));
    let pattern = format!("x{OSC52}");
    let out = tw(
        &home,
        &["search", missing.to_str().unwrap(), &pattern],
        true,
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("Error: "), "{stderr:?}");
    assert!(stderr.contains("]52;c;"), "the name is quoted: {stderr:?}");
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
    // No match: the pattern is quoted back without its escape.
    let doc = document(dir.path());
    let out = tw(&home, &["search", doc.to_str().unwrap(), &pattern], true);
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("No matches for"), "{stderr:?}");
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
}
