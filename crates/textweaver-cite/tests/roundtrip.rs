//! Import and export round trips through every format, and imports of the
//! hand-written fixtures.

use std::path::PathBuf;

use textweaver_cite::{CslDate, Format, Library, Name, Reference, formats};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/p")
        .join(name)
}

fn library() -> Vec<Reference> {
    formats::read_file(&fixture("library.json")).expect("fixture library")
}

/// The fields every format carries, normalized so formats that cannot say
/// something (BibTeX has no "webpage" type) compare equal where they agree.
#[derive(Debug, PartialEq)]
struct Core {
    id: String,
    kind: String,
    title: Option<String>,
    author: Vec<Name>,
    editor: Vec<Name>,
    year: Option<i32>,
    month: Option<u8>,
    container: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    page: Option<String>,
    publisher: Option<String>,
    place: Option<String>,
    doi: Option<String>,
    isbn: Option<String>,
    url: Option<String>,
}

fn core(r: &Reference, format: Format) -> Core {
    let kind = match (format, r.kind.as_str()) {
        // BibTeX has no web page type; `@misc` comes back as a document.
        (Format::BibTex, "webpage") => "document".to_owned(),
        _ => r.kind.clone(),
    };
    let lossy_ids = format == Format::Ris;
    Core {
        id: if lossy_ids {
            String::new()
        } else {
            r.id.clone()
        },
        kind,
        title: r.title.clone(),
        author: r.author.clone(),
        editor: r.editor.clone(),
        year: r.year(),
        month: r.issued.as_ref().and_then(CslDate::month),
        container: r.container_title.clone(),
        volume: r.volume.clone(),
        issue: r.issue.clone().or_else(|| r.number.clone()),
        page: r.page.clone(),
        publisher: r.publisher.clone(),
        place: r.publisher_place.clone(),
        doi: r.doi.clone(),
        isbn: r.isbn.clone(),
        url: r.url.clone(),
    }
}

#[test]
fn every_format_round_trips_the_core_fields() {
    let refs = library();
    assert_eq!(refs.len(), 7);
    for format in Format::ALL {
        let text = formats::write(&refs, format).unwrap();
        let back =
            formats::parse(&text, format).unwrap_or_else(|e| panic!("{format:?}: {e}\n{text}"));
        assert_eq!(back.len(), refs.len(), "{format:?}");
        for (a, b) in refs.iter().zip(&back) {
            let mut expected = core(a, format);
            let mut got = core(b, format);
            if format == Format::Ris {
                // RIS keeps the key in ID.
                got.id.clear();
                expected.id.clear();
                assert_eq!(b.id, a.id, "RIS ID for {}", a.id);
            }
            assert_eq!(got, expected, "{format:?} round trip of {}\n{text}", a.id);
        }
    }
}

#[test]
fn csl_json_is_lossless_including_unknown_fields() {
    let mut refs = library();
    refs[0]
        .extra
        .insert("custom-field".into(), serde_json::json!({"nested": [1, 2]}));
    refs[1].translator = vec![Name::new("Translator", "Tina")];
    refs[1].author[0]
        .extra
        .insert("static-ordering".into(), serde_json::json!(true));
    let text = formats::write(&refs, Format::CslJson).unwrap();
    let back = formats::parse(&text, Format::CslJson).unwrap();
    assert_eq!(back, refs);
    // Writing again gives identical text.
    assert_eq!(formats::write(&back, Format::CslJson).unwrap(), text);
}

#[test]
fn library_file_round_trips_through_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("references.json");
    let mut lib = Library::from_references(library());
    lib.set_path(&path);
    lib.save().unwrap();
    let back = Library::load(&path).unwrap();
    assert_eq!(back.references(), lib.references());
}

#[test]
fn hand_written_bibtex_imports() {
    let refs = formats::read_file(&fixture("sample.bib")).unwrap();
    let ids: Vec<&str> = refs.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "kucsko2013",
            "dahl1988",
            "um2007",
            "smyth2007",
            "muller2019",
            "who2021"
        ]
    );
    let by = |id: &str| refs.iter().find(|r| r.id == id).unwrap();

    let k = by("kucsko2013");
    assert_eq!(k.kind, "article-journal");
    assert_eq!(k.container_title.as_deref(), Some("Nature"), "string macro");
    assert_eq!(
        k.issued.as_ref().and_then(CslDate::month),
        Some(7),
        "month macro"
    );
    assert_eq!(k.page.as_deref(), Some("54-58"), "en dash back to hyphen");
    assert_eq!(k.issue.as_deref(), Some("7460"));
    assert_eq!(k.author.len(), 3);

    let d = by("dahl1988");
    assert_eq!(
        d.author,
        vec![Name::new("Dahl", "Roald")],
        "given-family order"
    );
    assert_eq!(
        d.title.as_deref(),
        Some("Fantastic Mr. Fox"),
        "braces dropped"
    );

    let s = by("smyth2007");
    assert_eq!(s.kind, "paper-conference");
    assert_eq!(
        s.container_title.as_deref(),
        Some("User Modeling 2007"),
        "crossref booktitle"
    );
    assert_eq!(s.editor.len(), 2, "crossref editors");
    assert_eq!(s.year(), Some(2007), "crossref year");

    let m = by("muller2019");
    assert_eq!(m.author, vec![Name::new("Müller", "Jörg")], "accents");
    assert_eq!(m.title.as_deref(), Some("Hörbücher und Sprachsynthese"));
    assert_eq!(m.publisher.as_deref(), Some("Universität Wien"), "school");
    assert_eq!(m.genre.as_deref(), Some("PhD thesis"));

    let w = by("who2021");
    assert_eq!(w.author, vec![Name::literal("World Health Organization")]);
    assert_eq!(
        w.title.as_deref(),
        Some("World Report on Vision & Print Disability")
    );
    assert_eq!(w.number.as_deref(), Some("WHO/2021.1"));
    assert_eq!(
        w.url.as_deref(),
        Some("https://example.org/who_report?id=1&lang=en")
    );
}

#[test]
fn zotero_style_ris_imports() {
    let refs = formats::read_file(&fixture("sample.ris")).unwrap();
    assert_eq!(refs.len(), 3);
    let (a, b, c) = (&refs[0], &refs[1], &refs[2]);
    assert_eq!(a.kind, "article-journal");
    assert_eq!(a.id, "kucsko2013", "generated key");
    assert_eq!(
        a.issued.as_ref().and_then(|d| d.iso()).as_deref(),
        Some("2013-07-31")
    );
    assert_eq!(a.page.as_deref(), Some("54-58"));
    assert_eq!(a.issn.as_deref(), Some("1476-4687"));
    assert_eq!(b.kind, "book");
    assert_eq!(b.isbn.as_deref(), Some("9780140328721"));
    assert_eq!(c.kind, "chapter");
    assert_eq!(c.editor.len(), 2);
    assert_eq!(
        c.collection_title.as_deref(),
        Some("Lecture Notes in Computer Science")
    );
    assert_eq!(
        c.abstract_text.as_deref(),
        Some("A keynote abstract that wraps onto a second line in some exporters.")
    );
    assert_eq!(
        c.keyword.as_deref(),
        Some("personalization, recommender systems")
    );
}

#[test]
fn importing_the_same_works_twice_updates_instead_of_duplicating() {
    let mut lib = Library::new();
    let first = lib.merge(formats::read_file(&fixture("sample.bib")).unwrap());
    assert_eq!(first.added(), 6);
    // The RIS file describes three of the same works (same DOI or ISBN).
    let second = lib.merge(formats::read_file(&fixture("sample.ris")).unwrap());
    assert_eq!(second.updated(), 3, "{}", second.announcement());
    assert_eq!(lib.len(), 6);
}
