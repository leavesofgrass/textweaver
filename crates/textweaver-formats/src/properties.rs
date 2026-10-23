//! A note's properties (YAML front matter), read by the vault crate's
//! parser so the reader and the vault commands agree: block and flow
//! lists, quoted values, and block scalars, with nested maps skipped.
//!
//! The well-known keys fill [`DocumentMeta`]: `title`, `author` (or
//! `authors`), `lang` (or `language`). The others stay in
//! `meta.properties` as text, a list as its items joined by ", ", with
//! Obsidian's names mapped explicitly: `aliases` gathers `alias` too,
//! `tags` gathers `tag` (without `#`), and `cssclasses` gathers
//! `cssclass`. Every property, in file order, is also kept for reading
//! aloud ([`front_matter_properties`]).

use textweaver_text::DocumentMeta;

/// The `DocumentMeta::properties` key holding the front matter, for
/// reading aloud: one `key<TAB>value` pair per line.
pub const FRONT_MATTER_PROPERTY: &str = "textweaver.front_matter";

/// Fills `meta` from the inside of a YAML front matter block.
pub(crate) fn apply_yaml(block: &str, meta: &mut DocumentMeta) {
    let fm = textweaver_vault::parse_front_matter(block);
    let shown = fm.properties();
    let joined = |keys: &[&str], split: fn(&textweaver_vault::FrontMatter, &str) -> Vec<String>| {
        let mut items: Vec<String> = Vec::new();
        for k in keys {
            for v in split(&fm, k) {
                let v = v.trim().trim_start_matches('#').trim().to_owned();
                if !v.is_empty() && !items.contains(&v) {
                    items.push(v);
                }
            }
        }
        items.join(", ")
    };
    for (key, value) in &shown {
        let lower = key.to_lowercase();
        match lower.as_str() {
            "title" => meta.title = Some(value.clone()),
            "author" | "authors" => meta.author = Some(value.clone()),
            "lang" | "language" => meta.language = Some(value.clone()),
            "alias" | "aliases" => {
                let all = joined(&["aliases", "alias"], textweaver_vault::FrontMatter::items);
                meta.properties.insert("aliases".to_owned(), all);
            }
            "tag" | "tags" => {
                let all = joined(&["tags", "tag"], textweaver_vault::FrontMatter::list);
                meta.properties.insert("tags".to_owned(), all);
            }
            "cssclass" | "cssclasses" => {
                let all = joined(
                    &["cssclasses", "cssclass"],
                    textweaver_vault::FrontMatter::list,
                );
                meta.properties.insert("cssclasses".to_owned(), all);
            }
            _ => {
                meta.properties.insert(lower, value.clone());
            }
        }
    }
    if !shown.is_empty() {
        let lines: Vec<String> = shown
            .iter()
            .map(|(k, v)| format!("{}\t{}", one_line(k), one_line(v)))
            .collect();
        meta.properties
            .insert(FRONT_MATTER_PROPERTY.to_owned(), lines.join("\n"));
    }
}

/// Text on one line: line breaks and tabs become spaces.
fn one_line(s: &str) -> String {
    s.split(['\n', '\r', '\t'])
        .filter(|p| !p.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A document's front matter properties, in file order, as `(name,
/// value)` for reading aloud: lists as their items joined by ", ". Empty
/// for a document without front matter.
pub fn front_matter_properties(meta: &DocumentMeta) -> Vec<(String, String)> {
    meta.properties
        .get(FRONT_MATTER_PROPERTY)
        .map(|s| {
            s.lines()
                .filter_map(|l| l.split_once('\t'))
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(block: &str) -> DocumentMeta {
        let mut m = DocumentMeta::default();
        apply_yaml(block, &mut m);
        m
    }

    #[test]
    fn block_and_flow_lists_are_read() {
        let m = meta(
            "title: Cells\naliases:\n  - Cell biology\n  - \"Cells, plural\"\ntags:\n- exam\n- '#review'\nauthors: [Ada Example, Bo Example]\n",
        );
        assert_eq!(m.title.as_deref(), Some("Cells"));
        assert_eq!(m.author.as_deref(), Some("Ada Example, Bo Example"));
        assert_eq!(
            m.properties.get("aliases").map(String::as_str),
            Some("Cell biology, Cells, plural")
        );
        assert_eq!(
            m.properties.get("tags").map(String::as_str),
            Some("exam, review")
        );
        assert_eq!(
            front_matter_properties(&m),
            vec![
                ("title".to_owned(), "Cells".to_owned()),
                (
                    "aliases".to_owned(),
                    "Cell biology, Cells, plural".to_owned()
                ),
                ("tags".to_owned(), "exam, #review".to_owned()),
                ("authors".to_owned(), "Ada Example, Bo Example".to_owned()),
            ]
        );
    }

    #[test]
    fn older_names_nested_maps_and_plain_tags() {
        let m = meta(
            "tag: physics waves\nalias: One name\ncssclass: wide\nnested:\n  a: 1\n  b: 2\ndone: true\nscore: 7\n",
        );
        assert_eq!(
            m.properties.get("tags").map(String::as_str),
            Some("physics, waves")
        );
        assert_eq!(
            m.properties.get("aliases").map(String::as_str),
            Some("One name")
        );
        assert_eq!(
            m.properties.get("cssclasses").map(String::as_str),
            Some("wide")
        );
        assert_eq!(m.properties.get("nested"), None);
        assert_eq!(m.properties.get("done").map(String::as_str), Some("true"));
        let names: Vec<String> = front_matter_properties(&m)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(names, vec!["tag", "alias", "cssclass", "done", "score"]);
    }

    #[test]
    fn no_front_matter_no_properties() {
        assert!(front_matter_properties(&DocumentMeta::default()).is_empty());
        assert!(front_matter_properties(&meta("# just a comment\n")).is_empty());
    }
}
