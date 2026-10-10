//! A fetcher for tests: it hands out fixtures by address, goes on from an
//! offset when asked, records every request, and can fail, stop, or cancel
//! part way. Nothing goes to the network.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::fetch::{Fetched, Fetcher};
use crate::manifest::{Action, Platform};

/// One request a [`FakeFetcher`] answered: the address and the offset.
pub type Request = (String, u64);

/// Hands out fixtures by address and records each request.
#[derive(Debug, Default)]
pub struct FakeFetcher {
    files: HashMap<String, Vec<u8>>,
    asked: Mutex<Vec<Request>>,
    /// Ignore offsets: answer every request with the whole file, as a
    /// server without `Range` support does.
    pub whole_files_only: bool,
    /// Set this flag after handing out this many bytes of one answer
    /// (to cancel a download part way).
    cancel_after: Option<(u64, Arc<AtomicBool>)>,
    /// Fail with an error after this many bytes of one answer (a dropped
    /// connection).
    fail_after: Option<u64>,
}

impl FakeFetcher {
    /// A fetcher that has no files.
    pub fn new() -> Self {
        FakeFetcher::default()
    }

    /// Serves `bytes` at `address`.
    pub fn with(mut self, address: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.insert(address, bytes);
        self
    }

    /// Serves `bytes` at `address`.
    pub fn insert(&mut self, address: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files.insert(address.into(), bytes.into());
    }

    /// The bytes served at `address`, to change in a test.
    pub fn bytes_mut(&mut self, address: &str) -> Option<&mut Vec<u8>> {
        self.files.get_mut(address)
    }

    /// Sets `flag` once `n` bytes of an answer were read.
    pub fn cancel_after(mut self, n: u64, flag: Arc<AtomicBool>) -> Self {
        self.cancel_after = Some((n, flag));
        self
    }

    /// Fails once `n` bytes of an answer were read.
    pub fn fail_after(mut self, n: u64) -> Self {
        self.fail_after = Some(n);
        self
    }

    /// Stops failing and cancelling.
    pub fn heal(&mut self) {
        self.fail_after = None;
        self.cancel_after = None;
    }

    /// Every request so far, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.asked.lock().map(|a| a.clone()).unwrap_or_default()
    }

    /// How many requests were made.
    pub fn request_count(&self) -> usize {
        self.asked.lock().map(|a| a.len()).unwrap_or(0)
    }
}

/// Reads a fixture, cancelling or failing part way when asked.
struct Feed {
    data: Vec<u8>,
    at: usize,
    read: u64,
    cancel_after: Option<(u64, Arc<AtomicBool>)>,
    fail_after: Option<u64>,
}

impl Read for Feed {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(limit) = self.fail_after
            && self.read >= limit
        {
            return Err(std::io::Error::other("the connection dropped"));
        }
        if let Some((limit, flag)) = &self.cancel_after
            && self.read >= *limit
        {
            flag.store(true, Ordering::Relaxed);
        }
        // Small pieces, so cancelling and failing land part way.
        let n = buf.len().min(1024).min(self.data.len() - self.at);
        buf[..n].copy_from_slice(&self.data[self.at..self.at + n]);
        self.at += n;
        self.read += n as u64;
        Ok(n)
    }
}

impl Fetcher for FakeFetcher {
    fn open(&self, address: &str, from: u64) -> Result<Fetched, String> {
        if let Ok(mut a) = self.asked.lock() {
            a.push((address.to_owned(), from));
        }
        let data = self.files.get(address).ok_or("not found")?;
        let start = if self.whole_files_only || from > data.len() as u64 {
            0
        } else {
            from
        };
        let at = usize::try_from(start).unwrap_or(0);
        Ok(Fetched {
            reader: Box::new(Feed {
                data: data[at..].to_vec(),
                at: 0,
                read: 0,
                cancel_after: self.cancel_after.clone(),
                fail_after: self.fail_after,
            }),
            start,
        })
    }
}

/// A components source in a folder, for tests: a `components.toml` and
/// each component's files under `<release>/` ([`Component::release`]),
/// as a clone of a components repository has them. Nothing goes to the
/// network.
///
/// [`Component::release`]: crate::Component::release
#[derive(Debug)]
pub struct FakeSource {
    root: PathBuf,
    list: String,
}

impl FakeSource {
    /// An empty source in `root` (a temporary folder).
    pub fn new(root: &Path) -> Self {
        FakeSource {
            root: root.to_owned(),
            list: format!("format = {}\n", crate::manifest::FORMAT),
        }
    }

    /// The source's folder: what `[components] source` is set to.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Adds a component with `files` (name and bytes), pinned by their
    /// real size and SHA-256, and writes the list again.
    pub fn add(
        &mut self,
        id: &str,
        version: &str,
        platform: Platform,
        action: Action,
        files: &[(&str, &[u8])],
    ) -> std::io::Result<()> {
        let release = if version.is_empty() {
            id.to_owned()
        } else {
            format!("{id}-{version}")
        };
        let dir = self.root.join(&release);
        std::fs::create_dir_all(&dir)?;
        self.list.push_str(&format!(
            "\n[[component]]\nid = \"{id}\"\ntitle = \"The {id} component\"\n\
             license = \"CC0-1.0, made up for tests\"\nfeatures = [\"test\"]\n\
             version = \"{version}\"\nplatform = \"{}\"\naction = \"{}\"\n",
            platform.word(),
            action.word()
        ));
        for (name, bytes) in files {
            std::fs::write(dir.join(name), bytes)?;
            self.list.push_str(&format!(
                "[[component.file]]\nname = \"{name}\"\nsize = {}\nsha256 = \"{}\"\n",
                bytes.len(),
                crate::sha256_hex(bytes)
            ));
        }
        std::fs::write(self.root.join(crate::manifest::FILE_NAME), &self.list)
    }

    /// Replaces a file's bytes after it was listed, so its checksum no
    /// longer matches (a damaged or altered file).
    pub fn tamper(&self, release: &str, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        std::fs::write(self.root.join(release).join(name), bytes)
    }
}

/// A zip holding `members` (path and bytes), stored, for tests of the
/// `unpack` action: a made-up program in a made-up archive.
pub fn zip_bytes(members: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut out = std::io::Cursor::new(Vec::new());
    let mut w = zip::ZipWriter::new(&mut out);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);
    for (name, bytes) in members {
        // A test's own archive: writing to memory does not fail.
        let _ = w.start_file(*name, opts);
        let _ = w.write_all(bytes);
    }
    let _ = w.finish();
    out.into_inner()
}

/// A gzip tarball holding `files` (path and bytes, each executable) and
/// `links` (path and target), for tests of the `unpack` action.
pub fn tar_gz_bytes(files: &[(&str, &[u8])], links: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let gz = flate2::write::GzEncoder::new(&mut out, flate2::Compression::fast());
    let mut b = tar::Builder::new(gz);
    for (name, bytes) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(bytes.len() as u64);
        h.set_mode(0o755);
        h.set_cksum();
        let _ = b.append_data(&mut h, name, *bytes);
    }
    for (name, target) in links {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Symlink);
        h.set_size(0);
        h.set_cksum();
        let _ = b.append_link(&mut h, name, target);
    }
    if let Ok(gz) = b.into_inner() {
        let _ = gz.finish();
    }
    out
}

/// Switches this process to a memory credential store and never runs
/// `gh`, so a test signs in, stores, and forgets a made-up token without
/// touching the system credential store or a real sign-in. It lasts for
/// the rest of the test program.
pub fn memory_credentials() {
    crate::credentials::use_memory();
}

/// One request a [`FakeGitHub`] answered: its path and query, and its
/// headers with names in lower case.
pub type Seen = (String, Vec<(String, String)>);

/// A fake GitHub on this computer (`127.0.0.1`), for the signed-in
/// fetcher: the API of one private repository, `example-org/parts` (a
/// release by tag, and each asset, which redirects to a signed download),
/// and the signed downloads. The API answers only with the expected
/// token, as a private repository does; a signed download refuses any
/// `Authorization` header. It records every request. Nothing goes to the
/// network.
#[derive(Debug)]
pub struct FakeGitHub {
    base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl FakeGitHub {
    /// Starts the server, expecting `token`, with `files` as release
    /// assets: `(tag, file name, bytes)`. Each release also lists
    /// `elsewhere.bin` at an address outside the API.
    pub fn start(token: &str, files: &[(&str, &str, &[u8])]) -> std::io::Result<FakeGitHub> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let base = format!("http://{}", listener.local_addr()?);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let files: Vec<(String, String, Vec<u8>)> = files
            .iter()
            .map(|(t, n, b)| ((*t).to_owned(), (*n).to_owned(), b.to_vec()))
            .collect();
        let auth = format!("Bearer {token}");
        let (b, s) = (base.clone(), Arc::clone(&seen));
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = serve_one(stream, &b, &auth, &files, &s);
            }
        });
        Ok(FakeGitHub { base, seen })
    }

    /// The API address to give [`SignedInFetcher::with_api`](crate::SignedInFetcher::with_api).
    pub fn api(&self) -> &str {
        &self.base
    }

    /// Every request so far, in order.
    pub fn requests(&self) -> Vec<Seen> {
        self.seen.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Answers one connection of a [`FakeGitHub`].
fn serve_one(
    stream: std::net::TcpStream,
    base: &str,
    auth: &str,
    files: &[(String, String, Vec<u8>)],
    seen: &Mutex<Vec<Seen>>,
) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader, Write};
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let path = line.split(' ').nth(1).unwrap_or("").to_owned();
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 {
            break;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        }
    }
    let signed_in = headers
        .iter()
        .any(|(k, v)| k == "authorization" && v == auth);
    let has_auth = headers.iter().any(|(k, _)| k == "authorization");
    if let Ok(mut s) = seen.lock() {
        s.push((path.clone(), headers));
    }
    let repo = "/repos/example-org/parts/releases/";
    let (status, extra, body): (&str, String, Vec<u8>) = if let Some(tag) =
        path.strip_prefix(&format!("{repo}tags/"))
        && signed_in
    {
        let mut assets: Vec<String> = files
            .iter()
            .enumerate()
            .filter(|(_, (t, _, _))| t == tag)
            .map(|(i, (_, n, _))| format!(r#"{{"name":"{n}","url":"{base}{repo}assets/{i}"}}"#))
            .collect();
        assets.push(r#"{"name":"elsewhere.bin","url":"https://example.invalid/x"}"#.to_owned());
        let json = format!(r#"{{"tag_name":"{tag}","assets":[{}]}}"#, assets.join(","));
        ("200 OK", String::new(), json.into_bytes())
    } else if let Some(id) = path.strip_prefix(&format!("{repo}assets/"))
        && signed_in
    {
        let location = format!("Location: {base}/signed/{id}?signature=1\r\n");
        ("302 Found", location, Vec::new())
    } else if let Some(id) = path
        .strip_prefix("/signed/")
        .and_then(|r| r.strip_suffix("?signature=1"))
        .and_then(|i| i.parse::<usize>().ok())
        .filter(|i| *i < files.len())
    {
        if has_auth {
            ("400 Bad Request", String::new(), Vec::new())
        } else {
            ("200 OK", String::new(), files[id].2.clone())
        }
    } else {
        ("404 Not Found", String::new(), Vec::new())
    };
    let mut stream = stream;
    let head = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}
