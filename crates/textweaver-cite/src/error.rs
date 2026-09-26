//! Errors. Every message is a full sentence that reads well aloud: it says
//! what happened, what was (or was not) changed, and what to try next.

use std::path::PathBuf;

/// Anything that can go wrong in this crate.
#[derive(Debug, thiserror::Error)]
pub enum CiteError {
    /// Reading or writing a file failed.
    #[error("Could not {action} {path}: {source}.")]
    Io {
        /// What was being done ("read", "write", "create the folder for").
        action: &'static str,
        /// The file.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// A file or string could not be parsed in the given format.
    #[error("Could not read the {format} data{}: {message}.", location_suffix(.line))]
    Parse {
        /// Format name as spoken ("BibTeX", "RIS", "CSL-JSON", "CSL style").
        format: &'static str,
        /// Line number from 1, when known.
        line: Option<usize>,
        /// What was wrong.
        message: String,
    },
    /// Writing data in a format failed.
    #[error("Could not write {format}: {message}.")]
    Serialize {
        /// Format name as spoken.
        format: &'static str,
        /// What was wrong.
        message: String,
    },
    /// A file's format could not be recognized.
    #[error(
        "Could not tell what kind of reference file {path} is. Use a .bib, .ris, or .json file."
    )]
    UnknownFormat {
        /// The file.
        path: PathBuf,
    },
    /// A citation style name is not one textweaver knows.
    #[error(
        "There is no built-in citation style called {name}. Try apa, mla, chicago, ieee, or vancouver, or give the path to a .csl file."
    )]
    UnknownStyle {
        /// The name that was asked for.
        name: String,
    },
    /// A style file could not be used.
    #[error("Could not use the citation style: {message}.")]
    Style {
        /// What was wrong.
        message: String,
    },
    /// Formatting a citation or bibliography failed.
    #[error("Could not format the reference {key}: {message}.")]
    Format {
        /// The reference's key.
        key: String,
        /// What was wrong.
        message: String,
    },
    /// A citation key is not in the library.
    #[error("The citation key {key} is not in the reference library.")]
    MissingKey {
        /// The key.
        key: String,
    },
    /// A DOI or ISBN lookup failed.
    #[error(transparent)]
    Lookup(#[from] LookupError),
}

fn location_suffix(line: &Option<usize>) -> String {
    match line {
        Some(n) => format!(" on line {n}"),
        None => String::new(),
    }
}

/// Why a DOI or ISBN lookup failed. Nothing is added to a library when a
/// lookup fails, and every message says so where it matters.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LookupError {
    /// The text is not a DOI.
    #[error(
        "{input} is not a DOI. A DOI starts with 10, then a dot, and contains a slash, for example 10.1038/nature12373."
    )]
    InvalidDoi {
        /// What was typed.
        input: String,
    },
    /// The text is not a valid ISBN.
    #[error(
        "{input} is not a valid ISBN. An ISBN has 10 or 13 digits, and the last one is a check digit; check for a mistyped digit."
    )]
    InvalidIsbn {
        /// What was typed.
        input: String,
    },
    /// The text is neither a DOI nor an ISBN.
    #[error(
        "{input} is neither a DOI nor an ISBN. Give a DOI such as 10.1038/nature12373, or a 10 or 13 digit ISBN."
    )]
    UnknownIdentifier {
        /// What was typed.
        input: String,
    },
    /// The service has no record for the identifier.
    #[error("{service} has no record for {what}. Nothing was added.")]
    NotFound {
        /// "doi.org" or "Open Library".
        service: &'static str,
        /// "DOI 10.1/x" or "ISBN 9780306406157".
        what: String,
    },
    /// The computer could not reach the service.
    #[error(
        "Could not reach {service}. You may be offline; nothing was added. Try again when you are connected. Details: {detail}"
    )]
    Offline {
        /// The service.
        service: &'static str,
        /// The underlying error, for troubleshooting.
        detail: String,
    },
    /// The service did not answer in time.
    #[error(
        "{service} did not answer within {seconds} seconds. Nothing was added; try again later."
    )]
    Timeout {
        /// The service.
        service: &'static str,
        /// The timeout.
        seconds: u64,
    },
    /// The service answered with an HTTP error.
    #[error(
        "{service} answered with an error, status {status}. Nothing was added; try again later."
    )]
    Http {
        /// The service.
        service: &'static str,
        /// HTTP status code.
        status: u16,
    },
    /// The service's answer could not be understood.
    #[error("{service} sent an answer textweaver could not read: {detail}. Nothing was added.")]
    BadResponse {
        /// The service.
        service: &'static str,
        /// What was wrong.
        detail: String,
    },
}

/// Shorthand for results in this crate.
pub type Result<T, E = CiteError> = std::result::Result<T, E>;

impl CiteError {
    pub(crate) fn io(
        action: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        CiteError::Io {
            action,
            path: path.into(),
            source,
        }
    }

    pub(crate) fn parse(
        format: &'static str,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        CiteError::Parse {
            format,
            line,
            message: message.into(),
        }
    }
}
