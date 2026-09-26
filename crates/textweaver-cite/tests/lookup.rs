//! DOI and ISBN lookup against recorded responses (no network), plus one
//! ignored live test.

use std::path::PathBuf;
use std::time::Duration;

use textweaver_cite::{
    Cache, CslDate, HttpClient, HttpResponse, Identifier, Lookup, LookupError, Name,
    RecordedClient, TransportError, UreqClient,
};

fn recorded(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/p/recorded")
        .join(name);
    std::fs::read_to_string(path).expect("recorded response")
}

fn client() -> RecordedClient {
    RecordedClient::new()
        .with(
            "https://doi.org/10.1038/nature12373",
            200,
            &recorded("doi-nature12373.json"),
        )
        .with(
            "https://doi.org/10.5061/dryad.8515",
            200,
            &recorded("doi-dryad.8515.json"),
        )
        .with(
            "https://doi.org/10.1007/978-3-540-74958-5_4",
            200,
            &recorded("doi-chapter.json"),
        )
        .with("https://doi.org/10.1234/missing", 404, "")
        .with(
            "https://doi.org/10.1234/broken",
            200,
            "<html>not json</html>",
        )
        .with("https://doi.org/10.1234/busy", 503, "")
        .with(
            "https://openlibrary.org/isbn/9780140328721.json",
            200,
            &recorded("ol-isbn-9780140328721.json"),
        )
        .with(
            "https://openlibrary.org/authors/OL34184A.json",
            200,
            &recorded("ol-author-OL34184A.json"),
        )
        .with("https://openlibrary.org/isbn/0140328726.json", 404, "{}")
        .with("https://openlibrary.org/isbn/9780306406157.json", 404, "{}")
        .with("https://openlibrary.org/isbn/0306406152.json", 404, "{}")
}

#[test]
fn crossref_doi_becomes_a_clean_journal_article() {
    let c = client();
    let r = Lookup::new(&c)
        .lookup("https://doi.org/10.1038/nature12373")
        .unwrap();
    assert_eq!(r.id, "", "the library assigns the key");
    assert_eq!(
        r.kind, "article-journal",
        "Crossref's journal-article is mapped"
    );
    assert_eq!(
        r.title.as_deref(),
        Some("Nanometre-scale thermometry in a living cell")
    );
    assert_eq!(r.author[0], Name::new("Kucsko", "G."));
    assert_eq!(r.author.len(), 8);
    assert_eq!(r.container_title.as_deref(), Some("Nature"));
    assert_eq!(r.container_title_short, None, "same as the full title");
    assert_eq!(r.volume.as_deref(), Some("500"));
    assert_eq!(r.issue.as_deref(), Some("7460"));
    assert_eq!(r.page.as_deref(), Some("54-58"));
    assert_eq!(
        r.issued.as_ref().and_then(|d| d.iso()).as_deref(),
        Some("2013-07-31")
    );
    assert_eq!(r.doi.as_deref(), Some("10.1038/nature12373"));
    assert_eq!(r.url, None, "a doi.org URL only repeats the DOI");
    for noisy in ["reference", "license", "indexed", "link", "published-print"] {
        assert!(!r.extra.contains_key(noisy), "{noisy} kept");
    }
    assert_eq!(c.requests(), ["https://doi.org/10.1038/nature12373"]);
}

#[test]
fn datacite_and_chapter_dois() {
    let c = client();
    let lookup = Lookup::new(&c);
    let d = lookup.lookup("10.5061/dryad.8515").unwrap();
    assert_eq!(d.kind, "dataset");
    assert_eq!(d.publisher.as_deref(), Some("Dryad"));
    assert_eq!(d.author[4], Name::new("Arnathau", "Céline"));
    assert_eq!(d.year(), Some(2011));

    let ch = lookup.lookup("doi:10.1007/978-3-540-74958-5_4").unwrap();
    assert_eq!(ch.kind, "chapter");
    assert_eq!(ch.page.as_deref(), Some("5"), "a 5-5 range is one page");
    assert_eq!(ch.issued, None, "Crossref's empty date is dropped");
    assert_eq!(ch.isbn.as_deref(), Some("9783540749578, 9783540749585"));
}

#[test]
fn isbn_reads_the_edition_and_its_authors() {
    let c = client();
    let r = Lookup::new(&c).lookup("ISBN 0-14-032872-6").unwrap();
    assert_eq!(r.kind, "book");
    assert_eq!(r.title.as_deref(), Some("Fantastic Mr. Fox"));
    assert_eq!(r.author, vec![Name::new("Dahl", "Roald")]);
    assert_eq!(r.publisher.as_deref(), Some("Puffin"));
    assert_eq!(r.issued, Some(CslDate::ymd(1988, Some(10), Some(1))));
    assert_eq!(r.isbn.as_deref(), Some("9780140328721"));
    assert_eq!(r.url, None, "a catalog page is not the book");
    assert_eq!(
        r.extra.get("openlibrary").and_then(|v| v.as_str()),
        Some("https://openlibrary.org/books/OL7353617M")
    );
    assert_eq!(
        c.requests(),
        [
            "https://openlibrary.org/isbn/0140328726.json",
            "https://openlibrary.org/isbn/9780140328721.json",
            "https://openlibrary.org/authors/OL34184A.json",
        ],
        "the ISBN-10 is unknown here, so the ISBN-13 is tried next"
    );
}

#[test]
fn failures_read_well_and_say_nothing_was_added() {
    let c = client();
    let lookup = Lookup::new(&c);
    let err = lookup.lookup("10.1234/missing").unwrap_err();
    assert_eq!(
        err,
        LookupError::NotFound {
            service: "doi.org",
            what: "DOI 10.1234/missing".into()
        }
    );
    assert_eq!(
        err.to_string(),
        "doi.org has no record for DOI 10.1234/missing. Nothing was added."
    );

    let err = lookup.lookup("9780306406157").unwrap_err();
    assert!(
        matches!(
            err,
            LookupError::NotFound {
                service: "Open Library",
                ..
            }
        ),
        "{err}"
    );

    let err = lookup.lookup("10.1234/broken").unwrap_err().to_string();
    assert!(
        err.starts_with("doi.org sent an answer textweaver could not read"),
        "{err}"
    );

    let err = lookup.lookup("10.1234/busy").unwrap_err().to_string();
    assert_eq!(
        err,
        "doi.org answered with an error, status 503. Nothing was added; try again later."
    );

    let err = lookup.lookup("10.1234/not-recorded").unwrap_err();
    assert!(matches!(err, LookupError::Offline { .. }));
    assert!(
        err.to_string()
            .starts_with("Could not reach doi.org. You may be offline; nothing was added.")
    );

    let err = lookup.lookup("978-0-306-40615-8").unwrap_err().to_string();
    assert!(err.contains("is not a valid ISBN"), "{err}");
}

struct SlowClient;

impl HttpClient for SlowClient {
    fn get(&self, _url: &str, _accept: &str) -> Result<HttpResponse, TransportError> {
        Err(TransportError::Timeout)
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(15)
    }
}

#[test]
fn timeouts_name_the_service_and_the_wait() {
    let err = Lookup::new(&SlowClient)
        .lookup("10.1038/nature12373")
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "doi.org did not answer within 15 seconds. Nothing was added; try again later."
    );
}

#[test]
fn the_cache_answers_offline_and_is_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let online = client();
    let first = Lookup::new(&online)
        .with_cache(Cache::new(dir.path()))
        .lookup("10.1038/nature12373")
        .unwrap();
    let offline = RecordedClient::new();
    let again = Lookup::new(&offline)
        .with_cache(Cache::new(dir.path()))
        .lookup("https://doi.org/10.1038/NATURE12373")
        .unwrap();
    assert_eq!(again, first, "DOIs are case-insensitive");
    assert!(offline.requests().is_empty());

    let mut small = Cache::new(dir.path().join("small"));
    small.max_entries = 2;
    for isbn in ["9780140328721", "9780306406157", "0306406152", "080442957X"] {
        let id = Identifier::parse(isbn).unwrap();
        small.put(&id, &first);
    }
    let count = std::fs::read_dir(small.dir()).unwrap().count();
    assert_eq!(count, 2);
}

/// Hits doi.org and Open Library for real. Run with
/// `cargo test -p textweaver-cite --test lookup -- --ignored`.
#[test]
#[ignore = "uses the network"]
fn live_lookup() {
    let client = UreqClient::default();
    let lookup = Lookup::new(&client);
    let r = lookup.lookup("10.1038/nature12373").unwrap();
    assert_eq!(r.kind, "article-journal");
    assert_eq!(r.container_title.as_deref(), Some("Nature"));
    let b = lookup.lookup("9780140328721").unwrap();
    assert_eq!(b.author, vec![Name::new("Dahl", "Roald")]);
}
