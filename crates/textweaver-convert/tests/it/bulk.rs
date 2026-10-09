//! Bulk conversion on temporary trees: mirrored paths, skip-unchanged,
//! force, in-place rules, collisions, the output folder inside the input,
//! Pandoc fallback, and the hot-folder watcher.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use textweaver_convert::{
    ConvertOptions, Converter, OutputFormat, Status, WatchEvent, WatchOptions, watch,
};
use textweaver_formats::pandoc::pandoc_available;

fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    fs::write(&p, text).expect("write");
    p
}

fn tree(root: &Path) {
    write(root, "a.md", "# Alpha\n\nSome *text* and $x^2$.\n");
    write(root, "sub/b.md", "# Beta\n\n- one\n- two\n");
    write(
        root,
        "sub/deep/c.html",
        "<html><head><title>Gamma page</title></head><body><h1>Gamma</h1><p>Hello.</p></body></html>",
    );
    write(root, "notes.txt", "Plain line one.\nLine two.\n");
    write(root, ".hidden/x.md", "# Hidden\n");
    write(root, "image.png", "not really a png");
}

fn options(to: OutputFormat, out: Option<&Path>) -> ConvertOptions {
    ConvertOptions {
        to,
        out_dir: out.map(Path::to_owned),
        pandoc: false,
        jobs: Some(4),
        ..ConvertOptions::default()
    }
}

fn statuses(summary: &textweaver_convert::Summary) -> Vec<(String, Status)> {
    let mut v: Vec<(String, Status)> = summary
        .files
        .iter()
        .map(|f| {
            (
                f.source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                f.status.clone(),
            )
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn set_mtime(p: &Path, t: SystemTime) {
    fs::File::options()
        .write(true)
        .open(p)
        .and_then(|f| f.set_modified(t))
        .expect("set mtime");
}

#[test]
fn mirrors_the_tree_and_skips_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    tree(&input);
    let conv = Converter::new(options(OutputFormat::Html, Some(&out))).expect("converter");

    let first = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!(first.converted, 4, "{:?}", statuses(&first));
    assert_eq!(first.failed, 0);
    for rel in ["a.html", "sub/b.html", "sub/deep/c.html", "notes.html"] {
        assert!(out.join(rel).is_file(), "missing {rel}");
    }
    assert!(!out.join(".hidden").exists());
    assert!(!out.join("image.html").exists());
    let a = fs::read_to_string(out.join("a.html")).expect("read");
    assert!(a.contains("<title>Alpha</title>"));
    assert!(a.contains("<math"));
    let c = fs::read_to_string(out.join("sub/deep/c.html")).expect("read");
    assert!(c.contains("<title>Gamma page</title>"), "{c}");

    // Nothing changed: everything is skipped.
    let second = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!((second.converted, second.skipped), (0, 4));

    // A source newer than its output converts again.
    let later = SystemTime::now() + Duration::from_secs(5);
    set_mtime(&input.join("sub/b.md"), later);
    let third = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!((third.converted, third.skipped), (1, 3));
    assert_eq!(
        third
            .files
            .iter()
            .find(|f| f.status == Status::Converted)
            .map(|f| f.source.ends_with("b.md")),
        Some(true)
    );

    // Force converts everything.
    let forced = Converter::new(ConvertOptions {
        force: true,
        ..options(OutputFormat::Html, Some(&out))
    })
    .expect("converter")
    .run(std::slice::from_ref(&input))
    .expect("run");
    assert_eq!(forced.converted, 4);
    assert!(
        forced
            .sentence()
            .starts_with("Converted 4 files to HTML in ")
    );
}

#[test]
fn text_and_markdown_outputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    tree(&input);
    let out = dir.path().join("txt");
    let s = Converter::new(options(OutputFormat::Text, Some(&out)))
        .expect("converter")
        .run(std::slice::from_ref(&input))
        .expect("run");
    assert_eq!(s.converted, 4);
    let b = fs::read_to_string(out.join("sub/b.txt")).expect("read");
    assert_eq!(b, "Beta\n\none\ntwo\n");
    let c = fs::read_to_string(out.join("sub/deep/c.txt")).expect("read");
    assert!(c.contains("Gamma") && c.contains("Hello."), "{c}");

    let md = dir.path().join("md");
    Converter::new(options(OutputFormat::Markdown, Some(&md)))
        .expect("converter")
        .run(std::slice::from_ref(&input))
        .expect("run");
    // Markdown sources are copied as they are.
    assert_eq!(
        fs::read_to_string(md.join("a.md")).expect("read"),
        "# Alpha\n\nSome *text* and $x^2$.\n"
    );
    let c = fs::read_to_string(md.join("sub/deep/c.md")).expect("read");
    assert!(c.contains("# Gamma"), "{c}");
}

#[test]
fn in_place_conversion_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    tree(&input);
    let conv = Converter::new(options(OutputFormat::Html, None)).expect("converter");
    let s = conv.run(std::slice::from_ref(&input)).expect("run");
    // c.html is already HTML: in place it counts as an output, not a source.
    assert_eq!(s.converted, 3, "{:?}", statuses(&s));
    assert!(input.join("a.html").is_file());
    assert!(input.join("sub/b.html").is_file());
    // A second run does not treat the new .html files as sources.
    let again = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!((again.converted, again.skipped, again.failed), (0, 3, 0));

    // Markdown to Markdown in place would overwrite the source.
    let md = Converter::new(options(OutputFormat::Markdown, None)).expect("converter");
    let s = md.run(&[input.join("a.md")]).expect("run");
    assert_eq!(s.failed, 1);
    let reason = s.failures().next().map(|f| f.status.clone());
    assert!(matches!(reason, Some(Status::Failed(r)) if r.contains("--out")));
}

#[test]
fn colliding_outputs_are_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    write(&input, "same.md", "# One\n");
    write(&input, "same.txt", "Two\n");
    let out = dir.path().join("out");
    let s = Converter::new(options(OutputFormat::Html, Some(&out)))
        .expect("converter")
        .run(std::slice::from_ref(&input))
        .expect("run");
    assert_eq!((s.converted, s.failed), (1, 1));
    let f = s.failures().next().expect("failure");
    assert!(matches!(&f.status, Status::Failed(r) if r.contains("same output")));
}

#[test]
fn output_folder_inside_input_is_not_walked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    tree(&input);
    let out = input.join("site");
    let conv = Converter::new(options(OutputFormat::Html, Some(&out))).expect("converter");
    conv.run(std::slice::from_ref(&input)).expect("run");
    let second = Converter::new(ConvertOptions {
        force: true,
        ..options(OutputFormat::Html, Some(&out))
    })
    .expect("converter")
    .run(std::slice::from_ref(&input))
    .expect("run");
    assert_eq!(second.converted, 4, "{:?}", statuses(&second));
    assert!(!out.join("site").exists());
}

#[test]
fn missing_input_is_an_error() {
    let conv = Converter::new(options(OutputFormat::Html, None)).expect("converter");
    let err = conv.run(&[PathBuf::from("no/such/file.md")]).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[cfg(feature = "carta")]
#[test]
fn a_named_format_reads_files_whatever_their_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = write(
        dir.path(),
        "start.txt",
        "====== Crows ======\nThey are //clever//.\n",
    );
    let out = dir.path().join("out");
    let conv = |from: &str| {
        Converter::new(ConvertOptions {
            from: Some(from.to_owned()),
            ..options(OutputFormat::Org, Some(&out))
        })
    };
    let s = conv("dokuwiki")
        .expect("converter")
        .run(&[src])
        .expect("run");
    assert_eq!(s.converted, 1, "{:?}", statuses(&s));
    let org = fs::read_to_string(out.join("start.org")).expect("read");
    assert!(org.contains("* Crows"), "{org}");
    assert!(org.contains("/clever/"), "{org}");
    let err = conv("no-such-format").unwrap_err().to_string();
    assert!(err.starts_with("No reader for the format no-such-format"), "{err}");
}

#[cfg(not(feature = "carta"))]
#[test]
fn carta_outputs_are_refused_without_the_feature() {
    let err = Converter::new(options(OutputFormat::Typst, None)).unwrap_err();
    assert!(err.to_string().starts_with("Typst output is not available"), "{err}");
}

#[test]
fn pandoc_fallback_when_installed() {
    if !pandoc_available() {
        skip_or_fail("pandoc", "pandoc not on PATH, so the pandoc fallback test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let src = write(
        dir.path(),
        "doc.rst",
        "Title\n=====\n\nSome *emphasis* and caf\u{e9}.\n",
    );
    let out = dir.path().join("out");
    let s = Converter::new(ConvertOptions {
        pandoc: true,
        ..options(OutputFormat::Html, Some(&out))
    })
    .expect("converter")
    .run(&[src])
    .expect("run");
    assert_eq!(s.converted, 1, "{:?}", statuses(&s));
    let html = fs::read_to_string(out.join("doc.html")).expect("read");
    assert!(html.contains("<em>emphasis</em>"), "{html}");
    assert!(
        html.contains("caf\u{e9}"),
        "non-ASCII text must survive: {html}"
    );
}

#[test]
fn hot_folder_converts_and_moves_sources() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("inbox");
    let out = dir.path().join("out");
    fs::create_dir_all(&input).expect("mkdir");
    // Present before watching starts.
    write(&input, "early.md", "# Early\n");
    let conv = Converter::new(options(OutputFormat::Text, Some(&out))).expect("converter");
    let stop = AtomicBool::new(false);
    let opts = WatchOptions {
        stable: Duration::from_millis(200),
        poll: Duration::from_millis(50),
        rescan: Duration::from_millis(300),
        ..WatchOptions::default()
    };
    // The watcher's events, shared so the test waits for what it expects
    // instead of for a fixed time.
    let log: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let seen = |pred: &dyn Fn(&str) -> bool| log.lock().expect("log").iter().any(|e| pred(e));
    // Generous: the conditions are signals, so a fast machine never waits
    // this long, and a slow CI runner still passes.
    let wait_for = |what: &str, done: &dyn Fn() -> bool| {
        let deadline = Instant::now() + Duration::from_secs(120);
        while !done() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {:?}",
                log.lock().expect("log")
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| {
            watch(&conv, &input, &out, &opts, &stop, &mut |e: &WatchEvent| {
                log.lock().expect("log").push(e.sentence())
            })
            .expect("watch");
        });
        // Files written once the watcher has started are "later" ones.
        wait_for("the watcher to start", &|| {
            seen(&|e: &str| e.starts_with("Watching "))
        });
        write(&input, "late.md", "# Late\n\nArrived later.\n");
        write(&input, "photo.png", "x");
        wait_for("both files to be handled", &|| {
            out.join("late.txt").is_file()
                && input.join("processed/late.md").is_file()
                && input.join("processed/early.md").is_file()
                && seen(&|e: &str| e == "Converted late.md.")
                && seen(&|e: &str| e.starts_with("Ignored photo.png"))
        });
        stop.store(true, Ordering::SeqCst);
        handle.join().expect("join");
    });
    let events = log.into_inner().expect("log");
    assert_eq!(
        fs::read_to_string(out.join("late.txt")).expect("late output"),
        "Late\n\nArrived later.\n"
    );
    assert!(out.join("early.txt").is_file());
    assert!(input.join("processed/early.md").is_file());
    assert!(!input.join("late.md").exists());
    assert!(
        input.join("photo.png").exists(),
        "unsupported files stay put"
    );
    assert!(
        events.first().is_some_and(|e| e.starts_with("Watching ")),
        "{events:?}"
    );
    assert!(
        events.iter().any(|e| e == "Converted late.md."),
        "{events:?}"
    );
    assert!(
        events.iter().any(|e| e.starts_with("Ignored photo.png")),
        "{events:?}"
    );
    assert_eq!(events.last().map(String::as_str), Some("Stopped watching."));
    let log = fs::read_to_string(out.join("textweaver-watch.log")).expect("log");
    assert!(log.contains("Converted early.md."), "{log}");
}

/// Skips a test whose tool is missing, loudly, or fails it when CI says the
/// tool must be there: `TEXTWEAVER_REQUIRE_TOOLS` names the required tools,
/// comma-separated (`docs/dev/testing.md`, "Tests that need a tool"). A test
/// that skips where the tool should exist is a gate that never runs.
fn skip_or_fail(tool: &str, why: &str) {
    let required = std::env::var("TEXTWEAVER_REQUIRE_TOOLS")
        .is_ok_and(|v| v.split(',').any(|t| t.trim() == tool));
    assert!(
        !required,
        "Fail: {why}, but TEXTWEAVER_REQUIRE_TOOLS requires {tool} here"
    );
    eprintln!("SKIPPED, not checked: {why}");
}
