//! Fetching bytes: the [`Fetcher`] trait, the standard fetcher (files on
//! this computer, and HTTPS with the `download` feature), and reading a
//! small answer whole.

use std::io::Read;
use std::path::PathBuf;

/// An answer to [`Fetcher::open`]: the bytes, and where they start in the
/// file.
pub struct Fetched {
    /// The bytes from `start` to the end of the file.
    pub reader: Box<dyn Read + Send>,
    /// Where the bytes start: the offset asked for when the source could
    /// go on from there, else 0 (the whole file again).
    pub start: u64,
}

impl std::fmt::Debug for Fetched {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fetched")
            .field("start", &self.start)
            .finish_non_exhaustive()
    }
}

/// Fetches files. The app passes [`StandardFetcher`]; tests pass
/// [`FakeFetcher`](crate::fake::FakeFetcher), which hands out fixtures and
/// records each request.
pub trait Fetcher: Send + Sync {
    /// Opens `address` from byte `from` (0 for the whole file; more when
    /// a stopped download goes on). The error is said to the reader, after
    /// the file's name.
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String>;
}

/// True when this build can download over HTTPS (the `download` feature).
pub const fn can_download() -> bool {
    cfg!(feature = "download")
}

/// Reads at most `limit` bytes from `address` (a mirror's manifest, a voice
/// catalogue). A longer answer is an error.
pub fn fetch_bytes(fetcher: &dyn Fetcher, address: &str, limit: u64) -> Result<Vec<u8>, String> {
    let fetched = fetcher.open(address, 0)?;
    let mut out = Vec::new();
    fetched
        .reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    if out.len() as u64 > limit {
        return Err(format!("more than {limit} bytes"));
    }
    Ok(out)
}

/// The path of a local address: `file://` or a plain path, not `http:` or
/// `https:`.
fn local_path(address: &str) -> Option<PathBuf> {
    if let Some(rest) = address.strip_prefix("file://") {
        // file:///home/x is /home/x; file:///D:/x is D:/x.
        let b = rest.as_bytes();
        let path = if b.len() > 2 && b[0] == b'/' && b[2] == b':' {
            &rest[1..]
        } else {
            rest
        };
        return Some(PathBuf::from(path));
    }
    let lower = address.to_ascii_lowercase();
    (!lower.starts_with("http://") && !lower.starts_with("https://"))
        .then(|| PathBuf::from(address))
}

/// Reads a local file from `from`.
fn open_local(path: &std::path::Path, from: u64) -> Result<Fetched, String> {
    use std::io::{Seek, SeekFrom};
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    let start = if from > 0 && from <= len {
        f.seek(SeekFrom::Start(from)).map_err(|e| e.to_string())?;
        from
    } else {
        0
    };
    Ok(Fetched {
        reader: Box::new(f),
        start,
    })
}

/// The fetcher the app uses: a folder on this computer (a mirror that is a
/// clone or a memory stick) is read directly; `https:` addresses go over
/// the network with the `download` feature, with the neutral
/// [`USER_AGENT`](crate::USER_AGENT), and say they are not in this build
/// without it.
#[derive(Clone, Copy, Debug, Default)]
pub struct StandardFetcher;

impl Fetcher for StandardFetcher {
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String> {
        if let Some(path) = local_path(address) {
            return open_local(&path, from);
        }
        #[cfg(feature = "download")]
        {
            HttpFetcher.open(address, from)
        }
        #[cfg(not(feature = "download"))]
        {
            let _ = from;
            Err("this build of textweaver cannot download files".to_owned())
        }
    }
}

/// HTTPS with `ureq` (feature `download`): the neutral User-Agent, time
/// limits, and a `Range` request to go on with a stopped download.
#[cfg(feature = "download")]
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpFetcher;

#[cfg(feature = "download")]
impl Fetcher for HttpFetcher {
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String> {
        http_get(address, from, &[])
    }
}

/// A GET of `address` from byte `from`, with `headers` added (the signed-in
/// fetcher's `Authorization`). A redirect is followed without the
/// `Authorization` header, so a token never reaches the place a redirect
/// points to.
#[cfg(feature = "download")]
pub(crate) fn http_get(
    address: &str,
    from: u64,
    headers: &[(&str, String)],
) -> Result<Fetched, String> {
    use std::time::Duration;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(120)))
        // Neutral on purpose: no user, machine, or account names ever
        // go out with a request (the owner's rule).
        .user_agent(crate::USER_AGENT)
        .redirect_auth_headers(ureq::config::RedirectAuthHeaders::Never)
        .build()
        .into();
    let mut req = agent.get(address);
    for (name, value) in headers {
        req = req.header(*name, value.as_str());
    }
    if from > 0 {
        req = req.header("Range", format!("bytes={from}-"));
    }
    let resp = req.call().map_err(|e| e.to_string())?;
    let start = if from > 0 && resp.status().as_u16() == 206 {
        from
    } else {
        0
    };
    let reader = resp.into_body().into_reader();
    Ok(Fetched {
        reader: Box::new(reader),
        start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_addresses_are_read_from_disk() {
        assert_eq!(local_path("https://x/y"), None);
        assert_eq!(local_path("HTTP://x/y"), None);
        assert_eq!(
            local_path("D:/mirror/a"),
            Some(PathBuf::from("D:/mirror/a"))
        );
        assert_eq!(
            local_path("file:///D:/mirror/a"),
            Some(PathBuf::from("D:/mirror/a"))
        );
        assert_eq!(
            local_path("file:///srv/mirror/a"),
            Some(PathBuf::from("/srv/mirror/a"))
        );
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.bin");
        std::fs::write(&p, b"hello").unwrap();
        let got = fetch_bytes(&StandardFetcher, &p.to_string_lossy(), 10).unwrap();
        assert_eq!(got, b"hello");
        assert!(fetch_bytes(&StandardFetcher, &p.to_string_lossy(), 3).is_err());
        let mut rest = String::new();
        let f = StandardFetcher.open(&p.to_string_lossy(), 2).unwrap();
        assert_eq!(f.start, 2);
        let mut r = f.reader;
        r.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "llo");
    }

    #[cfg(not(feature = "download"))]
    #[test]
    fn without_the_feature_nothing_goes_to_the_network() {
        let e = StandardFetcher
            .open("https://example.invalid/x", 0)
            .unwrap_err();
        assert_eq!(e, "this build of textweaver cannot download files");
        assert!(!can_download());
    }
}
