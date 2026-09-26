//! DOI and ISBN lookup.
//!
//! - **DOI**: doi.org content negotiation, asking for CSL-JSON
//!   (`Accept: application/vnd.citationstyles.csl+json`). This works for
//!   Crossref, DataCite, and mEDRA DOIs alike; Star asked the Crossref API
//!   only, so DataCite DOIs (datasets, many theses) failed.
//! - **ISBN**: Open Library's edition records (`/isbn/<isbn>.json`) and
//!   author records (no key needed). Star used Open Library's Books API
//!   (`/api/books?bibkeys=`), which answered 404 for every ISBN when this
//!   was written.
//!
//! Lookups are blocking (one request each; ADR-0001 allows async outside
//! speech, but nothing here needs parallel requests) and go through the
//! [`HttpClient`] trait, so tests use recorded responses and never touch
//! the network. [`UreqClient`] is the real client, with a timeout. Answers
//! are cached on disk ([`Cache`]) so looking up the same DOI again works
//! offline. Every failure is a [`LookupError`] whose message says that
//! nothing was added and what to try.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use crate::error::LookupError;
use crate::reference::{CslDate, Name, Reference, non_empty};

/// The default request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

const DOI_SERVICE: &str = "doi.org";
const ISBN_SERVICE: &str = "Open Library";
const CSL_JSON_ACCEPT: &str = "application/vnd.citationstyles.csl+json";

// ---- identifiers ------------------------------------------------------------

/// A DOI or ISBN, validated and normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Identifier {
    /// A DOI such as `10.1038/nature12373`.
    Doi(String),
    /// An ISBN (digits, with `X` as a possible ISBN-10 check digit).
    Isbn(String),
}

impl Identifier {
    /// Recognizes a DOI (bare, `doi:`, or a doi.org URL) or an ISBN (with
    /// or without hyphens and spaces, optionally prefixed `ISBN`).
    pub fn parse(input: &str) -> Result<Identifier, LookupError> {
        let t = input.trim();
        if let Some(doi) = normalize_doi(t) {
            return Ok(Identifier::Doi(doi));
        }
        let lower = t.to_ascii_lowercase();
        let looks_like_doi =
            lower.starts_with("10.") || lower.starts_with("doi") || lower.contains("doi.org");
        if looks_like_doi {
            return Err(LookupError::InvalidDoi {
                input: t.to_owned(),
            });
        }
        let stripped = lower
            .strip_prefix("isbn")
            .map(|s| s.trim_start_matches([':', ' ', '-']))
            .unwrap_or(&lower);
        let candidate: String = stripped
            .chars()
            .filter(|c| !matches!(c, '-' | ' '))
            .collect();
        if !candidate.is_empty() && candidate.chars().all(|c| c.is_ascii_digit() || c == 'x') {
            return match normalize_isbn(&candidate) {
                Some(isbn) => Ok(Identifier::Isbn(isbn)),
                None => Err(LookupError::InvalidIsbn {
                    input: t.to_owned(),
                }),
            };
        }
        Err(LookupError::UnknownIdentifier {
            input: t.to_owned(),
        })
    }

    /// "DOI 10.1/x" or "ISBN 9780306406157", for messages.
    pub fn describe(&self) -> String {
        match self {
            Identifier::Doi(d) => format!("DOI {d}"),
            Identifier::Isbn(i) => format!("ISBN {i}"),
        }
    }
}

/// Strips `https://doi.org/`, `http://dx.doi.org/`, `doi:` and surrounding
/// space; returns the DOI when it has the `10.<registrant>/<suffix>` shape.
pub fn normalize_doi(s: &str) -> Option<String> {
    let mut t = s.trim();
    let lower = t.to_ascii_lowercase();
    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi.org/",
        "doi:",
        "doi ",
    ] {
        if lower.starts_with(prefix) {
            t = t[prefix.len()..].trim();
            break;
        }
    }
    let (prefix, suffix) = t.split_once('/')?;
    let registrant = prefix.strip_prefix("10.")?;
    let valid = !registrant.is_empty()
        && registrant.chars().all(|c| c.is_ascii_digit() || c == '.')
        && !suffix.trim().is_empty()
        && !t.chars().any(char::is_whitespace);
    valid.then(|| t.to_owned())
}

/// Returns the ISBN's digits (uppercase `X` allowed as an ISBN-10 check
/// digit) when the checksum is valid; hyphens and spaces are ignored.
pub fn normalize_isbn(s: &str) -> Option<String> {
    let isbn: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    valid_isbn(&isbn).then_some(isbn)
}

/// Whether `s` is a checksum-valid ISBN-10 or ISBN-13 (Star's
/// `_valid_isbn`, kept: hyphens and spaces are ignored).
pub fn valid_isbn(s: &str) -> bool {
    let isbn: Vec<char> = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    match isbn.len() {
        10 => {
            let ok_shape = isbn[..9].iter().all(char::is_ascii_digit)
                && (isbn[9].is_ascii_digit() || isbn[9] == 'X');
            ok_shape && {
                let total: u32 = isbn
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (10 - i as u32) * c.to_digit(10).unwrap_or(10))
                    .sum();
                total.is_multiple_of(11)
            }
        }
        13 => {
            isbn.iter().all(char::is_ascii_digit) && {
                let total: u32 = isbn
                    .iter()
                    .enumerate()
                    .map(|(i, c)| c.to_digit(10).unwrap_or(0) * if i % 2 == 0 { 1 } else { 3 })
                    .sum();
                total.is_multiple_of(10)
            }
        }
        _ => false,
    }
}

/// The ISBN-13 form of a valid ISBN-10 or ISBN-13.
pub fn isbn_to_13(isbn: &str) -> Option<String> {
    let isbn = normalize_isbn(isbn)?;
    if isbn.len() == 13 {
        return Some(isbn);
    }
    let body = format!("978{}", &isbn[..9]);
    let total: u32 = body
        .chars()
        .enumerate()
        .map(|(i, c)| c.to_digit(10).unwrap_or(0) * if i % 2 == 0 { 1 } else { 3 })
        .sum();
    let check = (10 - total % 10) % 10;
    Some(format!("{body}{check}"))
}

/// The ISBN-10 form of a valid ISBN, when one exists (978 prefix only).
pub fn isbn_to_10(isbn: &str) -> Option<String> {
    let isbn = normalize_isbn(isbn)?;
    if isbn.len() == 10 {
        return Some(isbn);
    }
    let body = isbn.strip_prefix("978")?.get(..9)?.to_owned();
    let total: u32 = body
        .chars()
        .enumerate()
        .map(|(i, c)| (10 - i as u32) * c.to_digit(10).unwrap_or(0))
        .sum();
    let check = (11 - total % 11) % 11;
    let check = if check == 10 {
        'X'
    } else {
        char::from_digit(check, 10)?
    };
    Some(format!("{body}{check}"))
}

// ---- transport --------------------------------------------------------------

/// An HTTP response: status and body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Body as text.
    pub body: String,
}

/// Why a request produced no response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// The request timed out.
    Timeout,
    /// The host could not be reached (DNS, connection, TLS).
    Unreachable(String),
}

/// A blocking HTTP GET. Implemented by [`UreqClient`] and by test fakes.
pub trait HttpClient {
    /// Fetches `url` with the given `Accept` header. Redirects are followed.
    fn get(&self, url: &str, accept: &str) -> Result<HttpResponse, TransportError>;

    /// The timeout this client uses, for messages.
    fn timeout(&self) -> Duration {
        DEFAULT_TIMEOUT
    }
}

/// The real HTTP client (`ureq` with rustls).
pub struct UreqClient {
    agent: ureq::Agent,
    timeout: Duration,
}

impl UreqClient {
    /// A client with the given overall timeout per request.
    pub fn new(timeout: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .user_agent(concat!(
                "textweaver/",
                env!("CARGO_PKG_VERSION"),
                " (reference lookup; https://github.com/leavesofgrass/textweaver)"
            ))
            .build();
        UreqClient {
            agent: config.into(),
            timeout,
        }
    }
}

impl Default for UreqClient {
    fn default() -> Self {
        UreqClient::new(DEFAULT_TIMEOUT)
    }
}

impl HttpClient for UreqClient {
    fn get(&self, url: &str, accept: &str) -> Result<HttpResponse, TransportError> {
        let result = self.agent.get(url).header("Accept", accept).call();
        match result {
            Ok(mut resp) => {
                let status = resp.status().as_u16();
                let body = resp
                    .body_mut()
                    .with_config()
                    .limit(4 * 1024 * 1024)
                    .read_to_string()
                    .map_err(|e| classify(&e))?;
                Ok(HttpResponse { status, body })
            }
            Err(e) => Err(classify(&e)),
        }
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}

fn classify(e: &ureq::Error) -> TransportError {
    match e {
        ureq::Error::Timeout(_) => TransportError::Timeout,
        other => TransportError::Unreachable(other.to_string()),
    }
}

// ---- test client --------------------------------------------------------------

/// An [`HttpClient`] that answers from recorded responses, for tests and
/// offline demonstrations. Unknown URLs fail as if the network were down,
/// so offline behavior is testable too.
#[derive(Clone, Debug, Default)]
pub struct RecordedClient {
    routes: Vec<(String, HttpResponse)>,
    requests: std::cell::RefCell<Vec<String>>,
}

impl RecordedClient {
    /// A client that knows no URLs (always "offline").
    pub fn new() -> Self {
        RecordedClient::default()
    }

    /// Answers `url` with `status` and `body`.
    pub fn with(mut self, url: &str, status: u16, body: &str) -> Self {
        self.routes.push((
            url.to_owned(),
            HttpResponse {
                status,
                body: body.to_owned(),
            },
        ));
        self
    }

    /// The URLs requested so far, in order.
    pub fn requests(&self) -> Vec<String> {
        self.requests.borrow().clone()
    }
}

impl HttpClient for RecordedClient {
    fn get(&self, url: &str, _accept: &str) -> Result<HttpResponse, TransportError> {
        self.requests.borrow_mut().push(url.to_owned());
        self.routes
            .iter()
            .find(|(u, _)| u == url)
            .map(|(_, r)| r.clone())
            .ok_or_else(|| TransportError::Unreachable(format!("no recorded response for {url}")))
    }
}

// ---- cache ------------------------------------------------------------------

/// A small on-disk cache of looked-up references (as CSL-JSON), one file
/// per identifier, oldest files removed past [`Cache::max_entries`].
#[derive(Clone, Debug)]
pub struct Cache {
    dir: PathBuf,
    /// Most files kept.
    pub max_entries: usize,
}

impl Cache {
    /// A cache in `dir` (created on first write), keeping 500 references.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Cache {
            dir: dir.into(),
            max_entries: 500,
        }
    }

    /// The cache folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn file(&self, id: &Identifier) -> PathBuf {
        let (kind, value) = match id {
            Identifier::Doi(d) => ("doi", d.to_lowercase()),
            Identifier::Isbn(i) => ("isbn", isbn_to_13(i).unwrap_or_else(|| i.clone())),
        };
        let readable: String = value
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .take(60)
            .collect();
        self.dir.join(format!(
            "{kind}-{readable}-{:016x}.json",
            fnv1a(value.as_bytes())
        ))
    }

    /// The cached reference for an identifier.
    pub fn get(&self, id: &Identifier) -> Option<Reference> {
        let text = std::fs::read_to_string(self.file(id)).ok()?;
        match serde_json::from_str::<Reference>(&text) {
            Ok(r) => Some(r),
            Err(e) => {
                log::warn!("citation lookup cache: ignoring an unreadable entry: {e}");
                None
            }
        }
    }

    /// Stores a reference. Failures only log: a cache must never stop a
    /// lookup that succeeded.
    pub fn put(&self, id: &Identifier, r: &Reference) {
        let written = serde_json::to_string_pretty(r)
            .map_err(std::io::Error::other)
            .and_then(|text| {
                std::fs::create_dir_all(&self.dir)?;
                std::fs::write(self.file(id), text)
            });
        if let Err(e) = written {
            log::warn!(
                "citation lookup cache: could not write to {}: {e}",
                self.dir.display()
            );
            return;
        }
        self.prune();
    }

    fn prune(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        if files.len() <= self.max_entries {
            return;
        }
        files.sort();
        let excess = files.len() - self.max_entries;
        for (_, p) in files.into_iter().take(excess) {
            let _ = std::fs::remove_file(p);
        }
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

// ---- lookup -----------------------------------------------------------------

/// Open Library's base URL.
pub const OPEN_LIBRARY: &str = "https://openlibrary.org";

/// Looks up references by DOI or ISBN.
pub struct Lookup<'a> {
    client: &'a dyn HttpClient,
    cache: Option<Cache>,
}

impl<'a> Lookup<'a> {
    /// A lookup without a cache.
    pub fn new(client: &'a dyn HttpClient) -> Self {
        Lookup {
            client,
            cache: None,
        }
    }

    /// Uses an on-disk cache.
    pub fn with_cache(mut self, cache: Cache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Looks up a DOI or ISBN given as typed.
    pub fn lookup(&self, input: &str) -> Result<Reference, LookupError> {
        self.identifier(&Identifier::parse(input)?)
    }

    /// Looks up an identifier, from the cache when it has the answer. The
    /// reference's key is left empty for the library to assign.
    pub fn identifier(&self, id: &Identifier) -> Result<Reference, LookupError> {
        if let Some(r) = self.cache.as_ref().and_then(|c| c.get(id)) {
            return Ok(r);
        }
        let r = match id {
            Identifier::Doi(d) => self.doi(d)?,
            Identifier::Isbn(i) => self.isbn(i)?,
        };
        if let Some(cache) = &self.cache {
            cache.put(id, &r);
        }
        Ok(r)
    }

    fn fetch(
        &self,
        service: &'static str,
        url: &str,
        accept: &str,
    ) -> Result<HttpResponse, LookupError> {
        self.client.get(url, accept).map_err(|e| match e {
            TransportError::Timeout => LookupError::Timeout {
                service,
                seconds: self.client.timeout().as_secs(),
            },
            TransportError::Unreachable(detail) => LookupError::Offline { service, detail },
        })
    }

    fn doi(&self, doi: &str) -> Result<Reference, LookupError> {
        let url = format!("https://doi.org/{}", encode_doi(doi));
        let resp = self.fetch(DOI_SERVICE, &url, CSL_JSON_ACCEPT)?;
        match resp.status {
            200..=299 => reference_from_doi_csl(doi, &resp.body),
            404 | 410 => Err(LookupError::NotFound {
                service: DOI_SERVICE,
                what: format!("DOI {doi}"),
            }),
            status => Err(LookupError::Http {
                service: DOI_SERVICE,
                status,
            }),
        }
    }

    /// The edition record (`/isbn/<isbn>.json`, trying the other ISBN form
    /// on a miss), then author names from the edition's author records, or
    /// the work's when the edition lists none.
    fn isbn(&self, isbn: &str) -> Result<Reference, LookupError> {
        let mut candidates = vec![isbn.to_owned()];
        for alt in [isbn_to_13(isbn), isbn_to_10(isbn)].into_iter().flatten() {
            if !candidates.contains(&alt) {
                candidates.push(alt);
            }
        }
        let mut edition = None;
        for c in &candidates {
            let resp = self.fetch(
                ISBN_SERVICE,
                &format!("{OPEN_LIBRARY}/isbn/{c}.json"),
                "application/json",
            )?;
            match resp.status {
                200..=299 => {
                    edition = Some(resp.body);
                    break;
                }
                404 | 410 => continue,
                status => {
                    return Err(LookupError::Http {
                        service: ISBN_SERVICE,
                        status,
                    });
                }
            }
        }
        let Some(edition) = edition else {
            return Err(LookupError::NotFound {
                service: ISBN_SERVICE,
                what: format!("ISBN {isbn}"),
            });
        };
        let edition: Value =
            serde_json::from_str(&edition).map_err(|e| LookupError::BadResponse {
                service: ISBN_SERVICE,
                detail: format!("not JSON ({e})"),
            })?;
        let mut author_keys = keys_at(&edition, "authors", &["key"]);
        if author_keys.is_empty()
            && let Some(work) = keys_at(&edition, "works", &["key"]).first()
        {
            let resp = self.fetch(
                ISBN_SERVICE,
                &format!("{OPEN_LIBRARY}{work}.json"),
                "application/json",
            )?;
            if (200..300).contains(&resp.status)
                && let Ok(w) = serde_json::from_str::<Value>(&resp.body)
            {
                author_keys = keys_at(&w, "authors", &["author", "key"]);
            }
        }
        let mut authors = Vec::new();
        for key in author_keys.iter().take(10) {
            let resp = self.fetch(
                ISBN_SERVICE,
                &format!("{OPEN_LIBRARY}{key}.json"),
                "application/json",
            )?;
            if (200..300).contains(&resp.status)
                && let Ok(a) = serde_json::from_str::<Value>(&resp.body)
                && let Some(name) = a.get("name").and_then(Value::as_str).and_then(non_empty)
            {
                authors.push(name);
            }
        }
        reference_from_open_library(isbn, &edition, &authors)
    }
}

/// Keys such as `/authors/OL34184A` found at `field[].path...`.
fn keys_at(v: &Value, field: &str, path: &[&str]) -> Vec<String> {
    v.get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let mut cur = item;
                    for p in path {
                        cur = cur.get(p)?;
                    }
                    cur.as_str()
                        .filter(|s| s.starts_with('/'))
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Percent-encodes a DOI for a URL path, keeping `/` (DOI suffixes may
/// contain `#`, `?`, `<`, and `>`, which would otherwise end or change the
/// path).
fn encode_doi(doi: &str) -> String {
    let mut out = String::with_capacity(doi.len());
    for b in doi.bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'.' | b'_' | b'~' | b'/' | b'(' | b')' | b':' | b';'
            )
        {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Crossref's content negotiation labels items with Crossref's own types
/// (`journal-article`, `book-chapter`) rather than CSL types; this maps
/// them. CSL types pass through.
fn csl_type_for(t: &str) -> &str {
    match t {
        "journal-article" => "article-journal",
        "book-chapter" | "book-section" | "book-part" => "chapter",
        "proceedings-article" => "paper-conference",
        "posted-content" => "article",
        "dissertation" => "thesis",
        "report" | "report-component" | "report-series" => "report",
        "book" | "monograph" | "edited-book" | "reference-book" | "book-set" | "book-track"
        | "book-series" => "book",
        "reference-entry" => "entry-encyclopedia",
        "dataset" | "database" => "dataset",
        "peer-review" => "review",
        "standard" | "standard-series" => "standard",
        "journal-issue" | "journal-volume" | "journal" | "proceedings" | "proceedings-series" => {
            "periodical"
        }
        "component" | "other" | "grant" => "document",
        other => other,
    }
}

/// Crossref and DataCite fields that are bulky and never cited.
const DROPPED_FIELDS: &[&str] = &[
    "reference",
    "references-count",
    "reference-count",
    "is-referenced-by-count",
    "link",
    "license",
    "assertion",
    "funder",
    "relation",
    "content-domain",
    "deposited",
    "indexed",
    "created",
    "update-policy",
    "score",
    "source",
    "prefix",
    "member",
    "published",
    "published-print",
    "published-online",
    "published-other",
    "journal-issue",
    "subject",
    "alternative-id",
    "short-container-title",
    "original-title",
    "short-title",
    "subtitle",
    "resource",
    "institution",
    "update-to",
    "review",
    "categories",
    "copyright",
    "id",
];

/// Turns doi.org's CSL-JSON answer into a clean reference: Crossref types
/// mapped to CSL types, markup removed from titles (it would be spelled out
/// aloud), bulky Crossref metadata dropped, a URL that only repeats the DOI
/// dropped, the DOI normalized, and the key left for the library.
pub fn reference_from_doi_csl(doi: &str, body: &str) -> Result<Reference, LookupError> {
    let bad = |detail: String| LookupError::BadResponse {
        service: DOI_SERVICE,
        detail,
    };
    let mut value: Value =
        serde_json::from_str(body).map_err(|e| bad(format!("not JSON ({e})")))?;
    let Value::Object(map) = &mut value else {
        return Err(bad("expected one reference".to_owned()));
    };
    if map.get("container-title-short").is_none()
        && let Some(s) = map.get("short-container-title").cloned()
    {
        map.insert("container-title-short".into(), s);
    }
    let subtitle = map.get("subtitle").cloned();
    for f in DROPPED_FIELDS {
        map.remove(*f);
    }
    let mut r: Reference = serde_json::from_value(value).map_err(|e| bad(e.to_string()))?;
    r.id = String::new();
    r.kind = csl_type_for(&r.kind).to_owned();
    // Crossref's per-name `sequence`, `affiliation`, and `ORCID` are not
    // citation data.
    for n in r
        .author
        .iter_mut()
        .chain(&mut r.editor)
        .chain(&mut r.translator)
    {
        n.extra.clear();
    }
    r.doi = Some(normalize_doi(doi).unwrap_or_else(|| doi.to_owned()));
    if r.url.as_deref().and_then(normalize_doi).is_some() {
        r.url = None;
    }
    for t in [
        &mut r.title,
        &mut r.container_title,
        &mut r.container_title_short,
        &mut r.collection_title,
        &mut r.abstract_text,
    ] {
        if let Some(s) = t.as_mut() {
            *s = crate::text::plain_title(s);
        }
    }
    if r.container_title_short == r.container_title {
        r.container_title_short = None;
    }
    let sub = subtitle.and_then(|v| match v {
        Value::String(s) => non_empty(&s),
        Value::Array(a) => a.into_iter().find_map(|x| x.as_str().and_then(non_empty)),
        _ => None,
    });
    if let (Some(title), Some(sub)) = (r.title.as_mut(), sub) {
        let sub = crate::text::plain_title(&sub);
        if !title.to_lowercase().contains(&sub.to_lowercase()) {
            title.push_str(": ");
            title.push_str(&sub);
        }
    }
    if let Some(p) = &r.page
        && let Some((a, b)) = p.split_once('-')
        && a == b
    {
        r.page = Some(a.to_owned());
    }
    if r.title.is_none() {
        return Err(bad("the record has no title".to_owned()));
    }
    Ok(r)
}

/// Turns an Open Library edition record (and the author names found for
/// it) into a book reference.
pub fn reference_from_open_library(
    isbn: &str,
    edition: &Value,
    authors: &[String],
) -> Result<Reference, LookupError> {
    let text = |k: &str| edition.get(k).and_then(Value::as_str).and_then(non_empty);
    let list = |k: &str| -> Vec<String> {
        edition
            .get(k)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| {
                        x.as_str()
                            .or_else(|| x.get("name").and_then(Value::as_str))
                            .and_then(non_empty)
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut r = Reference::new("", "book");
    r.title = text("title").map(|t| match text("subtitle") {
        Some(sub) => format!("{t}: {sub}"),
        None => t,
    });
    if r.title.is_none() {
        return Err(LookupError::BadResponse {
            service: ISBN_SERVICE,
            detail: "the record has no title".to_owned(),
        });
    }
    r.author = authors.iter().map(|n| Name::parse(n)).collect();
    r.issued = text("publish_date").and_then(|d| parse_loose_date(&d));
    let publishers = list("publishers");
    r.publisher = (!publishers.is_empty()).then(|| publishers.join(", "));
    r.publisher_place = list("publish_places").into_iter().next();
    r.edition = text("edition_name");
    r.isbn = Some(isbn_to_13(isbn).unwrap_or_else(|| isbn.to_owned()));
    // The Open Library page is kept for reference but not as the URL:
    // styles print a book's URL, and a catalog page is not the book.
    if let Some(key) = text("key") {
        r.extra.insert(
            "openlibrary".into(),
            Value::String(format!("{OPEN_LIBRARY}{key}")),
        );
    }
    if let Some(n) = edition.get("number_of_pages").and_then(Value::as_u64) {
        r.extra
            .insert("number-of-pages".into(), Value::String(n.to_string()));
    }
    Ok(r)
}

/// Dates as Open Library writes them: "2008", "May 2008", "May 5, 2008",
/// "October 1, 1988", "5 May 2008", "2008-05-05".
fn parse_loose_date(s: &str) -> Option<CslDate> {
    if let Some(d) = CslDate::parse(s).filter(|d| !d.date_parts.is_empty()) {
        return Some(d);
    }
    let year = crate::reference::find_year(s)?;
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let lower = s.to_lowercase();
    let month = MONTHS
        .iter()
        .position(|m| {
            lower
                .split(|c: char| !c.is_alphabetic())
                .any(|w| w.starts_with(m))
        })
        .and_then(|i| u8::try_from(i + 1).ok());
    let day = month.and_then(|_| {
        lower
            .split(|c: char| !c.is_ascii_digit())
            .filter(|w| !w.is_empty() && w.len() <= 2)
            .find_map(|w| w.parse::<u8>().ok())
            .filter(|d| (1..=31).contains(d))
    });
    Some(CslDate::ymd(year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_isbn_vectors() {
        // Star's tests/test_citations.py::test_valid_isbn*.
        assert!(valid_isbn("0306406152"));
        assert!(valid_isbn("080442957X"));
        assert!(valid_isbn("9780306406157"));
        assert!(valid_isbn("978-0-306-40615-7"));
        assert!(valid_isbn(" 0 306 406152 "));
        assert!(!valid_isbn("0306406153"));
        assert!(!valid_isbn("9780306406158"));
        assert!(!valid_isbn("not-an-isbn"));
        assert!(!valid_isbn("12345"));
    }

    #[test]
    fn isbn_conversions() {
        assert_eq!(isbn_to_13("0306406152").as_deref(), Some("9780306406157"));
        assert_eq!(isbn_to_10("9780306406157").as_deref(), Some("0306406152"));
        assert_eq!(
            isbn_to_10("978-0-8044-2957-3").as_deref(),
            Some("080442957X")
        );
        assert_eq!(isbn_to_10("9791234567896"), None);
    }

    #[test]
    fn identifiers_are_recognized() {
        assert_eq!(
            Identifier::parse("https://doi.org/10.1038/nature12373"),
            Ok(Identifier::Doi("10.1038/nature12373".into()))
        );
        assert_eq!(
            Identifier::parse("doi:10.1000/xyz"),
            Ok(Identifier::Doi("10.1000/xyz".into()))
        );
        assert_eq!(
            Identifier::parse("ISBN 978-0-306-40615-7"),
            Ok(Identifier::Isbn("9780306406157".into()))
        );
        assert!(matches!(
            Identifier::parse("10.1038"),
            Err(LookupError::InvalidDoi { .. })
        ));
        assert!(matches!(
            Identifier::parse("9780306406158"),
            Err(LookupError::InvalidIsbn { .. })
        ));
        assert!(matches!(
            Identifier::parse("hello"),
            Err(LookupError::UnknownIdentifier { .. })
        ));
    }

    #[test]
    fn dois_are_encoded_for_urls() {
        assert_eq!(
            encode_doi("10.1002/(SICI)1097-4571"),
            "10.1002/(SICI)1097-4571"
        );
        assert_eq!(encode_doi("10.1000/a#b?c<d>"), "10.1000/a%23b%3Fc%3Cd%3E");
    }

    #[test]
    fn loose_dates() {
        let d = parse_loose_date("May 5, 2008").unwrap();
        assert_eq!(d.iso().as_deref(), Some("2008-05-05"));
        assert_eq!(
            parse_loose_date("1998").and_then(|d| d.iso()).as_deref(),
            Some("1998")
        );
        assert_eq!(
            parse_loose_date("c1998").and_then(|d| d.iso()).as_deref(),
            Some("1998")
        );
    }
}
