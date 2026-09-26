//! Define word: the user's glossary first, then Open English WordNet, with
//! CMUdict pronunciations; and the words a frontend shows and says for a
//! result, from the message catalog.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::data::{Lexicon, normalize};
use crate::glossary::Glossary;
use crate::i18n::Catalog;
use crate::model::{Definition, Source};
use crate::{LexiconError, args};

/// The name of the data file textweaver ships.
pub const DATA_FILE: &str = "lexicon-en.twlex";

/// The glossary and the lexicon, either of which may be missing.
#[derive(Clone, Debug, Default)]
pub struct Dictionary {
    /// The user's glossary.
    pub glossary: Option<Arc<Glossary>>,
    /// The WordNet and CMUdict data.
    pub lexicon: Option<Arc<Lexicon>>,
}

impl Dictionary {
    /// Looks `word` up: the glossary's senses (for the word or its base
    /// forms) come first, then WordNet's. The pronunciation is the
    /// glossary's when it gives one, else CMUdict's. `None` when neither
    /// knows the word.
    pub fn define(&self, word: &str) -> Result<Option<Definition>, LexiconError> {
        let w = normalize(word);
        if w.is_empty() {
            return Ok(None);
        }
        let wordnet = match &self.lexicon {
            Some(l) => l.define(&w)?,
            None => None,
        };
        let Some(g) = &self.glossary else {
            return Ok(wordnet);
        };
        let bases: Vec<String> = match &self.lexicon {
            Some(l) => l.base_forms(&w)?.into_iter().map(|(f, _)| f).collect(),
            None => Vec::new(),
        };
        let groups = g.groups(&w, &bases);
        if groups.is_empty() {
            return Ok(wordnet);
        }
        let mut pronunciations = g.pronunciations(&w);
        let mut rest = Vec::new();
        if let Some(d) = wordnet {
            if pronunciations.is_empty() {
                pronunciations = d.pronunciations;
            }
            rest = d.groups;
        }
        let mut all = groups;
        all.extend(rest);
        Ok(Some(Definition {
            word: w,
            source: Source::Glossary,
            pronunciations,
            groups: all,
        }))
    }
}

/// Where to look for the data file, in order: `explicit` (a setting),
/// `TEXTWEAVER_LEXICON`, next to the program (`lexicon/` beside it, and
/// `../share/textweaver/lexicon/` and `../Resources/lexicon/` for the
/// Linux and macOS layouts), the user's data folder, and the source tree
/// the program was built from.
pub fn data_file_candidates(explicit: Option<&Path>, data_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = explicit {
        out.push(p.to_owned());
    }
    if let Some(p) = std::env::var_os("TEXTWEAVER_LEXICON") {
        out.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.join("lexicon").join(DATA_FILE));
        out.push(dir.join("../share/textweaver/lexicon").join(DATA_FILE));
        out.push(dir.join("../Resources/lexicon").join(DATA_FILE));
    }
    if let Some(d) = data_dir {
        out.push(d.join("lexicon").join(DATA_FILE));
    }
    out.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/lexicon")
            .join(DATA_FILE),
    );
    out
}

/// Opens the first data file found among [`data_file_candidates`].
/// `Ok(None)` when there is none; an error when one is found but cannot be
/// read.
pub fn find_lexicon(
    explicit: Option<&Path>,
    data_dir: Option<&Path>,
) -> Result<Option<(PathBuf, Lexicon)>, LexiconError> {
    for p in data_file_candidates(explicit, data_dir) {
        if p.is_file() {
            return Lexicon::open(&p).map(|l| Some((p, l)));
        }
        if explicit.is_some_and(|e| e == p) {
            return Err(LexiconError::Io {
                path: p,
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            });
        }
    }
    Ok(None)
}

/// A result as list items for a frontend's accessible list: the
/// pronunciation, then one item per sense with its examples, synonyms,
/// opposites, and what it is a kind of. Every item reads well aloud on its
/// own.
pub fn list_items(c: &Catalog, d: &Definition) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(p) = pronounced(c, d) {
        items.push(p);
    }
    for g in &d.groups {
        let n = g.senses.len();
        let pos = g.pos.map(|p| c.tr(&format!("pos-{}", p.name())));
        for (i, s) in g.senses.iter().enumerate() {
            let head = match &pos {
                Some(pos) => c.fmt(
                    "define-sense-head",
                    &args!["lemma" => &g.lemma, "pos" => pos, "i" => i + 1, "n" => n],
                ),
                None => c.fmt(
                    "define-sense-head-nopos",
                    &args!["lemma" => &g.lemma, "i" => i + 1, "n" => n],
                ),
            };
            let definition = s.definition.trim().trim_end_matches('.');
            let mut item = c.fmt(
                "define-sense",
                &args!["head" => head, "definition" => definition],
            );
            if let Some(ex) = s.examples.first() {
                item.push(' ');
                item.push_str(&c.fmt("define-example", &args!["text" => ex]));
            }
            if !s.synonyms.is_empty() {
                item.push(' ');
                item.push_str(&c.fmt("define-synonyms", &args!["words" => s.synonyms.join(", ")]));
            }
            if !s.antonyms.is_empty() {
                item.push(' ');
                item.push_str(&c.fmt("define-antonyms", &args!["words" => s.antonyms.join(", ")]));
            }
            if !s.kind_of.is_empty() {
                item.push(' ');
                item.push_str(&c.fmt("define-kind-of", &args!["words" => s.kind_of.join(", ")]));
            }
            items.push(item);
        }
    }
    items
}

/// "Pronounced RUN-ing." (with a second pronunciation when there is one),
/// or `None` when there is none.
pub fn pronounced(c: &Catalog, d: &Definition) -> Option<String> {
    let shown: Vec<String> = d
        .pronunciations
        .iter()
        .take(2)
        .map(|p| {
            if p.respelling.is_empty() {
                p.arpabet.clone()
            } else {
                p.respelling.clone()
            }
        })
        .collect();
    match shown.as_slice() {
        [] => None,
        [one] => Some(c.fmt("define-pronounced", &args!["say" => one])),
        [one, two, ..] => Some(c.fmt("define-pronounced-or", &args!["say" => one, "other" => two])),
    }
}

/// The list's title: "Definitions of dog, 7 senses, from Open English
/// WordNet".
pub fn list_title(c: &Catalog, d: &Definition) -> String {
    let source = match d.source {
        Source::Glossary => c.tr("source-glossary"),
        Source::WordNet => c.tr("source-wordnet"),
        Source::Pronunciation => c.tr("source-cmudict"),
    };
    c.fmt(
        "define-title",
        &args!["word" => &d.word, "n" => d.sense_count(), "source" => source],
    )
}

/// The result as Markdown, for `tw define` and notes: a heading, the
/// pronunciation (with the ARPAbet), and the senses grouped by headword
/// and part of speech.
pub fn to_markdown(c: &Catalog, d: &Definition) -> String {
    let mut out = format!("# {}\n\n", d.word);
    if let Some(p) = pronounced(c, d) {
        let arpabet: Vec<&str> = d
            .pronunciations
            .iter()
            .map(|p| p.arpabet.as_str())
            .collect();
        out.push_str(&format!("{p} (`{}`)\n\n", arpabet.join("`, `")));
    }
    for g in &d.groups {
        let heading = match g.pos {
            Some(pos) => format!("{}, {}", g.lemma, c.tr(&format!("pos-{}", pos.name()))),
            None => g.lemma.clone(),
        };
        out.push_str(&format!("## {heading}\n\n"));
        for (i, s) in g.senses.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, s.definition));
            for ex in &s.examples {
                out.push_str(&format!(
                    "   - {}\n",
                    c.fmt("define-example", &args!["text" => ex])
                ));
            }
            for (id, words) in [
                ("define-synonyms", &s.synonyms),
                ("define-antonyms", &s.antonyms),
                ("define-kind-of", &s.kind_of),
            ] {
                if !words.is_empty() {
                    out.push_str(&format!(
                        "   - {}\n",
                        c.fmt(id, &args!["words" => words.join(", ")])
                    ));
                }
            }
        }
        out.push('\n');
    }
    out.push_str(&format!("{}\n", list_title(c, d)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::tests::tiny;

    #[test]
    fn glossary_first_then_wordnet() {
        let dict = Dictionary {
            glossary: Some(Arc::new(Glossary::from_text("dog: a loyal friend\n"))),
            lexicon: Some(Arc::new(tiny())),
        };
        let d = dict.define("dogs").unwrap().unwrap();
        assert_eq!(d.source, Source::Glossary);
        assert_eq!(d.groups[0].senses[0].definition, "a loyal friend");
        assert_eq!(
            d.groups[1].senses[0].definition,
            "a member of the genus Canis"
        );
        assert_eq!(d.pronunciations[0].respelling, "DAWG");
        let d = dict.define("cat").unwrap().unwrap();
        assert_eq!(d.source, Source::WordNet);
        assert!(dict.define("qqq").unwrap().is_none());
        // Glossary alone.
        let only = Dictionary {
            glossary: dict.glossary.clone(),
            lexicon: None,
        };
        assert_eq!(only.define("Dog").unwrap().unwrap().sense_count(), 1);
        assert!(Dictionary::default().define("dog").unwrap().is_none());
    }

    #[test]
    fn items_read_well() {
        let c = Catalog::english();
        let d = tiny().define("dog").unwrap().unwrap();
        let items = list_items(&c, &d);
        assert_eq!(items[0], "Pronounced DAWG.");
        assert_eq!(
            items[1],
            "dog, noun, 1 of 1: a member of the genus Canis. For example: the dog barked all night. \
             Synonyms: domestic dog. Opposite: cat. A kind of: canine."
        );
        assert_eq!(
            list_title(&c, &d),
            "Definitions of dog, 1 sense, from Open English WordNet"
        );
        let md = to_markdown(&c, &d);
        assert!(
            md.starts_with("# dog\n\nPronounced DAWG. (`D AO1 G`)\n\n## dog, noun\n\n1. a member")
        );
        let the = tiny().define("the").unwrap().unwrap();
        assert_eq!(list_items(&c, &the), ["Pronounced thuh, or thee."]);
        assert_eq!(
            list_title(&c, &the),
            "Pronunciation of the, from the CMU Pronouncing Dictionary"
        );
    }

    #[test]
    fn finds_the_shipped_file_in_the_source_tree() {
        let c = data_file_candidates(Some(Path::new("x.twlex")), Some(Path::new("data")));
        assert_eq!(c[0], Path::new("x.twlex"));
        assert!(
            c.iter()
                .any(|p| p.ends_with(Path::new("third_party/lexicon").join(DATA_FILE)))
        );
        assert!(find_lexicon(Some(Path::new("no/such/file.twlex")), None).is_err());
    }
}
