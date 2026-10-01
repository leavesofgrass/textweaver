//! Formatting snapshots: every featured style's bibliography and a set of
//! in-text citations, as plain text; APA also as Markdown and HTML.

use std::fmt::Write as _;
use std::path::PathBuf;

use textweaver_cite::{CitationStyle, Formatter, Library, OutputFormat, formats, pandoc};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/p")
        .join(name)
}

fn library() -> Library {
    Library::from_references(formats::read_file(&fixture("library.json")).unwrap())
}

const TEXT: &str = "Thermometry [@kucsko2013] and stories [@dahl1988, p. 12]. \
    See [see @smyth2007, pp. 5-6; also @vangogh2020]. @muller2019 [chap. 3] argues. \
    Reports [@who2021; -@access2024] and a missing key [@nobody2000].";

fn render(style: &str, format: OutputFormat) -> String {
    let lib = library();
    let style = CitationStyle::builtin(style).unwrap();
    let fmt = Formatter::new(&style, format);
    let cites = pandoc::find_citations(TEXT);
    let doc = fmt.document(&cites, &lib).unwrap();
    let mut out = String::new();
    writeln!(out, "style: {}", style.title()).unwrap();
    writeln!(out, "citations:").unwrap();
    for (c, text) in cites.iter().zip(&doc.citations) {
        writeln!(out, "  {} => {}", &TEXT[c.range.clone()], text).unwrap();
    }
    writeln!(out, "missing: {}", doc.missing.join(", ")).unwrap();
    writeln!(out, "bibliography of cited works:").unwrap();
    for e in &doc.bibliography {
        writeln!(out, "  [{}] {}", e.key, e.text).unwrap();
    }
    let all: Vec<_> = lib.references().iter().collect();
    writeln!(out, "whole library:").unwrap();
    for e in fmt.bibliography(&all).unwrap() {
        writeln!(out, "  [{}] {}", e.key, e.text).unwrap();
    }
    out
}

macro_rules! style_snapshot {
    ($test:ident, $style:literal) => {
        #[test]
        fn $test() {
            insta::assert_snapshot!(render($style, OutputFormat::Plain));
        }
    };
}

style_snapshot!(apa_plain, "apa");
style_snapshot!(mla_plain, "mla");
style_snapshot!(chicago_author_date_plain, "chicago");
style_snapshot!(chicago_notes_plain, "chicago-notes");
style_snapshot!(ieee_plain, "ieee");
style_snapshot!(vancouver_plain, "vancouver");
style_snapshot!(harvard_plain, "harvard");
style_snapshot!(ama_plain, "ama");

#[test]
fn apa_markdown() {
    insta::assert_snapshot!(render("apa", OutputFormat::Markdown));
}

#[test]
fn apa_html() {
    insta::assert_snapshot!(render("apa", OutputFormat::Html));
}

#[test]
fn plain_output_has_no_markup_and_numbers_are_not_glued_to_words() {
    for style in [
        "apa",
        "mla",
        "chicago",
        "ieee",
        "vancouver",
        "ama",
        "nature",
    ] {
        let out = render(style, OutputFormat::Plain);
        for bad in ["<i>", "</", "&amp;", "*", "\u{1b}["] {
            assert!(!out.contains(bad), "{style}: {bad:?} in\n{out}");
        }
    }
    // AMA and Nature use superscript numbers; plain text brackets them.
    let ama = render("ama", OutputFormat::Plain);
    assert!(ama.contains("=> [1]"), "{ama}");
}

#[test]
fn csl_files_load_including_dependent_styles() {
    let lib = library();
    let own = CitationStyle::from_file(&fixture("styles/textweaver-plain.csl")).unwrap();
    assert_eq!(own.title(), "Textweaver Plain Test Style");
    let fmt = Formatter::new(&own, OutputFormat::Markdown);
    let r = lib.get("dahl1988").unwrap();
    assert_eq!(
        fmt.entry(r).unwrap(),
        "Dahl, Roald. 1988. *Fantastic Mr. Fox*. Puffin."
    );
    assert_eq!(fmt.cite(r).unwrap(), "(Dahl, 1988)");

    let dependent =
        CitationStyle::resolve(fixture("styles/dependent-on-apa.csl").to_str().unwrap()).unwrap();
    assert_eq!(dependent.title(), "Journal of Accessible Reading (test)");
    let apa = CitationStyle::builtin("apa").unwrap();
    assert_eq!(
        Formatter::new(&dependent, OutputFormat::Plain)
            .entry(r)
            .unwrap(),
        Formatter::new(&apa, OutputFormat::Plain).entry(r).unwrap()
    );
}

#[test]
fn a_dependent_style_with_an_unknown_parent_explains_itself() {
    let xml = std::fs::read_to_string(fixture("styles/dependent-on-apa.csl"))
        .unwrap()
        .replace(
            "http://www.zotero.org/styles/apa",
            "http://www.zotero.org/styles/not-built-in",
        );
    let err = CitationStyle::from_xml(&xml).unwrap_err().to_string();
    assert!(err.contains("download that parent style"), "{err}");
}

#[test]
fn awkward_fields_are_dropped_not_fatal() {
    let mut r = textweaver_cite::Reference::new("odd", "webpage");
    r.title = Some("A title with {braces} and $dollars$".into());
    r.url = Some("not a url".into());
    r.language = Some("not a language tag!".into());
    let style = CitationStyle::builtin("apa").unwrap();
    let text = Formatter::new(&style, OutputFormat::Plain)
        .entry(&r)
        .unwrap();
    assert!(
        text.contains("A title with {braces} and $dollars$"),
        "{text}"
    );
}
