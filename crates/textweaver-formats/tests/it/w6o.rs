//! W6o's formats (ADR-0044) from `fixtures/o`: Obsidian notes, JSON, JSON
//! Lines, a notebook, SVG drawings, and a MathML formula; and hostile
//! inputs for each new reader, which must load (or fail with an error)
//! quickly, never panic, and never read outside the document's folder.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_core::MarkerKind;
use textweaver_formats::{LoadError, LoadOptions, Registry, Source, warnings};
use textweaver_text::Document;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/o")
        .join(name)
}

fn open(name: &str) -> Document {
    Registry::with_builtins()
        .load(&Source::Path(fixture(name)), &LoadOptions::default())
        .expect("the fixture loads")
}

fn bytes(data: impl Into<Vec<u8>>, hint: &str) -> Result<Document, LoadError> {
    Registry::with_builtins().load(
        &Source::Bytes {
            data: data.into(),
            hint: hint.into(),
        },
        &LoadOptions::default(),
    )
}

fn kinds(d: &Document, kind: MarkerKind) -> Vec<String> {
    d.marker_index()
        .iter(kind, None)
        .map(|m| d.slice(m.range))
        .collect()
}

/// Every marker inside the text and in order, as the fuzz targets check.
fn check(d: &Document) {
    let len = d.len_chars();
    let mut last = 0;
    for m in d.markers() {
        assert!(m.range.start <= m.range.end, "{m:?}");
        assert!(m.range.end.0 <= len, "{m:?} past {len}");
        assert!(m.range.start.0 >= last, "{m:?} out of order");
        last = m.range.start.0;
    }
}

/// Loads hostile bytes: an error or a checked document, within `limit`.
fn hostile(data: impl Into<Vec<u8>>, hint: &str, limit: Duration) -> Option<Document> {
    let start = Instant::now();
    let result = bytes(data, hint);
    let took = start.elapsed();
    assert!(took < limit, "{hint}: {took:?}");
    let d = result.ok()?;
    check(&d);
    Some(d)
}

const QUICK: Duration = Duration::from_secs(20);

#[test]
fn owner_checklist_callouts_embeds_json_svg_notebook() {
    // (1) A warning callout says "Warning" first.
    let c = open("vault/Callouts.md");
    check(&c);
    let text = c.text().to_string();
    assert!(text.contains("Warning: Hot surface\n"), "{text}");
    assert!(text.contains("Tip, collapsed: A folded tip"), "{text}");
    assert!(text.contains("tag physics slash waves"), "{text}");
    assert!(!text.contains("comment"), "{text}");
    assert!(textweaver_formats::obsidian::block_position(&c.meta, "last-para").is_some());
    // (2) An embed's start and end are said once each.
    let e = open("vault/Embeds.md");
    check(&e);
    let text = e.text().to_string();
    assert_eq!(text.matches("Embedded from Shared").count(), 1, "{text}");
    assert!(
        text.contains("Embeds (embedded above, not repeated)"),
        "{text}"
    );
    assert!(
        text.contains("Embedded from Topics/Waves, Frequency\n\nFrequency\n\nHow often"),
        "{text}"
    );
    assert!(!text.contains("How tall"), "{text}");
    assert_eq!(text.matches("End of embed").count(), 2, "{text}");
    // (3) JSON: `h` moves by key; no brackets.
    let j = open("profile.json");
    check(&j);
    assert_eq!(
        kinds(&j, MarkerKind::Heading)[..3],
        [
            "name: Ada Example",
            "role: Student",
            "address, object, 3 keys"
        ]
    );
    assert!(!j.text().to_string().contains(['{', '[', '"']));
    let l = open("events.jsonl");
    assert_eq!(kinds(&l, MarkerKind::Heading).len(), 3);
    // (4) SVG: the title first; none: "Drawing with no description".
    let s = open("chart.svg");
    check(&s);
    assert!(
        s.text()
            .to_string()
            .starts_with("Cells counted by hour\n\nA bar chart")
    );
    assert_eq!(kinds(&s, MarkerKind::ListItem).len(), 4);
    assert_eq!(
        open("untitled.svg").text().to_string(),
        "Drawing with no description"
    );
    // (5) A notebook: code cells named by their language.
    let n = open("growth.ipynb");
    check(&n);
    let text = n.text().to_string();
    assert!(text.contains("Python code\n\nn0 = 100"), "{text}");
    assert!(text.contains("After 3 hours: 800 cells"), "{text}");
    assert_eq!(n.meta.title.as_deref(), Some("Cell growth"));
    // A MathML file is one formula.
    let m = open("growth.mml");
    assert_eq!(
        kinds(&m, MarkerKind::Math),
        ["$$N\\left( t \\right) = N_{0} \\times 2^{t}$$"]
    );
}

#[test]
fn unclosed_void_elements_keep_the_whole_page() {
    // star's critical bug: an unclosed `<meta charset>` emptied the page.
    let d = open("unclosed-meta.html");
    check(&d);
    let text = d.text().to_string();
    for kept in [
        "Still here",
        "Every word after the unclosed meta",
        "A small circle",
        "and a line after a break.",
        "The end.",
        "A dot",
    ] {
        assert!(text.contains(kept), "{kept:?} missing from {text:?}");
    }
}

#[test]
fn hostile_json() {
    let deep = "[".repeat(100_000);
    let d = hostile(deep.clone(), "json", QUICK).expect("read as text");
    assert!(warnings(&d.meta)[0].contains("nested too deeply"));
    hostile(deep, "jsonl", QUICK);
    hostile("{\"a\":".repeat(50_000), "json", QUICK);
    let wide = format!(
        "{{{}\"end\": 1}}",
        (0..200_000)
            .map(|i| format!("\"k{i}\": [1, 2],"))
            .collect::<String>()
    );
    hostile(wide, "json", Duration::from_secs(60));
    hostile(b"\xff\xfe{\x00}\x00\"\x00".to_vec(), "json", QUICK);
    hostile("\"\\ud800\\u12\"", "json", QUICK);
    hostile(
        "{\"cells\": [{\"cell_type\": \"code\", \"outputs\": [{\"data\": 7}]}, 5, null]}",
        "ipynb",
        QUICK,
    );
    hostile("[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]", "ipynb", QUICK);
}

#[test]
fn hostile_svg_and_mathml() {
    // Entity expansion: roxmltree refuses what would grow without end.
    let laughs = "<?xml version=\"1.0\"?><!DOCTYPE svg [<!ENTITY a \"aaaaaaaaaa\"><!ENTITY b \"&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;\"><!ENTITY c \"&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;\"><!ENTITY d \"&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;\"><!ENTITY e \"&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;\"><!ENTITY f \"&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;\">]><svg><title>&f;&f;&f;&f;</title></svg>";
    hostile(laughs, "svg", QUICK);
    // An external entity is never read.
    let external = "<?xml version=\"1.0\"?><!DOCTYPE svg [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><svg><title>&x;</title></svg>";
    if let Some(d) = hostile(external, "svg", QUICK) {
        assert!(!d.text().to_string().contains("root:"));
    }
    let deep = format!(
        "<svg>{}<text>x</text>{}</svg>",
        "<g>".repeat(100_000),
        "</g>".repeat(100_000)
    );
    hostile(deep.clone(), "svg", QUICK);
    hostile(format!("<p>{deep}</p>"), "html", QUICK);
    hostile("<svg><title>unclosed", "svg", QUICK);
    let wide = format!("<svg>{}</svg>", "<g><title>t</title></g>".repeat(200_000));
    hostile(wide, "svg", Duration::from_secs(60));
    let apply = format!(
        "<math>{}<ci>x</ci>{}</math>",
        "<apply><plus/><cn>1</cn>".repeat(20_000),
        "</apply>".repeat(20_000)
    );
    hostile(apply, "mml", QUICK);
    hostile(
        "<math><apply/><apply><sum/><bvar/><interval/></apply><cn type=\"rational\"><sep/><sep/></cn></math>",
        "mml",
        QUICK,
    );
    assert!(hostile("no math here", "mml", QUICK).is_none());
}

#[test]
fn hostile_obsidian_notes() {
    hostile("%%".repeat(50_000), "md", QUICK);
    hostile("==".repeat(50_000), "md", QUICK);
    hostile("> [!".repeat(50_000), "md", QUICK);
    hostile(format!("> [!{}]", "a".repeat(100_000)), "md", QUICK);
    hostile("![[x]] ".repeat(50_000), "md", QUICK);
    hostile("#a/".repeat(50_000), "md", QUICK);
    hostile("text ^id\n\n".repeat(20_000), "md", QUICK);
    // Embeds reach no further than the note's own folder.
    let dir = tempfile::tempdir().expect("a folder");
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(&vault).expect("the vault");
    std::fs::write(dir.path().join("Outside.md"), "OUTSIDE").expect("a note outside");
    std::fs::write(
        vault.join("Note.md"),
        "![[../Outside]] ![[Outside]] ![[/Outside]] ![[C:/Outside]] ![[Note]]",
    )
    .expect("a note");
    let d = Registry::with_builtins()
        .load(
            &Source::Path(vault.join("Note.md")),
            &LoadOptions::default(),
        )
        .expect("it loads");
    assert!(!d.text().to_string().contains("OUTSIDE"), "{}", d.text());
}

#[test]
fn hostile_latex_past_w5c3() {
    // star's crash: a lone trailing backslash.
    hostile("text\\", "tex", QUICK);
    hostile("\\newcommand\\x[1]{#1}\\x", "tex", QUICK);
    hostile("\\newcommand\\x[9]{#9#8#7#6#5#4#3#2#1}\\x{", "tex", QUICK);
    hostile("\\newcommand\\a[1]{\\a{#1#1}}\\a{x}", "tex", QUICK);
    hostile("\\newcommand\\a[1]{$\\a{#1#1}$}$\\a{x}$", "tex", QUICK);
    hostile(
        "\\newenvironment{a}{\\begin{a}}{}\\begin{a}x\\end{a}",
        "tex",
        QUICK,
    );
    hostile(
        "\\newenvironment{a}{}{\\end{a}}\\begin{a}x\\end{a}",
        "tex",
        QUICK,
    );
    hostile("\\def\\a#1#2{\\a{#2}{#1#1}}\\a xy", "tex", QUICK);
    hostile(
        "\\multicolumn{99999999999}{c}{x} \\multirow{-3}{*}{y}",
        "tex",
        QUICK,
    );
    hostile("\\includegraphics[alt={{{]{x}", "tex", QUICK);
    hostile(
        "\\bibliography{../../etc/passwd}\\printbibliography",
        "tex",
        QUICK,
    );
    hostile("{".repeat(100_000), "tex", QUICK);
}
