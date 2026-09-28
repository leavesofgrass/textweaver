//! `tw search --json` and `tw info --json` on a closed pipe (usability
//! pass, item 10): `tw search x --json | head` panicked with "failed
//! printing to stdout" once `head` closed the pipe. Output goes through
//! `print_all`, which ends quietly on a closed pipe.

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
