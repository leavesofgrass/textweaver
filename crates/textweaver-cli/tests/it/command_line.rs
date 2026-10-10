//! The command-line rules (the "Command line" guide, `docs/command-line.md`),
//! checked over `tw --help` and a few runs: one test per rule.
//!
//! - Output is `--out`, with `-o` and `--output` hidden aliases; no option
//!   list shows `--output`, `--format`, `--export` or `--language`.
//! - Every command that prints facts has `--json` (on itself or one of
//!   its subcommands).
//! - Every command that reads or writes the data folder has `--home`.
//! - Every `--yes` has `-y`, and a question is never asked without a
//!   terminal.
//! - Exit codes: 0 done, 1 failed or nothing found, 2 a usage error.
//! - Errors are one line: "Error: what failed: why. What to do."

use std::io::Write as _;
use std::process::{Command, Output, Stdio};

/// Runs `tw` with `args`, standard input empty and not a terminal, the
/// data folder in `home`, and speech never played.
fn tw_in(home: &std::path::Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tw"))
        .args(args)
        .env("TEXTWEAVER_HOME", home)
        .env("TEXTWEAVER_ESPEAK_OUTPUT", "virtual")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);
    child.wait_with_output().unwrap()
}

fn help(home: &std::path::Path, args: &[&str]) -> String {
    let mut argv: Vec<&str> = args.to_vec();
    argv.push("--help");
    let out = tw_in(home, &argv, "");
    assert!(out.status.success(), "{argv:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// The names listed under "Commands:" in a help text, without `help`.
fn commands(help: &str) -> Vec<String> {
    help.lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_owned)
        .collect()
}

/// Every command's help, with its subcommands' (two levels) appended.
fn all_help(home: &std::path::Path) -> Vec<(String, String)> {
    let top = help(home, &[]);
    let mut all = Vec::new();
    for name in commands(&top) {
        let mut text = help(home, &[&name]);
        for sub in commands(&text.clone()) {
            let sub_help = help(home, &[&name, &sub]);
            for subsub in commands(&sub_help) {
                text.push_str(&help(home, &[&name, &sub, &subsub]));
            }
            text.push_str(&sub_help);
        }
        all.push((name, text));
    }
    all
}

/// The option lines of a help text: those starting with a dash.
fn option_lines(help: &str) -> impl Iterator<Item = &str> {
    help.lines()
        .map(str::trim_start)
        .filter(|l| l.starts_with('-'))
}

/// Commands whose output is facts a script may read.
const PRINTS_FACTS: &[&str] = &[
    "text",
    "info",
    "search",
    "speak",
    "voices",
    "backends",
    "eloquence",
    "convert",
    "export-audio",
    "library",
    "vault",
    "dictate",
    "marks",
    "notes",
    "lint",
    "migrate-star",
    "cite",
    "settings",
    "define",
    "stats",
    "study",
    "summarize",
    "changes",
    "sync",
    "ocr",
    "components",
];

/// Commands that read or write the data folder. `tw ocr` takes `--home`
/// on `status` and `download`, where its models are.
const TOUCHES_DATA: &[&str] = &[
    "open",
    "ocr",
    "speak",
    "voices",
    "backends",
    "convert",
    "export-audio",
    "library",
    "vault",
    "dictate",
    "marks",
    "notes",
    "migrate-star",
    "cite",
    "settings",
    "define",
    "stats",
    "study",
    "summarize",
    "changes",
    "sync",
    "serve",
    "components",
];

#[test]
fn the_option_rules_hold_for_every_command() {
    let home = tempfile::tempdir().unwrap();
    let all = all_help(home.path());
    assert!(all.len() >= 25, "only {} commands", all.len());
    let mut problems = Vec::new();
    for (name, text) in &all {
        for line in option_lines(text) {
            for old in [
                "--output",
                "--format",
                "--export ",
                "--language",
                // Verbs are subcommands: `tw library add`, `tw stats
                // clear`, `tw dictate list`.
                "--add ",
                "--remove ",
                "--search ",
                "--continue",
                "--clear",
                "--list",
            ] {
                if line.contains(old) {
                    problems.push(format!("{name}: shows {old}: {line}"));
                }
            }
            if line.contains("--yes") && !line.contains("-y, --yes") {
                problems.push(format!("{name}: --yes without -y: {line}"));
            }
            if line.contains("--out ") && !line.contains("-o, --out") {
                problems.push(format!("{name}: --out without -o: {line}"));
            }
        }
        let options: Vec<&str> = option_lines(text).collect();
        let has = |flag: &str| options.iter().any(|l| l.contains(flag));
        if PRINTS_FACTS.contains(&name.as_str()) && !has("--json") {
            problems.push(format!("{name}: no --json"));
        }
        if TOUCHES_DATA.contains(&name.as_str()) && !has("--home") {
            problems.push(format!("{name}: no --home"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn old_spellings_still_work_as_hidden_aliases() {
    let home = tempfile::tempdir().unwrap();
    let doc = home.path().join("a.md");
    std::fs::write(&doc, "# Title\n\nSome text.\n").unwrap();
    let doc = doc.to_str().unwrap();
    let new = tw_in(home.path(), &["text", doc, "--to", "json"], "");
    let old = tw_in(home.path(), &["text", doc, "--format", "json"], "");
    assert!(new.status.success(), "{new:?}");
    assert_eq!(new.stdout, old.stdout);
    let flag = tw_in(home.path(), &["text", doc, "--json"], "");
    assert_eq!(new.stdout, flag.stdout);
}

#[test]
fn verbs_are_subcommands_and_the_old_flags_still_work() {
    let home = tempfile::tempdir().unwrap();
    let folder = home.path().join("books");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(
        folder.join("a.md"),
        "# Title

Some text.
",
    )
    .unwrap();
    let folder = folder.to_str().unwrap();
    let new = tw_in(home.path(), &["library", "add", folder, "--json"], "");
    assert!(new.status.success(), "{new:?}");
    let old = tw_in(home.path(), &["library", "--remove", folder, "--json"], "");
    assert!(old.status.success(), "{old:?}");
    let again = tw_in(home.path(), &["library", "--add", folder, "--json"], "");
    assert_eq!(new.stdout, again.stdout);
    let listed = tw_in(home.path(), &["library", "list"], "");
    let plain = tw_in(home.path(), &["library"], "");
    assert_eq!(listed.stdout, plain.stdout);
    let found = tw_in(home.path(), &["library", "search", "Title"], "");
    assert!(found.status.success(), "{found:?}");
    let new = tw_in(home.path(), &["stats", "clear", "--yes"], "");
    let old = tw_in(home.path(), &["stats", "--clear", "--yes"], "");
    assert!(new.status.success(), "{new:?}");
    assert_eq!(new.stdout, old.stdout);
    let new = tw_in(home.path(), &["dictate", "list", "--json"], "");
    let old = tw_in(home.path(), &["dictate", "--list", "--json"], "");
    assert!(new.status.success(), "{new:?}");
    assert_eq!(new.stdout, old.stdout);
}

#[test]
fn errors_are_one_line_with_exit_status_1() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("missing.md");
    let out = tw_in(
        home.path(),
        &["speak", "--file", missing.to_str().unwrap()],
        "",
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8(out.stderr).unwrap();
    let last = err.lines().last().unwrap_or_default();
    assert!(last.starts_with("Error: "), "{err}");
    assert!(!err.contains("Caused by"), "{err}");
    let first = last.trim_start_matches("Error: ").chars().next().unwrap();
    assert!(first.is_uppercase(), "{err}");
}

#[test]
fn version_prints_the_version_then_the_copyright_line() {
    let home = tempfile::tempdir().unwrap();
    let out = tw_in(home.path(), &["--version"], "");
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some(concat!("tw ", env!("CARGO_PKG_VERSION")))
    );
    assert_eq!(lines.next(), Some(textweaver_app::COPYRIGHT));
}

#[test]
fn a_usage_error_exits_with_status_2() {
    let home = tempfile::tempdir().unwrap();
    let out = tw_in(home.path(), &["text", "--no-such-option"], "");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

#[test]
fn cite_check_exits_1_when_keys_are_missing() {
    let home = tempfile::tempdir().unwrap();
    let doc = home.path().join("essay.md");
    std::fs::write(&doc, "As shown [@nobody2020].\n").unwrap();
    let out = tw_in(home.path(), &["cite", "check", doc.to_str().unwrap()], "");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let said = String::from_utf8(out.stdout).unwrap();
    assert!(said.contains("nobody2020"), "{said}");
    std::fs::write(&doc, "No citations here.\n").unwrap();
    let out = tw_in(home.path(), &["cite", "check", doc.to_str().unwrap()], "");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

#[test]
fn cite_prints_json_on_every_subcommand_that_prints_facts() {
    let home = tempfile::tempdir().unwrap();
    let doc = home.path().join("essay.md");
    std::fs::write(&doc, "As shown [@nobody2020].\n").unwrap();
    let doc = doc.to_str().unwrap();
    let out = tw_in(home.path(), &["cite", "check", doc, "--json"], "");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["missing"][0], "nobody2020");
    let out = tw_in(home.path(), &["cite", "styles", "--json"], "");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["styles"].as_array().is_some_and(|a| a.len() > 3), "{v}");
    let bib = home.path().join("refs.bib");
    std::fs::write(
        &bib,
        "@book{example2020, title={A Placeholder Book}, author={Example, Ada}, year={2020}}\n",
    )
    .unwrap();
    let out = tw_in(
        home.path(),
        &["cite", "import", bib.to_str().unwrap(), "--json"],
        "",
    );
    assert!(out.status.success(), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["action"], "imported");
    let out = tw_in(
        home.path(),
        &["cite", "remove", "example2020", "--yes", "--json"],
        "",
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["action"], "removed", "{v}");
}

#[test]
fn ocr_status_uses_the_data_folder_from_home() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path().to_str().unwrap();
    let out = tw_in(home.path(), &["ocr", "status", "--json", "--home", h], "");
    assert!(out.status.success(), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let folder = v["models_folder"].as_str().unwrap_or_default().to_owned();
    assert!(
        folder.starts_with(h) || std::env::var_os("TEXTWEAVER_OCR_MODELS").is_some(),
        "{folder}"
    );
    assert_eq!(v["models"][0]["status"], "not-downloaded");
}

#[test]
fn no_question_without_a_terminal() {
    let home = tempfile::tempdir().unwrap();
    let out = tw_in(home.path(), &["cite", "remove", "doe2020"], "y\n");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("Add --yes"), "{err}");
}

#[test]
fn speak_reads_standard_input_from_a_dash() {
    let home = tempfile::tempdir().unwrap();
    let out = tw_in(
        home.path(),
        &["speak", "-", "--backend", "null", "--json"],
        "Hello from a pipe.",
    );
    assert!(out.status.success(), "{out:?}");
    let said = String::from_utf8(out.stdout).unwrap();
    assert!(said.contains("Hello from a pipe."), "{said}");
}

#[test]
fn components_list_prints_json() {
    let home = tempfile::tempdir().unwrap();
    let out = tw_in(home.path(), &["components", "list", "--json"], "");
    assert!(out.status.success(), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["components"].as_array().is_some_and(|a| !a.is_empty()));
    assert_eq!(v["installed"], 0);
    // The one components status line ends the list, as in tw info and
    // About: "Components: 0 of 12 installed."
    let total = v["total"].as_u64().unwrap();
    let line = format!("Components: 0 of {total} installed.");
    let out = tw_in(home.path(), &["components", "list"], "");
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.lines().last(), Some(line.as_str()), "{text}");
}
