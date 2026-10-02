//! One pinned file: its name, size, hash, and public address.

use std::borrow::Cow;
use std::io::Read;
use std::path::Path;

use sha1::Digest as _;

/// How a file is checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// SHA-256, lowercase hex. Every built-in component uses it.
    Sha256(Cow<'static, str>),
    /// Git's blob SHA-1, lowercase hex: what Hugging Face publishes for the
    /// small files it keeps in git rather than LFS (a Piper voice's
    /// settings file).
    GitBlobSha1(Cow<'static, str>),
}

impl Check {
    /// The expected hash, lowercase hex.
    pub fn expected(&self) -> &str {
        match self {
            Check::Sha256(h) | Check::GitBlobSha1(h) => h,
        }
    }

    /// True when the hash is well formed: 64 hex digits for SHA-256, 40
    /// for a git blob.
    pub fn is_well_formed(&self) -> bool {
        let (h, len) = match self {
            Check::Sha256(h) => (h, 64),
            Check::GitBlobSha1(h) => (h, 40),
        };
        h.len() == len && h.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

/// One file of a component, pinned by size and hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePin {
    /// Its name in the component's folder: a plain name ([`is_plain_name`]).
    pub name: Cow<'static, str>,
    /// Its public address (a fixed revision). Empty: it comes only from a
    /// mirror or a file.
    pub url: Cow<'static, str>,
    /// Its size in bytes.
    pub size: u64,
    /// How it is checked.
    pub check: Check,
}

impl FilePin {
    /// A file checked by SHA-256, for a `const`.
    pub const fn sha256(
        name: &'static str,
        url: &'static str,
        size: u64,
        sha256: &'static str,
    ) -> Self {
        FilePin {
            name: Cow::Borrowed(name),
            url: Cow::Borrowed(url),
            size,
            check: Check::Sha256(Cow::Borrowed(sha256)),
        }
    }

    /// True when `bytes` are this file.
    pub fn matches_bytes(&self, bytes: &[u8]) -> bool {
        bytes.len() as u64 == self.size && hash_bytes(&self.check, bytes) == self.check.expected()
    }

    /// True when the file at `path` is this file (size, then hash).
    pub fn matches_file(&self, path: &Path) -> bool {
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == self.size)
            && hash_file(&self.check, path).is_ok_and(|h| h == self.check.expected())
    }
}

/// SHA-256 of `bytes`, lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&sha2::Sha256::digest(bytes))
}

/// Git's blob SHA-1 of `data` (what `git hash-object` prints).
pub fn git_blob_sha1(data: &[u8]) -> String {
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", data.len()).as_bytes());
    h.update(data);
    hex(&h.finalize())
}

fn hash_bytes(check: &Check, bytes: &[u8]) -> String {
    match check {
        Check::Sha256(_) => sha256_hex(bytes),
        Check::GitBlobSha1(_) => git_blob_sha1(bytes),
    }
}

/// The hash `check` asks for of the file at `path`, read in pieces (a
/// model can be hundreds of megabytes).
pub fn hash_file(check: &Check, path: &Path) -> std::io::Result<String> {
    let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut buf = vec![0u8; 1 << 16];
    match check {
        Check::Sha256(_) => {
            let mut h = sha2::Sha256::new();
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex(&h.finalize()))
        }
        Check::GitBlobSha1(_) => {
            let len = std::fs::metadata(path)?.len();
            let mut h = sha1::Sha1::new();
            h.update(format!("blob {len}\0").as_bytes());
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex(&h.finalize()))
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// True for a plain file or folder name: letters, digits, `-`, `_`, and
/// `.`, not starting with `.` or `-`, not ending with `.`, at most 120
/// characters, and not a Windows device name. Names from a mirror's
/// manifest or a zip must pass before anything is written under them.
pub fn is_plain_name(name: &str) -> bool {
    let ok_chars = name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    !name.is_empty()
        && name.len() <= 120
        && ok_chars
        && !name.starts_with(['.', '-'])
        && !name.ends_with('.')
        && !name.contains("..")
        && !device
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_match_git_and_sha256() {
        assert_eq!(
            git_blob_sha1(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a");
        std::fs::write(&p, b"hello\n").unwrap();
        let git = Check::GitBlobSha1("ce013625030ba8dba906f756967f9e9ca394464a".into());
        assert_eq!(hash_file(&git, &p).unwrap(), git.expected());
        let pin = FilePin {
            name: "a".into(),
            url: "".into(),
            size: 6,
            check: git,
        };
        assert!(pin.matches_file(&p));
        assert!(pin.matches_bytes(b"hello\n"));
        assert!(!pin.matches_bytes(b"hello!"));
    }

    #[test]
    fn plain_names_only() {
        for good in [
            "encoder_model_int8.onnx",
            "Lexend-Regular.ttf",
            "a",
            "base.en",
        ] {
            assert!(is_plain_name(good), "{good}");
        }
        for bad in [
            "",
            ".hidden",
            "-x",
            "a b",
            "a:b",
            "a/b",
            "a\\b",
            "x.",
            "..",
            "a..b",
            "con.txt",
            "COM1",
            "nul",
            "a$b",
            "a`b",
            "na\u{f03a}me",
            "caf\u{e9}",
        ] {
            assert!(!is_plain_name(bad), "{bad:?}");
        }
        assert!(is_plain_name("console.txt"));
        assert!(is_plain_name("COM10"));
    }
}
