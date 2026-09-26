//! Citations (ADR-0019): a reference library, DOI and ISBN lookup, import
//! and export in BibTeX, BibLaTeX, RIS, and CSL-JSON, formatting with CSL
//! styles, and Pandoc citation keys for inserting citations while writing.
//!
//! Owner: Agent P.
//!
//! # Pieces
//!
//! - [`Reference`]: one CSL-JSON item, the storage model.
//! - [`Library`]: references with unique keys, saved as CSL-JSON, per user
//!   ([`user_library_path`]) or per document folder
//!   ([`folder_library_path`]); [`Layered`] resolves keys folder first.
//! - [`formats`]: read and write BibTeX, BibLaTeX, RIS, CSL-JSON.
//! - [`lookup`]: DOI (doi.org) and ISBN (Open Library) lookup with a cache.
//! - [`CitationStyle`] and [`Formatter`]: CSL formatting through hayagriva,
//!   as plain text, Markdown, or HTML.
//! - [`pandoc`]: find `[@key, p. 12]` citations in text and write them.
//! - [`insert`]: what an editor needs to pick, insert, and announce
//!   citations.
//! - [`commands`]: the logic behind `tw cite`, testable without a terminal.
//!
//! # Example
//!
//! ```
//! use textweaver_cite::{CitationStyle, Formatter, Library, OutputFormat, formats, pandoc};
//!
//! let bib = "@book{doe2020, author = {Doe, Jane}, title = {Reading Machines}, \
//!            publisher = {Accessible Press}, year = 2020}";
//! let lib = Library::from_references(formats::parse(bib, formats::Format::BibLatex)?);
//! let style = CitationStyle::builtin("apa")?;
//! let fmt = Formatter::new(&style, OutputFormat::Plain);
//! let cites = pandoc::find_citations("As shown [@doe2020, p. 12].");
//! let doc = fmt.document(&cites, &lib)?;
//! assert_eq!(doc.citations[0], "(Doe, 2020, p. 12)");
//! assert_eq!(doc.bibliography[0].text, "Doe, J. (2020). Reading Machines. Accessible Press.");
//! # Ok::<(), textweaver_cite::CiteError>(())
//! ```

pub mod bibtex;
pub mod commands;
pub mod csljson;
mod error;
pub mod formats;
pub mod insert;
pub mod key;
pub mod library;
pub mod lookup;
pub mod pandoc;
mod reference;
mod render;
pub mod ris;
mod style;
pub mod text;

pub use error::{CiteError, LookupError, Result};
pub use formats::Format;
pub use library::{
    AddOutcome, Layered, Library, MergeReport, ReferenceSource, folder_library_path,
    user_library_path,
};
pub use lookup::{
    Cache, HttpClient, HttpResponse, Identifier, Lookup, RecordedClient, TransportError, UreqClient,
};
pub use pandoc::{Citation, CiteItem, Locator, LocatorLabel};
pub use reference::{CslDate, Name, Reference};
pub use render::{FormattedEntry, Formatter, OutputFormat, RenderedDocument};
pub use style::{BuiltinStyle, CitationStyle, builtin_styles};
