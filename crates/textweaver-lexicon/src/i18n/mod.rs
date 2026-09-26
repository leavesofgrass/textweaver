//! Message catalogs for the interface: the words textweaver says and shows,
//! kept apart from the code so they can be translated.
//!
//! # Why not the `fluent` crates
//!
//! The catalogs are written in a subset of [Project Fluent]'s syntax, read
//! by a small parser here, instead of with `fluent-bundle`:
//!
//! - it needs no new dependencies (`fluent-bundle` brings `intl-memoizer`,
//!   `unic-langid`, `intl_pluralrules`, and `self_cell`), and the reader
//!   stays small;
//! - the catalog is data a frontend passes around ([`Catalog`], shared in
//!   an `Arc`), not global state, so tests can render in several
//!   languages at once;
//! - the files are valid Fluent, so Wave 4 (W4d, translations) can move to
//!   `fluent-bundle` without rewriting them if a language needs more than
//!   this subset.
//!
//! The subset: messages (`id = text`), terms (`-brand = textweaver`),
//! comments, multiline text, and placeables for variables (`{ $word }`),
//! terms and other messages (`{ -brand }`, `{ other-id }`), string
//! literals (`{ "{" }`), and selectors on a variable with one-line variants
//! (`{ $n -> [one] one sense *[other] { $n } senses }`), where a number
//! chooses by its exact value (`[0]`) or its plural category. Attributes
//! and functions are not supported.
//!
//! # Languages
//!
//! - `en`: English, complete: every message the code asks for is in
//!   `locales/en.ftl` (a test scans the sources).
//! - `en-XA`: a pseudo-locale for testing coverage. Every message comes
//!   out accented, a third longer, and in `⟦ ⟧` brackets, so a string
//!   shown without brackets was never looked up in a catalog, and layouts
//!   that cannot take longer text show it.
//! - `ar-XB`: a right-to-left pseudo-locale. Its text is wrapped in
//!   right-to-left override marks, it reports [`Direction::RightToLeft`],
//!   and the values put into messages are isolated (U+2068 to U+2069), as
//!   they are for every right-to-left language, so a word or number in a
//!   message cannot reorder the text around it. [`bidi_problems`] checks
//!   that a rendered string's direction marks are balanced.
//! - Other languages load from `<code>.ftl` files, falling back to English
//!   for any message they lack.
//!
//! [Project Fluent]: https://projectfluent.org/

mod parse;
pub mod plural;
mod pseudo;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};

pub use parse::CatalogError;
use parse::Part;
pub use pseudo::bidi_problems;
use pseudo::pseudo_text;

/// The English catalog's source.
const ENGLISH: &str = include_str!("../../locales/en.ftl");

/// The accented pseudo-locale.
pub const PSEUDO_ACCENTED: &str = "en-XA";
/// The right-to-left pseudo-locale.
pub const PSEUDO_RTL: &str = "ar-XB";

/// First strong isolate: the start of an isolated value.
pub const FSI: char = '\u{2068}';
/// Pop directional isolate: the end of an isolated value.
pub const PDI: char = '\u{2069}';

/// Nested message and term references followed before giving up.
const MAX_DEPTH: usize = 8;

/// Which way a language's text runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Left to right.
    LeftToRight,
    /// Right to left.
    RightToLeft,
}

/// True when `lang` (a language tag such as `ar`, `he-IL`, or the
/// pseudo-locale `ar-XB`) is written right to left.
pub fn is_rtl(lang: &str) -> bool {
    let primary = lang
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        primary.as_str(),
        "ar" | "he" | "iw" | "fa" | "ur" | "ps" | "sd" | "ug" | "yi" | "dv" | "ckb"
    )
}

/// A value put into a message.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// Text.
    Str(String),
    /// A whole number, which also chooses plural variants.
    Num(i64),
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Str(s.to_owned())
    }
}

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Str(s)
    }
}

impl From<&String> for Arg {
    fn from(s: &String) -> Self {
        Arg::Str(s.clone())
    }
}

macro_rules! num_arg {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(n: $t) -> Self {
                Arg::Num(i64::try_from(n).unwrap_or(i64::MAX))
            }
        }
    )*};
}
num_arg!(i32, i64, u8, u16, u32, u64, usize);

/// A language's messages.
#[derive(Clone, Debug)]
pub struct Catalog {
    lang: String,
    messages: HashMap<String, Vec<Part>>,
    terms: HashMap<String, Vec<Part>>,
    fallback: Option<Arc<Catalog>>,
    pseudo: Option<pseudo::Kind>,
}

impl Catalog {
    /// Parses a catalog in `lang` from Fluent text. Every problem is
    /// reported, with its line.
    pub fn parse(lang: &str, text: &str) -> Result<Catalog, Vec<CatalogError>> {
        let (messages, terms) = parse::resource(text)?;
        Ok(Catalog {
            lang: lang.to_owned(),
            messages,
            terms,
            fallback: None,
            pseudo: None,
        })
    }

    /// The English catalog, parsed once.
    pub fn english() -> Arc<Catalog> {
        static EN: OnceLock<Arc<Catalog>> = OnceLock::new();
        EN.get_or_init(|| {
            Arc::new(Catalog::parse("en", ENGLISH).unwrap_or_else(|errors| {
                // A test keeps en.ftl valid; never fail at run time.
                for e in &errors {
                    log::error!("en.ftl: {e}");
                }
                Catalog {
                    lang: "en".into(),
                    messages: HashMap::new(),
                    terms: HashMap::new(),
                    fallback: None,
                    pseudo: None,
                }
            }))
        })
        .clone()
    }

    /// The catalog for `lang`: English, a pseudo-locale, or `<lang>.ftl`
    /// from `dir` over English. An unknown or unreadable language gives
    /// English and a message saying why.
    pub fn for_language(lang: &str, dir: Option<&Path>) -> (Arc<Catalog>, Option<String>) {
        let lang = lang.trim();
        let en = Catalog::english();
        if lang.is_empty()
            || lang.eq_ignore_ascii_case("en")
            || lang.starts_with("en-") && !is_pseudo(lang)
        {
            return (en, None);
        }
        if lang.eq_ignore_ascii_case(PSEUDO_ACCENTED) {
            return (Arc::new(en.pseudo(pseudo::Kind::Accented)), None);
        }
        if lang.eq_ignore_ascii_case(PSEUDO_RTL) {
            return (Arc::new(en.pseudo(pseudo::Kind::Bidi)), None);
        }
        let Some(dir) = dir else {
            return (
                en,
                Some(format!("No translation for {lang} yet; using English.")),
            );
        };
        let path = dir.join(format!("{lang}.ftl"));
        match std::fs::read_to_string(&path) {
            Ok(text) => match Catalog::parse(lang, &text) {
                Ok(mut c) => {
                    c.fallback = Some(en);
                    (Arc::new(c), None)
                }
                Err(errors) => (
                    en,
                    Some(format!(
                        "{} has {} problems, the first: {}; using English.",
                        path.display(),
                        errors.len(),
                        errors[0]
                    )),
                ),
            },
            Err(_) => (
                en,
                Some(format!("No translation for {lang} yet; using English.")),
            ),
        }
    }

    fn pseudo(&self, kind: pseudo::Kind) -> Catalog {
        Catalog {
            lang: match kind {
                pseudo::Kind::Accented => PSEUDO_ACCENTED.into(),
                pseudo::Kind::Bidi => PSEUDO_RTL.into(),
            },
            messages: self.messages.clone(),
            terms: self.terms.clone(),
            fallback: None,
            pseudo: Some(kind),
        }
    }

    /// The language tag.
    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// Which way the language runs.
    pub fn direction(&self) -> Direction {
        if is_rtl(&self.lang) {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        }
    }

    /// True when the catalog (or its English fallback) has message `id`.
    pub fn has(&self, id: &str) -> bool {
        self.messages.contains_key(id) || self.fallback.as_ref().is_some_and(|f| f.has(id))
    }

    /// The ids of this catalog's own messages, sorted.
    pub fn ids(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.messages.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    /// The variables message `id` uses (`$word` as `word`), including those
    /// of the messages and terms it refers to.
    pub fn variables(&self, id: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        if let Some(parts) = self.messages.get(id) {
            parse::collect_vars(parts, &mut out);
        }
        out
    }

    /// Message `id` with no values.
    pub fn tr(&self, id: &str) -> String {
        self.fmt(id, &[])
    }

    /// Message `id` with `args`. A message missing everywhere gives its id
    /// (and a warning in the log), so nothing is ever silent.
    pub fn fmt(&self, id: &str, args: &[(&str, Arg)]) -> String {
        let Some(parts) = self.messages.get(id) else {
            if let Some(f) = &self.fallback {
                return f.fmt(id, args);
            }
            log::warn!("no message {id} in the {} catalog", self.lang);
            return id.to_owned();
        };
        let mut out = String::new();
        self.write(parts, args, &mut out, 0);
        match self.pseudo {
            Some(pseudo::Kind::Accented) => format!("⟦{out}⟧"),
            _ => out,
        }
    }

    fn write(&self, parts: &[Part], args: &[(&str, Arg)], out: &mut String, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let isolate = self.direction() == Direction::RightToLeft;
        for p in parts {
            match p {
                Part::Text(t) => match self.pseudo {
                    Some(kind) => out.push_str(&pseudo_text(kind, t)),
                    None => out.push_str(t),
                },
                Part::Var(name) => {
                    let value = match args.iter().find(|(k, _)| k == name) {
                        Some((_, Arg::Str(s))) => s.clone(),
                        Some((_, Arg::Num(n))) => n.to_string(),
                        None => format!("{{${name}}}"),
                    };
                    if isolate {
                        out.push(FSI);
                        out.push_str(&value);
                        out.push(PDI);
                    } else {
                        out.push_str(&value);
                    }
                }
                Part::Term(name) => {
                    if let Some(t) = self.terms.get(name) {
                        self.write(t, args, out, depth + 1);
                    }
                }
                Part::Ref(id) => {
                    if let Some(m) = self.messages.get(id) {
                        self.write(m, args, out, depth + 1);
                    } else if let Some(f) = &self.fallback {
                        out.push_str(&f.fmt(id, args));
                    }
                }
                Part::Select {
                    var,
                    variants,
                    default,
                } => {
                    let chosen = match args.iter().find(|(k, _)| k == var) {
                        Some((_, Arg::Num(n))) => {
                            let exact = n.to_string();
                            let cat = plural::category(&self.lang, *n).name();
                            variants
                                .iter()
                                .position(|(k, _)| *k == exact)
                                .or_else(|| variants.iter().position(|(k, _)| k == cat))
                        }
                        Some((_, Arg::Str(s))) => variants.iter().position(|(k, _)| k == s),
                        None => None,
                    }
                    .unwrap_or(*default);
                    if let Some((_, v)) = variants.get(chosen) {
                        self.write(v, args, out, depth + 1);
                    }
                }
            }
        }
    }
}

fn is_pseudo(lang: &str) -> bool {
    lang.eq_ignore_ascii_case(PSEUDO_ACCENTED) || lang.eq_ignore_ascii_case(PSEUDO_RTL)
}

/// Builds the argument list for [`Catalog::fmt`]:
/// `args!["word" => w, "n" => 3]`.
#[macro_export]
macro_rules! args {
    ($($k:literal => $v:expr),* $(,)?) => {
        [$(($k, $crate::i18n::Arg::from($v))),*]
    };
}

#[cfg(test)]
mod tests;
