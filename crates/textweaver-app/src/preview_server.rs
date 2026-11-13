//! The preview's reload server (`[preview] follow`, "Browser preview
//! follows").
//!
//! By default the browser preview is a file, and after each save
//! textweaver says "Preview updated. Press F5 in the browser." With
//! `follow` set to save or typing, the preview is served by this small
//! server instead, and the page reloads itself after each save (and, with
//! typing, after a pause in typing):
//!
//! - It listens on 127.0.0.1 only, on a port the system picks, and answers
//!   only paths that start with a random secret (`/<token>/`), so other
//!   programs and other computers cannot read the document.
//! - `/<token>/` is the page, with a short script added; `/<token>/events`
//!   is a Server-Sent Events stream; `/<token>/files/…` serves images,
//!   style sheets, and fonts from the document's folder (never outside
//!   it, and never a file or folder whose name starts with a dot).
//!   Anything else is "404 Not Found", so a page cannot read the other
//!   files beside the document.
//! - Every answer carries a `Content-Security-Policy`: scripts run only
//!   with the nonce the reload script is given for that one answer, and
//!   the page may load and connect only to this server, so a script left
//!   in a document could neither run nor send anything out (security L2).
//!   A page from a trusted folder (`[preview] trusted_folders`) keeps its
//!   own scripts, and is served without the policy.
//! - [`PreviewServer::reload`] sends a `reload` event carrying the id of
//!   the heading nearest the caret. The script stores it, reloads, and
//!   then scrolls to that heading and focuses it, so a screen reader lands
//!   near the edited place. A reload still resets the screen reader's
//!   place in the page, which is why this is off by default.
//! - It stops when the document closes or textweaver quits
//!   ([`PreviewServer::stop`], also on drop).
//!
//! Only `std::net` is used: one thread accepts connections, and each
//! request is answered on a short-lived thread; event streams are kept
//! open until the next reload or the end.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use textweaver_lexicon::i18n::Catalog;

/// How long a request may take to arrive.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// The largest request head read.
const MAX_HEAD: usize = 16 * 1024;

/// What the server serves.
#[derive(Debug, Default)]
struct State {
    /// The preview page (an HTML file).
    page: PathBuf,
    /// The document's folder, for images and other files.
    folder: PathBuf,
    /// Open event streams.
    listeners: Vec<TcpStream>,
    /// The interface's messages, for the words a page shows; English
    /// until [`PreviewServer::set_catalog`].
    catalog: Option<Arc<Catalog>>,
    /// The page is from a trusted folder: served without the content
    /// security policy, so its own scripts run.
    trusted: bool,
}

#[derive(Debug, Default)]
struct Shared {
    stop: AtomicBool,
    state: Mutex<State>,
}

impl Shared {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A running reload server.
#[derive(Debug)]
pub struct PreviewServer {
    addr: SocketAddr,
    token: String,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

/// A random secret for the address, and the nonce of each answer's
/// reload script: 128 bits from the system's random hash keys. (ring's
/// `SystemRandom` would be the better source, but the app crate does not
/// link ring.)
fn token() -> String {
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        h.write_u32(std::process::id());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

impl PreviewServer {
    /// Starts serving `page`, with files from `folder`, on 127.0.0.1.
    ///
    /// # Errors
    ///
    /// When the port cannot be opened or the thread cannot start.
    pub fn start(page: PathBuf, folder: PathBuf) -> io::Result<PreviewServer> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let addr = listener.local_addr()?;
        let token = token();
        let shared = Arc::new(Shared::default());
        {
            let mut st = shared.state();
            st.page = page;
            st.folder = folder;
        }
        let (sh, tok) = (Arc::clone(&shared), token.clone());
        let thread = std::thread::Builder::new()
            .name("tw-preview".into())
            .spawn(move || accept_loop(&listener, &sh, &tok))?;
        Ok(PreviewServer {
            addr,
            token,
            shared,
            thread: Some(thread),
        })
    }

    /// The page's address, to open in the browser.
    pub fn url(&self) -> String {
        format!("http://{}/{}/", self.addr, self.token)
    }

    /// Where the server listens.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// The secret the paths start with.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// The interface's messages, for the words the server's own pages
    /// show.
    pub fn set_catalog(&self, catalog: Arc<Catalog>) {
        self.shared.state().catalog = Some(catalog);
    }

    /// Serves another page and folder from now on.
    pub fn set_page(&self, page: PathBuf, folder: PathBuf) {
        let mut st = self.shared.state();
        st.page = page;
        st.folder = folder;
    }

    /// Whether the page is from a trusted folder: served without the
    /// content security policy, so the scripts it keeps run. Off until
    /// set.
    pub fn set_trusted(&self, trusted: bool) {
        self.shared.state().trusted = trusted;
    }

    /// Tells every open page to reload, then focus the element with the id
    /// `anchor` (a heading) when there is one. Returns how many pages were
    /// told.
    pub fn reload(&self, anchor: Option<&str>) -> usize {
        let data = anchor.unwrap_or("").replace(['\r', '\n'], "");
        let msg = format!("event: reload\ndata: {data}\n\n");
        let mut st = self.shared.state();
        st.listeners
            .retain_mut(|s| s.write_all(msg.as_bytes()).and_then(|()| s.flush()).is_ok());
        st.listeners.len()
    }

    /// How many pages are listening for reloads.
    pub fn listening(&self) -> usize {
        self.shared.state().listeners.len()
    }

    /// Stops the server and closes every connection.
    pub fn stop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop.
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(200));
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let mut st = self.shared.state();
        for s in st.listeners.drain(..) {
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl Drop for PreviewServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>, token: &str) {
    for conn in listener.incoming() {
        if shared.stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(stream) = conn else {
            continue;
        };
        // Only this computer: the socket is bound to 127.0.0.1, and a peer
        // elsewhere is refused all the same.
        if stream
            .peer_addr()
            .map(|a| !a.ip().is_loopback())
            .unwrap_or(true)
        {
            continue;
        }
        let (sh, tok) = (Arc::clone(shared), token.to_owned());
        let _ = std::thread::Builder::new()
            .name("tw-preview-request".into())
            .spawn(move || {
                let _ = handle(stream, &sh, &tok);
            });
    }
}

/// The path of a `GET` request, or `None` for anything else.
fn request_path(stream: &TcpStream) -> io::Result<Option<String>> {
    stream.set_read_timeout(Some(REQUEST_TIMEOUT))?;
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut read = first.len();
    // Skip the headers.
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        read += n;
        if n == 0 || line == "\r\n" || line == "\n" || read > MAX_HEAD {
            break;
        }
    }
    let mut parts = first.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some("GET"), Some(path)) => Ok(Some(path.to_owned())),
        _ => Ok(None),
    }
}

/// The content security policy of an answer whose reload script carries
/// `nonce`: scripts only with the nonce, styles and fonts written in the
/// page or from this server, images from this server or written in, and
/// connections (the reload events) to this server only.
fn content_policy(nonce: &str) -> String {
    format!(
        "default-src 'self'; script-src 'nonce-{nonce}'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'none'"
    )
}

/// An answer with the content security policy (a nonce no script has).
fn respond(stream: &TcpStream, status: &str, kind: &str, body: &[u8]) -> io::Result<()> {
    respond_with(stream, status, kind, body, Some(&content_policy(&token())))
}

/// An answer, with `policy` as its `Content-Security-Policy` when given.
fn respond_with(
    mut stream: &TcpStream,
    status: &str,
    kind: &str,
    body: &[u8],
    policy: Option<&str>,
) -> io::Result<()> {
    let policy = policy
        .map(|p| format!("Content-Security-Policy: {p}\r\n"))
        .unwrap_or_default();
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n{policy}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn not_found(stream: &TcpStream) -> io::Result<()> {
    respond(
        stream,
        "404 Not Found",
        "text/plain; charset=utf-8",
        b"Not found.",
    )
}

fn handle(stream: TcpStream, shared: &Shared, token: &str) -> io::Result<()> {
    let Some(path) = request_path(&stream)? else {
        return respond(
            &stream,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"Only GET.",
        );
    };
    let path = path.split(['?', '#']).next().unwrap_or("");
    let prefix = format!("/{token}/");
    let Some(rest) = path.strip_prefix(&prefix) else {
        return not_found(&stream);
    };
    match rest {
        "" | "index.html" => {
            let (page, trusted) = {
                let st = shared.state();
                (st.page.clone(), st.trusted)
            };
            match std::fs::read_to_string(&page) {
                Ok(html) => {
                    // A new nonce for each answer, on the reload script only.
                    let nonce = self::token();
                    let body = served_page(&html, token, &nonce);
                    let policy = (!trusted).then(|| content_policy(&nonce));
                    respond_with(
                        &stream,
                        "200 OK",
                        "text/html; charset=utf-8",
                        body.as_bytes(),
                        policy.as_deref(),
                    )
                }
                Err(_) => {
                    let catalog = shared
                        .state()
                        .catalog
                        .clone()
                        .unwrap_or_else(Catalog::english);
                    let text = catalog.tr("preview-being-written");
                    respond(
                        &stream,
                        "503 Service Unavailable",
                        "text/plain; charset=utf-8",
                        text.as_bytes(),
                    )
                }
            }
        }
        "events" => {
            let mut s = &stream;
            s.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\nretry: 1000\n\n",
            )?;
            s.flush()?;
            stream.set_read_timeout(None)?;
            shared.state().listeners.push(stream);
            Ok(())
        }
        other => match other.strip_prefix("files/") {
            Some(rel) => serve_file(&stream, &shared.state().folder.clone(), rel),
            None => not_found(&stream),
        },
    }
}

/// An image, style sheet, or font from `folder`, never outside it, and
/// never a dotfile or a file in a dot folder.
fn serve_file(stream: &TcpStream, folder: &Path, rel: &str) -> io::Result<()> {
    let rel = percent_decode(rel);
    let rel_path = Path::new(&rel);
    let safe = rel_path.components().all(|c| match c {
        Component::Normal(name) => !name.to_string_lossy().starts_with('.'),
        _ => false,
    });
    let kind = content_type(rel_path);
    let Some(kind) = kind.filter(|_| safe && !rel.is_empty()) else {
        return not_found(stream);
    };
    let file = folder.join(rel_path);
    let inside = match (file.canonicalize(), folder.canonicalize()) {
        (Ok(f), Ok(d)) => f.starts_with(d) && f.is_file(),
        _ => false,
    };
    if !inside {
        return not_found(stream);
    }
    match std::fs::read(&file) {
        Ok(bytes) => respond(stream, "200 OK", kind, &bytes),
        Err(_) => not_found(stream),
    }
}

/// The type of a file a page may load: an image, a style sheet, or a
/// font. `None` for anything else (the document's own text, other
/// documents, data), which is never served.
fn content_type(path: &Path) -> Option<&'static str> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "css" => "text/css; charset=utf-8",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => return None,
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(v) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The page as served: its `<base>` points at the served files, and the
/// reload script, carrying `nonce`, is added.
pub(crate) fn served_page(html: &str, token: &str, nonce: &str) -> String {
    let mut out = without_base(html);
    let base = format!("<base href=\"/{token}/files/\">");
    out = match out.to_ascii_lowercase().find("<head>") {
        Some(i) => {
            let at = i + "<head>".len();
            format!("{}\n{base}{}", &out[..at], &out[at..])
        }
        None => format!("{base}{out}"),
    };
    let script = reload_script(token, nonce);
    match out.to_ascii_lowercase().rfind("</body>") {
        Some(i) => format!("{}{script}\n{}", &out[..i], &out[i..]),
        None => format!("{out}\n{script}"),
    }
}

/// `html` without its `<base …>` tag.
fn without_base(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    match lower.find("<base ") {
        Some(a) => match html[a..].find('>') {
            Some(b) => format!("{}{}", &html[..a], &html[a + b + 1..]),
            None => html.to_owned(),
        },
        None => html.to_owned(),
    }
}

/// The script that reloads on a `reload` event and, after the reload,
/// scrolls to the heading it names and focuses it. It carries `nonce`, so
/// the content security policy lets it alone run.
fn reload_script(token: &str, nonce: &str) -> String {
    format!(
        "<script nonce=\"{nonce}\">(function(){{var k=\"tw-preview-anchor\";try{{var a=sessionStorage.getItem(k);if(a!==null){{sessionStorage.removeItem(k);var go=function(){{var el=a?document.getElementById(a):null;if(el){{if(!el.hasAttribute(\"tabindex\"))el.setAttribute(\"tabindex\",\"-1\");el.scrollIntoView();el.focus();}}}};if(document.readyState===\"loading\"){{document.addEventListener(\"DOMContentLoaded\",go);}}else{{go();}}}}}}catch(e){{}}var es=new EventSource(\"/{token}/events\");es.addEventListener(\"reload\",function(e){{try{{sessionStorage.setItem(k,e.data||\"\");}}catch(x){{}}location.reload();}});}})();</script>"
    )
}

/// The ids of the headings in `html`, in order (`<h2 id="methods">`).
pub(crate) fn heading_ids(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = lower[at..].find("<h") {
        let start = at + i;
        at = start + 2;
        let level = lower.as_bytes().get(start + 2).copied();
        if !level.is_some_and(|b| (b'1'..=b'6').contains(&b)) {
            continue;
        }
        let Some(end) = lower[start..].find('>') else {
            break;
        };
        let tag = &html[start..start + end];
        let id = tag.find(" id=\"").and_then(|j| {
            let v = &tag[j + 5..];
            v.find('"').map(|e| v[..e].to_owned())
        });
        out.push(id.unwrap_or_default());
        at = start + end;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    fn get(addr: SocketAddr, path: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let page = dir.path().join("page.html");
        std::fs::write(
            &page,
            "<!doctype html><html><head><base href=\"file:///x/\"><title>t</title></head><body><h1 id=\"intro\">Intro</h1><h2 id=\"methods\">Methods</h2></body></html>",
        )
        .unwrap();
        std::fs::write(dir.path().join("pic.png"), b"PNG").unwrap();
        (dir, page)
    }

    #[test]
    fn binds_localhost_only_and_needs_the_token() {
        let (dir, page) = setup();
        let mut server = PreviewServer::start(page, dir.path().to_owned()).unwrap();
        assert!(server.local_addr().ip().is_loopback());
        assert!(server.url().starts_with("http://127.0.0.1:"));
        assert_eq!(server.token().len(), 32);
        let addr = server.local_addr();
        let page = get(addr, &format!("/{}/", server.token()));
        assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
        assert!(page.contains("EventSource"));
        assert!(page.contains(&format!("<base href=\"/{}/files/\">", server.token())));
        assert!(!page.contains("file:///x/"));
        let wrong = get(addr, "/0123456789abcdef0123456789abcdef/");
        assert!(wrong.starts_with("HTTP/1.1 404"), "{wrong}");
        assert!(get(addr, "/").starts_with("HTTP/1.1 404"));
        let pic = get(addr, &format!("/{}/files/pic.png", server.token()));
        assert!(
            pic.starts_with("HTTP/1.1 200 OK") && pic.ends_with("PNG"),
            "{pic}"
        );
        let escape = get(addr, &format!("/{}/files/../page.html", server.token()));
        assert!(escape.starts_with("HTTP/1.1 404"), "{escape}");
        let escape = get(addr, &format!("/{}/files/%2e%2e/page.html", server.token()));
        assert!(escape.starts_with("HTTP/1.1 404"), "{escape}");
        server.stop();
    }

    #[test]
    fn sends_a_reload_event_and_shuts_down_cleanly() {
        let (dir, page) = setup();
        let mut server = PreviewServer::start(page, dir.path().to_owned()).unwrap();
        let addr = server.local_addr();
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(s, "GET /{}/events HTTP/1.1\r\n\r\n", server.token()).unwrap();
        let mut reader = BufReader::new(s.try_clone().unwrap());
        let mut head = String::new();
        while !head.contains("retry: 1000") {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            head.push_str(&line);
        }
        assert!(head.contains("text/event-stream"), "{head}");
        // Wait until the server has registered the stream.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while server.listening() == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(server.reload(Some("methods")), 1);
        let mut event = String::new();
        while !event.ends_with("\n\n") {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            if line.trim().is_empty() && event.is_empty() {
                continue;
            }
            event.push_str(&line);
        }
        assert_eq!(event, "event: reload\ndata: methods\n\n");
        server.stop();
        // The port is closed and the stream ended.
        let mut rest = String::new();
        let _ = reader.read_to_string(&mut rest);
        assert!(
            TcpStream::connect_timeout(&addr, Duration::from_millis(300))
                .and_then(|mut c| {
                    write!(c, "GET / HTTP/1.1\r\n\r\n")?;
                    let mut b = [0u8; 1];
                    c.set_read_timeout(Some(Duration::from_millis(300)))?;
                    c.read(&mut b)
                })
                .map_or(true, |n| n == 0)
        );
    }

    /// Security L2: the page carries a content security policy whose nonce
    /// is on the reload script alone; `files/` serves only images, style
    /// sheets, and fonts, and never dotfiles; a trusted page goes without
    /// the policy.
    #[test]
    fn the_page_has_a_policy_and_files_are_only_what_a_page_loads() {
        let (dir, page) = setup();
        std::fs::write(dir.path().join("notes.txt"), b"private").unwrap();
        std::fs::write(dir.path().join("other.md"), b"# Other").unwrap();
        std::fs::write(dir.path().join("style.css"), b"p{}").unwrap();
        std::fs::write(dir.path().join(".hidden.png"), b"PNG").unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".git").join("logo.png"), b"PNG").unwrap();
        let mut server = PreviewServer::start(page, dir.path().to_owned()).unwrap();
        let (addr, tok) = (server.local_addr(), server.token().to_owned());
        let served = get(addr, &format!("/{tok}/"));
        let head = served.split("\r\n\r\n").next().unwrap().to_owned();
        let policy = head
            .lines()
            .find_map(|l| l.strip_prefix("Content-Security-Policy: "))
            .expect("a content security policy")
            .to_owned();
        assert!(
            policy.starts_with("default-src 'self'; script-src 'nonce-"),
            "{policy}"
        );
        assert!(policy.contains("connect-src 'self'"), "{policy}");
        assert!(policy.contains("img-src 'self' data:"), "{policy}");
        let nonce = policy
            .split("'nonce-")
            .nth(1)
            .and_then(|r| r.split('\'').next())
            .unwrap()
            .to_owned();
        assert_eq!(nonce.len(), 32);
        assert_eq!(served.matches(&format!("nonce=\"{nonce}\"")).count(), 1);
        // A new nonce for each answer.
        let again = get(addr, &format!("/{tok}/"));
        assert!(!again.contains(&nonce), "{again}");
        for (file, ok) in [
            ("pic.png", true),
            ("style.css", true),
            ("notes.txt", false),
            ("other.md", false),
            ("page.html", false),
            (".hidden.png", false),
            (".git/logo.png", false),
            ("%2egit/logo.png", false),
        ] {
            let answer = get(addr, &format!("/{tok}/files/{file}"));
            assert_eq!(
                answer.starts_with("HTTP/1.1 200 OK"),
                ok,
                "{file}: {answer}"
            );
            assert!(answer.contains("Content-Security-Policy: "), "{file}");
        }
        server.set_trusted(true);
        let trusted = get(addr, &format!("/{tok}/"));
        assert!(trusted.starts_with("HTTP/1.1 200 OK"), "{trusted}");
        assert!(!trusted.contains("Content-Security-Policy"), "{trusted}");
        server.stop();
    }

    #[test]
    fn heading_ids_in_order() {
        assert_eq!(
            heading_ids(
                "<h1 id=\"a\">A</h1><p>x</p><h2 class=\"c\" id=\"b-1\">B</h2><hr><h3>C</h3>"
            ),
            vec!["a".to_owned(), "b-1".to_owned(), String::new()]
        );
        assert_eq!(percent_decode("a%20b%2"), "a b%2");
    }
}
