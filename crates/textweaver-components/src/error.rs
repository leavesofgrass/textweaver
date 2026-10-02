//! Why a component could not be downloaded, installed, or removed.

use std::path::PathBuf;

/// Why a component could not be downloaded, installed, or removed. The
/// texts are for logs and command lines; the app puts each kind into its
/// own message, in the reader's language.
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    /// Every source failed (no network, a server error, a missing file).
    #[error("{file}: {reason}")]
    Fetch {
        /// The file.
        file: String,
        /// What went wrong, from the last source tried.
        reason: String,
    },
    /// The file arrived with the wrong size.
    #[error("{file} is {got} bytes, not {expected}")]
    Size {
        /// The file.
        file: String,
        /// Bytes received.
        got: u64,
        /// Bytes expected.
        expected: u64,
    },
    /// The file does not match its pinned hash.
    #[error("{file} does not match its published hash")]
    Hash {
        /// The file.
        file: String,
    },
    /// A pinned file is not in the zip or folder being installed from.
    #[error("{file} is not in {}", from.display())]
    Missing {
        /// The file.
        file: String,
        /// The zip or folder.
        from: PathBuf,
    },
    /// The download was cancelled. A `.part` file is kept, so the next
    /// download goes on from where this one stopped.
    #[error("the download was cancelled")]
    Cancelled,
    /// Another download or install of the same component is under way, in
    /// this program or another.
    #[error("{0} is already being downloaded")]
    Busy(String),
    /// A file has no source: no public address and no mirror set.
    #[error("{file} has no address to download from, and no mirror is set")]
    NoSource {
        /// The file.
        file: String,
    },
    /// A name or folder that is not plain (from a mirror's manifest or a
    /// zip), refused before anything is written.
    #[error("{0} is not a plain name")]
    BadName(String),
    /// A mirror's manifest could not be read.
    #[error("the mirror's list of components: {0}")]
    Manifest(String),
    /// Reading or writing a file failed.
    #[error("{}: {source}", path.display())]
    Io {
        /// Where.
        path: PathBuf,
        /// The error.
        source: std::io::Error,
    },
}

impl ComponentError {
    /// An I/O error at `path`.
    pub fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        ComponentError::Io {
            path: path.to_owned(),
            source,
        }
    }
}
