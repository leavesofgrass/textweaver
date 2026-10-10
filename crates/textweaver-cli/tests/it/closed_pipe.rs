//! `tw` with no arguments and no terminal, and `tw search --json` and `tw info --json` on
//! a closed pipe (usability pass, items 6 and 10). `tw search x --json |
//! head` panicked with "failed printing to stdout" once `head` closed the
//! pipe; output goes through `print_all` now, which ends quietly.

use std::io::Read as _;
use std::process::{Command, Stdio};

/// A document with enough matches that the JSON is far larger than any
/// pipe buffer, so `tw` is still writing when the pipe closes.
fn big_document(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("many.txt");
    let line = "Every line has the word needle in it, and some more words after.\n";
    std::fs::write(&path, line.repeat(20_000)).unwrap();
    path
}

/// Runs `tw` with `args`, closes its standard output at once, and returns
/// its exit status and standard error.
fn run_with_closed_stdout(args: &[&str]) -> (std::process::ExitStatus, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tw"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Closing the read end is what `head` does once it has its lines.
    drop(child.stdout.take());
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    (child.wait().unwrap(), err)
}

#[test]
fn search_json_ends_quietly_on_a_closed_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let doc = big_document(dir.path());
    let doc = doc.to_str().unwrap();
    let (status, err) = run_with_closed_stdout(&["search", doc, "needle", "--json"]);
    assert!(!err.contains("panicked"), "{err}");
    assert!(!err.contains("failed printing"), "{err}");
    assert!(status.success(), "{status:?}: {err}");
    // The text output did already; it still does.
    let (status, err) = run_with_closed_stdout(&["search", doc, "needle"]);
    assert!(!err.contains("panicked"), "{err}");
    assert!(status.success(), "{status:?}: {err}");
}

#[test]
fn info_ends_quietly_on_a_closed_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let doc = big_document(dir.path());
    let doc = doc.to_str().unwrap();
    for args in [vec!["info", doc, "--json"], vec!["info", doc]] {
        let (status, err) = run_with_closed_stdout(&args);
        assert!(!err.contains("panicked"), "{args:?}: {err}");
        assert!(status.success(), "{args:?}: {status:?}: {err}");
    }
}

/// `tw` with no arguments opens the terminal reader (B1-o1). With standard
/// output not a terminal, as here, it refuses in one error line with exit
/// status 1 instead of drawing into the pipe, under either name; `tw
/// --help` keeps the full list.
#[test]
fn no_arguments_without_a_terminal_is_one_error_line() {
    for exe in [env!("CARGO_BIN_EXE_tw"), env!("CARGO_BIN_EXE_textweaver")] {
        let out = Command::new(exe).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{exe}: {out:?}");
        assert!(out.stdout.is_empty(), "{exe}: {out:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert_eq!(err.lines().count(), 1, "{err}");
        assert!(
            err.starts_with("Error: The terminal reader needs a terminal"),
            "{err}"
        );
    }
    let help = Command::new(env!("CARGO_BIN_EXE_tw"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    for command in ["open", "search", "info", "convert", "serve"] {
        assert!(help.contains(command), "{command}: {help}");
    }
}

/// Every command writes through `tw`'s one standard output writer, so a
/// closed pipe never panics, whichever command is printing (W9a-c). The
/// output here is small, but the read end is closed before `tw` writes,
/// so the first write already meets the closed pipe.
#[test]
fn every_command_ends_quietly_on_a_closed_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_str().unwrap();
    let doc = big_document(dir.path());
    let doc = doc.to_str().unwrap();
    let runs: Vec<Vec<&str>> = vec![
        vec!["eloquence"],
        vec!["backends"],
        vec!["text", doc],
        vec!["summarize", doc],
        vec!["settings", "path", "--home", home],
        vec!["settings", "export", "--home", home],
        vec!["stats", "--home", home],
        vec!["components", "list", "--home", home],
        vec!["library", "list", "--home", home],
        vec!["cite", "list", "--home", home],
        vec!["ocr", "status"],
    ];
    for args in runs {
        let (status, err) = run_with_closed_stdout(&args);
        assert!(!err.contains("panicked"), "{args:?}: {err}");
        assert!(!err.contains("failed printing"), "{args:?}: {err}");
        assert!(status.code().is_some(), "{args:?}: {status:?}: {err}");
    }
}
