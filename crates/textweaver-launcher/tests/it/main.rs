//! The launcher runs `textweaver` beside it with `--tw` and its own
//! arguments, and ends with that program's exit status. A copy of the
//! launcher and a stand-in for `textweaver` (the `fake_textweaver`
//! example, which `cargo test` builds) are put in a folder of their own.

use std::path::{Path, PathBuf};
use std::process::Command;

const EXE: &str = std::env::consts::EXE_SUFFIX;

fn launcher() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tw-launcher"))
}

/// The stand-in, built beside the launcher under `examples/`.
fn fake() -> PathBuf {
    let dir = launcher().parent().unwrap().join("examples");
    let p = dir.join(format!("fake_textweaver{EXE}"));
    assert!(
        p.is_file(),
        "{} is missing: run the tests with cargo test, which builds the examples",
        p.display()
    );
    p
}

/// A folder holding `tw` (the launcher) and, with `with_target`,
/// `textweaver` (the stand-in).
fn install(with_target: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let tw = dir.path().join(format!("tw{EXE}"));
    std::fs::copy(launcher(), &tw).unwrap();
    if with_target {
        std::fs::copy(fake(), dir.path().join(format!("textweaver{EXE}"))).unwrap();
    }
    (dir, tw)
}

fn run(tw: &Path, args: &[&str]) -> std::process::Output {
    Command::new(tw).args(args).output().unwrap()
}

#[test]
fn the_arguments_reach_textweaver_after_the_terminal_flag() {
    let (_dir, tw) = install(true);
    let out = run(&tw, &["convert", "a b.md", "--to", "html"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim_end(),
        "--tw|convert|a b.md|--to|html"
    );
    let out = run(&tw, &[]);
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim_end(), "--tw");
}

#[test]
fn the_exit_status_is_textweavers() {
    let (_dir, tw) = install(true);
    for code in [1, 2, 7] {
        let out = run(&tw, &["text", &format!("exit={code}")]);
        assert_eq!(out.status.code(), Some(code), "{out:?}");
    }
}

#[test]
fn a_missing_textweaver_fails_in_words() {
    let (_dir, tw) = install(false);
    let out = run(&tw, &["--version"]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("Error: textweaver was not found beside tw"),
        "{err}"
    );
}
