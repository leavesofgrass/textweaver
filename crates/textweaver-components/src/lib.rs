//! Optional components: the models, fonts, and voices textweaver can use
//! but does not ship, and the one way they are fetched, checked, and
//! installed (Wave 8, W8a-d).
//!
//! Before this crate, the OCR models, Lexend, and Piper voices each had
//! their own downloader. Now each is a [`Component`]: an id, a title, its
//! license and credit, the features that need it, the folder it lives in
//! (under the data folder), and its files, each pinned by size and hash
//! ([`FilePin`]). The pins are the contract, not the addresses: a file is
//! the same file whether it came from its public source, a mirror, or a
//! zip on a memory stick.
//!
//! - [`download()`] fetches a component's files through a [`Fetcher`]:
//!   each file goes to a `.part` file in a staging folder beside the
//!   component's folder, is checked by size and hash, and is renamed into
//!   place; files that already check out are skipped, a `.part` file left
//!   by a stopped download is resumed, progress is reported, and a flag
//!   cancels. Nothing reaches the component's folder unless it matched.
//! - [`install_from()`] does the same from a downloaded zip or a folder
//!   (managed laptops, accommodation offices): only pinned files that
//!   match are used, and anything else is refused with the reason.
//! - [`Sources`] lists where a file may come from: a mirror first, when
//!   one is set (`[components] mirror` or `TEXTWEAVER_COMPONENTS_MIRROR`),
//!   then the public address. A mirror may carry a [`manifest`] of extra
//!   components in the same format.
//! - Every request carries the neutral [`USER_AGENT`], never anything
//!   about the person or the computer (the owner's rule).
//!
//! Nothing here asks the reader: the app names the size and license and
//! waits for a yes. Tests use [`fake::FakeFetcher`]; only the `download`
//! feature's HTTP fetcher goes to the network, so the lean reader links no
//! HTTP client.

mod component;
mod download;
mod error;
pub mod fake;
mod fetch;
mod install;
pub mod manifest;
mod pin;

pub use component::{Component, FileState, Status};
pub use download::{Outcome, Progress, Sources, Tenths, download};
pub use error::ComponentError;
#[cfg(feature = "download")]
pub use fetch::HttpFetcher;
pub use fetch::{Fetched, Fetcher, StandardFetcher, can_download, fetch_bytes};
pub use install::{InstallReport, install_from};
pub use pin::{Check, FilePin, git_blob_sha1, hash_file, is_plain_name, sha256_hex};

/// The User-Agent sent with every request: the project, nothing personal
/// (the owner's rule).
pub const USER_AGENT: &str = "textweaver-research (+https://github.com/leavesofgrass/textweaver)";

/// The environment variable that names a mirror, tried before the public
/// sources. It wins over the `[components] mirror` setting.
pub const MIRROR_ENV: &str = "TEXTWEAVER_COMPONENTS_MIRROR";

/// A size for a person, in decimal units: "206 KB", "8.0 MB", "79.3 MB",
/// "251 MB".
pub fn size_text(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    if bytes < MB {
        return format!("{} KB", (bytes + 500) / 1000);
    }
    if bytes < 100 * MB {
        let tenths = (bytes + MB / 20) / (MB / 10);
        return format!("{}.{} MB", tenths / 10, tenths % 10);
    }
    format!("{} MB", (bytes + MB / 2) / MB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_well() {
        assert_eq!(size_text(205_828), "206 KB");
        assert_eq!(size_text(8_048_840), "8.0 MB");
        assert_eq!(size_text(79_299_779), "79.3 MB");
        assert_eq!(size_text(251_481_882), "251 MB");
        assert_eq!(size_text(0), "0 KB");
    }
}
