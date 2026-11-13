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
        "doi.org has no record for DOI 10.1234/missing. Nothing was added; check it for a typing mistake."
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

const ESUMMARY: &str =
    "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&retmode=json&id=";

fn pubmed_client() -> RecordedClient {
    RecordedClient::new()
        .with(
            &format!("{ESUMMARY}12345678"),
            200,
            &recorded("pubmed-esummary-12345678.json"),
        )
        .with(
            &format!("{ESUMMARY}99999999"),
            200,
            r#"{"header":{"type":"esummary","version":"0.3"},"result":{"uids":["99999999"],"99999999":{"uid":"99999999","error":"cannot get document summary"}}}"#,
        )
        .with(
            &format!("{ESUMMARY}88888888"),
            200,
            r#"{"header":{"type":"esummary","version":"0.3"},"result":{"uids":[]}}"#,
        )
}

#[test]
fn pubmed_ids_are_recognized_but_never_guessed() {
    for typed in [
        "PMID:12345678",
        "pmid 12345678",
        "PMID12345678",
        "PubMed: 012345678",
    ] {
        let id = Identifier::parse(typed);
        if typed.contains("012345678") {
            // Nine digits: too long for a PubMed ID.
            assert!(
                matches!(id, Err(LookupError::InvalidPmid { .. })),
                "{typed}"
            );
        } else {
            assert_eq!(id, Ok(Identifier::Pmid("12345678".into())), "{typed}");
        }
    }
    assert_eq!(
        Identifier::parse("pmid: 0042"),
        Ok(Identifier::Pmid("42".into()))
    );
    for bad in ["PMID", "pmid: 0", "pmid abc"] {
        assert!(
            matches!(Identifier::parse(bad), Err(LookupError::InvalidPmid { .. })),
            "{bad}"
        );
    }
    // A bare number is not taken for a PubMed ID.
    let err = Identifier::parse("12345678").unwrap_err();
    assert_eq!(
        err.to_string(),
        "12345678 is too short for an ISBN. If it is a PubMed ID, write PMID 12345678."
    );
    assert!(matches!(
        Identifier::parse("ISBN 12345"),
        Err(LookupError::InvalidIsbn { .. })
    ));
    // Unless the user says so.
    assert_eq!(
        Identifier::parse_pmid(" 12345678 "),
        Ok(Identifier::Pmid("12345678".into()))
    );
    assert_eq!(
        Identifier::parse_pmid("PMID: 7"),
        Ok(Identifier::Pmid("7".into()))
    );
    assert!(Identifier::parse_pmid("123456789").is_err());
    assert_eq!(Identifier::Pmid("7".into()).describe(), "PMID 7");
}

#[test]
fn pubmed_summary_becomes_a_journal_article() {
    let c = pubmed_client();
    let r = Lookup::new(&c).lookup("PMID 12345678").unwrap();
    assert_eq!(r.kind, "article-journal");
    assert_eq!(
        r.title.as_deref(),
        Some("Reading by ear: text-to-speech in medical school"),
        "markup and the closing period removed"
    );
    assert_eq!(
        r.author,
        vec![
            Name::new("Example", "A."),
            Name::new("van Example", "B. C."),
            Name::literal("Example Study Group"),
        ]
    );
    assert_eq!(
        r.container_title.as_deref(),
        Some("Journal of example medical education")
    );
    assert_eq!(
        r.container_title_short.as_deref(),
        Some("J Example Med Educ")
    );
    assert_eq!(r.volume.as_deref(), Some("12"));
    assert_eq!(r.issue.as_deref(), Some("3"));
    assert_eq!(r.page.as_deref(), Some("101-109"));
    assert_eq!(
        r.issued.as_ref().and_then(|d| d.iso()).as_deref(),
        Some("2024-03-05")
    );
    assert_eq!(r.doi.as_deref(), Some("10.5555/example.2024.0003"));
    assert_eq!(r.issn.as_deref(), Some("0000-0000"));
    assert_eq!(
        r.extra.get("PMID").and_then(|v| v.as_str()),
        Some("12345678")
    );
    assert_eq!(
        r.extra.get("PMCID").and_then(|v| v.as_str()),
        Some("PMC0000001")
    );
    // One request, naming nobody: no tool, email, or API key.
    let requests = c.requests();
    assert_eq!(requests, [format!("{ESUMMARY}12345678")]);
    for word in ["tool=", "email=", "api_key="] {
        assert!(!requests[0].contains(word), "{word}");
    }
}

#[test]
fn pubmed_refusals_read_well_with_a_next_step() {
    let c = pubmed_client();
    let lookup = Lookup::new(&c);
    for missing in ["PMID 99999999", "PMID 88888888"] {
        let err = lookup.lookup(missing).unwrap_err();
        assert!(matches!(err, LookupError::NotFound { .. }), "{err}");
        assert!(
            err.to_string().starts_with("PubMed has no record for PMID"),
            "{err}"
        );
        assert!(err.to_string().ends_with("check it for a typing mistake."));
    }
    let err = lookup.lookup("PMID 11111111").unwrap_err();
    assert!(matches!(err, LookupError::Offline { .. }));
    assert!(
        err.to_string().starts_with(
            "Could not reach PubMed. You may be offline; nothing was added. Try again when you are connected."
        ),
        "{err}"
    );
    let err = Lookup::new(&SlowClient)
        .lookup("PMID 12345678")
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "PubMed did not answer within 15 seconds. Nothing was added; try again later."
    );
}

#[test]
fn pubmed_answers_are_cached_offline() {
    let dir = tempfile::tempdir().unwrap();
    let online = pubmed_client();
    let first = Lookup::new(&online)
        .with_cache(Cache::new(dir.path()))
        .lookup("PMID:12345678")
        .unwrap();
    let offline = RecordedClient::new();
    let again = Lookup::new(&offline)
        .with_cache(Cache::new(dir.path()))
        .identifier(&Identifier::parse_pmid("12345678").unwrap())
        .unwrap();
    assert_eq!(again, first);
    assert!(offline.requests().is_empty());
}

/// Hits doi.org and Open Library for real. Run with
/// `cargo test -p textweaver-cite --test it -- lookup:: --ignored`.
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
