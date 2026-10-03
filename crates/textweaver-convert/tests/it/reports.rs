//! Conversion reports with provenance: a sample with an image without a
//! description, a formula that does not parse, and a table whose rows do
//! not line up gives one report entry for each, with where it is; the
//! source's hash and textweaver's version are there; and no path outside
//! the converted folder appears.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use textweaver_convert::report::VERSION;
use textweaver_convert::{
    ConvertOptions, Converter, IssueKind, OutputFormat, REPORT_FILE, REPORT_JSON_FILE, ReportFormat,
};

const SAMPLE: &str = r#"<html><head><title>Lab notes</title></head><body>
<h1>Results</h1>
<p><img src="figures/chart.png" alt="chart.png"></p>
<h2>Data</h2>
<table>
<tr><th>Dose</th><th>Effect</th></tr>
<tr><td>1 mg</td></tr>
</table>
<h2>Model</h2>
<p><math><semantics><mi>x</mi><annotation encoding="application/x-tex">\frac{a}{</annotation></semantics></math></p>
</body></html>
"#;

fn sample(dir: &Path) -> std::path::PathBuf {
    let input = dir.join("course");
    fs::create_dir_all(&input).expect("mkdir");
    fs::write(input.join("lab.html"), SAMPLE).expect("write");
    fs::write(
        input.join("clean.md"),
        "# Clean\n\n![A cat on a mat](cat.png)\n",
    )
    .expect("write");
    // An image with no description at all. (The HTML loader leaves such
    // an image out of the document, so it is a Markdown image here.)
    fs::write(input.join("scan.md"), "# Scan\n\n![](figures/scan.png)\n").expect("write");
    input
}

fn options(out: &Path) -> ConvertOptions {
    ConvertOptions {
        to: OutputFormat::Epub,
        out_dir: Some(out.to_owned()),
        pandoc: false,
        jobs: Some(2),
        ..ConvertOptions::default()
    }
}

#[test]
fn the_sample_yields_one_entry_for_each_problem_with_its_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = sample(dir.path());
    let out = dir.path().join("converted");
    let conv = Converter::new(options(&out)).expect("converter");
    let s = conv.run(std::slice::from_ref(&input)).expect("run");
    assert_eq!(s.converted, 3, "{}", s.sentence());
    assert_eq!(s.with_issues, 2, "{:#?}", s.files);
    assert!(
        s.sentence()
            .contains("2 files have items that could not be made accessible."),
        "{}",
        s.sentence()
    );
    let lab = s
        .files
        .iter()
        .find(|f| f.name == "lab.html")
        .expect("lab.html");
    let kinds: Vec<IssueKind> = lab.issues.iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        [
            IssueKind::ImageWithoutDescription,
            IssueKind::RaggedTable,
            IssueKind::MathNotParsed,
        ],
        "{:#?}",
        lab.issues
    );
    let headings: Vec<Option<&str>> = lab
        .issues
        .iter()
        .map(|i| i.location.heading.as_deref())
        .collect();
    assert_eq!(headings, [Some("Results"), Some("Data"), Some("Model")]);
    assert_eq!(
        lab.issues[0].detail.as_deref(),
        Some("chart.png, described only by its file name")
    );
    let scan = s
        .files
        .iter()
        .find(|f| f.name == "scan.md")
        .expect("scan.md");
    assert_eq!(scan.issues.len(), 1, "{:?}", scan.issues);
    assert_eq!(scan.issues[0].kind, IssueKind::ImageWithoutDescription);
    assert_eq!(scan.issues[0].detail.as_deref(), Some("scan.png"));
    assert_eq!(scan.issues[0].location.heading.as_deref(), Some("Scan"));
    // The hash of the source as it was read.
    let expected = Sha256::digest(SAMPLE.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(lab.sha256.as_deref(), Some(expected.as_str()));
    let clean = s
        .files
        .iter()
        .find(|f| f.name == "clean.md")
        .expect("clean.md");
    assert!(clean.issues.is_empty(), "{:?}", clean.issues);

    // The Markdown report.
    let md_path = s
        .write_report(&out, ReportFormat::Markdown)
        .expect("report");
    assert_eq!(md_path, out.join(REPORT_FILE));
    let md = fs::read_to_string(&md_path).expect("read");
    assert!(
        md.contains(&format!("- Made by: textweaver {VERSION}\n")),
        "{md}"
    );
    assert!(md.contains("- Output format: EPUB\n"), "{md}");
    assert!(
        md.contains(&format!("- Source SHA-256: {expected}\n")),
        "{md}"
    );
    assert!(
        md.contains("2 files with items that could not be made accessible, 4 items in all:\n\n- lab.html: 3 items\n- scan.md: 1 item\n"),
        "{md}"
    );
    assert!(
        md.contains("### lab.html\n\n- Result: converted to lab.epub\n"),
        "{md}"
    );
    assert!(
        md.contains(
            "1. Image without a description: chart.png, described only by its file name. Under the heading \u{201c}Results\u{201d}"
        ),
        "{md}"
    );
    assert!(
        md.contains("2. Table whose columns do not line up: 2 rows, with 2, 1 cells. Under the heading \u{201c}Data\u{201d}"),
        "{md}"
    );
    assert!(md.contains("3. Math that did not parse: `"), "{md}");
    assert!(
        md.contains("Under the heading \u{201c}Model\u{201d}"),
        "{md}"
    );
    let written = md
        .lines()
        .find(|l| l.starts_with("- Written: "))
        .expect("a date line");
    assert!(written.ends_with(" UTC"), "{written}");

    // The JSON report.
    let json_path = s.write_report(&out, ReportFormat::Json).expect("report");
    assert_eq!(json_path, out.join(REPORT_JSON_FILE));
    let json = fs::read_to_string(&json_path).expect("read");
    let v: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(v["textweaver"], VERSION);
    assert_eq!(v["format"], "epub");
    let files = v["files"].as_array().expect("files");
    let lab = files
        .iter()
        .find(|f| f["name"] == "lab.html")
        .expect("lab.html");
    assert_eq!(lab["sha256"], expected.as_str());
    assert_eq!(lab["issues"][0]["kind"], "image-without-description");
    assert_eq!(lab["issues"][0]["location"]["heading"], "Results");
    assert_eq!(lab["issues"][1]["kind"], "ragged-table");
    assert_eq!(lab["issues"][2]["kind"], "math-not-parsed");
    assert!(
        lab["issues"][2]["location"]["line"]
            .as_u64()
            .is_some_and(|l| l > 0)
    );

    // No path outside the converted folder, in either report: not the
    // temporary folder, not the image's folder.
    let outside = dir.path().to_string_lossy().into_owned();
    for text in [&md, &json] {
        assert!(!text.contains(&outside), "{text}");
        assert!(!text.contains("figures"), "{text}");
        assert!(!text.contains(":\\"), "{text}");
    }
}

#[test]
fn a_file_converted_on_its_own_gets_its_own_report() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = sample(dir.path());
    let src = input.join("lab.html");
    let conv = Converter::new(ConvertOptions {
        to: OutputFormat::Markdown,
        pandoc: false,
        ..ConvertOptions::default()
    })
    .expect("converter");
    let s = conv.run(std::slice::from_ref(&src)).expect("run");
    let written = s.write_file_reports(ReportFormat::Markdown);
    let path = input.join("lab.md.report.md");
    assert_eq!(written, [Ok(path.clone())]);
    let md = fs::read_to_string(&path).expect("read");
    assert!(md.starts_with("# Conversion report for lab.html\n"), "{md}");
    assert!(md.contains("- Source: lab.html\n"), "{md}");
    assert!(
        md.contains("Could not be made accessible, 3 items:"),
        "{md}"
    );
    assert!(
        !md.contains(&dir.path().to_string_lossy().into_owned()),
        "{md}"
    );
}

#[test]
fn without_the_audit_nothing_is_looked_for() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = sample(dir.path());
    let out = dir.path().join("converted");
    let conv = Converter::new(ConvertOptions {
        audit: false,
        ..options(&out)
    })
    .expect("converter");
    let s = conv.run(std::slice::from_ref(&input)).expect("run");
    assert!(s.files.iter().all(|f| f.issues.is_empty()));
    // The hash is still there: it costs little, and it is provenance.
    assert!(s.files.iter().all(|f| f.sha256.is_some()));
}
