//! Release files from a private GitHub repository, signed in (B1-c2).
//!
//! A repository source's files are release assets, addressed as
//! `https://github.com/owner/name/releases/download/<tag>/<file>`
//! ([`Sources`](crate::Sources)). That address works for a public
//! repository with no sign-in. For a private one, GitHub documents its REST
//! API instead: the release by tag
//! (`GET /repos/owner/name/releases/tags/<tag>`) lists each asset with its
//! API address, and that address with `Accept: application/octet-stream`
//! answers with a redirect to a short-lived signed download.
//!
//! [`SignedInFetcher`] does exactly that. The token goes in the
//! `Authorization` header of requests to the API and nowhere else: never
//! to the signed download the API redirects to (the redirect drops it),
//! never to a mirror or a public address, and never into a log line, an
//! error, or a message. Every request carries the neutral
//! [`USER_AGENT`](crate::USER_AGENT). Any other address goes to
//! [`StandardFetcher`] as before.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::credentials::Token;
use crate::fetch::{Fetched, Fetcher, StandardFetcher};

/// The GitHub REST API, the only place the token is sent.
pub const GITHUB_API: &str = "https://api.github.com";

/// Where the release downloads of a repository start.
const RELEASES_START: &str = "https://github.com/";

/// The largest release description read (its list of assets).
#[cfg_attr(not(feature = "download"), allow(dead_code))]
const MAX_RELEASE_BYTES: u64 = 4 * 1024 * 1024;

/// `(owner, name, tag, file)` when `address` is a release download,
/// `https://github.com/owner/name/releases/download/<tag>/<file>`.
pub(crate) fn release_file(address: &str) -> Option<(&str, &str, &str, &str)> {
    let rest = address.strip_prefix(RELEASES_START)?;
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.as_slice() {
        [owner, name, "releases", "download", tag, file]
            if [owner, name, tag, file]
                .iter()
                .all(|p| !p.is_empty() && !p.contains(['?', '#'])) =>
        {
            Some((owner, name, tag, file))
        }
        _ => None,
    }
}

/// Fetches release files from a private GitHub repository with a token,
/// through the GitHub API; anything else as [`StandardFetcher`] does.
pub struct SignedInFetcher {
    token: Token,
    api: String,
    /// The assets of each release, `(file name, API address)`, by
    /// `owner/name/tag`, read once.
    releases: Mutex<HashMap<String, Vec<(String, String)>>>,
}

impl std::fmt::Debug for SignedInFetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedInFetcher")
            .field("api", &self.api)
            .finish_non_exhaustive()
    }
}

impl SignedInFetcher {
    /// A fetcher that signs in to [`GITHUB_API`] with `token`.
    pub fn new(token: Token) -> Self {
        SignedInFetcher {
            token,
            api: GITHUB_API.to_owned(),
            releases: Mutex::new(HashMap::new()),
        }
    }

    /// The same, signing in to `api` instead: a fake server in tests. Only
    /// code can set it, never a setting or the environment, so the token
    /// cannot be sent anywhere else by configuration.
    pub fn with_api(mut self, api: &str) -> Self {
        self.api = api.trim_end_matches('/').to_owned();
        self
    }

    /// True when `address` is on the API (the only place the token goes).
    fn on_api(&self, address: &str) -> bool {
        address
            .strip_prefix(&self.api)
            .is_some_and(|rest| rest.starts_with('/'))
    }

    /// The headers for a request to the API.
    #[cfg_attr(not(feature = "download"), allow(dead_code))]
    fn signed(&self, accept: &str) -> [(&'static str, String); 3] {
        [
            ("Authorization", format!("Bearer {}", self.token.secret())),
            ("Accept", accept.to_owned()),
            ("X-GitHub-Api-Version", "2022-11-28".to_owned()),
        ]
    }

    /// The API address of `file` in the release `tag` of `owner/name`.
    fn asset_address(
        &self,
        owner: &str,
        name: &str,
        tag: &str,
        file: &str,
    ) -> Result<String, String> {
        let key = format!("{owner}/{name}/{tag}");
        let known = self.releases.lock().ok().and_then(|r| r.get(&key).cloned());
        let assets = match known {
            Some(a) => a,
            None => {
                let a = self.read_release(owner, name, tag)?;
                if let Ok(mut r) = self.releases.lock() {
                    r.insert(key, a.clone());
                }
                a
            }
        };
        let address = assets
            .into_iter()
            .find(|(n, _)| n == file)
            .map(|(_, a)| a)
            .ok_or_else(|| format!("not in the release {tag}"))?;
        if !self.on_api(&address) {
            return Err("GitHub gave an asset address outside its API".to_owned());
        }
        Ok(address)
    }

    /// The assets of a release, from the API.
    #[cfg(feature = "download")]
    fn read_release(
        &self,
        owner: &str,
        name: &str,
        tag: &str,
    ) -> Result<Vec<(String, String)>, String> {
        use std::io::Read;
        let address = format!("{}/repos/{owner}/{name}/releases/tags/{tag}", self.api);
        let headers = self.signed("application/vnd.github+json");
        let fetched = crate::fetch::http_get(&address, 0, &headers)?;
        let mut body = Vec::new();
        fetched
            .reader
            .take(MAX_RELEASE_BYTES)
            .read_to_end(&mut body)
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|_| "the answer from GitHub was unreadable".to_owned())?;
        let assets = json
            .get("assets")
            .and_then(|a| a.as_array())
            .ok_or("the answer from GitHub listed no files")?;
        // shortcut: GitHub lists every asset of a release in one answer;
        // pages are needed only past 1000 assets, far more than a
        // components release holds.
        Ok(assets
            .iter()
            .filter_map(|a| {
                let n = a.get("name")?.as_str()?;
                let u = a.get("url")?.as_str()?;
                Some((n.to_owned(), u.to_owned()))
            })
            .collect())
    }

    #[cfg(not(feature = "download"))]
    fn read_release(
        &self,
        _owner: &str,
        _name: &str,
        _tag: &str,
    ) -> Result<Vec<(String, String)>, String> {
        Err("this build of textweaver cannot download files".to_owned())
    }
}

impl Fetcher for SignedInFetcher {
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String> {
        let Some((owner, name, tag, file)) = release_file(address) else {
            return StandardFetcher.open(address, from);
        };
        let asset = self.asset_address(owner, name, tag, file)?;
        #[cfg(feature = "download")]
        {
            // The API answers with a redirect to a signed download, which
            // is followed without the Authorization header.
            crate::fetch::http_get(&asset, from, &self.signed("application/octet-stream"))
        }
        #[cfg(not(feature = "download"))]
        {
            let _ = (asset, from);
            Err("this build of textweaver cannot download files".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_downloads_are_recognized() {
        assert_eq!(
            release_file("https://github.com/example-org/parts/releases/download/sample-1.2/a.bin"),
            Some(("example-org", "parts", "sample-1.2", "a.bin"))
        );
        assert_eq!(release_file("https://github.com/example-org/parts"), None);
        assert_eq!(
            release_file("https://example.invalid/example-org/parts/releases/download/t/a"),
            None
        );
        assert_eq!(
            release_file("https://github.com/o/n/releases/download/t/a?x=1"),
            None
        );
        assert_eq!(
            release_file("https://github.com/o/n/releases/download//a"),
            None
        );
    }

    #[test]
    fn the_token_goes_only_to_the_api() {
        let t = Token::new("ghp_EXAMPLEonlyNotARealToken2222").unwrap();
        let f = SignedInFetcher::new(t);
        assert!(f.on_api("https://api.github.com/repos/o/n/releases/assets/1"));
        assert!(!f.on_api("https://api.github.com.example.invalid/x"));
        assert!(!f.on_api("https://objects.githubusercontent.com/x"));
        assert!(!format!("{f:?}").contains("EXAMPLEonly"));
    }
}

#[cfg(all(test, feature = "download"))]
mod signed_in_tests {
    use std::io::Read;

    use super::*;
    use crate::fake::FakeGitHub;

    const TOKEN: &str = "ghp_EXAMPLEonlyNotARealToken3333";
    const ADDRESS: &str = "https://github.com/example-org/parts/releases/download/sample-1.2/a.bin";

    fn read(f: &dyn Fetcher, address: &str) -> Result<String, String> {
        let mut got = String::new();
        f.open(address, 0)?
            .reader
            .read_to_string(&mut got)
            .map_err(|e| e.to_string())?;
        Ok(got)
    }

    #[test]
    fn a_private_release_file_comes_through_the_api_with_the_token_only_there() {
        let gh =
            FakeGitHub::start(TOKEN, &[("sample-1.2", "a.bin", b"hello, private file")]).unwrap();
        let f = SignedInFetcher::new(Token::new(TOKEN).unwrap()).with_api(gh.api());
        assert_eq!(read(&f, ADDRESS).unwrap(), "hello, private file");
        // The release is read once, then remembered.
        assert_eq!(read(&f, ADDRESS).unwrap(), "hello, private file");
        // An asset outside the API is refused, so the token never goes there.
        let e = read(&f, &ADDRESS.replace("a.bin", "elsewhere.bin")).unwrap_err();
        assert!(e.contains("outside its API"), "{e}");
        // A missing file says so, without the token.
        let e = read(&f, &ADDRESS.replace("a.bin", "none.bin")).unwrap_err();
        assert!(e.contains("not in the release"), "{e}");
        assert!(!e.contains("EXAMPLEonly"), "{e}");

        let seen = gh.requests();
        let paths: Vec<&str> = seen.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            paths,
            [
                "/repos/example-org/parts/releases/tags/sample-1.2",
                "/repos/example-org/parts/releases/assets/0",
                "/signed/0?signature=1",
                "/repos/example-org/parts/releases/assets/0",
                "/signed/0?signature=1",
            ]
        );
        let header = |h: &[(String, String)], k: &str| {
            h.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
        };
        for (path, headers) in &seen {
            assert_eq!(
                header(headers, "user-agent").as_deref(),
                Some(crate::USER_AGENT),
                "{path}"
            );
            let auth = header(headers, "authorization");
            if path.starts_with("/repos/") {
                assert_eq!(auth, Some(format!("Bearer {TOKEN}")), "{path}");
            } else {
                assert_eq!(auth, None, "the signed download gets no token: {path}");
            }
        }
        assert_eq!(
            header(&seen[1].1, "accept").as_deref(),
            Some("application/octet-stream")
        );
    }

    #[test]
    fn a_wrong_token_is_refused_without_saying_it() {
        let gh = FakeGitHub::start(TOKEN, &[("sample-1.2", "a.bin", b"x")]).unwrap();
        let wrong = "ghp_EXAMPLEonlyWrongToken44444444";
        let f = SignedInFetcher::new(Token::new(wrong).unwrap()).with_api(gh.api());
        let e = read(&f, ADDRESS).unwrap_err();
        assert!(!e.contains("EXAMPLEonly"), "{e}");
    }

    #[test]
    fn other_addresses_go_as_before() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.bin");
        std::fs::write(&p, b"local").unwrap();
        let f = SignedInFetcher::new(Token::new(TOKEN).unwrap()).with_api("http://127.0.0.1:9");
        assert_eq!(read(&f, &p.to_string_lossy()).unwrap(), "local");
    }
}
