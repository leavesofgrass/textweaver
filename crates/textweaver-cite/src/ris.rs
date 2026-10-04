//! RIS import and export.
//!
//! star wrote the CSL type upper-cased as the RIS type (`TY  - ARTICLE`),
//! which no reference manager recognizes; this module maps types both ways
//! (`JOUR`, `BOOK`, `CHAP`, ...). It also reads the full date (`DA`), page
//! ranges (`SP`/`EP`), editors, ISBN and ISSN (`SN`), keywords, and
//! abstracts, which star dropped, and joins wrapped continuation lines.

use crate::reference::{CslDate, Name, Reference, non_empty};

/// RIS type for a CSL type.
fn ris_type(csl: &str) -> &'static str {
    match csl {
        "article-journal" | "article" | "review" | "review-book" => "JOUR",
        "article-magazine" => "MGZN",
        "article-newspaper" => "NEWS",
        "book" | "classic" | "periodical" | "collection" => "BOOK",
        "chapter" => "CHAP",
        "paper-conference" => "CPAPER",
        "thesis" => "THES",
        "report" => "RPRT",
        "webpage" => "ELEC",
        "post-weblog" | "post" => "BLOG",
        "dataset" => "DATA",
        "software" => "COMP",
        "patent" => "PAT",
        "legal_case" => "CASE",
        "legislation" | "bill" | "regulation" => "STAT",
        "manuscript" => "UNPB",
        "entry-encyclopedia" | "entry" => "ENCYC",
        "entry-dictionary" => "DICT",
        "motion_picture" | "broadcast" => "VIDEO",
        "song" => "SOUND",
        "map" => "MAP",
        "graphic" | "figure" => "ART",
        "pamphlet" => "PAMP",
        "interview" | "speech" | "personal_communication" | "hearing" | "performance" => "GEN",
        _ => "GEN",
    }
}

/// CSL type for a RIS type.
fn csl_type(ris: &str) -> &'static str {
    match ris.trim().to_ascii_uppercase().as_str() {
        "JOUR" | "JFULL" | "EJOUR" | "ABST" | "INPR" => "article-journal",
        "MGZN" => "article-magazine",
        "NEWS" => "article-newspaper",
        "BOOK" | "EBOOK" | "EDBOOK" | "SER" | "CLSWK" => "book",
        "CHAP" | "ECHAP" => "chapter",
        "CONF" | "CPAPER" => "paper-conference",
        "THES" => "thesis",
        "RPRT" | "GOVDOC" | "STAND" => "report",
        "ELEC" | "WEB" | "ICOMM" => "webpage",
        "BLOG" => "post-weblog",
        "DATA" | "DBASE" | "AGGR" => "dataset",
        "COMP" => "software",
        "PAT" => "patent",
        "CASE" => "legal_case",
        "STAT" | "BILL" | "LEGAL" => "legislation",
        "UNPB" | "MANSCPT" => "manuscript",
        "ENCYC" => "entry-encyclopedia",
        "DICT" => "entry-dictionary",
        "VIDEO" | "MPCT" | "ADVS" => "motion_picture",
        "SOUND" | "MUSIC" => "song",
        "MAP" => "map",
        "ART" | "FIGURE" | "CHART" => "graphic",
        "PAMP" => "pamphlet",
        _ => "document",
    }
}

/// RIS writes people as `Family, Given`; a name without a comma is an
/// organization ("World Health Organization"), as Zotero exports them.
fn ris_name(s: &str) -> Name {
    if s.contains(',') {
        Name::parse(s)
    } else {
        Name::literal(s.trim())
    }
}

/// Splits a line into `(tag, value)`: two uppercase letters or a letter
/// and a digit, spaces, a hyphen, and the value. Tolerates one or two
/// spaces before the hyphen and a missing space after it.
fn split_line(line: &str) -> Option<(&str, &str)> {
    let b = line.as_bytes();
    if b.len() < 4
        || !b[0].is_ascii_uppercase()
        || !(b[1].is_ascii_uppercase() || b[1].is_ascii_digit())
    {
        return None;
    }
    let rest = &line[2..];
    let spaces = rest.len() - rest.trim_start_matches(' ').len();
    if !(1..=2).contains(&spaces) {
        return None;
    }
    let rest = rest[spaces..].strip_prefix('-')?;
    let value = rest.strip_prefix(' ').unwrap_or(rest);
    Some((&line[..2], value.trim()))
}

#[derive(Default)]
struct Record {
    ty: String,
    fields: Vec<(String, String)>,
}

impl Record {
    fn all(&self, tags: &[&str]) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|(t, v)| tags.contains(&t.as_str()) && !v.is_empty())
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn first(&self, tags: &[&str]) -> Option<String> {
        // Tag order is the preference order.
        tags.iter()
            .find_map(|tag| self.fields.iter().find(|(t, v)| t == tag && !v.is_empty()))
            .and_then(|(_, v)| non_empty(v))
    }
}

/// Parses RIS records. Lines outside a `TY` … `ER` record are ignored; a
/// record without an `ER` line at the end of the file is kept.
pub fn parse(text: &str) -> Vec<Reference> {
    let text = text.trim_start_matches('\u{feff}');
    let mut out = Vec::new();
    let mut cur: Option<Record> = None;
    for raw in text.lines() {
        let line = raw.trim_end();
        match split_line(line) {
            Some(("TY", v)) => {
                if let Some(rec) = cur.take() {
                    out.push(record_to_reference(rec));
                }
                cur = Some(Record {
                    ty: v.to_owned(),
                    fields: Vec::new(),
                });
            }
            Some(("ER", _)) => {
                if let Some(rec) = cur.take() {
                    out.push(record_to_reference(rec));
                }
            }
            Some((tag, v)) => {
                if let Some(rec) = cur.as_mut() {
                    rec.fields.push((tag.to_owned(), v.to_owned()));
                }
            }
            None => {
                // A wrapped continuation of the previous field.
                let t = line.trim();
                if let (Some(rec), false) = (cur.as_mut(), t.is_empty())
                    && let Some((_, last)) = rec.fields.last_mut()
                {
                    if !last.is_empty() {
                        last.push(' ');
                    }
                    last.push_str(t);
                }
            }
        }
    }
    if let Some(rec) = cur.take() {
        out.push(record_to_reference(rec));
    }
    out
}

fn record_to_reference(rec: Record) -> Reference {
    let kind = csl_type(&rec.ty);
    let mut r = Reference::new(&rec.first(&["ID"]).unwrap_or_default(), kind);
    r.title = rec.first(&["TI", "T1"]);
    if r.title.is_none() && kind == "book" {
        r.title = rec.first(&["BT"]);
    }
    r.title_short = rec.first(&["ST"]);
    r.author = rec.all(&["AU", "A1"]).into_iter().map(ris_name).collect();
    r.editor = rec.all(&["ED", "A2"]).into_iter().map(ris_name).collect();
    r.translator = rec.all(&["A4"]).into_iter().map(ris_name).collect();
    let container_tags: &[&str] = if kind == "book" {
        &["T2", "JF", "JO"]
    } else {
        &["T2", "JF", "JO", "BT", "JA", "J2"]
    };
    r.container_title = rec.first(container_tags);
    r.container_title_short = rec
        .first(&["J2", "JA"])
        .filter(|s| Some(s) != r.container_title.as_ref());
    r.collection_title = rec.first(&["T3"]);
    let full = rec.first(&["DA"]).and_then(|d| CslDate::parse(&d));
    let year = rec.first(&["PY", "Y1"]).and_then(|d| CslDate::parse(&d));
    r.issued = match (full, year) {
        (Some(f), _) if f.month().is_some() => Some(f),
        (_, Some(y)) => Some(y),
        (f, None) => f,
    };
    r.accessed = rec.first(&["Y2"]).and_then(|d| CslDate::parse(&d));
    r.page = match (rec.first(&["SP"]), rec.first(&["EP"])) {
        (Some(s), Some(e)) if s != e => Some(format!("{s}-{e}")),
        (Some(s), _) => Some(s),
        (None, e) => e,
    };
    r.volume = rec.first(&["VL"]);
    r.issue = rec.first(&["IS"]);
    r.edition = rec.first(&["ET"]);
    r.publisher = rec.first(&["PB"]);
    r.publisher_place = rec.first(&["CY", "PP"]);
    for sn in rec.all(&["SN"]) {
        let digits: String = sn.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if digits.len() == 8 {
            r.issn.get_or_insert_with(|| sn.to_owned());
        } else {
            r.isbn.get_or_insert_with(|| sn.to_owned());
        }
    }
    r.doi = rec
        .first(&["DO", "DI"])
        .map(|d| crate::lookup::normalize_doi(&d).unwrap_or(d));
    r.url = rec.first(&["UR", "L2", "LK"]);
    r.abstract_text = rec.first(&["AB", "N2"]);
    r.note = rec.first(&["N1"]);
    r.language = rec.first(&["LA"]);
    r.genre = rec.first(&["M3"]);
    let kws = rec.all(&["KW"]);
    if !kws.is_empty() {
        r.keyword = Some(kws.join(", "));
    }
    r
}

/// Writes references as RIS with CRLF line endings, as the format's
/// specification asks.
pub fn write(refs: &[Reference]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for r in refs {
        let mut put = |tag: &str, v: &str| {
            let v = crate::text::collapse_whitespace(v);
            if !v.is_empty() {
                lines.push(format!("{tag}  - {v}"));
            }
        };
        put("TY", ris_type(&r.kind));
        put("ID", &r.id);
        if let Some(t) = &r.title {
            put("TI", &crate::text::plain_title(t));
        }
        if let Some(t) = &r.title_short {
            put("ST", &crate::text::plain_title(t));
        }
        for n in &r.author {
            put("AU", &n.sort_form());
        }
        for n in &r.editor {
            put("ED", &n.sort_form());
        }
        for n in &r.translator {
            put("A4", &n.sort_form());
        }
        if let Some(d) = &r.issued {
            if let Some(y) = d.first_year() {
                put("PY", &y.to_string());
            }
            if let Some(m) = d.month() {
                let y = d.first_year().unwrap_or_default();
                let day = d.day().map(|d| format!("{d:02}")).unwrap_or_default();
                put("DA", &format!("{y:04}/{m:02}/{day}"));
            }
        }
        if let Some(c) = &r.container_title {
            put("T2", &crate::text::plain_title(c));
        }
        if let Some(c) = &r.container_title_short {
            put("J2", c);
        }
        if let Some(c) = &r.collection_title {
            put("T3", &crate::text::plain_title(c));
        }
        if let Some(v) = &r.volume {
            put("VL", v);
        }
        if let Some(v) = r.issue.as_ref().or(r.number.as_ref()) {
            put("IS", v);
        }
        if let Some(p) = &r.page {
            let p = p.replace(['–', '—'], "-");
            match p.split_once('-') {
                Some((s, e)) => {
                    put("SP", s.trim());
                    put("EP", e.trim_start_matches('-').trim());
                }
                None => put("SP", &p),
            }
        }
        if let Some(v) = &r.edition {
            put("ET", v);
        }
        if let Some(v) = &r.publisher {
            put("PB", v);
        }
        if let Some(v) = &r.publisher_place {
            put("CY", v);
        }
        if let Some(v) = &r.isbn {
            put("SN", v);
        }
        if let Some(v) = &r.issn {
            put("SN", v);
        }
        if let Some(v) = &r.doi {
            put("DO", v);
        }
        if let Some(v) = &r.url {
            put("UR", v);
        }
        if let Some(iso) = r.accessed.as_ref().and_then(CslDate::iso) {
            put("Y2", &iso.replace('-', "/"));
        }
        if let Some(v) = &r.language {
            put("LA", v);
        }
        if let Some(v) = &r.genre {
            put("M3", v);
        }
        if let Some(v) = &r.keyword {
            for kw in v.split([',', ';']) {
                put("KW", kw.trim());
            }
        }
        if let Some(v) = &r.abstract_text {
            put("AB", &crate::text::plain_title(v));
        }
        if let Some(v) = &r.note {
            put("N1", v);
        }
        lines.push("ER  - ".to_owned());
        lines.push(String::new());
    }
    let mut out = lines.join("\r\n");
    if !out.is_empty() && !out.ends_with("\r\n") {
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_basic_record_parses() {
        // star's tests/test_citations.py::test_parse_ris_basic.
        let refs = parse(
            "TY  - JOUR\nTI  - A Title\nAU  - Doe, Jane\nAU  - Roe, Rick\nPY  - 2019/01/01\nDO  - 10.2/y\nER  - \n",
        );
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(r.kind, "article-journal");
        assert_eq!(r.title.as_deref(), Some("A Title"));
        assert_eq!(
            r.author,
            vec![Name::new("Doe", "Jane"), Name::new("Roe", "Rick")]
        );
        assert_eq!(r.year(), Some(2019));
        assert_eq!(r.doi.as_deref(), Some("10.2/y"));
    }

    #[test]
    fn continuation_lines_pages_and_serials() {
        let refs = parse(
            "TY  - BOOK\r\nTI  - A long\r\n  wrapped title\r\nSP  - 12\r\nEP  - 15\r\nSN  - 978-0-306-40615-7\r\nKW  - one\r\nKW  - two\r\nER  -\r\nTY  - JOUR\r\nSN  - 1234-5679\r\n",
        );
        assert_eq!(refs.len(), 2, "a final record without ER is kept");
        assert_eq!(refs[0].title.as_deref(), Some("A long wrapped title"));
        assert_eq!(refs[0].page.as_deref(), Some("12-15"));
        assert_eq!(refs[0].isbn.as_deref(), Some("978-0-306-40615-7"));
        assert_eq!(refs[0].keyword.as_deref(), Some("one, two"));
        assert_eq!(refs[1].issn.as_deref(), Some("1234-5679"));
    }

    #[test]
    fn types_map_to_real_ris_types() {
        let mut r = Reference::new("a", "article-journal");
        r.title = Some("T".into());
        let out = write(&[r]);
        assert!(out.starts_with("TY  - JOUR\r\n"), "{out}");
        assert!(out.ends_with("ER  - \r\n"), "{out:?}");
    }
}
