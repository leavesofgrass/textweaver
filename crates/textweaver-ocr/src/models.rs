//! OCR model files: what they are, where they live, whether they are
//! present, and downloading them (only ever after the user agrees).
//!
//! Each file is pinned to a fixed revision and checked by SHA-256 when it is
//! downloaded and again when it is loaded, so a truncated or altered file
//! is never used.
//!
//! Models live in `models/ocr/<set>/` under textweaver's data folder
//! (`TEXTWEAVER_HOME/data` when that is set), or in the folder named by
//! `TEXTWEAVER_OCR_MODELS` (which holds every set's files directly; tests
//! use it).

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};
use textweaver_components::{
    Component, ComponentError, Fetcher, FilePin, Sources, StandardFetcher,
};

use crate::OcrError;

/// One model file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelFile {
    /// The file's name on disk.
    pub name: &'static str,
    /// Where it is downloaded from (a pinned revision).
    pub url: &'static str,
    /// Its size in bytes.
    pub size: u64,
    /// Its SHA-256, in lowercase hexadecimal.
    pub sha256: &'static str,
}

/// A set of model files that work together, with its licence and credit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelSet {
    /// Short id: `ocrs` or `paddle-latin`.
    pub id: &'static str,
    /// What it is, for a person ("the ocrs English text models").
    pub title: &'static str,
    /// The licence the files are under.
    pub licence: &'static str,
    /// Who made them and where they come from (for the notices).
    pub credit: &'static str,
    /// The files.
    pub files: &'static [ModelFile],
}

/// The ocrs models (Robert Knight, CC BY-SA 4.0): text detection and
/// recognition for English (ASCII and the euro sign), 12.2 MB.
///
/// These are the files ocrs's own command-line tool downloads. The older
/// January 2024 models on Hugging Face (`robertknight/ocrs`) read far
/// worse with ocrs 0.13 (80% of words wrong on a clean test page, against
/// under 10% for these), so they are not used. The address is not
/// versioned: if the files change upstream, the SHA-256 check refuses them
/// and the pins here must be updated.
pub const OCRS: ModelSet = ModelSet {
    id: "ocrs",
    title: "the ocrs text recognition models for English",
    licence: "CC BY-SA 4.0",
    credit: "ocrs models by Robert Knight, trained on the HierText dataset (CC BY-SA 4.0), https://github.com/robertknight/ocrs-models",
    files: &[
        ModelFile {
            name: "text-detection.onnx",
            url: "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx",
            size: 2_499_479,
            sha256: "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe",
        },
        ModelFile {
            name: "text-recognition.onnx",
            url: "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx",
            size: 9_713_177,
            sha256: "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5",
        },
    ],
};

/// PaddleOCR's PP-OCRv5 Latin recognition model (Apache-2.0), 8.0 MB,
/// with its configuration (which holds the alphabet). It reads accented
/// Latin-script text; the ocrs detection model finds the lines, so
/// [`OCRS`] is needed too. Experimental.
pub const PADDLE_LATIN: ModelSet = ModelSet {
    id: "paddle-latin",
    title: "the PaddleOCR Latin text recognition model (experimental)",
    licence: "Apache-2.0",
    credit: "latin_PP-OCRv5_mobile_rec by the PaddlePaddle authors, https://huggingface.co/PaddlePaddle/latin_PP-OCRv5_mobile_rec_onnx",
    files: &[
        ModelFile {
            name: "latin_PP-OCRv5_mobile_rec.onnx",
            url: "https://huggingface.co/PaddlePaddle/latin_PP-OCRv5_mobile_rec_onnx/resolve/89d3a50e2c27e2e7cceeab0e944c25c807d5db4f/inference.onnx",
            size: 8_042_023,
            sha256: "7888113072263cb471b93f66dd5e2ad70548dc526fa1ace760d0d973dd121498",
        },
        ModelFile {
            name: "latin_PP-OCRv5_mobile_rec.yml",
            url: "https://huggingface.co/PaddlePaddle/latin_PP-OCRv5_mobile_rec_onnx/resolve/89d3a50e2c27e2e7cceeab0e944c25c807d5db4f/inference.yml",
            size: 6_817,
            sha256: "0bbe984570f597af3638e50bdf2e8276f3ab26a61966096538b3b0d1849f5c84",
        },
    ],
};

/// Every model set textweaver knows.
pub const ALL: [ModelSet; 2] = [OCRS, PADDLE_LATIN];

/// Whether a model set is ready.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelStatus {
    /// Every file is present with the right size.
    Present,
    /// Some files are missing (their names).
    Missing(Vec<&'static str>),
    /// A file has the wrong size (its name): a partial or altered file.
    Damaged(&'static str),
}

/// A folder set by [`set_flat_dir`], which wins over everything.
static FLAT_DIR: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// Makes every model set's files come from one folder, as
/// `TEXTWEAVER_OCR_MODELS` does (a portable install, or tests); `None`
/// goes back to the usual places.
pub fn set_flat_dir(dir: Option<PathBuf>) {
    *FLAT_DIR.write().unwrap_or_else(|p| p.into_inner()) = dir;
}

/// The folder holding every set's files directly, when one is chosen
/// ([`set_flat_dir`] or `TEXTWEAVER_OCR_MODELS`).
pub fn flat_dir() -> Option<PathBuf> {
    FLAT_DIR
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .or_else(|| env_dir("TEXTWEAVER_OCR_MODELS"))
}

/// The folder every model set lives under: `TEXTWEAVER_OCR_MODELS` when
/// set, else `models/ocr` in the data folder.
pub fn root_dir() -> Option<PathBuf> {
    if let Some(dir) = flat_dir() {
        return Some(dir);
    }
    if let Some(home) = env_dir("TEXTWEAVER_HOME") {
        return Some(home.join("data").join("models").join("ocr"));
    }
    directories::ProjectDirs::from("org", "leavesofgrass", "textweaver")
        .map(|d| d.data_dir().join("models").join("ocr"))
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.to_string_lossy().trim().is_empty())
        .map(PathBuf::from)
}

impl ModelSet {
    /// Total size of the files in bytes.
    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    /// The size for a person: "12.2 MB".
    pub fn size_text(&self) -> String {
        // Tenths of a megabyte, rounded (sizes are far below f64's exact range).
        let tenths = (self.size() + 50_000) / 100_000;
        format!("{}.{} MB", tenths / 10, tenths % 10)
    }

    /// The folder this set's files are in (`TEXTWEAVER_OCR_MODELS` holds
    /// every set's files together).
    pub fn dir(&self) -> Option<PathBuf> {
        if let Some(dir) = flat_dir() {
            return Some(dir);
        }
        root_dir().map(|r| r.join(self.id))
    }

    /// The path of `file` in this set's folder.
    pub fn path(&self, file: &ModelFile) -> Option<PathBuf> {
        self.dir().map(|d| d.join(file.name))
    }

    /// Whether the files are on disk (sizes checked; hashes are checked on
    /// load).
    pub fn status(&self) -> ModelStatus {
        let Some(dir) = self.dir() else {
            return ModelStatus::Missing(self.files.iter().map(|f| f.name).collect());
        };
        status_in(self, &dir)
    }

    /// Reads and checks every file (size and SHA-256), in order.
    pub fn load_verified(&self) -> Result<Vec<Vec<u8>>, OcrError> {
        let dir = self.dir().ok_or(OcrError::NoDataDir)?;
        self.files
            .iter()
            .map(|f| {
                let path = dir.join(f.name);
                let bytes = std::fs::read(&path).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        OcrError::ModelsMissing(self.title)
                    } else {
                        OcrError::Io(path.clone(), e)
                    }
                })?;
                verify(f, &bytes)?;
                Ok(bytes)
            })
            .collect()
    }
}

fn status_in(set: &ModelSet, dir: &Path) -> ModelStatus {
    let mut missing = Vec::new();
    for f in set.files {
        match std::fs::metadata(dir.join(f.name)) {
            Ok(m) if m.len() == f.size => {}
            Ok(_) => return ModelStatus::Damaged(f.name),
            Err(_) => missing.push(f.name),
        }
    }
    if missing.is_empty() {
        ModelStatus::Present
    } else {
        ModelStatus::Missing(missing)
    }
}

/// The SHA-256 of `bytes` in lowercase hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Checks a file's size and SHA-256.
pub fn verify(file: &ModelFile, bytes: &[u8]) -> Result<(), OcrError> {
    if bytes.len() as u64 != file.size {
        return Err(OcrError::Checksum(file.name));
    }
    if sha256_hex(bytes) != file.sha256 {
        return Err(OcrError::Checksum(file.name));
    }
    Ok(())
}

/// How far a download has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Bytes received so far, across the set.
    pub done: u64,
    /// Bytes in the whole set.
    pub total: u64,
}

impl ModelSet {
    /// This set as an optional component (`ocr-ocrs`, `ocr-paddle-latin`):
    /// the same pins, for the components registry and the shared
    /// downloader (W8a-d).
    pub fn component(&self) -> Component {
        Component {
            id: Cow::Owned(format!("ocr-{}", self.id)),
            title: Cow::Borrowed(self.title),
            license: Cow::Borrowed(self.licence),
            credit: Cow::Borrowed(self.credit),
            features: Cow::Borrowed(&[Cow::Borrowed("ocr")]),
            folder: Cow::Owned(format!("models/ocr/{}", self.id)),
            files: Cow::Owned(
                self.files
                    .iter()
                    .map(|f| FilePin::sha256(f.name, f.url, f.size, f.sha256))
                    .collect(),
            ),
            notice: None,
            listing: None,
        }
    }

    /// The pinned file named `name`, with its `'static` name.
    fn file_named(&self, name: &str) -> &'static str {
        self.files
            .iter()
            .find(|f| f.name == name)
            .map_or("models", |f| f.name)
    }
}

/// Downloads every missing or damaged file of `set` into its folder,
/// through the shared downloader: the mirror first when one is set
/// (`TEXTWEAVER_COMPONENTS_MIRROR`), then the public address; each file is
/// checked by SHA-256 before it is kept. `progress` is called as bytes
/// arrive; setting `cancel` stops the download (a later one goes on from
/// where it stopped). Only call this after the user has agreed to the
/// download (its size and licence are in [`ModelSet`]). Without the
/// `download` feature, only a mirror that is a folder on this computer
/// works.
pub fn download(
    set: &ModelSet,
    progress: &mut dyn FnMut(DownloadProgress),
    cancel: &AtomicBool,
) -> Result<(), OcrError> {
    download_with(
        set,
        &Sources::from_env_or(""),
        &StandardFetcher,
        progress,
        cancel,
    )
}

/// [`download`] from `sources` through `fetcher` (tests pass a fake one;
/// `tw` passes the mirror from its settings).
pub fn download_with(
    set: &ModelSet,
    sources: &Sources,
    fetcher: &dyn Fetcher,
    progress: &mut dyn FnMut(DownloadProgress),
    cancel: &AtomicBool,
) -> Result<(), OcrError> {
    let dir = set.dir().ok_or(OcrError::NoDataDir)?;
    textweaver_components::download(
        &set.component(),
        &dir,
        sources,
        fetcher,
        &mut |p| {
            progress(DownloadProgress {
                done: p.done,
                total: p.total,
            })
        },
        cancel,
    )
    .map(|_| ())
    .map_err(|e| match e {
        ComponentError::Cancelled => OcrError::Cancelled,
        ComponentError::Hash { file } | ComponentError::Size { file, .. } => {
            OcrError::Checksum(set.file_named(&file))
        }
        ComponentError::Fetch { file, reason } => OcrError::Download(set.file_named(&file), reason),
        other => OcrError::Download(set.file_named(""), other.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_hashes_are_well_formed() {
        assert_eq!(OCRS.size_text(), "12.2 MB");
        assert_eq!(PADDLE_LATIN.size_text(), "8.0 MB");
        for set in ALL {
            for f in set.files {
                assert_eq!(f.sha256.len(), 64, "{}", f.name);
                assert!(f.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
                assert!(f.url.starts_with("https://"));
            }
        }
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn status_and_verify_catch_damage() {
        let dir = tempfile::tempdir().unwrap();
        let set = ModelSet {
            id: "t",
            title: "test",
            licence: "",
            credit: "",
            files: &[ModelFile {
                name: "a.bin",
                url: "https://huggingface.co/x",
                size: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            }],
        };
        assert_eq!(
            status_in(&set, dir.path()),
            ModelStatus::Missing(vec!["a.bin"])
        );
        std::fs::write(dir.path().join("a.bin"), b"ab").unwrap();
        assert_eq!(status_in(&set, dir.path()), ModelStatus::Damaged("a.bin"));
        std::fs::write(dir.path().join("a.bin"), b"abc").unwrap();
        assert_eq!(status_in(&set, dir.path()), ModelStatus::Present);
        assert!(verify(&set.files[0], b"abc").is_ok());
        assert!(verify(&set.files[0], b"abd").is_err());
    }

    #[test]
    fn each_set_is_a_component_with_the_same_pins_and_downloads_through_it() {
        for set in ALL {
            let c = set.component();
            assert!(c.check_names().is_ok(), "{}", set.id);
            assert_eq!(c.files.len(), set.files.len());
            for (pin, f) in c.files.iter().zip(set.files) {
                assert_eq!((pin.name.as_ref(), pin.size), (f.name, f.size));
                assert_eq!(pin.check.expected(), f.sha256);
            }
        }
        let tmp = tempfile::tempdir().unwrap();
        let set = ModelSet {
            id: "t",
            title: "test",
            licence: "",
            credit: "",
            files: &[ModelFile {
                name: "a.bin",
                url: "https://example.invalid/a.bin",
                size: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            }],
        };
        let flat = tmp.path().join("ocr");
        set_flat_dir(Some(flat.clone()));
        let fake = textweaver_components::fake::FakeFetcher::new()
            .with("https://example.invalid/a.bin", b"abd".to_vec());
        let cancel = AtomicBool::new(false);
        let e = download_with(&set, &Sources::public(), &fake, &mut |_| {}, &cancel);
        assert!(matches!(e, Err(OcrError::Checksum("a.bin"))), "{e:?}");
        let fake = textweaver_components::fake::FakeFetcher::new()
            .with("https://example.invalid/a.bin", b"abc".to_vec());
        download_with(&set, &Sources::public(), &fake, &mut |_| {}, &cancel).unwrap();
        assert_eq!(set.status(), ModelStatus::Present);
        set_flat_dir(None);
    }
}
