//! The user's own glossary, looked up before WordNet.
//!
//! Two formats, chosen by the file's extension:
//!
//! - **JSON** (`.json`), star's custom dictionary format: an object from
//!   word to either a definition string or an object with `definition`
//!   and, optionally, `pronunciation`, `pos`, `examples`, and `synonyms`.
//!   A list of such objects is a word with several senses.
//! - **Text** (anything else, such as `.txt` or `.md`): one entry per line,
//!   `term: definition`, `term = definition`, or `term - definition`
//!   (spaces around the dash). List markers (`- `, `* `), and bold or
//!   italic marks around the term (`**term**:`), are ignored, and lines
//!   starting with `#` are comments. A term on several lines has several
//!   senses.
//!
//! Terms match without regard to case, and through morphy's base forms
//! when WordNet is available (`mitochondria` finds a `mitochondrion`
//! entry only if the glossary has it; `cells` finds `cell`).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::LexiconError;
use crate::data::{normalize, pron};
use crate::model::{Definition, Pos, Pronunciation, Sense, SenseGroup, Source};

/// One glossary sense.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// The term as written.
    pub term: String,
    /// The definition.
    pub definition: String,
    /// The part of speech, if given.
    pub pos: Option<Pos>,
    /// A pronunciation, if given (ARPAbet, or any respelling).
    pub pronunciation: Option<String>,
    /// Examples.
    pub examples: Vec<String>,
    /// Synonyms.
    pub synonyms: Vec<String>,
}

/// A loaded glossary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Glossary {
    entries: BTreeMap<String, Vec<GlossaryEntry>>,
    /// Lines of a text glossary that were not entries (no separator), by
    /// line number from 1, so the user can be told.
    pub skipped: Vec<usize>,
}

impl Glossary {
    /// Loads a glossary file: JSON for `.json`, text otherwise.
    pub fn load(path: &Path) -> Result<Glossary, LexiconError> {
        let bytes = std::fs::read(path).map_err(|source| LexiconError::Io {
            path: path.to_owned(),
            source,
        })?;
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_start_matches('\u{feff}');
        let json = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        if json {
            Glossary::from_json(text).map_err(|e| LexiconError::Glossary {
                path: path.to_owned(),
                message: e,
            })
        } else {
            Ok(Glossary::from_text(text))
        }
    }

    /// A glossary from star's JSON format.
    pub fn from_json(text: &str) -> Result<Glossary, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let Value::Object(map) = v else {
            return Err("the glossary must be a JSON object from word to definition".into());
        };
        let mut g = Glossary::default();
        for (term, value) in map {
            let items = match value {
                Value::Array(a) => a,
                other => vec![other],
            };
            for item in items {
                if let Some(e) = entry_from_json(&term, &item) {
                    g.add(e);
                }
            }
        }
        Ok(g)
    }

    /// A glossary from the text format.
    pub fn from_text(text: &str) -> Glossary {
        let mut g = Glossary::default();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .unwrap_or(line);
            let split = [": ", " = ", " - ", " \u{2013} ", " \u{2014} ", ":\t", "\t"]
                .iter()
                .filter_map(|sep| line.find(sep).map(|i| (i, sep.len())))
                .min_by_key(|(i, _)| *i);
            let Some((i, len)) = split else {
                g.skipped.push(n + 1);
                continue;
            };
            let term = line[..i].trim().trim_matches(['*', '_']).trim();
            let definition = line[i + len..].trim();
            if term.is_empty() || definition.is_empty() {
                g.skipped.push(n + 1);
                continue;
            }
            g.add(GlossaryEntry {
                term: term.to_owned(),
                definition: definition.to_owned(),
                ..GlossaryEntry::default()
            });
        }
        g
    }

    fn add(&mut self, e: GlossaryEntry) {
        let key = normalize(&e.term);
        if !key.is_empty() {
            self.entries.entry(key).or_default().push(e);
        }
    }

    /// How many terms.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the glossary has no terms.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries for a normalized term.
    pub fn get(&self, term: &str) -> Option<&[GlossaryEntry]> {
        self.entries.get(term).map(Vec::as_slice)
    }

    /// The glossary's sense groups for `word` (normalized) or, failing
    /// that, for the first of `bases` it has.
    pub(crate) fn groups(&self, word: &str, bases: &[String]) -> Vec<SenseGroup> {
        let found = std::iter::once(word)
            .chain(bases.iter().map(String::as_str))
            .find_map(|w| self.entries.get(w).map(|e| (w, e)));
        let Some((lemma, entries)) = found else {
            return Vec::new();
        };
        let mut groups: Vec<SenseGroup> = Vec::new();
        for e in entries {
            let sense = Sense {
                definition: e.definition.clone(),
                examples: e.examples.clone(),
                synonyms: e.synonyms.clone(),
                ..Sense::default()
            };
            match groups.iter_mut().find(|g| g.pos == e.pos) {
                Some(g) => g.senses.push(sense),
                None => groups.push(SenseGroup {
                    lemma: lemma.to_owned(),
                    pos: e.pos,
                    senses: vec![sense],
                }),
            }
        }
        groups
    }

    /// The pronunciations the glossary gives for `word` (normalized).
    pub(crate) fn pronunciations(&self, word: &str) -> Vec<Pronunciation> {
        self.entries
            .get(word)
            .into_iter()
            .flatten()
            .filter_map(|e| e.pronunciation.as_deref())
            .map(pron)
            .collect()
    }

    /// Looks `word` up in the glossary alone.
    pub fn define(&self, word: &str) -> Option<Definition> {
        let w = normalize(word);
        let groups = self.groups(&w, &[]);
        (!groups.is_empty()).then(|| Definition {
            pronunciations: self.pronunciations(&w),
            word: w,
            source: Source::Glossary,
            groups,
        })
    }
}

fn entry_from_json(term: &str, v: &Value) -> Option<GlossaryEntry> {
    let strings = |v: Option<&Value>| -> Vec<String> {
        match v {
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => Vec::new(),
        }
    };
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(GlossaryEntry {
            term: term.to_owned(),
            definition: s.trim().to_owned(),
            ..GlossaryEntry::default()
        }),
        Value::Object(o) => {
            let definition = o.get("definition")?.as_str()?.trim().to_owned();
            if definition.is_empty() {
                return None;
            }
            Some(GlossaryEntry {
                term: term.to_owned(),
                definition,
                pos: o.get("pos").and_then(Value::as_str).and_then(Pos::parse),
                pronunciation: o
                    .get("pronunciation")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
                examples: strings(o.get("examples")),
                synonyms: strings(o.get("synonyms")),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_like_star() {
        let g = Glossary::from_json(
            r#"{"Mitosis": "cell division", "osmosis": {"definition": "diffusion of water",
                "pos": "n", "pronunciation": "AA0 Z M OW1 S AH0 S", "examples": ["plants"],
                "synonyms": "diffusion"}, "ATP": [{"definition": "energy"}, "a molecule"],
                "bad": 3, "empty": ""}"#,
        )
        .unwrap();
        assert_eq!(g.len(), 3);
        let d = g.define("mitosis").unwrap();
        assert_eq!(d.source, Source::Glossary);
        assert_eq!(d.groups[0].senses[0].definition, "cell division");
        let d = g.define("Osmosis.").unwrap();
        assert_eq!(d.groups[0].pos, Some(Pos::Noun));
        assert_eq!(d.pronunciations[0].respelling, "ahz-MOH-sus");
        assert_eq!(d.groups[0].senses[0].synonyms, ["diffusion"]);
        assert_eq!(g.define("atp").unwrap().sense_count(), 2);
        assert!(Glossary::from_json("[1]").is_err());
        assert!(Glossary::from_json("{").is_err());
    }

    #[test]
    fn text_formats() {
        let g = Glossary::from_text(
            "# Biology\n\
             - **Cell**: the smallest unit of life\n\
             mitochondrion = the powerhouse of the cell\n\
             ribosome - makes proteins\n\
             well-known \u{2014} famous\n\
             cell: a small room\n\
             no separator here\n\
             : nothing\n",
        );
        assert_eq!(g.len(), 4);
        assert_eq!(g.skipped, [7, 8]);
        assert_eq!(g.define("cell").unwrap().sense_count(), 2);
        assert_eq!(
            g.define("well-known").unwrap().groups[0].senses[0].definition,
            "famous"
        );
        assert!(g.define("ribosomes").is_none());
        let groups = g.groups("ribosomes", &["ribosome".into()]);
        assert_eq!(groups[0].lemma, "ribosome");
    }
}
