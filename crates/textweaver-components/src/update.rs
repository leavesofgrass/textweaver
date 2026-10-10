//! Updating textweaver itself (B1-u1): reading the public list of
//! releases, choosing this computer's package, and downloading it checked
//! against the release's `SHA256SUMS.txt`.
//!
//! - [`check`] reads GitHub's public list of textweaver's releases (no
//!   sign-in, the neutral [`USER_AGENT`](crate::USER_AGENT), nothing about
//!   the person or the computer) and offers the newest release that is
//!   newer than this one, pre-release or stable, when it has a package for
//!   this computer ([`Target`]).
//! - [`download_update`] fetches the release's `SHA256SUMS.txt`, then the
//!   package through the one downloader ([`download()`](crate::download)),
//!   pinned by the size GitHub lists and the SHA-256 the checksum file
//!   gives. A package that does not match is refused and nothing is kept.
//! - Installing is [`crate::swap`]'s.
//!
//! GitHub also attests every package (build provenance, Sigstore). Checking
//! an attestation needs a Sigstore verifier (certificate chains, the
//! transparency log), far more than a few lines and no crate in the tree
//! does it, so textweaver relies on the checksum file read over HTTPS from
//! GitHub; `gh attestation verify` checks a package by hand
//! (docs/updates.md).

use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::component::Component;
use crate::download::{Progress, Sources};
use crate::error::ComponentError;
use crate::fetch::{Fetcher, fetch_bytes};
use crate::pin::{Check, FilePin};

/// The public list of textweaver's releases, newest first.
pub const RELEASES: &str = "https://api.github.com/repos/leavesofgrass/textweaver/releases";

/// The checksum file every release carries.
pub const SUMS_FILE: &str = "SHA256SUMS.txt";

/// The most bytes read of the release list.
const MAX_LIST_BYTES: u64 = 8 * 1024 * 1024;

/// The most bytes read of a checksum file.
const MAX_SUMS_BYTES: u64 = 1024 * 1024;

/// Why an update could not be checked, downloaded, or installed. The texts
/// are for logs and `tw`; the app says each kind in its own message.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The release list could not be read (no network, a server error).
    #[error("the release list: {0}")]
    Fetch(String),
    /// The release list was not what GitHub sends.
    #[error("the release list is unreadable")]
    Unreadable,
    /// The newest release has no package for this computer.
    #[error("release {0} has no package for this computer")]
    NoPackage(String),
    /// The checksum file does not list the package.
    #[error("{0} is not in SHA256SUMS.txt")]
    NoChecksum(String),
    /// The package does not match its checksum or its listed size.
    #[error("{0} does not match its checksum")]
    Mismatch(String),
    /// The download was stopped; it goes on from there next time.
    #[error("the download was canceled")]
    Cancelled,
    /// The download failed.
    #[error("{0}")]
    Download(String),
    /// textweaver is not running from a release package (a build from
    /// source, a system package), so it is updated the way it was
    /// installed.
    #[error("{} is not a textweaver release package", .0.display())]
    NotPackage(PathBuf),
    /// The package could not be unpacked, or holds a name that is not
    /// plain.
    #[error("the package: {0}")]
    Unpack(String),
    /// Putting the new files in place failed; the old ones were kept.
    #[error("{}: {source}", path.display())]
    Install {
        /// Where.
        path: PathBuf,
        /// The error.
        source: std::io::Error,
    },
}

impl UpdateError {
    /// An install error at `path`.
    pub fn install(path: &Path, source: std::io::Error) -> Self {
        UpdateError::Install {
            path: path.to_owned(),
            source,
        }
    }
}

/// A version number, `MAJOR.MINOR.PATCH` with an optional pre-release
/// (`0.1.0-alpha.9`, `0.2.0-beta.1`), ordered as semantic versioning
/// orders them: a pre-release comes before its release, and `alpha.10`
/// after `alpha.9`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    pre: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Number(u64),
    Text(String),
}

impl Version {
    /// Reads `0.1.0-alpha.9`, with or without a leading `v`; build data
    /// after `+` is ignored. `None` when it is not a version.
    pub fn parse(text: &str) -> Option<Version> {
        let t = text.trim();
        let t = t.strip_prefix('v').unwrap_or(t);
        let t = t.split('+').next().unwrap_or(t);
        let (core, pre) = match t.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (t, None),
        };
        let nums: Vec<u64> = core
            .split('.')
            .map(|n| n.parse().ok())
            .collect::<Option<_>>()?;
        let [a, b, c] = nums.as_slice() else {
            return None;
        };
        let pre = match pre {
            None => Vec::new(),
            Some(p) => p.split('.').map(part).collect::<Option<_>>()?,
        };
        Some(Version {
            core: [*a, *b, *c],
            pre,
        })
    }
}

/// One dot-separated part of a pre-release: a number or a word.
fn part(s: &str) -> Option<Part> {
    if s.is_empty() {
        None
    } else if s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok().map(Part::Number)
    } else {
        Some(Part::Text(s.to_owned()))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core
            .cmp(&other.core)
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self
                    .pre
                    .iter()
                    .zip(&other.pre)
                    .map(|(a, b)| match (a, b) {
                        (Part::Number(x), Part::Number(y)) => x.cmp(y),
                        (Part::Number(_), Part::Text(_)) => Ordering::Less,
                        (Part::Text(_), Part::Number(_)) => Ordering::Greater,
                        (Part::Text(x), Part::Text(y)) => x.cmp(y),
                    })
                    .find(|o| *o != Ordering::Equal)
                    .unwrap_or_else(|| self.pre.len().cmp(&other.pre.len())),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// One file of a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Its name (`textweaver-0.2.0-windows-x86_64.zip`).
    pub name: String,
    /// Where it downloads from.
    pub address: String,
    /// Its size in bytes, as GitHub lists it.
    pub size: u64,
}

/// One release from the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// The version, from the tag without its `v`.
    pub version: String,
    /// True for a pre-release (offered all the same: the owner's choice).
    pub prerelease: bool,
    /// Its files.
    pub assets: Vec<Asset>,
}

/// One asset of GitHub's answer.
fn asset(x: &serde_json::Value) -> Option<Asset> {
    Some(Asset {
        name: x.get("name")?.as_str()?.to_owned(),
        address: x.get("browser_download_url")?.as_str()?.to_owned(),
        size: x.get("size")?.as_u64()?,
    })
}

/// One release of GitHub's answer, unless it is a draft or its tag is not
/// a version.
fn release(r: &serde_json::Value) -> Option<Release> {
    if r.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
        return None;
    }
    let tag = r.get("tag_name")?.as_str()?;
    Version::parse(tag)?;
    let assets = r
        .get("assets")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(asset).collect())
        .unwrap_or_default();
    Some(Release {
        version: tag.trim().trim_start_matches('v').to_owned(),
        prerelease: r
            .get("prerelease")
            .and_then(|p| p.as_bool())
            .unwrap_or(false),
        assets,
    })
}

/// Reads GitHub's release list (JSON). Drafts and tags that are not
/// versions are left out.
pub fn parse_releases(json: &[u8]) -> Result<Vec<Release>, UpdateError> {
    let v: serde_json::Value = serde_json::from_slice(json).map_err(|_| UpdateError::Unreadable)?;
    let list = v.as_array().ok_or(UpdateError::Unreadable)?;
    Ok(list.iter().filter_map(release).collect())
}

/// How textweaver is installed on this computer, which decides its
/// package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageKind {
    /// Windows: a zip, unpacked into the install folder.
    WindowsZip,
    /// Linux: a tarball, unpacked into the install folder.
    Tarball,
    /// Linux: one AppImage file, replaced whole.
    AppImage,
    /// macOS: a zip holding `textweaver.app` and `tw`.
    MacZip,
}

/// This computer's operating system, processor, and kind of install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    /// `windows`, `linux`, or `macos`.
    pub os: &'static str,
    /// `x86_64` or `aarch64`.
    pub arch: &'static str,
    /// The kind of package.
    pub kind: PackageKind,
}

impl Target {
    /// This computer: an AppImage when `APPIMAGE` is set (the AppImage
    /// runtime sets it), else the platform's package. `None` on a system
    /// textweaver has no packages for.
    pub fn this() -> Option<Target> {
        let os = std::env::consts::OS;
        let kind = match os {
            "windows" => PackageKind::WindowsZip,
            "macos" => PackageKind::MacZip,
            "linux" if std::env::var_os("APPIMAGE").is_some_and(|v| !v.is_empty()) => {
                PackageKind::AppImage
            }
            "linux" => PackageKind::Tarball,
            _ => return None,
        };
        Some(Target {
            os,
            arch: std::env::consts::ARCH,
            kind,
        })
    }

    /// The package's file name in release `version`: one package per
    /// operating system and processor, holding both programs.
    // shortcut: these are beta 1's single-package names (B1-o2); a release
    // that renames its packages must change this too.
    pub fn package_name(&self, version: &str) -> String {
        let (os, arch) = (self.os, self.arch);
        match self.kind {
            PackageKind::WindowsZip => format!("textweaver-{version}-{os}-{arch}.zip"),
            PackageKind::Tarball => format!("textweaver-{version}-{os}-{arch}.tar.gz"),
            PackageKind::AppImage => format!("textweaver-{version}-{os}-{arch}.AppImage"),
            PackageKind::MacZip => format!("textweaver-{version}-{os}-universal.zip"),
        }
    }
}

/// A newer release, with this computer's package and the checksum file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// Its version.
    pub version: String,
    /// True for a pre-release.
    pub prerelease: bool,
    /// The package for this computer.
    pub package: Asset,
    /// `SHA256SUMS.txt`.
    pub sums: Asset,
}

impl Offer {
    /// The package's size for a person: "92.1 MB".
    pub fn size_text(&self) -> String {
        crate::size_text(self.package.size)
    }
}

/// Reads the release list at `address` ([`RELEASES`], or a fake one in
/// tests) and offers the newest release newer than `current`, pre-release
/// or stable; `Ok(None)` when there is none. When that release has no
/// package for `target` or no checksum file, it is an error rather than a
/// quiet skip, so a broken release is noticed.
pub fn check(
    fetcher: &dyn Fetcher,
    address: &str,
    current: &str,
    target: &Target,
) -> Result<Option<Offer>, UpdateError> {
    let now = Version::parse(current).ok_or(UpdateError::Unreadable)?;
    let bytes = fetch_bytes(fetcher, address, MAX_LIST_BYTES).map_err(UpdateError::Fetch)?;
    let newest = parse_releases(&bytes)?
        .into_iter()
        .filter_map(|r| Version::parse(&r.version).map(|v| (v, r)))
        .filter(|(v, _)| *v > now)
        .max_by(|a, b| a.0.cmp(&b.0));
    let Some((_, release)) = newest else {
        return Ok(None);
    };
    let name = target.package_name(&release.version);
    let find = |n: &str| release.assets.iter().find(|a| a.name == n).cloned();
    let package = find(&name).ok_or_else(|| UpdateError::NoPackage(release.version.clone()))?;
    let sums = find(SUMS_FILE).ok_or_else(|| UpdateError::NoChecksum(name.clone()))?;
    Ok(Some(Offer {
        version: release.version,
        prerelease: release.prerelease,
        package,
        sums,
    }))
}

/// True when `version` is newer than `than`; a version that cannot be
/// read is never newer.
pub fn is_newer(version: &str, than: &str) -> bool {
    match (Version::parse(version), Version::parse(than)) {
        (Some(v), Some(t)) => v > t,
        (Some(_), None) => true,
        _ => false,
    }
}

/// The SHA-256 of `name` in a checksum file (`sha256sum`'s format: the
/// hash, then the name after a space and a space or `*`), lowercase.
pub fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, rest) = line.trim().split_once(char::is_whitespace)?;
        let file = rest.trim_start().trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

/// Downloads `offer`'s package into `dir` through the one downloader: the
/// checksum file first, then the package pinned by its listed size and
/// that checksum. A `.part` file of a stopped download is gone on from.
/// Returns the checked package's path. A package that does not match is
/// refused, and `dir` gets nothing.
pub fn download_update(
    offer: &Offer,
    fetcher: &dyn Fetcher,
    dir: &Path,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<PathBuf, UpdateError> {
    let sums = fetch_bytes(fetcher, &offer.sums.address, MAX_SUMS_BYTES)
        .map_err(|e| UpdateError::Download(format!("{SUMS_FILE}: {e}")))?;
    let sums = String::from_utf8_lossy(&sums);
    let name = &offer.package.name;
    let hash = sum_for(&sums, name).ok_or_else(|| UpdateError::NoChecksum(name.clone()))?;
    let pin = FilePin {
        name: Cow::Owned(name.clone()),
        url: Cow::Owned(offer.package.address.clone()),
        size: offer.package.size,
        check: Check::Sha256(Cow::Owned(hash)),
    };
    let component = Component {
        id: Cow::Borrowed("textweaver-update"),
        title: Cow::Owned(format!("textweaver {}", offer.version)),
        license: Cow::Borrowed("GPL-3.0-or-later"),
        credit: Cow::Borrowed("textweaver's releases on GitHub"),
        features: Cow::Borrowed(&[]),
        folder: Cow::Borrowed("updates"),
        files: Cow::Owned(vec![pin]),
        notice: None,
        listing: None,
    };
    crate::download(
        &component,
        dir,
        &Sources::public(),
        fetcher,
        progress,
        cancel,
    )
    .map_err(|e| match e {
        ComponentError::Hash { .. } | ComponentError::Size { .. } => {
            UpdateError::Mismatch(name.clone())
        }
        ComponentError::Cancelled => UpdateError::Cancelled,
        other => UpdateError::Download(other.to_string()),
    })?;
    Ok(dir.join(name))
}

#[cfg(test)]
mod tests;
