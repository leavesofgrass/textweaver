//! Archives: zip always, and tar, tar.gz, and 7z (feature `archives`).
//!
//! - **Opening an archive** ([`ArchiveLoader`]) lists the files inside that
//!   textweaver can read, each a link to open it. A DAISY book (a zip from
//!   Bookshare) opens as the book, and a zip that is really an EPUB opens
//!   as the EPUB.
//! - **Opening a member**: a path written `book.zip!chapter.pdf` names a
//!   file inside an archive (the part after `!` uses `/` between folders).
//!   [`Source::read`](crate::Source::read) reads it through
//!   [`read_path`], so every loader opens members, and the document's path,
//!   which keys its notes and reading position, is that whole form.
//!   Archives inside archives work too: `outer.zip!inner.tar!notes.md`.
//!
//! A real file whose name contains `!` still opens as itself: a path is a
//! member only when it does not exist and the part before a `!` is a file.
//!
//! Limits, so a hostile archive cannot exhaust memory or time: a member is
//! read up to [`MAX_MEMBER_BYTES`] (a zip member declaring more is refused
//! before it is decompressed); finding a member in a tar or solid 7z
//! archive decompresses at most [`MAX_SCAN_BYTES`]; 7z dictionaries over
//! [`MAX_DICTIONARY`] are refused; listings stop at [`MAX_LISTED`]
//! entries; archives nest at most [`MAX_NESTING`] deep.

use std::io::{self, Cursor, Read, Seek};
use std::path::{Path, PathBuf};

use ropey::Rope;
use textweaver_core::{CharRange, MarkerKind};
use textweaver_text::{Document, Marker};

use crate::builder::Builder;
use crate::{LoadError, LoadOptions, Loader, Registry, Source, meta_for};

/// Separates an archive's path from a member's name.
pub const SEPARATOR: char = '!';

/// Largest member read (uncompressed).
pub const MAX_MEMBER_BYTES: u64 = crate::package::MAX_MEMBER_BYTES;

/// Most bytes decompressed while looking for a member or listing a
/// streamed archive (tar.gz, solid 7z).
pub const MAX_SCAN_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Largest 7z (LZMA) dictionary accepted: each needs that much memory.
pub const MAX_DICTIONARY: u64 = 256 * 1024 * 1024;

/// Most entries listed.
pub const MAX_LISTED: usize = 10_000;

/// Deepest nesting of archives in archives.
pub const MAX_NESTING: usize = 4;

/// The kinds of archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveKind {
    /// Zip (and the formats built on it).
    Zip,
    /// Tar.
    Tar,
    /// Gzip-compressed tar.
    TarGz,
    /// 7z.
    SevenZ,
}

impl ArchiveKind {
    /// The kind of archive `head` (the first bytes of a file) starts.
    pub fn sniff(head: &[u8]) -> Option<ArchiveKind> {
        if head.starts_with(b"PK\x03\x04")
            || head.starts_with(b"PK\x05\x06")
            || head.starts_with(b"PK\x07\x08")
        {
            Some(ArchiveKind::Zip)
        } else if head.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
            Some(ArchiveKind::SevenZ)
        } else if head.starts_with(&[0x1F, 0x8B]) {
            Some(ArchiveKind::TarGz)
        } else if head.get(257..262) == Some(b"ustar") {
            Some(ArchiveKind::Tar)
        } else {
            None
        }
    }

    #[cfg_attr(feature = "archives", allow(dead_code))]
    fn name(self) -> &'static str {
        match self {
            ArchiveKind::Zip => "zip",
            ArchiveKind::Tar => "tar",
            ArchiveKind::TarGz => "tar.gz",
            ArchiveKind::SevenZ => "7z",
        }
    }
}

/// One file in an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its name, folders separated by `/`.
    pub name: String,
    /// Its size, as the archive states it.
    pub size: u64,
}

/// A member name as stored, made comparable: `/` separators, no leading
/// `./` or `/`.
fn normalize(name: &str) -> String {
    let s = name.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for seg in s.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            seg => parts.push(seg),
        }
    }
    parts.join("/")
}

fn io_err(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

fn not_found(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{name} is not in the archive"),
    )
}

/// A reader that fails once more than its budget has been read.
#[cfg_attr(not(feature = "archives"), allow(dead_code))]
struct Budget<R> {
    inner: R,
    left: u64,
}

impl<R: Read> Read for Budget<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.left = self.left.checked_sub(n as u64).ok_or_else(|| {
            io::Error::other("the archive expands to too much data to search safely")
        })?;
        Ok(n)
    }
}

/// Reads at most [`MAX_MEMBER_BYTES`] from `r`, refusing a larger member.
fn read_capped(r: impl Read, name: &str) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    r.take(MAX_MEMBER_BYTES + 1).read_to_end(&mut out)?;
    if out.len() as u64 > MAX_MEMBER_BYTES {
        return Err(io::Error::other(format!("{name} is too large to open")));
    }
    Ok(out)
}

/// The files in an archive (directories left out), at most [`MAX_LISTED`]
/// plus one (so a caller can tell there were more).
pub fn list<R: Read + Seek>(kind: ArchiveKind, r: R) -> io::Result<Vec<Entry>> {
    let limit = MAX_LISTED + 1;
    match kind {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipArchive::new(r).map_err(io_err)?;
            let mut out = Vec::new();
            for i in 0..zip.len() {
                if out.len() >= limit {
                    break;
                }
                let Ok(f) = zip.by_index_raw(i) else { continue };
                if f.is_dir() {
                    continue;
                }
                out.push(Entry {
                    name: normalize(f.name()),
                    size: f.size(),
                });
            }
            Ok(out)
        }
        #[cfg(feature = "archives")]
        ArchiveKind::Tar => tar_list(r, limit),
        #[cfg(feature = "archives")]
        ArchiveKind::TarGz => tar_list(flate2::read::MultiGzDecoder::new(r), limit),
        #[cfg(feature = "archives")]
        ArchiveKind::SevenZ => {
            let reader = seven_open(r)?;
            Ok(reader
                .archive()
                .files
                .iter()
                .filter(|f| !f.is_directory && !f.is_anti_item)
                .take(limit)
                .map(|f| Entry {
                    name: normalize(&f.name),
                    size: f.size,
                })
                .collect())
        }
        #[cfg(not(feature = "archives"))]
        other => Err(unsupported(other)),
    }
}

#[cfg(not(feature = "archives"))]
fn unsupported(kind: ArchiveKind) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "this build of textweaver cannot open {} archives",
            kind.name()
        ),
    )
}

/// Reads member `name` of an archive.
pub fn read_member<R: Read + Seek>(kind: ArchiveKind, r: R, name: &str) -> io::Result<Vec<u8>> {
    let want = normalize(name);
    match kind {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipArchive::new(r).map_err(io_err)?;
            let real = if zip.index_for_name(&want).is_some() {
                Some(want.clone())
            } else {
                // Stored as `./a`, `\a`, or in another case.
                let lower = want.to_lowercase();
                zip.file_names()
                    .find(|n| normalize(n).to_lowercase() == lower)
                    .map(str::to_owned)
            };
            let real = real.ok_or_else(|| not_found(&want))?;
            let f = zip.by_name(&real).map_err(io_err)?;
            // Refused before decompressing: LZMA sizes its dictionary from
            // the declared size, so a huge claim would cost that memory.
            if f.size() > MAX_MEMBER_BYTES {
                return Err(io::Error::other(format!("{want} is too large to open")));
            }
            read_capped(f, &want)
        }
        #[cfg(feature = "archives")]
        ArchiveKind::Tar => tar_read(r, &want),
        #[cfg(feature = "archives")]
        ArchiveKind::TarGz => tar_read(flate2::read::MultiGzDecoder::new(r), &want),
        #[cfg(feature = "archives")]
        ArchiveKind::SevenZ => seven_read(r, &want),
        #[cfg(not(feature = "archives"))]
        other => Err(unsupported(other)),
    }
}

#[cfg(feature = "archives")]
fn tar_list(r: impl Read, limit: usize) -> io::Result<Vec<Entry>> {
    let mut archive = tar::Archive::new(Budget {
        inner: r,
        left: MAX_SCAN_BYTES,
    });
    let mut out = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        if out.len() >= limit {
            break;
        }
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry.path()?.to_string_lossy().into_owned();
        out.push(Entry {
            name: normalize(&name),
            size: entry.size(),
        });
    }
    Ok(out)
}

#[cfg(feature = "archives")]
fn tar_read(r: impl Read, want: &str) -> io::Result<Vec<u8>> {
    let mut archive = tar::Archive::new(Budget {
        inner: r,
        left: MAX_SCAN_BYTES,
    });
    for entry in archive.entries()? {
        let entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = normalize(&entry.path()?.to_string_lossy());
        if name == want {
            return read_capped(entry, want);
        }
    }
    Err(not_found(want))
}

#[cfg(feature = "archives")]
fn seven_open<R: Read + Seek>(r: R) -> io::Result<sevenz_rust2::ArchiveReader<R>> {
    let open = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sevenz_rust2::ArchiveReader::new(r, sevenz_rust2::Password::new(""))
    }));
    match open {
        Ok(Ok(reader)) => Ok(reader),
        Ok(Err(e)) => Err(io_err(e)),
        Err(_) => Err(io_err("not a readable 7z archive")),
    }
}

/// The dictionary size an LZMA or LZMA2 coder asks for.
#[cfg(feature = "archives")]
fn dictionary(coder: &sevenz_rust2::Coder) -> Option<u64> {
    let props = coder.properties();
    match coder.encoder_method_id() {
        // LZMA: a properties byte, then the size (little-endian).
        [3, 1, 1] => props
            .get(1..5)
            .map(|b| u64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
        // LZMA2: one byte, the size's code.
        [0x21] => props.first().map(|&bits| {
            let bits = u64::from(bits & 0x3F);
            if bits >= 40 {
                u64::from(u32::MAX)
            } else {
                (2 | (bits & 1)) << (bits / 2 + 11)
            }
        }),
        _ => None,
    }
}

#[cfg(feature = "archives")]
fn seven_read<R: Read + Seek>(r: R, want: &str) -> io::Result<Vec<u8>> {
    let mut reader = seven_open(r)?;
    for block in &reader.archive().blocks {
        if block
            .coders
            .iter()
            .filter_map(dictionary)
            .any(|d| d > MAX_DICTIONARY)
        {
            return Err(io::Error::other(
                "this 7z archive needs too much memory to open safely",
            ));
        }
    }
    let mut found: Option<io::Result<Vec<u8>>> = None;
    let mut left = MAX_SCAN_BYTES;
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reader.for_each_entries(|entry, data| {
            if normalize(&entry.name) == want && !entry.is_directory {
                found = Some(read_capped(data, want));
                return Ok(false);
            }
            let skipped = io::copy(&mut data.take(left.saturating_add(1)), &mut io::sink())?;
            left = left.checked_sub(skipped).ok_or_else(|| {
                io::Error::other("the archive expands to too much data to search safely")
            })?;
            Ok(true)
        })
    }));
    match run {
        Ok(Ok(())) => {}
        Ok(Err(e)) if found.is_none() => return Err(io_err(e)),
        Ok(Err(_)) => {}
        Err(_) => return Err(io_err("not a readable 7z archive")),
    }
    found.unwrap_or_else(|| Err(not_found(want)))
}

/// An archive's bytes, from a file or from memory.
enum Input {
    File(PathBuf),
    Bytes(Vec<u8>),
}

impl Input {
    fn kind(&self) -> io::Result<ArchiveKind> {
        let head = match self {
            Input::File(p) => {
                let mut head = Vec::new();
                std::fs::File::open(p)?.take(512).read_to_end(&mut head)?;
                head
            }
            Input::Bytes(b) => b[..b.len().min(512)].to_vec(),
        };
        ArchiveKind::sniff(&head)
            .ok_or_else(|| io_err("this is not an archive textweaver can open"))
    }

    fn list(&self) -> io::Result<Vec<Entry>> {
        let kind = self.kind()?;
        match self {
            Input::File(p) => list(kind, std::io::BufReader::new(std::fs::File::open(p)?)),
            Input::Bytes(b) => list(kind, Cursor::new(b.as_slice())),
        }
    }

    fn read(&self, name: &str) -> io::Result<Vec<u8>> {
        let kind = self.kind()?;
        match self {
            Input::File(p) => {
                read_member(kind, std::io::BufReader::new(std::fs::File::open(p)?), name)
            }
            Input::Bytes(b) => read_member(kind, Cursor::new(b.as_slice()), name),
        }
    }
}

/// Splits a member path into the archive's path and the member's name
/// (with `/` separators), or `None` when no `!` follows an existing file.
pub fn split_member(path: &Path) -> Option<(PathBuf, String)> {
    let s = path.to_string_lossy();
    for (i, _) in s.match_indices(SEPARATOR) {
        let archive = Path::new(&s[..i]);
        if archive.is_file() {
            let inner = normalize(&s[i + 1..]);
            if inner.is_empty() {
                return None;
            }
            return Some((archive.to_owned(), inner));
        }
    }
    None
}

/// The member path for `name` inside `archive`: `archive!name`.
pub fn member_path(archive: &Path, name: &str) -> PathBuf {
    let mut s = archive.as_os_str().to_owned();
    s.push(SEPARATOR.to_string());
    s.push(name);
    PathBuf::from(s)
}

/// Reads `inner` from `input`, following `!` into nested archives.
fn read_nested(input: &Input, inner: &str, depth: usize) -> io::Result<Vec<u8>> {
    match input.read(inner) {
        Ok(bytes) => return Ok(bytes),
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        Err(_) => {}
    }
    if depth + 1 >= MAX_NESTING {
        return Err(io::Error::other("archives are nested too deeply"));
    }
    for (i, _) in inner.match_indices(SEPARATOR) {
        let (outer, rest) = (&inner[..i], &inner[i + 1..]);
        match input.read(outer) {
            Ok(bytes) => return read_nested(&Input::Bytes(bytes), &normalize(rest), depth + 1),
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        }
    }
    Err(not_found(inner))
}

/// Reads a file, or an archive member written `archive!member`.
pub fn read_path(path: &Path) -> io::Result<Vec<u8>> {
    if path.exists() {
        return std::fs::read(path);
    }
    match split_member(path) {
        Some((archive, inner)) => read_nested(&Input::File(archive), &inner, 0),
        None => std::fs::read(path),
    }
}

/// True when `path` is a file, or an archive member that exists.
pub fn exists(path: &Path) -> bool {
    if path.exists() {
        return true;
    }
    split_member(path).is_some_and(|_| read_path(path).is_ok())
}

/// Opens archives: lists their readable files, or opens the DAISY book or
/// EPUB inside.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArchiveLoader;

impl Loader for ArchiveLoader {
    fn id(&self) -> &'static str {
        "archive"
    }

    fn extensions(&self) -> &'static [&'static str] {
        if cfg!(feature = "archives") {
            &["zip", "tar", "tgz", "gz", "7z"]
        } else {
            &["zip"]
        }
    }

    fn scan_extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn priority(&self) -> i32 {
        crate::NATIVE_PRIORITY
    }

    fn load(&self, source: &Source, options: &LoadOptions) -> Result<Document, LoadError> {
        let name = crate::title_from_path(source).unwrap_or_else(|| "This archive".to_owned());
        let input = match source {
            Source::Path(p) if p.is_file() => Input::File(p.clone()),
            other => Input::Bytes(other.read()?),
        };
        let bad = |e: io::Error| LoadError::Parse(format!("{name}: {e}"));
        let entries = input.list().map_err(bad)?;
        // A DAISY book or an EPUB opens as itself.
        if let Some(doc) = open_book(&input, &entries, source, options)? {
            return Ok(doc);
        }
        let mut meta = meta_for(source, self.id());
        meta.title = Some(name.clone());
        let registry = Registry::with_builtins();
        let readable: Vec<&Entry> = entries
            .iter()
            .take(MAX_LISTED)
            .filter(|e| {
                let hint = Source::Bytes {
                    data: Vec::new(),
                    hint: Path::new(&e.name)
                        .extension()
                        .map(|x| x.to_string_lossy().to_lowercase())
                        .unwrap_or_default(),
                };
                registry.loader_for(&hint).is_some()
            })
            .collect();
        let base = match source {
            Source::Path(p) => p
                .to_string_lossy()
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("")
                .to_owned(),
            _ => String::new(),
        };
        let total = entries.len().min(MAX_LISTED);
        let mut b = Builder::new();
        let p = b.open(Marker::new(MarkerKind::Paragraph, CharRange::empty(0)));
        b.text(&match (total, readable.len()) {
            (0, _) => format!("{name} is empty."),
            (_, 0) => format!(
                "{name} holds {} and none of them can be read here.",
                files(total)
            ),
            (t, r) if t == r => {
                format!("{name} holds {}. Follow a link to open one.", files(total))
            }
            (_, r) => format!(
                "{name} holds {}; {} can be read here. Follow a link to open one.",
                files(total),
                files(r)
            ),
        });
        b.close(p);
        if !readable.is_empty() {
            b.paragraph_break();
            let list = b.open(Marker::new(MarkerKind::List, CharRange::empty(0)).with_level(1));
            for e in &readable {
                b.line_break();
                let item =
                    b.open(Marker::new(MarkerKind::ListItem, CharRange::empty(0)).with_level(1));
                let target = if base.is_empty() {
                    e.name.clone()
                } else {
                    format!("{base}{SEPARATOR}{}", e.name)
                };
                let link = b.open(
                    Marker::new(MarkerKind::Link, CharRange::empty(0)).with_reference(target),
                );
                b.text(&e.name);
                b.close(link);
                b.close(item);
            }
            b.close(list);
        }
        if entries.len() > MAX_LISTED {
            b.paragraph_break();
            b.text(&format!(
                "Only the first {} files are listed.",
                group_digits(MAX_LISTED)
            ));
        }
        meta.properties
            .insert("archive.files".into(), total.to_string());
        let (text, markers) = b.finish();
        Ok(Document::new(meta, Rope::from_str(&text), markers))
    }
}

fn files(n: usize) -> String {
    if n == 1 {
        "1 file".to_owned()
    } else {
        format!("{} files", group_digits(n))
    }
}

fn group_digits(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A DAISY book (its package file names a DTBook) or an EPUB (its
/// `mimetype` says so) inside the archive, loaded; `None` otherwise.
fn open_book(
    input: &Input,
    entries: &[Entry],
    source: &Source,
    options: &LoadOptions,
) -> Result<Option<Document>, LoadError> {
    let bad = |e: io::Error| LoadError::Parse(e.to_string());
    if entries.iter().any(|e| e.name == "mimetype")
        && let Ok(m) = input.read("mimetype")
        && m.trim_ascii() == b"application/epub+zip"
    {
        let bytes = match input {
            Input::File(p) => std::fs::read(p).map_err(|e| LoadError::Io(p.clone(), e))?,
            Input::Bytes(b) => b.clone(),
        };
        let mut doc = crate::EpubLoader.load(
            &Source::Bytes {
                data: bytes,
                hint: "epub".into(),
            },
            options,
        )?;
        restore_identity(&mut doc, source);
        return Ok(Some(doc));
    }
    // The shallowest package file that is a DAISY book.
    let mut opfs: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.name.to_ascii_lowercase().ends_with(".opf"))
        .collect();
    opfs.sort_by_key(|e| e.name.matches('/').count());
    for opf in opfs.into_iter().take(4) {
        let text = input.read(&opf.name).map_err(bad)?;
        if !crate::daisy::is_daisy_package(&text) {
            continue;
        }
        let mut read = |name: &str| -> Result<Option<Vec<u8>>, LoadError> {
            match input.read(name) {
                Ok(b) => Ok(Some(b)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(LoadError::Parse(e.to_string())),
            }
        };
        let mut doc = crate::daisy::load_package(&opf.name, &text, &mut read, options)?;
        restore_identity(&mut doc, source);
        return Ok(Some(doc));
    }
    Ok(None)
}

/// A book found in an archive keeps the archive's path (for notes and
/// positions) and gets the archive's name as a title when it has none.
fn restore_identity(doc: &mut Document, source: &Source) {
    if let Source::Path(p) = source {
        doc.meta.path = Some(p.clone());
    }
    if doc.meta.title.is_none() {
        doc.meta.title = crate::title_from_path(source);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;

    use super::*;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, body) in files {
            z.start_file(*name, SimpleFileOptions::default()).unwrap();
            z.write_all(body).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn member_paths_split_at_the_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("a!b.zip");
        std::fs::write(&zip, zip_of(&[("x/y.txt", b"hello")])).unwrap();
        let member = member_path(&zip, "x/y.txt");
        let (archive, inner) = split_member(&member).unwrap();
        assert_eq!(archive, zip);
        assert_eq!(inner, "x/y.txt");
        assert_eq!(read_path(&member).unwrap(), b"hello");
        assert!(exists(&member));
        assert!(!exists(&member_path(&zip, "nope.txt")));
        // Backslashes and dot segments from a joined Windows path.
        let joined = PathBuf::from(format!("{}!x\\.\\y.txt", zip.display()));
        assert_eq!(read_path(&joined).unwrap(), b"hello");
        assert_eq!(normalize("./a/../b//c"), "b/c");
    }

    #[test]
    fn nested_archives_open() {
        let dir = tempfile::tempdir().unwrap();
        let inner = zip_of(&[("notes.md", b"# Notes")]);
        let outer = dir.path().join("outer.zip");
        std::fs::write(&outer, zip_of(&[("in/inner.zip", &inner)])).unwrap();
        let path = PathBuf::from(format!("{}!in/inner.zip!notes.md", outer.display()));
        assert_eq!(read_path(&path).unwrap(), b"# Notes");
    }

    #[test]
    fn listing_links_readable_members() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("course.zip");
        std::fs::write(
            &zip,
            zip_of(&[
                ("week1/notes.md", b"# Week one"),
                ("week1/slides.bin", b"\x00\x01"),
                ("readme.txt", b"hi"),
            ]),
        )
        .unwrap();
        let doc = ArchiveLoader
            .load(&Source::Path(zip.clone()), &LoadOptions::default())
            .unwrap();
        let text = doc.text().to_string();
        assert!(
            text.starts_with("course.zip holds 3 files; 2 files can be read here."),
            "{text}"
        );
        let links: Vec<String> = doc
            .marker_index()
            .iter(MarkerKind::Link, None)
            .filter_map(|m| m.reference.clone())
            .collect();
        assert_eq!(
            links,
            ["course.zip!week1/notes.md", "course.zip!readme.txt"]
        );
        // Opening a link's target through the registry reads the member,
        // and the document keeps the member path.
        let member = dir.path().join(&links[0]);
        let doc = Registry::with_builtins()
            .load(&Source::Path(member.clone()), &LoadOptions::default())
            .unwrap();
        assert_eq!(doc.text().to_string(), "Week one");
        assert_eq!(doc.meta.path.as_deref(), Some(member.as_path()));
    }

    #[test]
    fn a_zipped_epub_opens_as_the_book() {
        let container = br#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="c.opf"/></rootfiles></container>"#;
        let opf = br#"<package xmlns="http://www.idpf.org/2007/opf"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Zipped</dc:title></metadata><manifest><item id="a" href="a.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="a"/></spine></package>"#;
        let data = zip_of(&[
            ("mimetype", b"application/epub+zip"),
            ("META-INF/container.xml", container),
            ("c.opf", opf),
            ("a.xhtml", b"<html><body><p>Inside.</p></body></html>"),
        ]);
        let doc = ArchiveLoader
            .load(
                &Source::Bytes {
                    data,
                    hint: "zip".into(),
                },
                &LoadOptions::default(),
            )
            .unwrap();
        assert_eq!(doc.text().to_string(), "Inside.");
        assert_eq!(doc.meta.title.as_deref(), Some("Zipped"));
    }

    /// Every compression method a zip member may use is read, all in pure
    /// Rust (LZMA cannot be written by the zip crate, so it is not here).
    #[test]
    fn members_in_every_compression_method_open() {
        use zip::CompressionMethod::{Bzip2, Deflated, Ppmd, Stored, Xz};
        let body = b"Pure Rust reads this member. ".repeat(40);
        for method in [Stored, Deflated, Bzip2, Xz, Ppmd] {
            let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let options = SimpleFileOptions::default().compression_method(method);
            z.start_file("notes.txt", options).unwrap();
            z.write_all(&body).unwrap();
            let data = z.finish().unwrap().into_inner();
            let read = read_member(ArchiveKind::Zip, Cursor::new(data), "notes.txt");
            assert_eq!(read.unwrap(), body, "{method:?}");
        }
    }

    #[test]
    fn digits_group() {
        assert_eq!(group_digits(10_000), "10,000");
        assert_eq!(group_digits(999), "999");
        assert_eq!(files(1), "1 file");
    }
}
