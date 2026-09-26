//! The reference model: one CSL-JSON item.
//!
//! CSL-JSON is the library's storage format (ADR-0019), so [`Reference`]
//! mirrors it: the common variables are typed fields, and every other
//! variable is kept verbatim in [`Reference::extra`], so a library written
//! by another tool (Zotero, Pandoc, doi.org) survives a load and save
//! unchanged. Deserialization is forgiving where real-world CSL-JSON is
//! loose: numbers may be strings, strings may be numbers, date parts may be
//! strings, and a title may arrive as a one-element array.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

/// One reference (a CSL-JSON item).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// The citation key, as used in `[@key]`.
    #[serde(default, deserialize_with = "de_id")]
    pub id: String,
    /// The CSL item type (`article-journal`, `book`, `chapter`, ...).
    #[serde(
        rename = "type",
        default = "default_type",
        deserialize_with = "de_type"
    )]
    pub kind: String,
    /// Title.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub title: Option<String>,
    /// Short title.
    #[serde(
        rename = "title-short",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub title_short: Option<String>,
    /// Authors.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "de_names"
    )]
    pub author: Vec<Name>,
    /// Editors.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "de_names"
    )]
    pub editor: Vec<Name>,
    /// Translators.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "de_names"
    )]
    pub translator: Vec<Name>,
    /// Publication date.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_date"
    )]
    pub issued: Option<CslDate>,
    /// Date the item was accessed (web pages).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_date"
    )]
    pub accessed: Option<CslDate>,
    /// Journal, book, or website the item appears in.
    #[serde(
        rename = "container-title",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub container_title: Option<String>,
    /// Short form of the container title (journal abbreviation).
    #[serde(
        rename = "container-title-short",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub container_title_short: Option<String>,
    /// Series title.
    #[serde(
        rename = "collection-title",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub collection_title: Option<String>,
    /// Publisher (or, for a thesis, the university).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub publisher: Option<String>,
    /// Place of publication.
    #[serde(
        rename = "publisher-place",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub publisher_place: Option<String>,
    /// Volume.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub volume: Option<String>,
    /// Issue.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub issue: Option<String>,
    /// Page range (`12-15`) or page.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub page: Option<String>,
    /// Edition.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub edition: Option<String>,
    /// Report or other number.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub number: Option<String>,
    /// DOI, without a `https://doi.org/` prefix.
    #[serde(
        rename = "DOI",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub doi: Option<String>,
    /// ISBN.
    #[serde(
        rename = "ISBN",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub isbn: Option<String>,
    /// ISSN.
    #[serde(
        rename = "ISSN",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub issn: Option<String>,
    /// URL.
    #[serde(
        rename = "URL",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub url: Option<String>,
    /// Abstract.
    #[serde(
        rename = "abstract",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub abstract_text: Option<String>,
    /// Language (`en`, `en-US`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub language: Option<String>,
    /// Note.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub note: Option<String>,
    /// Genre ("Doctoral dissertation", "Technical report").
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub genre: Option<String>,
    /// Keywords.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub keyword: Option<String>,
    /// Every other CSL variable, kept as it was read.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A personal or organizational name (a CSL name variable entry).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name {
    /// Family name.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub family: Option<String>,
    /// Given names.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub given: Option<String>,
    /// A name that is not split (an organization, a mononym).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub literal: Option<String>,
    /// Particle kept with the family name when sorting ("van" in "van Gogh").
    #[serde(
        rename = "non-dropping-particle",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub non_dropping_particle: Option<String>,
    /// Particle dropped when sorting ("de" in some French names).
    #[serde(
        rename = "dropping-particle",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub dropping_particle: Option<String>,
    /// Suffix ("Jr.", "III").
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub suffix: Option<String>,
    /// Other name fields (`comma-suffix`, `static-ordering`, ...), kept as
    /// read.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A CSL date: `date-parts`, or a literal or raw string.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CslDate {
    /// `[[year, month, day]]`, or two such arrays for a range.
    #[serde(
        rename = "date-parts",
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "de_date_parts"
    )]
    pub date_parts: Vec<Vec<i32>>,
    /// A date written out ("Spring 2020").
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub literal: Option<String>,
    /// An unparsed date string.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string"
    )]
    pub raw: Option<String>,
    /// Whether the date is approximate.
    #[serde(
        default,
        skip_serializing_if = "std::ops::Not::not",
        deserialize_with = "de_bool"
    )]
    pub circa: bool,
    /// Other date fields (`season`), kept as read.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_type() -> String {
    "document".to_owned()
}

impl Name {
    /// A name with family and given parts.
    pub fn new(family: &str, given: &str) -> Self {
        Name {
            family: non_empty(family),
            given: non_empty(given),
            ..Name::default()
        }
    }

    /// A name whose family part may start with lowercase particles
    /// ("van Gogh"), which become the non-dropping particle.
    fn with_particle(family: &str, given: &str) -> Self {
        let words: Vec<&str> = family.split_whitespace().collect();
        let n = words
            .iter()
            .take_while(|w| w.chars().next().is_some_and(char::is_lowercase))
            .count();
        if n == 0 || n == words.len() {
            return Name::new(family, given);
        }
        Name {
            non_dropping_particle: non_empty(&words[..n].join(" ")),
            ..Name::new(&words[n..].join(" "), given)
        }
    }

    /// An unsplit name (an organization).
    pub fn literal(name: &str) -> Self {
        Name {
            literal: non_empty(name),
            ..Name::default()
        }
    }

    /// Parses `"Family, Given"`, `"Family, Suffix, Given"`, or `"Given Family"`.
    /// A single word becomes a family name; `{Braced Name}` becomes a literal.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if let Some(inner) = s.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
            return Name::literal(inner.trim());
        }
        let parts: Vec<&str> = s.split(',').map(str::trim).collect();
        match parts.as_slice() {
            [family, suffix, given] => Name {
                suffix: non_empty(suffix),
                ..Name::with_particle(family, given)
            },
            [family, given] => Name::with_particle(family, given),
            _ => {
                let words: Vec<&str> = s.split_whitespace().collect();
                match words.split_last() {
                    Some((family, given)) if !given.is_empty() => {
                        // Lowercase particles before the family name: "Vincent van Gogh".
                        let first_particle = given
                            .iter()
                            .position(|w| w.chars().next().is_some_and(char::is_lowercase))
                            .unwrap_or(given.len());
                        let particle = given[first_particle..].join(" ");
                        let given = given[..first_particle].join(" ");
                        Name {
                            non_dropping_particle: non_empty(&particle),
                            ..Name::new(family, &given)
                        }
                    }
                    Some((family, _)) => Name::new(family, ""),
                    None => Name::default(),
                }
            }
        }
    }

    /// The family name with its particle ("van Gogh"), or the literal name.
    pub fn family_display(&self) -> String {
        if let Some(lit) = &self.literal {
            return lit.clone();
        }
        let mut out = String::new();
        if let Some(p) = &self.non_dropping_particle {
            out.push_str(p);
            out.push(' ');
        }
        out.push_str(self.family.as_deref().unwrap_or(""));
        out.trim().to_owned()
    }

    /// `"Family, Given"` (with particle and suffix), or the literal name.
    pub fn sort_form(&self) -> String {
        if let Some(lit) = &self.literal {
            return lit.clone();
        }
        let mut out = self.family_display();
        if let Some(s) = &self.suffix {
            out.push_str(", ");
            out.push_str(s);
        }
        let mut given = String::new();
        if let Some(g) = &self.given {
            given.push_str(g);
        }
        if let Some(d) = &self.dropping_particle {
            if !given.is_empty() {
                given.push(' ');
            }
            given.push_str(d);
        }
        if !given.is_empty() {
            out.push_str(", ");
            out.push_str(&given);
        }
        out
    }

    /// `"Given Family"`, as a person would say it.
    pub fn natural_form(&self) -> String {
        if let Some(lit) = &self.literal {
            return lit.clone();
        }
        let mut words: Vec<&str> = Vec::new();
        if let Some(g) = &self.given {
            words.push(g);
        }
        if let Some(d) = &self.dropping_particle {
            words.push(d);
        }
        let family = self.family_display();
        if !family.is_empty() {
            words.push(&family);
        }
        let mut out = words.join(" ");
        if let Some(s) = &self.suffix {
            out.push_str(", ");
            out.push_str(s);
        }
        out
    }

    /// Whether the name has no content.
    pub fn is_empty(&self) -> bool {
        self.family.is_none() && self.given.is_none() && self.literal.is_none()
    }
}

impl CslDate {
    /// A date from a year and optional month and day.
    pub fn ymd(year: i32, month: Option<u8>, day: Option<u8>) -> Self {
        let mut parts = vec![year];
        if let Some(m) = month {
            parts.push(i32::from(m));
            if let Some(d) = day {
                parts.push(i32::from(d));
            }
        }
        CslDate {
            date_parts: vec![parts],
            ..CslDate::default()
        }
    }

    /// A date from a year.
    pub fn year(year: i32) -> Self {
        CslDate::ymd(year, None, None)
    }

    /// The first year: from `date-parts`, or the first four-digit number in
    /// the literal or raw form.
    pub fn first_year(&self) -> Option<i32> {
        if let Some(y) = self.date_parts.first().and_then(|p| p.first()) {
            return Some(*y);
        }
        [&self.raw, &self.literal]
            .into_iter()
            .flatten()
            .find_map(|s| find_year(s))
    }

    /// Month (1-12) of the start date, if known.
    pub fn month(&self) -> Option<u8> {
        self.date_parts
            .first()
            .and_then(|p| p.get(1))
            .and_then(|m| u8::try_from(*m).ok())
            .filter(|m| (1..=12).contains(m))
    }

    /// Day (1-31) of the start date, if known.
    pub fn day(&self) -> Option<u8> {
        self.date_parts
            .first()
            .and_then(|p| p.get(2))
            .and_then(|d| u8::try_from(*d).ok())
            .filter(|d| (1..=31).contains(d))
    }

    /// Parses `2020`, `2020-05`, `2020-05-03`, or `2020/05/03` (RIS style,
    /// with an optional trailing `/`); anything else becomes a `raw` date.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_end_matches('/');
        if s.is_empty() {
            return None;
        }
        let parts: Vec<&str> = s.split(['-', '/']).collect();
        let nums: Option<Vec<i32>> = parts
            .iter()
            .filter(|p| !p.is_empty())
            .map(|p| p.trim().parse::<i32>().ok())
            .collect();
        match nums {
            Some(n) if !n.is_empty() && n.len() <= 3 && parts[0].len() == 4 => {
                let month = n
                    .get(1)
                    .and_then(|m| u8::try_from(*m).ok())
                    .filter(|m| (1..=12).contains(m));
                let day = n
                    .get(2)
                    .and_then(|d| u8::try_from(*d).ok())
                    .filter(|d| (1..=31).contains(d));
                Some(CslDate::ymd(n[0], month, month.and(day)))
            }
            _ => Some(CslDate {
                raw: Some(s.to_owned()),
                ..CslDate::default()
            }),
        }
    }

    /// ISO form of the start date: `2020`, `2020-05`, or `2020-05-03`.
    pub fn iso(&self) -> Option<String> {
        let y = self.first_year()?;
        let mut out = if y < 0 {
            format!("-{:04}", -y)
        } else {
            format!("{y:04}")
        };
        if let Some(m) = self.month() {
            out.push_str(&format!("-{m:02}"));
            if let Some(d) = self.day() {
                out.push_str(&format!("-{d:02}"));
            }
        }
        Some(out)
    }
}

/// The first standalone four-digit year in a string.
pub(crate) fn find_year(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let window = &bytes[i..i + 4];
        let before_ok = i == 0 || !bytes[i - 1].is_ascii_digit();
        let after_ok = i + 4 == bytes.len() || !bytes[i + 4].is_ascii_digit();
        if before_ok && after_ok && window.iter().all(u8::is_ascii_digit) {
            return s[i..i + 4].parse().ok();
        }
        i += 1;
    }
    None
}

impl Reference {
    /// An empty reference of the given CSL type.
    pub fn new(id: &str, kind: &str) -> Self {
        Reference {
            id: id.to_owned(),
            kind: kind.to_owned(),
            ..Reference::default()
        }
    }

    /// The publication year, if known.
    pub fn year(&self) -> Option<i32> {
        self.issued.as_ref().and_then(CslDate::first_year)
    }

    /// Authors, or editors when there are no authors.
    pub fn creators(&self) -> &[Name] {
        if self.author.is_empty() {
            &self.editor
        } else {
            &self.author
        }
    }

    /// A short spoken form of the creators: "Doe", "Doe and Roe", or
    /// "Doe and others" (three or more).
    pub fn creators_short(&self) -> Option<String> {
        let names = self.creators();
        match names {
            [] => None,
            [a] => Some(a.family_display()),
            [a, b] => Some(format!("{} and {}", a.family_display(), b.family_display())),
            [a, ..] => Some(format!("{} and others", a.family_display())),
        }
    }

    /// A one-line label for pickers and lists, readable aloud:
    /// `Doe and Roe, 2020. On X. Key doe2020.`
    ///
    /// Star's label was `[Doe2020] Doe  (2020)  On X`: brackets and a
    /// double space that screen readers either spell out or swallow.
    pub fn label(&self) -> String {
        let mut out = String::new();
        let head = match (self.creators_short(), self.year()) {
            (Some(c), Some(y)) => format!("{c}, {y}"),
            (Some(c), None) => format!("{c}, no date"),
            (None, Some(y)) => y.to_string(),
            (None, None) => String::new(),
        };
        if !head.is_empty() {
            out.push_str(&head);
            out.push_str(". ");
        }
        let title = self
            .title
            .as_deref()
            .map(crate::text::plain_title)
            .unwrap_or_default();
        if title.is_empty() {
            out.push_str("Untitled. ");
        } else {
            out.push_str(&title);
            if !title.ends_with(['.', '?', '!']) {
                out.push('.');
            }
            out.push(' ');
        }
        if !self.id.is_empty() {
            out.push_str("Key ");
            out.push_str(&self.id);
            out.push('.');
        }
        out.trim_end().to_owned()
    }

    /// The DOI in its canonical lowercase form, for duplicate checks.
    pub fn doi_key(&self) -> Option<String> {
        self.doi
            .as_deref()
            .and_then(crate::lookup::normalize_doi)
            .map(|d| d.to_lowercase())
    }

    /// The ISBN as ISBN-13 digits, for duplicate checks.
    pub fn isbn_key(&self) -> Option<String> {
        let first = self.isbn.as_deref()?.split([',', ';']).next()?;
        crate::lookup::normalize_isbn(first).and_then(|i| crate::lookup::isbn_to_13(&i))
    }

    /// Fills every empty field of `self` from `other` and replaces fields
    /// `other` has. The key and unknown fields of `self` are kept unless
    /// `other` sets them.
    pub fn update_from(&mut self, other: Reference) {
        let Reference {
            id: _,
            kind,
            title,
            title_short,
            author,
            editor,
            translator,
            issued,
            accessed,
            container_title,
            container_title_short,
            collection_title,
            publisher,
            publisher_place,
            volume,
            issue,
            page,
            edition,
            number,
            doi,
            isbn,
            issn,
            url,
            abstract_text,
            language,
            note,
            genre,
            keyword,
            extra,
        } = other;
        if !kind.is_empty() && kind != "document" {
            self.kind = kind;
        }
        fn take<T>(dst: &mut Option<T>, src: Option<T>) {
            if src.is_some() {
                *dst = src;
            }
        }
        fn take_vec<T>(dst: &mut Vec<T>, src: Vec<T>) {
            if !src.is_empty() {
                *dst = src;
            }
        }
        take(&mut self.title, title);
        take(&mut self.title_short, title_short);
        take_vec(&mut self.author, author);
        take_vec(&mut self.editor, editor);
        take_vec(&mut self.translator, translator);
        take(&mut self.issued, issued);
        take(&mut self.accessed, accessed);
        take(&mut self.container_title, container_title);
        take(&mut self.container_title_short, container_title_short);
        take(&mut self.collection_title, collection_title);
        take(&mut self.publisher, publisher);
        take(&mut self.publisher_place, publisher_place);
        take(&mut self.volume, volume);
        take(&mut self.issue, issue);
        take(&mut self.page, page);
        take(&mut self.edition, edition);
        take(&mut self.number, number);
        take(&mut self.doi, doi);
        take(&mut self.isbn, isbn);
        take(&mut self.issn, issn);
        take(&mut self.url, url);
        take(&mut self.abstract_text, abstract_text);
        take(&mut self.language, language);
        take(&mut self.note, note);
        take(&mut self.genre, genre);
        take(&mut self.keyword, keyword);
        for (k, v) in extra {
            self.extra.insert(k, v);
        }
    }
}

pub(crate) fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

// ---- forgiving deserializers ------------------------------------------------

/// A JSON value as a string: strings as is, numbers printed, arrays joined
/// with ", " (Crossref sends `"ISBN": ["…", "…"]` and sometimes one-element
/// title arrays), anything else absent.
fn value_to_string(v: Value) -> Option<String> {
    match v {
        Value::String(s) => non_empty(&s),
        Value::Number(n) => Some(n.to_string()),
        Value::Array(items) => {
            let parts: Vec<String> = items.into_iter().filter_map(value_to_string).collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
        }
        _ => None,
    }
}

fn de_opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(value_to_string(Value::deserialize(d)?))
}

fn de_id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(value_to_string(Value::deserialize(d)?).unwrap_or_default())
}

fn de_type<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(value_to_string(Value::deserialize(d)?).unwrap_or_else(default_type))
}

fn de_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Bool(b) => b,
        Value::Number(n) => n.as_i64().is_some_and(|n| n != 0),
        Value::String(s) => !s.is_empty() && s != "0" && s != "false",
        _ => false,
    })
}

fn de_names<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Name>, D::Error> {
    let v = Value::deserialize(d)?;
    let items = match v {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        other => vec![other],
    };
    Ok(items
        .into_iter()
        .filter_map(|item| match item {
            Value::String(s) => Some(Name::parse(&s)),
            Value::Object(_) => serde_json::from_value::<Name>(item).ok(),
            _ => None,
        })
        .filter(|n| !n.is_empty())
        .collect())
}

fn de_opt_date<'de, D: Deserializer<'de>>(d: D) -> Result<Option<CslDate>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Object(map) => serde_json::from_value::<CslDate>(Value::Object(map)).ok(),
        Value::String(s) => CslDate::parse(&s),
        Value::Number(n) => n
            .as_i64()
            .and_then(|y| i32::try_from(y).ok())
            .map(CslDate::year),
        _ => None,
    }
    .filter(|d| !d.date_parts.is_empty() || d.literal.is_some() || d.raw.is_some()))
}

fn de_date_parts<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec<i32>>, D::Error> {
    let v = Value::deserialize(d)?;
    let Value::Array(outer) = v else {
        return Ok(Vec::new());
    };
    Ok(outer
        .into_iter()
        .filter_map(|inner| {
            let Value::Array(parts) = inner else {
                return None;
            };
            let nums: Vec<i32> = parts
                .into_iter()
                .map_while(|p| match p {
                    Value::Number(n) => n.as_i64().and_then(|n| i32::try_from(n).ok()),
                    Value::String(s) => s.trim().parse().ok(),
                    _ => None,
                })
                .collect();
            if nums.is_empty() { None } else { Some(nums) }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_in_all_common_shapes() {
        assert_eq!(Name::parse("Doe, Jane"), Name::new("Doe", "Jane"));
        assert_eq!(Name::parse("Jane Doe"), Name::new("Doe", "Jane"));
        assert_eq!(Name::parse("UNICEF"), Name::new("UNICEF", ""));
        assert_eq!(
            Name::parse("{World Health Organization}"),
            Name::literal("World Health Organization")
        );
        let king = Name::parse("King, Jr., Martin Luther");
        assert_eq!(king.suffix.as_deref(), Some("Jr."));
        assert_eq!(king.natural_form(), "Martin Luther King, Jr.");
        let gogh = Name::parse("Vincent van Gogh");
        assert_eq!(gogh.family_display(), "van Gogh");
        assert_eq!(gogh.sort_form(), "van Gogh, Vincent");
    }

    #[test]
    fn loose_csl_json_is_accepted() {
        let r: Reference = serde_json::from_str(
            r#"{"id": 12, "type": "article-journal", "title": ["T"], "volume": 3,
                "issued": {"date-parts": [["2018", "5"]]}, "ISBN": ["1", "2"],
                "author": [{"family": "Doe", "given": "Jane", "sequence": "first"}],
                "custom": {"a": 1}}"#,
        )
        .unwrap();
        assert_eq!(r.id, "12");
        assert_eq!(r.title.as_deref(), Some("T"));
        assert_eq!(r.volume.as_deref(), Some("3"));
        assert_eq!(r.year(), Some(2018));
        assert_eq!(r.issued.as_ref().and_then(CslDate::month), Some(5));
        assert_eq!(r.isbn.as_deref(), Some("1, 2"));
        assert_eq!(r.author[0].family.as_deref(), Some("Doe"));
        assert!(r.extra.contains_key("custom"));
    }

    #[test]
    fn dates_parse_iso_and_ris_forms() {
        assert_eq!(
            CslDate::parse("2019/01/01/")
                .and_then(|d| d.iso())
                .as_deref(),
            Some("2019-01-01")
        );
        assert_eq!(
            CslDate::parse("2019").and_then(|d| d.iso()).as_deref(),
            Some("2019")
        );
        assert_eq!(
            CslDate::parse("2019-13").and_then(|d| d.iso()).as_deref(),
            Some("2019")
        );
        let spring = CslDate::parse("Spring 2020").unwrap();
        assert_eq!(spring.first_year(), Some(2020));
        assert_eq!(spring.raw.as_deref(), Some("Spring 2020"));
    }

    #[test]
    fn label_reads_well_aloud() {
        let mut r = Reference::new("doe2020", "article-journal");
        r.author = vec![Name::new("Doe", "Jane"), Name::new("Roe", "Rick")];
        r.issued = Some(CslDate::year(2020));
        r.title = Some("On X".into());
        assert_eq!(r.label(), "Doe and Roe, 2020. On X. Key doe2020.");
        assert_eq!(Reference::default().label(), "Untitled.");
    }
}
