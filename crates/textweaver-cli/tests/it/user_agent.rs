//! One User-Agent for every request (the owner's privacy rule; security
//! review, QW7). Scans the workspace's Rust sources and fails when a
//! client's User-Agent method, a `User-Agent` header or a `USER_AGENT` constant
//! uses anything but the neutral string, or a constant that holds it.
//!
//! Found by text search, as `textweaver-store`'s settings test finds
//! readers: a heuristic that catches the usual ways a client is given a
//! User-Agent.

use std::path::{Path, PathBuf};

/// The one User-Agent textweaver sends.
const NEUTRAL: &str = "textweaver-research (+https://github.com/leavesofgrass/textweaver)";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Every `.rs` file under `root`, skipping build output and hidden
/// folders (worktrees live under `.claude`).
fn sources(root: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if p.is_dir() {
            if name == "target" || name.starts_with('.') {
                continue;
            }
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs")
            && let Ok(text) = std::fs::read_to_string(&p)
        {
            out.push((p, text));
        }
    }
}

/// The text of `s` from the start up to its closing bracket, the brackets
/// opened inside it included, with whitespace squeezed out.
fn argument(s: &str) -> String {
    let mut depth = 0usize;
    let mut end = s.len();
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => {
                end = i;
                break;
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    s[..end].split_whitespace().collect::<Vec<_>>().join(" ")
}

/// True for a value that is the neutral string, or a constant named
/// `USER_AGENT` (whose definition is checked on its own).
fn is_neutral(value: &str) -> bool {
    let value = value.trim().trim_end_matches(',').trim();
    value == format!("{NEUTRAL:?}")
        || value
            .rsplit("::")
            .next()
            .is_some_and(|last| last == "USER_AGENT")
}

/// What is wrong in one file's text, as "line: what".
fn problems(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let line_of = |at: usize| text[..at].matches('\n').count() + 1;
    // `.user_agent(VALUE)` method calls.
    let call = concat!(".user_agent", "(");
    for (at, _) in text.match_indices(call) {
        let value = argument(&text[at + call.len()..]);
        if !is_neutral(&value) {
            found.push(format!("{}: {call}{value})", line_of(at)));
        }
    }
    // `.header("User-Agent", VALUE)` and the like, in any case.
    let lower = text.to_ascii_lowercase();
    let header = concat!("\"user", "-agent\"");
    for (at, _) in lower.match_indices(header) {
        let rest = text[at + header.len()..].trim_start();
        // A header being set has its value next; a lookup does not.
        let Some(value) = rest.strip_prefix(',') else {
            continue;
        };
        let value = argument(value);
        if !is_neutral(&value) {
            found.push(format!("{}: User-Agent header {value}", line_of(at)));
        }
    }
    // `const USER_AGENT: &str = VALUE;`
    let constant = concat!("USER_", "AGENT: &str =");
    for (at, _) in text.match_indices(constant) {
        let rest = &text[at + constant.len()..];
        let value = rest.split(';').next().unwrap_or("");
        if !is_neutral(value) {
            found.push(format!("{}: USER_AGENT = {}", line_of(at), value.trim()));
        }
    }
    found
}

#[test]
fn every_request_uses_the_one_neutral_user_agent() {
    let root = workspace();
    let mut files = Vec::new();
    for folder in ["crates", "xtask", "fuzz"] {
        sources(&root.join(folder), &mut files);
    }
    assert!(files.len() > 100, "found only {} sources", files.len());
    let mut bad = Vec::new();
    let mut calls = 0;
    for (path, text) in &files {
        // This file describes the patterns it looks for.
        if path.ends_with(Path::new("tests").join("it").join("user_agent.rs")) {
            continue;
        }
        calls += text.matches(concat!(".user_agent", "(")).count();
        for p in problems(text) {
            let shown = path.strip_prefix(&root).unwrap_or(path).display();
            bad.push(format!("{shown}:{p}"));
        }
    }
    assert!(calls >= 3, "the scan found only {calls} user_agent calls");
    assert!(
        bad.is_empty(),
        "Use the neutral User-Agent, {NEUTRAL:?}, or a USER_AGENT constant that holds it:\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_scan_catches_another_user_agent() {
    let call = concat!(".user_agent", "(");
    let versioned = format!("let a = b{call}concat!(\"textweaver/\", env!(\"V\")));");
    assert_eq!(problems(&versioned).len(), 1, "{versioned}");
    let header = format!(
        "req.header(\"{}\", \"Ada Example's laptop\")",
        concat!("User", "-Agent")
    );
    assert_eq!(problems(&header).len(), 1, "{header}");
    let constant = format!(
        "pub const {}: &str = \"curl/8\";",
        concat!("USER_", "AGENT")
    );
    assert_eq!(problems(&constant).len(), 1, "{constant}");
    let good = format!(
        "b{call}crate::USER_AGENT); b{call}{NEUTRAL:?}); header(h, \"{}\").is_some()",
        concat!("user", "-agent")
    );
    assert!(problems(&good).is_empty(), "{:?}", problems(&good));
}
