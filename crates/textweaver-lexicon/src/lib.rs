//! Language and study aids for textweaver.
//!
//! - **Define word, offline** ([`Dictionary`]): the user's own
//!   [`Glossary`] first, then Open English WordNet 2025 (CC BY 4.0), with
//!   pronunciations from CMUdict (BSD-style) respelled for reading aloud
//!   ([`pronounce::respell`]). Inflected words reach their base forms
//!   through [`morphy`] (`running` finds `run`, `geese` finds `goose`).
//! - **The data file** ([`Lexicon`], [`build`]): headwords in an `fst` map,
//!   entries in zstd blocks read with the pure-Rust `ruzstd`, all in one
//!   file of about 10 MB built by `tools/build_lexicon.py` (see
//!   `third_party/lexicon/README.md`). A lookup unpacks a few blocks and
//!   takes about a millisecond.
//! - **Interface messages** ([`i18n`]): a message catalog in a subset of
//!   Fluent's syntax, English complete, with an accented pseudo-locale for
//!   finding untranslated strings and a right-to-left pseudo-locale.
//!
//! Nothing here touches the network: the data file is built once from
//! downloads checked by SHA-256, and shipped.
//!
//! Owner: Agent W3e.

mod codec;
mod data;
mod dictionary;
mod glossary;
pub mod i18n;
mod model;
pub mod morphy;
mod pack;
pub mod pronounce;
pub mod sources;
mod store;

pub use data::{DataInfo, FORMAT, Lexicon, MAGIC, SourceInfo, build, normalize};
pub use dictionary::{
    DATA_FILE, Dictionary, data_file_candidates, find_lexicon, list_items, list_title, pronounced,
    to_markdown,
};
pub use glossary::{Glossary, GlossaryEntry};
pub use model::{Definition, Pos, Pronunciation, Sense, SenseGroup, Source};

/// Lexicon failures.
#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    /// A file could not be read.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a textweaver lexicon.
    #[error("not a textweaver lexicon data file")]
    NotLexicon,
    /// The file is a lexicon of a format this version cannot read.
    #[error("the lexicon data file is format {0}; this textweaver reads format 1")]
    Version(u32),
    /// The file is damaged.
    #[error("the lexicon data file is damaged: {0}")]
    Corrupt(&'static str),
    /// A glossary file could not be read.
    #[error("{path}: {message}")]
    Glossary {
        /// The file.
        path: std::path::PathBuf,
        /// What is wrong.
        message: String,
    },
    /// The source data (WordNet, CMUdict) is not as expected.
    #[error("unexpected source data: {what}")]
    Source {
        /// Where.
        what: String,
    },
    /// Building the data file failed.
    #[error("building the lexicon failed: {0}")]
    Build(String),
}
