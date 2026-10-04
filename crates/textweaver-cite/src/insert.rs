//! What an editor needs to insert citations by keyboard and hear them.
//!
//! The app's "Insert citation" flow (star: Ctrl+Alt+R) is:
//!
//! 1. Build the picker with [`picker_entries`] (folder library first, then
//!    the user library). Each entry's [`PickerEntry::label`] is one line
//!    that reads well aloud; typing filters with [`filter_picker`].
//! 2. Optionally ask for a page or other locator; [`parse_locator`] turns
//!    what was typed ("12", "pp. 3-5", "chapter 2") into a [`Locator`].
//! 3. Insert the text from [`insertion_text`] at the cursor (`[@doe2020,
//!    p. 12]`), or, when the cursor is already inside a citation
//!    ([`crate::pandoc::citation_at`]), replace that citation's range with
//!    [`add_to_citation`] so the new key joins the group.
//! 4. Announce [`announce_inserted`].
//!
//! When reading, [`describe_citation`] turns a citation under the cursor
//! into a spoken description: "Citation: Doe and Roe, 2020, On X, page 12."
//! Continuous reading uses the shorter [`spoken_citation`] ("Doe and Roe,
//! 2020, page 12") when citations are on, and [`spoken_citation_authors`]
//! for an in-text citation when they are off, since it is part of the
//! sentence ("Doe and Roe argue ...").

use crate::library::ReferenceSource;
use crate::pandoc::{Citation, CiteItem, Locator, write_citation};
use crate::reference::Reference;

/// One row of the citation picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerEntry {
    /// The citation key.
    pub key: String,
    /// One readable line: "Doe and Roe, 2020. On X. Key doe2020."
    pub label: String,
    /// Where the reference comes from ("folder" or "user"), when layered.
    pub source: &'static str,
}

/// Picker rows from a folder library (if any) and the user library, sorted
/// by author, year, and title; a key in both is listed once, from the
/// folder.
pub fn picker_entries(folder: Option<&crate::Library>, user: &crate::Library) -> Vec<PickerEntry> {
    let mut out: Vec<PickerEntry> = Vec::new();
    if let Some(f) = folder {
        out.extend(f.sorted().into_iter().map(|r| entry(r, "folder")));
    }
    for r in user.sorted() {
        if !out.iter().any(|e| e.key == r.id) {
            out.push(entry(r, "user"));
        }
    }
    out
}

fn entry(r: &Reference, source: &'static str) -> PickerEntry {
    PickerEntry {
        key: r.id.clone(),
        label: r.label(),
        source,
    }
}

/// Rows whose label contains every word typed (case-insensitive).
pub fn filter_picker<'a>(entries: &'a [PickerEntry], query: &str) -> Vec<&'a PickerEntry> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    entries
        .iter()
        .filter(|e| {
            let l = e.label.to_lowercase();
            words.iter().all(|w| l.contains(w.as_str()))
        })
        .collect()
}

/// Parses a locator as typed in a prompt: "12" (a page), "12-15",
/// "p. 12", "pp. 3-5", "page 4", "chapter 2", "sec. 3.1". Empty input, or
/// input that is not a locator ("introduction"), is `None`; the app should
/// then say it could not read the locator rather than insert it.
pub fn parse_locator(input: &str) -> Option<Locator> {
    let t = input.trim();
    if t.is_empty() || t.contains([']', ';', '[']) {
        return None;
    }
    let probe = format!("[@k, {t}]");
    let cites = crate::pandoc::find_citations(&probe);
    let item = cites.into_iter().next()?.items.into_iter().next()?;
    item.locator.filter(|_| item.suffix.is_empty())
}

/// The text to insert for citing `keys` (Pandoc syntax), with an optional
/// locator on the last key. `narrative` gives `@doe2020 [p. 12]`, for "Doe
/// (2020, p. 12) argues".
pub fn insertion_text(keys: &[&str], locator: Option<Locator>, narrative: bool) -> String {
    let mut items: Vec<CiteItem> = keys
        .iter()
        .map(|k| CiteItem {
            key: (*k).to_owned(),
            ..CiteItem::default()
        })
        .collect();
    if let Some(last) = items.last_mut() {
        last.locator = locator;
    }
    write_citation(&items, narrative && items.len() == 1)
}

/// Adds keys to an existing bracketed citation; returns the replacement
/// text for the citation's range.
pub fn add_to_citation(existing: &Citation, keys: &[&str]) -> String {
    let mut items = existing.items.clone();
    for k in keys {
        if !items.iter().any(|i| i.key == *k) {
            items.push(CiteItem {
                key: (*k).to_owned(),
                ..CiteItem::default()
            });
        }
    }
    write_citation(&items, false)
}

/// "Inserted citation of Doe and Roe, 2020, page 12."
pub fn announce_inserted(
    keys: &[&str],
    locator: Option<&Locator>,
    source: &dyn ReferenceSource,
) -> String {
    let works: Vec<String> = keys.iter().map(|k| short_description(k, source)).collect();
    let mut s = format!("Inserted citation of {}", join_and(&works));
    if let Some(l) = locator {
        s.push_str(", ");
        s.push_str(&l.spoken());
    }
    s.push('.');
    s
}

/// A citation as continuous reading says it when citations are on: each
/// work's authors and year, its locator, and the prefix and suffix typed
/// around it, joined with semicolons: "Doe and Roe, 2020, page 12" for
/// `[@doe2020, p. 12]`, "see Doe and Roe, 2020; Roe, 2019" for `[see
/// @doe2020; @roe2019]`. `[-@key]` leaves the authors out ("2020"). A key
/// no library has is read as the key.
pub fn spoken_citation(c: &Citation, source: &dyn ReferenceSource) -> String {
    let parts: Vec<String> = c
        .items
        .iter()
        .map(|i| {
            let mut s = String::new();
            if !i.prefix.is_empty() {
                s.push_str(i.prefix.trim());
                s.push(' ');
            }
            match source.get(&i.key) {
                Some(r) if i.suppress_author => {
                    s.push_str(
                        &r.year()
                            .map_or_else(|| "no date".to_owned(), |y| y.to_string()),
                    );
                }
                Some(_) => s.push_str(&short_description(&i.key, source)),
                None => s.push_str(&i.key),
            }
            if let Some(l) = &i.locator {
                s.push_str(", ");
                s.push_str(&l.spoken());
            }
            let suffix = i.suffix.trim_start_matches([',', ' ']).trim_end();
            if !suffix.is_empty() {
                s.push_str(", ");
                s.push_str(suffix);
            }
            s
        })
        .collect();
    parts.join("; ")
}

/// The authors of an in-text citation, for continuous reading with
/// citations off, where `@doe2020 argues` is the subject of a sentence:
/// "Doe and Roe" (the key when no library has it, or the title when the
/// work has no authors).
pub fn spoken_citation_authors(c: &Citation, source: &dyn ReferenceSource) -> String {
    let names: Vec<String> = c
        .items
        .iter()
        .map(|i| match source.get(&i.key) {
            Some(r) => r.creators_short().unwrap_or_else(|| {
                r.title
                    .as_deref()
                    .map(crate::text::plain_title)
                    .unwrap_or_else(|| i.key.clone())
            }),
            None => i.key.clone(),
        })
        .collect();
    join_and(&names)
}

/// A spoken description of a citation: "Citation: Doe and Roe, 2020, On
/// X, page 12; see also Roe, 2019, Other." Unknown keys are named as
/// missing.
pub fn describe_citation(c: &Citation, source: &dyn ReferenceSource) -> String {
    let parts: Vec<String> = c
        .items
        .iter()
        .map(|i| {
            let mut s = String::new();
            if !i.prefix.is_empty() {
                s.push_str(&i.prefix);
                s.push(' ');
            }
            match source.get(&i.key) {
                Some(r) => {
                    s.push_str(&short_description(&i.key, source));
                    if let Some(t) = r
                        .title
                        .as_deref()
                        .map(crate::text::plain_title)
                        .filter(|t| !t.is_empty())
                    {
                        s.push_str(", ");
                        s.push_str(t.trim_end_matches('.'));
                    }
                }
                None => {
                    s.push_str("missing reference ");
                    s.push_str(&i.key);
                }
            }
            if let Some(l) = &i.locator {
                s.push_str(", ");
                s.push_str(&l.spoken());
            }
            let suffix = i.suffix.trim_start_matches([',', ' ']);
            if !suffix.is_empty() {
                s.push_str(", ");
                s.push_str(suffix);
            }
            if i.suppress_author {
                s.push_str(", author not shown");
            }
            s
        })
        .collect();
    let head = if c.narrative {
        "In-text citation"
    } else {
        "Citation"
    };
    format!("{head}: {}.", parts.join("; "))
}

/// "Doe and Roe, 2020", or the key and "missing" when it is not found.
fn short_description(key: &str, source: &dyn ReferenceSource) -> String {
    match source.get(key) {
        Some(r) => match (r.creators_short(), r.year()) {
            (Some(c), Some(y)) => format!("{c}, {y}"),
            (Some(c), None) => format!("{c}, no date"),
            (None, Some(y)) => format!(
                "{}, {y}",
                r.title
                    .as_deref()
                    .map(crate::text::plain_title)
                    .unwrap_or_else(|| key.to_owned())
            ),
            (None, None) => key.to_owned(),
        },
        None => format!("{key}, which is not in the library"),
    }
}

fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join("; ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::reference::{CslDate, Name};

    fn lib() -> Library {
        let mut r = Reference::new("doe2020", "book");
        r.author = vec![Name::new("Doe", "Jane"), Name::new("Roe", "Rick")];
        r.issued = Some(CslDate::year(2020));
        r.title = Some("On <i>X</i>".into());
        Library::from_references(vec![r])
    }

    #[test]
    fn locators_from_prompts() {
        assert_eq!(
            parse_locator("12").map(|l| l.written()).as_deref(),
            Some("p. 12")
        );
        assert_eq!(
            parse_locator("pp. 3-5").map(|l| l.written()).as_deref(),
            Some("pp. 3-5")
        );
        assert_eq!(
            parse_locator("chapter 2").map(|l| l.written()).as_deref(),
            Some("chap. 2")
        );
        assert_eq!(parse_locator(" "), None);
        assert_eq!(parse_locator("introduction"), None);
        assert_eq!(parse_locator("12, emphasis added"), None);
        assert_eq!(
            parse_locator("iv").map(|l| l.written()).as_deref(),
            Some("p. iv")
        );
    }

    #[test]
    fn insertion_and_group_extension() {
        assert_eq!(
            insertion_text(&["doe2020"], parse_locator("12"), false),
            "[@doe2020, p. 12]"
        );
        assert_eq!(
            insertion_text(&["doe2020"], parse_locator("12"), true),
            "@doe2020 [p. 12]"
        );
        assert_eq!(insertion_text(&["a", "b"], None, false), "[@a; @b]");
        let existing = &crate::pandoc::find_citations("[@a, p. 3]")[0];
        assert_eq!(add_to_citation(existing, &["b", "a"]), "[@a, p. 3; @b]");
    }

    #[test]
    fn spoken_descriptions() {
        let lib = lib();
        let c = &crate::pandoc::find_citations("[see @doe2020, pp. 3-5; @nobody]")[0];
        assert_eq!(
            describe_citation(c, &lib),
            "Citation: see Doe and Roe, 2020, On X, pages 3 to 5; missing reference nobody."
        );
        assert_eq!(
            announce_inserted(&["doe2020"], parse_locator("12").as_ref(), &lib),
            "Inserted citation of Doe and Roe, 2020, page 12."
        );
        let rows = picker_entries(None, &lib);
        assert_eq!(rows[0].label, "Doe and Roe, 2020. On X. Key doe2020.");
        let find = |t: &str| crate::pandoc::find_citations(t).remove(0);
        assert_eq!(
            spoken_citation(&find("[@doe2020, p. 12]"), &lib),
            "Doe and Roe, 2020, page 12"
        );
        assert_eq!(
            spoken_citation(
                &find("[see @doe2020, pp. 33-35, emphasis added; @nobody]"),
                &lib
            ),
            "see Doe and Roe, 2020, pages 33 to 35, emphasis added; nobody"
        );
        assert_eq!(spoken_citation(&find("[-@doe2020]"), &lib), "2020");
        assert_eq!(
            spoken_citation(&find("@doe2020 [p. 3] shows"), &lib),
            "Doe and Roe, 2020, page 3"
        );
        assert_eq!(
            spoken_citation(&find("[@nobody, p. 4]"), &lib),
            "nobody, page 4"
        );
        assert_eq!(
            spoken_citation_authors(&find("@doe2020 argues"), &lib),
            "Doe and Roe"
        );
        assert_eq!(
            spoken_citation_authors(&find("@nobody argues"), &lib),
            "nobody"
        );
        assert_eq!(filter_picker(&rows, "roe 2020").len(), 1);
        assert!(filter_picker(&rows, "smith").is_empty());
    }
}
