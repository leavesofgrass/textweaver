//! Downloading a voice (feature `download`), only after the user confirms.
//!
//! 1. [`plan`] asks Hugging Face for the voice's files and their hashes
//!    (the tree API: `lfs.oid` is the SHA-256 of a large file; `oid` is
//!    the git blob SHA-1 of a small one) and reads its `MODEL_CARD` for the
//!    licence. Nothing is saved yet.
//! 2. The app reads [`DownloadPlan::describe`] to the user: name,
//!    language, quality, size, and licence. A non-commercial licence is
//!    said plainly.
//! 3. When the user confirms, [`download`] fetches the voice as an
//!    optional component through the shared downloader
//!    (`textweaver-components`): each file into a staging folder, checked
//!    by its size and hash, and only then moved into the voice store. A
//!    file that does not match is deleted and the download fails; a
//!    stopped download goes on where it stopped.

use std::borrow::Cow;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use sha1::Digest as _;
use textweaver_components::{Component, Fetcher, FilePin, Sources, StandardFetcher};

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
    /// States), medium quality, 63 MB? License: CC0. Free to use for
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

/// The User-Agent sent with every request: the project, nothing personal
/// (the shared downloader's).
pub const USER_AGENT: &str = textweaver_components::USER_AGENT;

/// Reads a small text answer (a file list, a model card, the catalogue).
fn get_text(fetcher: &dyn Fetcher, url: &str) -> Result<String, PiperError> {
    let bytes = textweaver_components::fetch_bytes(fetcher, url, 16 * 1024 * 1024)
        .map_err(|e| PiperError::Download(format!("{url}: {e}")))?;
    String::from_utf8(bytes).map_err(|e| PiperError::Download(format!("{url}: {e}")))
}

/// Downloads and parses the voice catalogue (`voices.json`, about 250 KB).
pub fn fetch_catalog() -> Result<Catalog, PiperError> {
    fetch_catalog_json().map(|(_, c)| c)
}

/// Downloads the voice catalogue: its text (to keep in the voices folder)
/// and the parsed catalogue.
pub fn fetch_catalog_json() -> Result<(String, Catalog), PiperError> {
    let json = get_text(&StandardFetcher, CATALOG_URL)?;
    let catalog = Catalog::from_json(&json)?;
    Ok((json, catalog))
}

/// Asks Hugging Face for `voice`'s files and hashes and reads its licence.
/// Downloads only the small `MODEL_CARD`; saves nothing.
pub fn plan(voice: &CatalogVoice) -> Result<DownloadPlan, PiperError> {
    let agent = StandardFetcher;
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

/// The optional component a plan downloads: `piper-<voice key>`, with
/// each file pinned by the hash Hugging Face published for it.
pub fn component(plan: &DownloadPlan) -> Component {
    Component {
        id: Cow::Owned(format!("piper-{}", plan.voice.key)),
        title: Cow::Owned(plan.voice.describe()),
        license: Cow::Owned(plan.licence.text.clone()),
        credit: Cow::Owned(format!("Piper voice from {REPOSITORY}")),
        features: Cow::Borrowed(&[Cow::Borrowed("voice")]),
        folder: Cow::Owned(format!("voices/{}", plan.voice.key)),
        files: Cow::Owned(
            plan.files
                .iter()
                .map(|f| FilePin {
                    name: Cow::Owned(f.name().to_owned()),
                    url: Cow::Owned(file_url(&f.path)),
                    size: f.size,
                    check: match &f.check {
                        Check::Sha256(h) => textweaver_components::Check::Sha256(h.clone().into()),
                        Check::GitBlobSha1(h) => {
                            textweaver_components::Check::GitBlobSha1(h.clone().into())
                        }
                    },
                })
                .collect(),
        ),
        notice: None,
    }
}

/// Downloads a confirmed plan into `store`. `progress` hears bytes done
/// and the total, and cancels the download by returning false. Nothing
/// is installed unless every file arrived whole and matched its hash.
pub fn download(
    plan: &DownloadPlan,
    store: &VoiceStore,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<InstalledVoice, PiperError> {
    download_with(
        plan,
        store,
        &Sources::from_env_or(""),
        &StandardFetcher,
        progress,
    )
}

/// [`download`] from `sources` through `fetcher` (tests pass a fake one).
pub fn download_with(
    plan: &DownloadPlan,
    store: &VoiceStore,
    sources: &Sources,
    fetcher: &dyn Fetcher,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<InstalledVoice, PiperError> {
    let key = &plan.voice.key;
    let dest = store.voice_dir(key);
    let cancel = AtomicBool::new(false);
    textweaver_components::download(
        &component(plan),
        &dest,
        sources,
        fetcher,
        &mut |p| {
            if !progress(p.done, p.total) {
                cancel.store(true, Ordering::Relaxed);
            }
        },
        &cancel,
    )
    .map_err(|e| PiperError::Download(e.to_string()))?;
    store
        .get(key)
        .ok_or_else(|| PiperError::Download(format!("{key} is missing a model or settings file")))
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
             License: CC BY-NC-SA 4.0. Personal and non-commercial use only."
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

    #[test]
    fn a_voice_downloads_through_the_shared_downloader() {
        let catalog = Catalog::from_json(crate::catalog::tests::VOICES).unwrap();
        let onnx = b"model bytes".to_vec();
        let json = b"{}".to_vec();
        let plan = DownloadPlan {
            voice: catalog.get("en_US-joe-medium").unwrap().clone(),
            files: vec![
                RemoteFile {
                    path: "en/en_US/joe/medium/en_US-joe-medium.onnx".into(),
                    size: onnx.len() as u64,
                    check: Check::Sha256(textweaver_components::sha256_hex(&onnx)),
                },
                RemoteFile {
                    path: "en/en_US/joe/medium/en_US-joe-medium.onnx.json".into(),
                    size: json.len() as u64,
                    check: Check::GitBlobSha1(git_blob_sha1(&json)),
                },
            ],
            licence: Licence::classify("CC0"),
        };
        let c = component(&plan);
        assert_eq!(c.id, "piper-en_US-joe-medium");
        assert!(c.check_names().is_ok());
        let fake = textweaver_components::fake::FakeFetcher::new()
            .with(file_url(&plan.files[0].path), onnx)
            .with(file_url(&plan.files[1].path), json);
        let tmp = tempfile::tempdir().unwrap();
        let store = VoiceStore::new(tmp.path().join("voices"));
        let mut last = (0, 0);
        let v = download_with(&plan, &store, &Sources::public(), &fake, &mut |d, t| {
            last = (d, t);
            true
        })
        .unwrap();
        assert_eq!(v.key, "en_US-joe-medium");
        assert_eq!(last, (13, 13));
        assert_eq!(store.installed().len(), 1);
        // Cancelling by returning false stops it.
        let store2 = VoiceStore::new(tmp.path().join("other"));
        let e = download_with(&plan, &store2, &Sources::public(), &fake, &mut |_, _| false);
        assert!(e.is_err());
        assert!(store2.installed().is_empty());
    }
}
