//! Progress and cancel (`Converter::run_plan_with`), and the report file:
//! every file in the plan is reported once, a canceled run starts no more
//! files and leaves none half-written, and a report is not converted by
//! the next run.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use textweaver_convert::{
    ConvertOptions, Converter, OutputFormat, REPORT_FILE, REPORT_JSON_FILE, ReportFormat,
};

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

/// Every file in `root`'s tree, relative, sorted.
fn listing(root: &Path) -> Vec<String> {
    let mut v: Vec<String> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            e.path()
                .strip_prefix(root)
                .unwrap_or(e.path())
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    v.sort();
    v
}

#[test]
fn progress_hears_about_every_file_in_the_plan_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    tree(&input);
    // Two sources with one output, so the plan rejects a file too.
    write(&input, "sub/deep/d.md", "# Delta\n");
    write(&input, "sub/deep/d.txt", "Same output.\n");
    let conv = Converter::new(options(OutputFormat::Html, Some(&out))).expect("converter");
    let plan = conv.plan(std::slice::from_ref(&input)).expect("plan");
    let total = plan.len();
    assert!(!plan.rejected.is_empty(), "the collision is rejected");
    let seen = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    let s = conv
        .run_plan_with(
            plan,
            |f| seen.lock().expect("lock").push(f.source.clone()),
            &cancel,
        )
        .expect("run");
    let mut seen = seen.into_inner().expect("lock");
    assert_eq!(seen.len(), total);
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), total, "each file once");
    assert_eq!(s.canceled, 0);
    assert_eq!(s.total(), total);
    assert!(!s.sentence().starts_with("Stopped"), "{}", s.sentence());

    // Again: every output is up to date, and progress still counts them.
    let plan = conv.plan(std::slice::from_ref(&input)).expect("plan");
    let total = plan.len();
    let n = AtomicUsize::new(0);
    let s = conv
        .run_plan_with(
            plan,
            |_| {
                n.fetch_add(1, Ordering::Relaxed);
            },
            &cancel,
        )
        .expect("run");
    assert_eq!(n.load(Ordering::Relaxed), total);
    assert_eq!(s.converted, 0);
}

#[test]
fn run_plan_is_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    tree(&input);
    let conv = Converter::new(options(OutputFormat::Markdown, Some(&out))).expect("converter");
    let plan = conv.plan(std::slice::from_ref(&input)).expect("plan");
    let s = conv.run_plan(plan).expect("run");
    assert_eq!((s.converted, s.failed, s.canceled), (4, 0, 0));
}

#[test]
fn canceling_starts_no_more_files_and_leaves_none_half_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    let out = dir.path().join("out");
    for i in 0..40 {
        write(
            &input,
            &format!("part{}/doc{i:02}.md", i / 10),
            &format!("# Document {i}\n\nSome text.\n"),
        );
    }
    let conv = Converter::new(ConvertOptions {
        jobs: Some(1),
        ..options(OutputFormat::Epub, Some(&out))
    })
    .expect("converter");

    // Canceled before the run: nothing is written.
    let cancel = AtomicBool::new(true);
    let plan = conv.plan(std::slice::from_ref(&input)).expect("plan");
    let s = conv.run_plan_with(plan, |_| {}, &cancel).expect("run");
    assert_eq!((s.converted, s.canceled), (0, 40));
    assert!(s.files.is_empty());
    assert!(listing(&out).is_empty(), "{:?}", listing(&out));
    assert!(
        s.sentence()
            .starts_with("Stopped. Converted 0 files to EPUB in "),
        "{}",
        s.sentence()
    );

    // Canceled after the third file: the files already done are whole,
    // and the rest are never started.
    let cancel = AtomicBool::new(false);
    let done = AtomicUsize::new(0);
    let plan = conv.plan(std::slice::from_ref(&input)).expect("plan");
    let s = conv
        .run_plan_with(
            plan,
            |_| {
                if done.fetch_add(1, Ordering::SeqCst) + 1 == 3 {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
            &cancel,
        )
        .expect("run");
    assert_eq!(s.converted, 3, "{}", s.sentence());
    assert_eq!(s.canceled, 37);
    assert_eq!(s.files.len(), 3);
    let written = listing(&out);
    assert_eq!(written.len(), 3, "no temporary files are left: {written:?}");
    for f in &s.files {
        let bytes = fs::read(&f.output).expect("output");
        assert!(bytes.starts_with(b"PK"), "a whole EPUB");
        assert_eq!(bytes.len() as u64, f.bytes_out);
    }
    assert!(
        s.sentence().ends_with("37 files were not converted."),
        "{}",
        s.sentence()
    );
}

#[test]
fn a_report_in_the_folder_is_not_converted_next_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in");
    tree(&input);
    // HTML output, so Markdown files in the folder are sources: the
    // Markdown reports must still be left alone.
    let conv = Converter::new(options(OutputFormat::Html, None)).expect("converter");
    let s = conv.run(std::slice::from_ref(&input)).expect("run");
    let report = s
        .write_report(&input, ReportFormat::Markdown)
        .expect("report");
    assert_eq!(report, input.join(REPORT_FILE));
    let json = s.write_report(&input, ReportFormat::Json).expect("report");
    assert_eq!(json, input.join(REPORT_JSON_FILE));
    let own = s.write_file_reports(ReportFormat::Markdown);
    assert!(own.iter().all(Result::is_ok), "{own:?}");
    assert!(input.join("a.html.report.md").is_file());
    let plan = conv.plan(std::slice::from_ref(&input)).expect("plan");
    assert!(!plan.jobs.is_empty());
    assert!(
        plan.jobs.iter().all(|j| {
            let name = j
                .source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            !name.is_some_and(|n| n.contains("report"))
        }),
        "{:?}",
        plan.jobs
    );
}
