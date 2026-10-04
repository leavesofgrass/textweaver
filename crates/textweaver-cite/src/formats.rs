//! Reference file formats: detection, import, and export.

use std::path::Path;

use crate::error::{CiteError, Result};
use crate::reference::Reference;
use crate::{bibtex, csljson, ris};

/// A reference interchange format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Classic BibTeX (`.bib`).
    BibTex,
    /// BibLaTeX (`.bib`, BibLaTeX field names).
    BibLatex,
    /// RIS (`.ris`).
    Ris,
    /// CSL-JSON (`.json`), the library's own format.
    CslJson,
}

impl Format {
    /// Every format, in the order menus list them.
    pub const ALL: [Format; 4] = [
        Format::BibTex,
        Format::BibLatex,
        Format::Ris,
        Format::CslJson,
    ];

    /// The format for a name as typed on the command line: `bibtex`, `bib`,
    /// `biblatex`, `ris`, `csl-json`, `csljson`, `json`, `csl`.
    pub fn from_name(name: &str) -> Option<Format> {
        match name.trim().to_ascii_lowercase().as_str() {
            "bibtex" | "bib" => Some(Format::BibTex),
            "biblatex" => Some(Format::BibLatex),
            "ris" => Some(Format::Ris),
            "csl-json" | "csljson" | "csl_json" | "json" | "csl" => Some(Format::CslJson),
            _ => None,
        }
    }

    /// The format a file extension implies (`.bib` is read as BibLaTeX,
    /// which is a superset of BibTeX).
    pub fn from_extension(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "bib" | "bibtex" => Some(Format::BibLatex),
            "ris" => Some(Format::Ris),
            "json" | "csljson" => Some(Format::CslJson),
            _ => None,
        }
    }

    /// Guesses the format from content: `@type{` is BibTeX, a `TY  -` line
    /// is RIS, `[` or `{` is CSL-JSON (star's heuristic, kept).
    pub fn sniff(text: &str) -> Option<Format> {
        let t = text.trim_start_matches('\u{feff}').trim_start();
        if t.starts_with('[') || t.starts_with('{') {
            return Some(Format::CslJson);
        }
        if t.lines()
            .any(|l| l.starts_with("TY  -") || l.starts_with("TY -"))
        {
            return Some(Format::Ris);
        }
        if t.starts_with('@') || t.contains("\n@") || t.contains("\r\n@") {
            return Some(Format::BibLatex);
        }
        None
    }

    /// The usual file extension, without a dot.
    pub fn extension(self) -> &'static str {
        match self {
            Format::BibTex | Format::BibLatex => "bib",
            Format::Ris => "ris",
            Format::CslJson => "json",
        }
    }

    /// The name as spoken and shown in menus.
    pub fn display_name(self) -> &'static str {
        match self {
            Format::BibTex => "BibTeX",
            Format::BibLatex => "BibLaTeX",
            Format::Ris => "RIS",
            Format::CslJson => "CSL-JSON",
        }
    }
}

/// Parses references in the given format.
pub fn parse(text: &str, format: Format) -> Result<Vec<Reference>> {
    match format {
        Format::BibTex | Format::BibLatex => bibtex::parse(text),
        Format::Ris => Ok(ris::parse(text)),
        Format::CslJson => csljson::parse(text),
    }
}

/// Writes references in the given format.
pub fn write(refs: &[Reference], format: Format) -> Result<String> {
    match format {
        Format::BibTex => Ok(bibtex::write(refs, bibtex::Dialect::BibTex)),
        Format::BibLatex => Ok(bibtex::write(refs, bibtex::Dialect::BibLatex)),
        Format::Ris => Ok(ris::write(refs)),
        Format::CslJson => csljson::write(refs),
    }
}

/// Reads a reference file, choosing the format by extension and then by
/// content. Invalid UTF-8 is replaced rather than rejected, as star did.
/// References without a key get one (see [`crate::key`]).
pub fn read_file(path: &Path) -> Result<Vec<Reference>> {
    let bytes = std::fs::read(path).map_err(|e| CiteError::io("read", path, e))?;
    let text = String::from_utf8_lossy(&bytes);
    let format = Format::from_extension(path)
        .or_else(|| Format::sniff(&text))
        .ok_or_else(|| CiteError::UnknownFormat {
            path: path.to_owned(),
        })?;
    let mut refs = parse(&text, format)?;
    crate::library::fill_missing_keys(&mut refs, |_| false);
    Ok(refs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_extensions_and_content() {
        assert_eq!(Format::from_name("BibTeX"), Some(Format::BibTex));
        assert_eq!(Format::from_name("csl-json"), Some(Format::CslJson));
        assert_eq!(Format::from_name("docx"), None);
        assert_eq!(
            Format::from_extension(Path::new("a.RIS")),
            Some(Format::Ris)
        );
        assert_eq!(
            Format::sniff("@article{z, title={Zed}}"),
            Some(Format::BibLatex)
        );
        assert_eq!(Format::sniff("TY  - JOUR\nER  - "), Some(Format::Ris));
        assert_eq!(Format::sniff(" [ ]"), Some(Format::CslJson));
        assert_eq!(Format::sniff("hello"), None);
    }
}
