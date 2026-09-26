//! Opening a web address (feature `url`): `https://...` or `http://...`.
//!
//! The page is fetched (redirects followed, at most [`MAX_BYTES`], within
//! [`TIMEOUT`]) and read by what the server says it is:
//!
//! - **HTML** is decoded with the charset the server names (which wins
//!   over a `<meta charset>` in the page, as in browsers), else the page's
//!   own declaration, else UTF-8 or Windows-1252, and read by the HTML
//!   loader.
//! - **A PDF, EPUB, Word document, or other file** is saved in the cache
//!   folder (`web/` under textweaver's cache) and opened from there.
//!
//! The document's path stays the web address, so notes and the reading
//! position are kept for the address; the `url` property holds it too.

use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use textweaver_text::Document;

use crate::{LoadError, LoadOptions, Registry, Source};

/// The most bytes read from one address.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// How long fetching one address may take.
pub const TIMEOUT: Duration = Duration::from_secs(60);

/// What a response is, from its media type, the address's extension, or
/// its first bytes: the extension to load it as.
fn kind_of(mime: &str, url: &str, head: &[u8]) -> &'static str {
    let by_mime = match mime {
        "text/html" | "application/xhtml+xml" => Some("html"),
        "application/pdf" => Some("pdf"),
        "application/epub+zip" => Some("epub"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some("docx"),
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Some("pptx"),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some("xlsx"),
        "text/markdown" | "text/x-markdown" => Some("md"),
        "text/csv" => Some("csv"),
        "text/plain" => Some("txt"),
        "application/zip" => Some("zip"),
        _ => None,
    };
    if let Some(k) = by_mime {
        return k;
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path
        .rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase());
    const KNOWN: &[&str] = &[
        "pdf", "epub", "docx", "pptx", "xlsx", "md", "markdown", "txt", "csv", "zip", "png", "jpg",
        "jpeg", "html", "htm", "xhtml",
    ];
    if let Some(e) = ext
        && let Some(k) = KNOWN.iter().find(|k| **k == e)
    {
        return k;
    }
    if head.starts_with(b"%PDF-") {
        "pdf"
    } else if head.starts_with(b"PK\x03\x04") {
        "zip"
    } else {
        "html"
    }
}

/// The media type and charset of a `Content-Type` value.
fn parse_content_type(value: &str) -> (String, Option<String>) {
    let mut parts = value.split(';');
    let mime = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let charset = parts.find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k.trim().eq_ignore_ascii_case("charset"))
            .then(|| v.trim().trim_matches('"').to_owned())
            .filter(|v| !v.is_empty())
    });
    (mime, charset)
}

/// The folder downloaded files are kept in.
fn download_dir() -> Option<PathBuf> {
    crate::cache_dir().map(|d| d.join("web"))
}

/// Fetches `url` and loads what it holds.
pub(crate) fn load(
    registry: &Registry,
    url: &str,
    source: &Source,
    options: &LoadOptions,
) -> Result<Document, LoadError> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(LoadError::Unsupported(format!(
            "only web addresses starting with http or https can be opened, not {url}"
        )));
    }
    options.progress.report(0, 1, "Opening the web page.");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .user_agent(concat!(
            "textweaver/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/leavesofgrass/textweaver)"
        ))
        .build()
        .into();
    let fail = |e: &dyn std::fmt::Display| {
        LoadError::Unsupported(format!("the web page {url} could not be opened: {e}"))
    };
    let mut resp = agent
        .get(url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/pdf;q=0.9,*/*;q=0.8",
        )
        .call()
        .map_err(|e| fail(&e))?;
    let status = resp.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("error");
        return Err(fail(&format!(
            "the server answered {} {reason}",
            status.as_u16()
        )));
    }
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let mut body = Vec::new();
    resp.body_mut()
        .as_reader()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|e| fail(&e))?;
    if body.len() as u64 > MAX_BYTES {
        return Err(fail(&"it is larger than 64 MB"));
    }
    if options.progress.is_cancelled() {
        return Err(LoadError::Unsupported(
            "opening the web page was cancelled".into(),
        ));
    }
    let (mime, charset) = parse_content_type(&content_type);
    let kind = kind_of(&mime, url, &body[..body.len().min(16)]);
    let mut doc = if kind == "html" {
        let declared = charset.or_else(|| crate::encoding::sniff_html_charset(&body));
        let decoded = crate::decode_bytes(&body, declared.as_deref());
        let mut meta = crate::meta_for(source, "html");
        crate::note_encoding(&mut meta, &decoded);
        let (text, markers) = crate::html::convert(&decoded.text, options, &mut meta);
        if meta.title.is_none() {
            meta.title = Some(url.to_owned());
        }
        Document::new(meta, ropey::Rope::from_str(&text), markers)
    } else {
        // Files are opened from the cache, as a file on disk would be.
        let saved = download_dir().and_then(|dir| {
            std::fs::create_dir_all(&dir).ok()?;
            let name = format!("{:016x}.{kind}", crate::cache::fnv1a64(url.as_bytes()));
            let path = dir.join(name);
            std::fs::write(&path, &body).ok()?;
            Some(path)
        });
        let src = match saved {
            Some(path) => Source::Path(path),
            None => Source::Bytes {
                data: body,
                hint: kind.to_owned(),
            },
        };
        let mut doc = registry.resolve(&src).load(&src, options)?;
        if let Source::Path(p) = &src {
            doc.meta
                .properties
                .insert("cached_file".into(), p.display().to_string());
        }
        doc
    };
    doc.meta.path = Some(PathBuf::from(url));
    doc.meta.properties.insert("url".into(), url.to_owned());
    if doc.meta.title.as_deref().is_none_or(str::is_empty) {
        doc.meta.title = Some(url.to_owned());
    }
    options.progress.report(1, 1, "The web page is open.");
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpListener;

    use super::*;

    /// Serves one canned response per connection, `n` times.
    fn serve(responses: Vec<Vec<u8>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for resp in responses {
                let Ok((mut s, _)) = listener.accept() else {
                    return;
                };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let _ = s.write_all(&resp);
            }
        });
        format!("http://{addr}")
    }

    fn response(ct: &str, body: &[u8]) -> Vec<u8> {
        let mut r = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        r.extend_from_slice(body);
        r
    }

    #[test]
    fn pages_decode_by_the_server_charset() {
        // The server says Windows-1252; the page claims UTF-8.
        let body = b"<html><head><meta charset=\"utf-8\"><title>Caf\xe9</title></head><body><h1>Men\xfc</h1></body></html>";
        let base = serve(vec![response("text/html; charset=windows-1252", body)]);
        let url = format!("{base}/menu");
        let doc = Registry::with_builtins()
            .load(&Source::Path(PathBuf::from(&url)), &LoadOptions::default())
            .unwrap();
        assert_eq!(doc.text().to_string(), "Menü");
        assert_eq!(doc.meta.title.as_deref(), Some("Café"));
        assert_eq!(doc.meta.path, Some(PathBuf::from(&url)));
        assert_eq!(doc.meta.properties.get("url"), Some(&url));
    }

    #[test]
    fn errors_read_well() {
        let base = serve(vec![
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ]);
        let err = Registry::with_builtins()
            .load(
                &Source::Url(format!("{base}/gone")),
                &LoadOptions::default(),
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("404 Not Found"), "{err}");
        let err = Registry::with_builtins()
            .load(&Source::Url("ftp://x/y".into()), &LoadOptions::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("only web addresses"), "{err}");
    }

    #[test]
    fn kinds_come_from_type_extension_or_bytes() {
        assert_eq!(kind_of("application/pdf", "https://x/a", b""), "pdf");
        assert_eq!(
            kind_of("application/octet-stream", "https://x/a.epub?x=1", b""),
            "epub"
        );
        assert_eq!(kind_of("", "https://x/download", b"%PDF-1.7"), "pdf");
        assert_eq!(kind_of("", "https://x/", b"<html>"), "html");
        assert_eq!(
            parse_content_type("text/html; Charset=\"ISO-8859-1\""),
            ("text/html".into(), Some("ISO-8859-1".into()))
        );
    }
}
