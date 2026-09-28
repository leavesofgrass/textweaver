//! Downloading a voice (feature `download`), only after the user confirms.
//!
//! 1. [`plan`] asks Hugging Face for the voice's files and their hashes
//!    (the tree API: `lfs.oid` is the SHA-256 of a large file; `oid` is
//!    the git blob SHA-1 of a small one) and reads its `MODEL_CARD` for the
//!    licence. Nothing is saved yet.
//! 2. The app reads [`DownloadPlan::describe`] to the user: name,
//!    language, quality, size, and licence. A non-commercial licence is
//!    said plainly.
//! 3. When the user confirms, [`download`] fetches each file into a
//!    temporary folder, checks its size and hash, and only then moves the
//!    folder into the voice store. A file that does not match is deleted
//!    and the download fails.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use sha1::Digest as _;

use crate::PiperError;
use crate::catalog::{CATALOG_URL, Catalog, CatalogVoice, Licence, REPOSITORY, file_url};
use crate::store::{InstalledVoice, VoiceStore};

/// How a downloaded file is checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// SHA-256, lowercase hex (Hugging Face's `lfs.oid`).
    Sha256(String),
    /// Git's blob SHA-1, lowercase hex (Hugging Face's `oid` for files
    /// kept in git rather than LFS).
    GitBlobSha1(String),
}

/// One file to download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteFile {
    /// Path in the repository.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// How to check it.
    pub check: Check,
}

impl RemoteFile {
    /// The file's name.
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

/// What would be downloaded, for the user to confirm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadPlan {
    /// The voice.
    pub voice: CatalogVoice,
    /// Its files.
    pub files: Vec<RemoteFile>,
    /// Its licence.
    pub licence: Licence,
}

impl DownloadPlan {
    /// Bytes to download.
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    /// The confirmation to read aloud: "Download Joe, English (United
    /// States), medium quality, 63 MB? Licence: CC0. Free to use for
    /// anything."
    pub fn describe(&self) -> String {
        format!(
            "Download {}? {}",
            self.voice.describe(),
            self.licence.describe()
        )
    }
}

#[derive(Deserialize)]
struct TreeEntry {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    oid: String,
    #[serde(default)]
    lfs: Option<LfsInfo>,
}

#[derive(Deserialize)]
struct LfsInfo {
    oid: String,
}

/// Turns a Hugging Face tree listing into the files `wanted` names.
pub fn files_from_tree(json: &str, wanted: &[&str]) -> Result<Vec<RemoteFile>, PiperError> {
    let entries: Vec<TreeEntry> = serde_json::from_str(json)
        .map_err(|e| PiperError::Download(format!("unexpected file list: {e}")))?;
    let mut out = Vec::new();
    for w in wanted {
        let e = entries
            .iter()
            .find(|e| e.kind == "file" && e.path == *w)
            .ok_or_else(|| PiperError::Download(format!("{w} is not in the repository")))?;
        let check = match &e.lfs {
            Some(l) if is_hex(&l.oid, 64) => Check::Sha256(l.oid.to_ascii_lowercase()),
            None if is_hex(&e.oid, 40) => Check::GitBlobSha1(e.oid.to_ascii_lowercase()),
            _ => {
                return Err(PiperError::Download(format!(
                    "{w} has no hash to check it against"
                )));
            }
        };
        out.push(RemoteFile {
            path: e.path.clone(),
            size: e.size,
            check,
        });
    }
    Ok(out)
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// SHA-256 of `r`, lowercase hex.
pub fn sha256_hex(mut r: impl Read) -> std::io::Result<String> {
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

/// Git's blob SHA-1 of `data` (what `git hash-object` prints).
pub fn git_blob_sha1(data: &[u8]) -> String {
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", data.len()).as_bytes());
    h.update(data);
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Checks the file at `path` against `file`'s size and hash.
pub fn verify(file: &RemoteFile, path: &Path) -> Result<(), PiperError> {
    let len = std::fs::metadata(path)
        .map_err(|e| PiperError::io(path, e))?
        .len();
    if len != file.size {
        return Err(PiperError::Download(format!(
            "{} is {len} bytes, expected {}",
            file.name(),
            file.size
        )));
    }
    let actual = match &file.check {
        Check::Sha256(_) => {
            let f = std::fs::File::open(path).map_err(|e| PiperError::io(path, e))?;
            sha256_hex(std::io::BufReader::new(f)).map_err(|e| PiperError::io(path, e))?
        }
        Check::GitBlobSha1(_) => {
            git_blob_sha1(&std::fs::read(path).map_err(|e| PiperError::io(path, e))?)
        }
    };
    let expected = match &file.check {
        Check::Sha256(h) | Check::GitBlobSha1(h) => h,
    };
    if actual != *expected {
        return Err(PiperError::Download(format!(
            "{} does not match its published hash (expected {expected}, got {actual})",
            file.name()
        )));
    }
    Ok(())
}

/// The User-Agent sent with every request: the project, nothing personal.
pub const USER_AGENT: &str = "textweaver-research (+https://github.com/leavesofgrass/textweaver)";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_body(Some(Duration::from_secs(60)))
        // Neutral on purpose: no user, machine, or account names ever go
        // out with a request (the owner's rule).
        .user_agent(USER_AGENT)
        .build()
        .into()
}

fn get_text(agent: &ureq::Agent, url: &str) -> Result<String, PiperError> {
    agent
        .get(url)
        .call()
        .map_err(|e| PiperError::Download(format!("{url}: {e}")))?
        .body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| PiperError::Download(format!("{url}: {e}")))
}

/// Downloads and parses the voice catalogue (`voices.json`, about 250 KB).
pub fn fetch_catalog() -> Result<Catalog, PiperError> {
    fetch_catalog_json().map(|(_, c)| c)
}

/// Downloads the voice catalogue: its text (to keep in the voices folder)
/// and the parsed catalogue.
pub fn fetch_catalog_json() -> Result<(String, Catalog), PiperError> {
    let json = get_text(&agent(), CATALOG_URL)?;
    let catalog = Catalog::from_json(&json)?;
    Ok((json, catalog))
}

/// Asks Hugging Face for `voice`'s files and hashes and reads its licence.
/// Downloads only the small `MODEL_CARD`; saves nothing.
pub fn plan(voice: &CatalogVoice) -> Result<DownloadPlan, PiperError> {
    let agent = agent();
    let wanted: Vec<&str> = voice.files.keys().map(String::as_str).collect();
    let dir = wanted
        .first()
        .and_then(|p| p.rsplit_once('/'))
        .map(|(d, _)| d)
        .ok_or_else(|| PiperError::Download(format!("{} lists no files", voice.key)))?;
    let tree = get_text(
        &agent,
        &format!("https://huggingface.co/api/models/{REPOSITORY}/tree/main/{dir}"),
    )?;
    let files = files_from_tree(&tree, &wanted)?;
    let licence = match voice.model_card_path() {
        Some(p) => Licence::from_model_card(&get_text(&agent, &file_url(p))?),
        None => Licence::classify(""),
    };
    Ok(DownloadPlan {
        voice: voice.clone(),
        files,
        licence,
    })
}

/// Downloads a confirmed plan into `store`. `progress` hears bytes done
/// and the total, and cancels the download by returning false. Nothing
/// is installed unless every file arrived whole and matched its hash.
pub fn download(
    plan: &DownloadPlan,
    store: &VoiceStore,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<InstalledVoice, PiperError> {
    let key = &plan.voice.key;
    let dest = store.voice_dir(key);
    let part = store.dir().join(format!(".{key}.part"));
    let _ = std::fs::remove_dir_all(&part);
    std::fs::create_dir_all(&part).map_err(|e| PiperError::io(&part, e))?;
    let result = fetch_all(plan, &part, progress);
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&part);
        return Err(e);
    }
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(&part, &dest).map_err(|e| PiperError::io(&dest, e))?;
    store
        .get(key)
        .ok_or_else(|| PiperError::Download(format!("{key} is missing a model or settings file")))
}

fn fetch_all(
    plan: &DownloadPlan,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<(), PiperError> {
    let agent = agent();
    let total = plan.total_bytes();
    let mut done = 0u64;
    for f in &plan.files {
        let path: PathBuf = dir.join(f.name());
        fetch(&agent, &file_url(&f.path), &path, &mut |n| {
            progress(done + n, total)
        })?;
        verify(f, &path)?;
        done += f.size;
    }
    Ok(())
}

fn fetch(
    agent: &ureq::Agent,
    url: &str,
    path: &Path,
    progress: &mut dyn FnMut(u64) -> bool,
) -> Result<(), PiperError> {
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| PiperError::Download(format!("{url}: {e}")))?;
    let mut reader = resp
        .body_mut()
        .with_config()
        .limit(4 * 1024 * 1024 * 1024)
        .reader();
    let mut out =
        std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| PiperError::io(path, e))?);
    let mut buf = vec![0u8; 1 << 16];
    let mut n = 0u64;
    loop {
        let got = reader
            .read(&mut buf)
            .map_err(|e| PiperError::Download(format!("{url}: {e}")))?;
        if got == 0 {
            break;
        }
        out.write_all(&buf[..got])
            .map_err(|e| PiperError::io(path, e))?;
        n += got as u64;
        if !progress(n) {
            return Err(PiperError::Download("cancelled".into()));
        }
    }
    out.flush().map_err(|e| PiperError::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREE: &str = r#"[
      {"type":"directory","oid":"a208","size":0,"path":"en/en_US/joe/medium/samples"},
      {"type":"file","oid":"f001792ffaccdde85ed0e21079d5636c67136bf0","size":281,"path":"en/en_US/joe/medium/MODEL_CARD"},
      {"type":"file","oid":"8c620f3be0b47774b4dc5a1beea372efc4a76a93","size":63201294,
       "lfs":{"oid":"58afce0321b8d9c46d7cdf9c16500cc55a793b4220212dba6b70fb788b3baf06","size":63201294,"pointerSize":133},
       "path":"en/en_US/joe/medium/en_US-joe-medium.onnx"},
      {"type":"file","oid":"b4ab600fbd71cee986a8e7714bd88996f33be945","size":4794,"path":"en/en_US/joe/medium/en_US-joe-medium.onnx.json"}
    ]"#;

    #[test]
    fn tree_listing_gives_hashes() {
        let files = files_from_tree(
            TREE,
            &[
                "en/en_US/joe/medium/en_US-joe-medium.onnx",
                "en/en_US/joe/medium/MODEL_CARD",
            ],
        )
        .unwrap();
        assert_eq!(files[0].name(), "en_US-joe-medium.onnx");
        assert_eq!(files[0].size, 63_201_294);
        assert_eq!(
            files[0].check,
            Check::Sha256(
                "58afce0321b8d9c46d7cdf9c16500cc55a793b4220212dba6b70fb788b3baf06".into()
            )
        );
        assert_eq!(
            files[1].check,
            Check::GitBlobSha1("f001792ffaccdde85ed0e21079d5636c67136bf0".into())
        );
        assert!(files_from_tree(TREE, &["missing"]).is_err());
        assert!(files_from_tree("{}", &[]).is_err());
    }

    #[test]
    fn hashes_match_git_and_sha256() {
        // `printf 'hello\n' | git hash-object --stdin`
        assert_eq!(
            git_blob_sha1(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
        assert_eq!(
            sha256_hex(&b"abc"[..]).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_refuses_a_wrong_size_or_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("MODEL_CARD");
        std::fs::write(&p, b"hello\n").unwrap();
        let mut f = RemoteFile {
            path: "x/MODEL_CARD".into(),
            size: 6,
            check: Check::GitBlobSha1("ce013625030ba8dba906f756967f9e9ca394464a".into()),
        };
        verify(&f, &p).unwrap();
        f.size = 7;
        assert!(verify(&f, &p).is_err());
        f.size = 6;
        f.check = Check::Sha256("00".repeat(32));
        let e = verify(&f, &p).unwrap_err().to_string();
        assert!(e.contains("does not match its published hash"), "{e}");
    }

    #[test]
    fn the_confirmation_says_size_and_licence() {
        let catalog = Catalog::from_json(crate::catalog::tests::VOICES).unwrap();
        let plan = DownloadPlan {
            voice: catalog.get("en_US-joe-medium").unwrap().clone(),
            files: files_from_tree(TREE, &["en/en_US/joe/medium/en_US-joe-medium.onnx"]).unwrap(),
            licence: Licence::classify("CC BY-NC-SA 4.0"),
        };
        assert_eq!(plan.total_bytes(), 63_201_294);
        assert_eq!(
            plan.describe(),
            "Download Joe, English (United States), medium quality, 63 MB? \
             Licence: CC BY-NC-SA 4.0. Personal and non-commercial use only."
        );
    }

    /// Downloads the catalogue for real. Run with `--ignored` and network.
    #[test]
    #[ignore = "needs the network"]
    fn fetches_the_real_catalogue() {
        let c = fetch_catalog().unwrap();
        assert!(c.voices.len() > 100);
        let joe = c.get("en_US-joe-medium").unwrap();
        let plan = plan(joe).unwrap();
        assert_eq!(plan.licence.text, "CC0");
        assert_eq!(plan.files.len(), 3);
    }
}
