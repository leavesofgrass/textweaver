//! The `unpack` action: an archive's files are unpacked into the
//! component's folder, and a small receipt takes the archive's place.
//!
//! The archive was checked against its pin before it got here, so what it
//! holds is what the publisher made. Its members still have to be plain:
//! a member with a parent step, an absolute path, or a name that is not
//! plain (letters, digits, `-`, `_`, `.`) is left out with the reason, and
//! links are never followed or made. The receipt, `<archive>.unpacked`,
//! keeps the archive's SHA-256 on its first line and then each unpacked
//! file, one per line, so the component counts as installed without the
//! archive, and Remove takes away exactly the files the archive put there.
//!
//! Zip archives and gzip tarballs are read natively. An xz tarball (the
//! static ffmpeg build for Linux) goes through the system's `tar`.

use std::io::{Read, Write};
use std::path::{Component as PathPart, Path, PathBuf};

use crate::component::Component;
use crate::error::ComponentError;
use crate::pin::{FilePin, is_plain_name};

/// The receipt's suffix: `<archive>.unpacked`.
pub const RECEIPT_SUFFIX: &str = ".unpacked";

/// The most members unpacked from one archive.
const MAX_ENTRIES: usize = 50_000;

/// The most bytes unpacked from one archive.
const MAX_BYTES: u64 = 4_000_000_000;

/// What kind of archive a file is, from its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveKind {
    /// `.zip`.
    Zip,
    /// `.tar.gz` or `.tgz`.
    TarGz,
    /// `.tar.xz` or `.txz`.
    TarXz,
}

impl ArchiveKind {
    /// The kind of archive `name` is, or `None` when it is not one.
    pub fn of(name: &str) -> Option<ArchiveKind> {
        let n = name.to_ascii_lowercase();
        if n.ends_with(".zip") {
            Some(ArchiveKind::Zip)
        } else if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
            Some(ArchiveKind::TarGz)
        } else if n.ends_with(".tar.xz") || n.ends_with(".txz") {
            Some(ArchiveKind::TarXz)
        } else {
            None
        }
    }
}

/// What unpacking left out: each member, with the reason in words.
pub type LeftOut = Vec<(String, String)>;

fn receipt_path(dir: &Path, pin: &FilePin) -> PathBuf {
    dir.join(format!("{}{RECEIPT_SUFFIX}", pin.name))
}

/// The receipt's lines: the hash, then the unpacked files.
fn read_receipt(dir: &Path, pin: &FilePin) -> Option<(String, Vec<String>)> {
    let text = std::fs::read_to_string(receipt_path(dir, pin)).ok()?;
    let mut lines = text.lines();
    let hash = lines.next()?.strip_prefix("sha256 ")?.trim().to_owned();
    Some((hash, lines.map(str::to_owned).collect()))
}

/// True when `dir` holds a receipt for `pin`: its archive, with `pin`'s
/// hash, was unpacked there.
pub(crate) fn receipt_matches(dir: &Path, pin: &FilePin) -> bool {
    read_receipt(dir, pin).is_some_and(|(h, _)| h == pin.check.expected())
}

/// A receipt's file, as a path under `dir`, when every part is plain.
fn listed(dir: &Path, rel: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = rel.split('/').collect();
    parts
        .iter()
        .all(|p| is_plain_name(p))
        .then(|| parts.iter().fold(dir.to_owned(), |p, s| p.join(s)))
}

/// Removes what `pin`'s receipt lists in `dir`, the folders that leaves
/// empty, and the receipt. Returns how many files were removed, the
/// receipt included. Nothing a receipt does not list is touched.
pub(crate) fn remove_unpacked(dir: &Path, pin: &FilePin) -> Result<usize, ComponentError> {
    let Some((_, files)) = read_receipt(dir, pin) else {
        return Ok(0);
    };
    let mut removed = 0;
    let mut folders = Vec::new();
    for rel in &files {
        let Some(path) = listed(dir, rel) else {
            continue;
        };
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| ComponentError::io(&path, e))?;
            removed += 1;
        }
        let mut up = path.parent();
        while let Some(p) = up.filter(|p| *p != dir && p.starts_with(dir)) {
            folders.push(p.to_owned());
            up = p.parent();
        }
    }
    // Deepest first; only folders left empty go.
    folders.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    folders.dedup();
    for f in folders {
        if std::fs::read_dir(&f).is_ok_and(|mut d| d.next().is_none()) {
            let _ = std::fs::remove_dir(&f);
        }
    }
    let receipt = receipt_path(dir, pin);
    std::fs::remove_file(&receipt).map_err(|e| ComponentError::io(&receipt, e))?;
    Ok(removed + 1)
}

/// Unpacks each of `component`'s archives found in `dest` into `dest`,
/// writes its receipt, and removes the archive. A file that is not an
/// archive stays as it is. Returns the members left out, with the reason.
pub fn unpack_in(component: &Component, dest: &Path) -> Result<LeftOut, ComponentError> {
    let mut left_out = Vec::new();
    for pin in component.files.iter() {
        let archive = dest.join(pin.name.as_ref());
        let Some(kind) = ArchiveKind::of(&pin.name) else {
            continue;
        };
        if !archive.is_file() {
            continue;
        }
        let mut written: Vec<String> = Vec::new();
        let result = match kind {
            ArchiveKind::Zip => unzip(&archive, dest, &mut written, &mut left_out),
            ArchiveKind::TarGz => std::fs::File::open(&archive)
                .map_err(|e| ComponentError::io(&archive, e))
                .and_then(|f| {
                    untar(
                        flate2::read::GzDecoder::new(f),
                        &archive,
                        dest,
                        &mut written,
                        &mut left_out,
                    )
                }),
            ArchiveKind::TarXz => untar_with_system(&archive, dest, &mut written, &mut left_out),
        };
        if let Err(e) = result {
            // Nothing half unpacked is left behind.
            for rel in &written {
                if let Some(p) = listed(dest, rel) {
                    let _ = std::fs::remove_file(p);
                }
            }
            return Err(e);
        }
        let mut receipt = format!("sha256 {}\n", pin.check.expected());
        for rel in &written {
            receipt.push_str(rel);
            receipt.push('\n');
        }
        let path = receipt_path(dest, pin);
        std::fs::write(&path, receipt).map_err(|e| ComponentError::io(&path, e))?;
        std::fs::remove_file(&archive).map_err(|e| ComponentError::io(&archive, e))?;
        log::info!(
            "unpacked {} files of {} into {}",
            written.len(),
            pin.name,
            dest.display()
        );
    }
    Ok(left_out)
}

fn unpack_err(archive: &Path, why: impl std::fmt::Display) -> ComponentError {
    ComponentError::Unpack(format!("{}: {why}", archive.display()))
}

/// A member's path as plain parts joined by `/`, or why it is left out.
fn plain_member(raw: &str) -> Result<String, &'static str> {
    if raw.contains('\\') {
        return Err("has a backslash in its name");
    }
    let mut parts = Vec::new();
    for p in Path::new(raw).components() {
        match p {
            PathPart::CurDir => {}
            PathPart::Normal(s) => {
                let s = s.to_str().ok_or("not a plain file name")?;
                if !is_plain_name(s) {
                    return Err("not a plain file name");
                }
                parts.push(s.to_owned());
            }
            _ => return Err("has a parent step or an absolute path"),
        }
    }
    if parts.is_empty() {
        return Err("has no name");
    }
    Ok(parts.join("/"))
}

/// Writes one member from `reader` to `dest/<rel>`, counting bytes.
fn write_member(
    reader: &mut dyn Read,
    dest: &Path,
    rel: &str,
    mode: Option<u32>,
    total: &mut u64,
    archive: &Path,
) -> Result<(), ComponentError> {
    let path = listed(dest, rel).ok_or_else(|| unpack_err(archive, rel))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ComponentError::io(parent, e))?;
    }
    let mut out = std::fs::File::create(&path).map_err(|e| ComponentError::io(&path, e))?;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = reader.read(&mut buf).map_err(|e| unpack_err(archive, e))?;
        if n == 0 {
            break;
        }
        *total += n as u64;
        if *total > MAX_BYTES {
            return Err(unpack_err(archive, "it unpacks to more than 4 GB"));
        }
        out.write_all(&buf[..n])
            .map_err(|e| ComponentError::io(&path, e))?;
    }
    out.flush().map_err(|e| ComponentError::io(&path, e))?;
    set_mode(&path, mode);
    Ok(())
}

/// Keeps a member's execute bits (a program in a tarball), on Unix.
#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>) {
    use std::os::unix::fs::PermissionsExt;
    if let Some(m) = mode {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(m & 0o755));
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: Option<u32>) {}

fn unzip(
    archive: &Path,
    dest: &Path,
    written: &mut Vec<String>,
    left_out: &mut LeftOut,
) -> Result<(), ComponentError> {
    let f = std::fs::File::open(archive).map_err(|e| ComponentError::io(archive, e))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| unpack_err(archive, e))?;
    if zip.len() > MAX_ENTRIES {
        return Err(unpack_err(archive, "it has too many files"));
    }
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| unpack_err(archive, e))?;
        if entry.is_dir() {
            continue;
        }
        let raw = entry.name().to_owned();
        if entry.is_symlink() {
            left_out.push((raw, "a link, not a file".to_owned()));
            continue;
        }
        let rel = match plain_member(&raw) {
            Ok(r) => r,
            Err(why) => {
                left_out.push((raw, why.to_owned()));
                continue;
            }
        };
        let mode = entry.unix_mode();
        write_member(&mut entry, dest, &rel, mode, &mut total, archive)?;
        written.push(rel);
    }
    Ok(())
}

fn untar(
    reader: impl Read,
    archive: &Path,
    dest: &Path,
    written: &mut Vec<String>,
    left_out: &mut LeftOut,
) -> Result<(), ComponentError> {
    let mut tar = tar::Archive::new(reader);
    let mut total = 0u64;
    let entries = tar.entries().map_err(|e| unpack_err(archive, e))?;
    for (n, entry) in entries.enumerate() {
        if n >= MAX_ENTRIES {
            return Err(unpack_err(archive, "it has too many files"));
        }
        let mut entry = entry.map_err(|e| unpack_err(archive, e))?;
        let raw = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            left_out.push((raw, "a link or a special file, not a file".to_owned()));
            continue;
        }
        let rel = match plain_member(&raw) {
            Ok(r) => r,
            Err(why) => {
                left_out.push((raw, why.to_owned()));
                continue;
            }
        };
        let mode = entry.header().mode().ok();
        write_member(&mut entry, dest, &rel, mode, &mut total, archive)?;
        written.push(rel);
    }
    Ok(())
}

/// An xz tarball through the system's `tar` (GNU tar on Linux, bsdtar on
/// macOS and Windows), into a folder beside `dest`, then each plain file
/// moved into `dest`.
// shortcut: no pure-Rust xz reader is in the workspace, so the system's
// `tar` unpacks it; read it natively if an xz crate joins the workspace.
fn untar_with_system(
    archive: &Path,
    dest: &Path,
    written: &mut Vec<String>,
    left_out: &mut LeftOut,
) -> Result<(), ComponentError> {
    let leaf = dest
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ComponentError::BadName(dest.display().to_string()))?;
    let tmp = dest.with_file_name(format!("{leaf}.unpacking"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).map_err(|e| ComponentError::io(&tmp, e))?;
    }
    std::fs::create_dir_all(&tmp).map_err(|e| ComponentError::io(&tmp, e))?;
    let result = (|| {
        let out = textweaver_core::process::command("tar")
            .arg("-xJf")
            .arg(archive)
            .arg("-C")
            .arg(&tmp)
            .output()
            .map_err(|e| unpack_err(archive, format!("tar could not start: {e}")))?;
        if !out.status.success() {
            let why = textweaver_core::process::decode_output(&out.stderr);
            return Err(unpack_err(archive, why.trim()));
        }
        move_tree(&tmp, &tmp, dest, written, left_out, 0)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// Moves the plain files under `dir` (inside `root`) to the same place
/// under `dest`, leaving links and odd names out.
fn move_tree(
    root: &Path,
    dir: &Path,
    dest: &Path,
    written: &mut Vec<String>,
    left_out: &mut LeftOut,
    depth: usize,
) -> Result<(), ComponentError> {
    let entries = std::fs::read_dir(dir).map_err(|e| ComponentError::io(dir, e))?;
    for e in entries.flatten() {
        let path = e.path();
        let rel_os = path.strip_prefix(root).unwrap_or(&path);
        let raw = rel_os
            .components()
            .map(|p| p.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if depth < 16 {
                move_tree(root, &path, dest, written, left_out, depth + 1)?;
            }
            continue;
        }
        if !meta.is_file() {
            left_out.push((raw, "a link or a special file, not a file".to_owned()));
            continue;
        }
        let rel = match plain_member(&raw) {
            Ok(r) => r,
            Err(why) => {
                left_out.push((raw, why.to_owned()));
                continue;
            }
        };
        if written.len() >= MAX_ENTRIES {
            return Err(ComponentError::Unpack("too many files".to_owned()));
        }
        let to = listed(dest, &rel).ok_or_else(|| ComponentError::BadName(rel.clone()))?;
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ComponentError::io(parent, e))?;
        }
        if to.is_file() {
            std::fs::remove_file(&to).map_err(|e| ComponentError::io(&to, e))?;
        }
        std::fs::rename(&path, &to).map_err(|e| ComponentError::io(&to, e))?;
        written.push(rel);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::manifest::{Action, Listing, Platform};
    use crate::pin::Check;
    use std::borrow::Cow;

    fn component(name: &str, bytes: &[u8]) -> Component {
        Component {
            id: "tool".into(),
            title: "A tool".into(),
            license: "CC0".into(),
            credit: "".into(),
            features: Cow::Borrowed(&[]),
            folder: "components/tool".into(),
            files: vec![FilePin {
                name: name.to_owned().into(),
                url: "".into(),
                size: bytes.len() as u64,
                check: Check::Sha256(crate::sha256_hex(bytes).into()),
            }]
            .into(),
            notice: None,
            listing: Some(Listing {
                version: "1.0".into(),
                platform: Platform::Any,
                action: Action::Unpack,
            }),
        }
    }

    #[test]
    fn a_zip_is_unpacked_with_a_receipt_and_removed_exactly() {
        let tmp = tempfile::tempdir().unwrap();
        let bytes = crate::fake::zip_bytes(&[
            ("tool-1.0/bin/tool.exe", b"a made-up program"),
            ("tool-1.0/README.txt", b"read me"),
            ("../escape.txt", b"no"),
            ("tool-1.0/odd name.txt", b"no"),
        ]);
        let c = component("tool-1.0-win64.zip", &bytes);
        let dir = c.dir_in(tmp.path());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tool-1.0-win64.zip"), &bytes).unwrap();
        std::fs::write(dir.join("mine.txt"), b"keep").unwrap();
        let left = unpack_in(&c, &dir).unwrap();
        assert_eq!(left.len(), 2, "{left:?}");
        assert!(left.iter().any(|l| l.1.contains("parent step")), "{left:?}");
        assert!(left.iter().any(|l| l.1.contains("not a plain")), "{left:?}");
        assert!(!tmp.path().join("components").join("escape.txt").exists());
        let exe = dir.join("tool-1.0").join("bin").join("tool.exe");
        assert_eq!(std::fs::read(&exe).unwrap(), b"a made-up program");
        assert!(!dir.join("tool-1.0-win64.zip").exists(), "the archive goes");
        assert_eq!(c.status_in(&dir), crate::Status::Installed);
        assert_eq!(c.verify_in(&dir)[0].1, crate::FileState::Good);
        // Remove takes the unpacked files and the receipt, nothing else.
        assert_eq!(c.remove_in(&dir).unwrap(), 3);
        assert!(!dir.join("tool-1.0").exists());
        assert!(dir.join("mine.txt").is_file());
        assert_eq!(c.status_in(&dir), crate::Status::NotInstalled);
    }

    #[test]
    fn a_gzip_tarball_is_unpacked_keeping_execute_bits_and_leaving_links_out() {
        let tmp = tempfile::tempdir().unwrap();
        let tar_bytes = crate::fake::tar_gz_bytes(
            &[("./tool-1.0/bin/tool", b"#!/bin/sh made-up")],
            &[("tool-1.0/bin/tool-link", "tool")],
        );
        let c = component("tool-1.0-linux.tar.gz", &tar_bytes);
        let dir = c.dir_in(tmp.path());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tool-1.0-linux.tar.gz"), &tar_bytes).unwrap();
        let left = unpack_in(&c, &dir).unwrap();
        assert_eq!(left.len(), 1, "{left:?}");
        assert!(left[0].1.contains("a link"), "{left:?}");
        let tool = dir.join("tool-1.0").join("bin").join("tool");
        assert!(tool.is_file());
        assert!(!dir.join("tool-1.0").join("bin").join("tool-link").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&tool).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "{mode:o}");
        }
        assert_eq!(c.status_in(&dir), crate::Status::Installed);
    }

    /// Downloaded from a fake server, checked, then unpacked; a second
    /// download fetches nothing, by the receipt.
    #[test]
    fn a_download_with_the_unpack_action_unpacks_once() {
        use crate::fake::FakeFetcher;
        use std::sync::atomic::AtomicBool;
        let tmp = tempfile::tempdir().unwrap();
        let bytes = crate::fake::zip_bytes(&[("tool-1.0/tool.exe", b"made up")]);
        let mut c = component("tool-1.0.zip", &bytes);
        c.files.to_mut()[0].url = "https://example.invalid/tool-1.0.zip".into();
        let fetcher = FakeFetcher::new().with("https://example.invalid/tool-1.0.zip", bytes);
        let dir = c.dir_in(tmp.path());
        let cancel = AtomicBool::new(false);
        let sources = crate::Sources::public();
        let out = crate::download(&c, &dir, &sources, &fetcher, &mut |_| {}, &cancel).unwrap();
        assert_eq!(out.fetched, ["tool-1.0.zip"]);
        assert!(dir.join("tool-1.0").join("tool.exe").is_file());
        assert_eq!(c.status_in(&dir), crate::Status::Installed);
        let again = crate::download(&c, &dir, &sources, &fetcher, &mut |_| {}, &cancel).unwrap();
        assert_eq!(again.kept, ["tool-1.0.zip"]);
        assert_eq!(fetcher.request_count(), 1);
    }

    #[test]
    fn archive_kinds_are_known_by_name() {
        assert_eq!(ArchiveKind::of("a.ZIP"), Some(ArchiveKind::Zip));
        assert_eq!(ArchiveKind::of("a.tar.gz"), Some(ArchiveKind::TarGz));
        assert_eq!(ArchiveKind::of("a.tar.xz"), Some(ArchiveKind::TarXz));
        assert_eq!(ArchiveKind::of("setup.exe"), None);
    }
}
