//! BibTeX and BibLaTeX import (through the `biblatex` crate) and export.
//!
//! Import resolves `@string` macros, `crossref`, month macros, and LaTeX
//! accents (`{\"o}` becomes ö), which Star's regex parser did not. Export
//! writes one field per line with LaTeX's special characters escaped, in
//! either dialect: BibTeX (`journal`, `year`/`month`, `address`, `school`)
//! or BibLaTeX (`journaltitle`, `date`, `location`, `institution`).

use biblatex::{Bibliography, ChunksExt, DateValue, EntryType, PermissiveType, Person};

use crate::error::{CiteError, Result};
use crate::reference::{CslDate, Name, Reference, non_empty};

/// Which BibTeX dialect to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// Classic BibTeX field names (`journal`, `year`, `month`, `address`).
    BibTex,
    /// BibLaTeX field names (`journaltitle`, `date`, `location`).
    BibLatex,
}

/// Parses BibTeX or BibLaTeX into references. Entry keys become reference
/// ids.
pub fn parse(text: &str) -> Result<Vec<Reference>> {
    let text = text.trim_start_matches('\u{feff}');
    let bib = Bibliography::parse(text).map_err(|e| {
        let line = text[..e.span.start.min(text.len())].matches('\n').count() + 1;
        CiteError::parse("BibTeX", Some(line), e.kind.to_string())
    })?;
    Ok(bib.iter().map(entry_to_reference).collect())
}

fn field(entry: &biblatex::Entry, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|n| entry.get(n))
        .map(|chunks| chunks.format_verbatim())
        .and_then(|s| non_empty(&crate::text::collapse_whitespace(&s)))
}

fn person_to_name(p: &Person) -> Name {
    let given = p.given_name.trim();
    let family = p.name.trim();
    if given.is_empty() && p.prefix.trim().is_empty() && family.contains(' ') {
        // `{World Health Organization}`: braced, unsplit.
        return Name::literal(family);
    }
    Name {
        family: non_empty(family),
        given: non_empty(given),
        non_dropping_particle: non_empty(&p.prefix),
        suffix: non_empty(&p.suffix),
        ..Name::default()
    }
}

fn csl_type(entry: &biblatex::Entry) -> &'static str {
    use EntryType::*;
    let has_journal = entry.get("journaltitle").is_some() || entry.get("journal").is_some();
    match &entry.entry_type {
        Article => {
            if has_journal {
                "article-journal"
            } else {
                "article"
            }
        }
        Book | MvBook | Collection | MvCollection | Reference | MvReference | Proceedings
        | MvProceedings => "book",
        InBook | InCollection | BookInBook | SuppBook | SuppCollection => "chapter",
        InProceedings => "paper-conference",
        InReference => "entry-encyclopedia",
        MastersThesis | PhdThesis | Thesis => "thesis",
        TechReport | Report | Manual => "report",
        Online => "webpage",
        Unpublished => "manuscript",
        Patent => "patent",
        Dataset => "dataset",
        Software => "software",
        Periodical | SuppPeriodical => "periodical",
        Booklet => "pamphlet",
        Unknown(name) => match name.to_ascii_lowercase().as_str() {
            "electronic" | "www" | "webpage" => "webpage",
            "conference" => "paper-conference",
            "standard" => "standard",
            _ => "document",
        },
        _ => "document",
    }
}

fn entry_to_reference(entry: &biblatex::Entry) -> Reference {
    let kind = csl_type(entry);
    let mut r = Reference::new(&entry.key, kind);
    r.title = field(entry, &["title"]);
    if let Some(sub) = field(entry, &["subtitle"]) {
        r.title = Some(match r.title.take() {
            Some(t) => format!("{t}: {sub}"),
            None => sub,
        });
    }
    r.title_short = field(entry, &["shorttitle"]);
    r.author = entry
        .author()
        .map(|ps| ps.iter().map(person_to_name).collect())
        .unwrap_or_default();
    if let Ok(eds) = entry.editors() {
        r.editor = eds
            .iter()
            .filter(|(_, t)| matches!(t, biblatex::EditorType::Editor))
            .flat_map(|(ps, _)| ps.iter().map(person_to_name))
            .collect();
    }
    r.translator = entry
        .get_as::<Vec<Person>>("translator")
        .map(|ps| ps.iter().map(person_to_name).collect())
        .unwrap_or_default();
    r.issued = entry.date().ok().and_then(|d| match d {
        PermissiveType::Typed(d) => {
            let start = match d.value {
                DateValue::At(t) | DateValue::After(t) | DateValue::Before(t) => t,
                DateValue::Between(t, _) => t,
            };
            let mut date = CslDate::ymd(
                start.year,
                start.month.map(|m| m + 1),
                start.day.map(|d| d + 1),
            );
            date.circa = d.approximate;
            Some(date)
        }
        PermissiveType::Chunks(c) => CslDate::parse(&c.format_verbatim()),
    });
    r.accessed = field(entry, &["urldate"]).and_then(|s| CslDate::parse(&s));
    r.container_title = match kind {
        "article-journal" | "article" => field(entry, &["journaltitle", "journal"]),
        "chapter" | "paper-conference" | "entry-encyclopedia" => field(entry, &["booktitle"]),
        _ => field(entry, &["howpublished", "booktitle"]).filter(|s| !s.starts_with("\\url")),
    };
    r.container_title_short = field(entry, &["shortjournal"]);
    r.collection_title = field(entry, &["series"]);
    r.publisher = match kind {
        "thesis" => field(entry, &["school", "institution", "publisher"]),
        "report" => field(entry, &["institution", "publisher", "organization"]),
        _ => field(entry, &["publisher", "organization"]),
    };
    r.publisher_place = field(entry, &["location", "address"]);
    r.volume = field(entry, &["volume"]);
    if matches!(kind, "article-journal" | "article" | "periodical") {
        r.issue = field(entry, &["number", "issue"]);
    } else {
        r.issue = field(entry, &["issue"]);
        r.number = field(entry, &["number"]);
    }
    r.page = field(entry, &["pages"]).map(|p| p.replace(['–', '—'], "-"));
    r.edition = field(entry, &["edition"]);
    r.doi = field(entry, &["doi"]).and_then(|d| crate::lookup::normalize_doi(&d).or(Some(d)));
    r.isbn = field(entry, &["isbn"]);
    r.issn = field(entry, &["issn"]);
    r.url = field(entry, &["url"]);
    r.abstract_text = field(entry, &["abstract"]);
    r.note = field(entry, &["note", "annote", "annotation"]);
    r.keyword = field(entry, &["keywords"]);
    r.language = field(entry, &["langid", "language"]);
    r.genre = match &entry.entry_type {
        EntryType::PhdThesis => Some("PhD thesis".to_owned()),
        EntryType::MastersThesis => Some("Master's thesis".to_owned()),
        _ => None,
    }
    .or_else(|| field(entry, &["type"]).map(|t| thesis_type_words(&t)));
    r
}

/// BibLaTeX's localization keys for thesis and report types, as words.
fn thesis_type_words(t: &str) -> String {
    match t {
        "phdthesis" => "PhD thesis".to_owned(),
        "mathesis" | "mastersthesis" => "Master's thesis".to_owned(),
        "techreport" => "Technical report".to_owned(),
        other => other.to_owned(),
    }
}

// ---- export ---------------------------------------------------------------

/// Writes references as BibTeX or BibLaTeX. Each entry is followed by a
/// blank line.
pub fn write(refs: &[Reference], dialect: Dialect) -> String {
    let mut out = String::new();
    for r in refs {
        write_entry(&mut out, r, dialect);
        out.push('\n');
    }
    out
}

fn entry_type(r: &Reference, dialect: Dialect) -> &'static str {
    let bib = dialect == Dialect::BibTex;
    let genre = r.genre.as_deref().unwrap_or("").to_ascii_lowercase();
    match r.kind.as_str() {
        "article-journal" | "article-magazine" | "article-newspaper" | "article" | "review"
        | "review-book" => "article",
        "book" | "classic" => "book",
        "chapter" => {
            if bib && r.editor.is_empty() {
                "inbook"
            } else {
                "incollection"
            }
        }
        "paper-conference" => "inproceedings",
        "entry-encyclopedia" | "entry-dictionary" | "entry" => {
            if bib {
                "incollection"
            } else {
                "inreference"
            }
        }
        "thesis" => {
            if !bib {
                "thesis"
            } else if genre.contains("master") {
                "mastersthesis"
            } else {
                "phdthesis"
            }
        }
        "report" => {
            if bib {
                "techreport"
            } else {
                "report"
            }
        }
        "webpage" | "post-weblog" | "post" => {
            if bib {
                "misc"
            } else {
                "online"
            }
        }
        "manuscript" => "unpublished",
        "pamphlet" => "booklet",
        "periodical" if !bib => "periodical",
        "patent" if !bib => "patent",
        "dataset" if !bib => "dataset",
        "software" if !bib => "software",
        _ => "misc",
    }
}

fn write_entry(out: &mut String, r: &Reference, dialect: Dialect) {
    let bib = dialect == Dialect::BibTex;
    let ty = entry_type(r, dialect);
    let key = if r.id.is_empty() {
        crate::key::base_key(r)
    } else {
        r.id.replace([' ', ',', '{', '}'], "")
    };
    out.push('@');
    out.push_str(ty);
    out.push('{');
    out.push_str(&key);
    out.push_str(",\n");

    if !r.author.is_empty() {
        put(out, "author", &names(&r.author), true);
    }
    if !r.editor.is_empty() {
        put(out, "editor", &names(&r.editor), true);
    }
    if !r.translator.is_empty() && !bib {
        put(out, "translator", &names(&r.translator), true);
    }
    if let Some(t) = &r.title {
        put(out, "title", &crate::text::plain_title(t), false);
    }
    if let Some(t) = &r.title_short {
        put(out, "shorttitle", &crate::text::plain_title(t), false);
    }
    if let Some(c) = &r.container_title {
        let c = crate::text::plain_title(c);
        let name = match ty {
            "article" => {
                if bib {
                    "journal"
                } else {
                    "journaltitle"
                }
            }
            "incollection" | "inbook" | "inproceedings" | "inreference" => "booktitle",
            _ => "howpublished",
        };
        put(out, name, &c, false);
    }
    if let Some(c) = &r.container_title_short {
        put(out, "shortjournal", c, false);
    }
    if let Some(c) = &r.collection_title {
        put(out, "series", &crate::text::plain_title(c), false);
    }
    if let Some(d) = &r.issued {
        if bib {
            if let Some(y) = d.first_year() {
                put(out, "year", &y.to_string(), false);
            }
            if let Some(m) = d.month() {
                // Month macros are unbraced: `month = may,`.
                out.push_str("  month = ");
                out.push_str(MONTHS[usize::from(m) - 1]);
                out.push_str(",\n");
            }
        } else if let Some(iso) = d.iso() {
            put(out, "date", &iso, false);
        } else if let Some(raw) = d.raw.as_deref().or(d.literal.as_deref()) {
            put(out, "date", raw, false);
        }
    }
    if let Some(p) = &r.publisher {
        let name = match (ty, bib) {
            ("phdthesis" | "mastersthesis", _) => "school",
            ("thesis", false) | ("report", false) | ("techreport", _) => "institution",
            _ => "publisher",
        };
        put(out, name, p, false);
    }
    if let Some(p) = &r.publisher_place {
        put(out, if bib { "address" } else { "location" }, p, false);
    }
    if let Some(v) = &r.volume {
        put(out, "volume", v, false);
    }
    if let Some(n) = r.issue.as_ref().or(r.number.as_ref()) {
        put(out, "number", n, false);
    }
    if let Some(p) = &r.page {
        // Page ranges use BibTeX's en dash, `--`.
        let pages: Vec<&str> = p.split(['-', '–', '—']).filter(|s| !s.is_empty()).collect();
        put(out, "pages", &pages.join("--"), true);
    }
    if let Some(e) = &r.edition {
        put(out, "edition", e, false);
    }
    if let Some(g) = &r.genre
        && matches!(ty, "thesis" | "report" | "techreport")
    {
        put(out, "type", g, false);
    }
    if let Some(v) = &r.doi {
        put(out, "doi", v, true);
    }
    if let Some(v) = &r.isbn {
        put(out, "isbn", v, false);
    }
    if let Some(v) = &r.issn {
        put(out, "issn", v, false);
    }
    if let Some(v) = &r.url {
        put(out, "url", v, true);
    }
    if let Some(d) = r.accessed.as_ref().and_then(CslDate::iso) {
        put(out, "urldate", &d, false);
    }
    if let Some(v) = &r.language {
        put(out, if bib { "language" } else { "langid" }, v, false);
    }
    if let Some(v) = &r.keyword {
        put(out, "keywords", v, false);
    }
    if let Some(v) = &r.abstract_text {
        put(out, "abstract", &crate::text::plain_title(v), false);
    }
    if let Some(v) = &r.note {
        put(out, "note", v, false);
    }
    out.push_str("}\n");
}

fn put(out: &mut String, name: &str, value: &str, raw: bool) {
    if value.trim().is_empty() {
        return;
    }
    out.push_str("  ");
    out.push_str(name);
    out.push_str(" = {");
    if raw {
        out.push_str(&escape_verbatim(value));
    } else {
        out.push_str(&escape(value));
    }
    out.push_str("},\n");
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// `Doe, Jane and Roe, Rick`; organizations braced so BibTeX does not split
/// them.
fn names(names: &[Name]) -> String {
    names
        .iter()
        .map(|n| {
            if n.literal.is_some() || (n.given.is_none() && n.family_display().contains(' ')) {
                format!("{{{}}}", escape(&n.family_display()))
            } else {
                escape(&n.sort_form())
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Escapes LaTeX's special characters in ordinary text.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' | '}' | '#' | '&' | '%' | '$' | '_' => {
                out.push('\\');
                out.push(c);
            }
            // `--` is an en dash in LaTeX; keep literal hyphens literal.
            '-' if out.ends_with('-') => out.push_str("{}-"),
            _ => out.push(c),
        }
    }
    out
}

/// Verbatim fields (`url`, `doi`) and pre-escaped name lists: only
/// unbalanced braces would break the entry, so they are dropped.
fn escape_verbatim(s: &str) -> String {
    let mut depth = 0i32;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                out.push(c);
            }
            '}' if depth > 0 => {
                depth -= 1;
                out.push(c);
            }
            '}' => {}
            _ => out.push(c),
        }
    }
    for _ in 0..depth {
        out.push('}');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_basic_entry_parses() {
        // Star's tests/test_citations.py::test_parse_bibtex_basic.
        let refs = parse(
            "@article{key1,\n  title = {A Great Paper},\n  author = {Doe, Jane},\n  year = {2021},\n  booktitle = {Proc of Things},\n  doi = {10.1/x},\n}\n",
        )
        .unwrap();
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(r.id, "key1");
        assert_eq!(r.kind, "article");
        assert_eq!(r.title.as_deref(), Some("A Great Paper"));
        assert_eq!(r.author, vec![Name::new("Doe", "Jane")]);
        assert_eq!(r.year(), Some(2021));
        assert_eq!(r.doi.as_deref(), Some("10.1/x"));
    }

    #[test]
    fn macros_accents_and_crossrefs_resolve() {
        let refs = parse(
            r#"@string{jx = "Journal of X"}
@article{a, author = {M{\"u}ller, J{\"o}rg and {World Health Organization}}, title = {T}, journal = jx, year = 2020, month = may}
@book{b, title={One}}
@article{c, title={Two}}"#,
        )
        .unwrap();
        assert_eq!(
            refs.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        let a = &refs[0];
        assert_eq!(a.author[0], Name::new("Müller", "Jörg"));
        assert_eq!(a.author[1], Name::literal("World Health Organization"));
        assert_eq!(a.container_title.as_deref(), Some("Journal of X"));
        assert_eq!(a.issued.as_ref().and_then(CslDate::month), Some(5));
        assert_eq!(a.kind, "article-journal");
    }

    #[test]
    fn syntax_errors_name_the_line() {
        let err =
            parse("@article{a, title = {T}}\n\n@article{b, title = {unclosed}\n").unwrap_err();
        assert!(err.to_string().contains("BibTeX"), "{err}");
    }

    #[test]
    fn special_characters_are_escaped() {
        let mut r = Reference::new("k", "book");
        r.title = Some("Profit & Loss: 100% of $5_x {y}".into());
        r.url = Some("https://example.org/a_b%20c".into());
        let out = write(&[r], Dialect::BibTex);
        assert!(
            out.contains(r"title = {Profit \& Loss: 100\% of \$5\_x \{y\}},"),
            "{out}"
        );
        assert!(
            out.contains("url = {https://example.org/a_b%20c},"),
            "{out}"
        );
    }
}
