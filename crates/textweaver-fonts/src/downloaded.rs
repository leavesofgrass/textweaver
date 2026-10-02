//! Reading fonts textweaver downloads on first choice, after asking:
//! Lexend, the one reading font Star offered that is not bundled.
//!
//! Each font is pinned ([`DownloadableFont`]): its files' URLs at an
//! immutable commit of the font's own repository, their sizes, and their
//! SHA-256 values. Each is also an optional component
//! ([`DownloadableFont::component`]), and [`DownloadableFont::install`]
//! downloads through the shared downloader (`textweaver-components`): each
//! file goes to a staging folder, is checked by size and hash, and only
//! when all matched are the files and the license (`OFL.txt`) moved into
//! place, so a font is either all there and checked or not there at all. The license text ships with textweaver
//! (`third_party/fonts/lexend/OFL.txt`), as the bundled fonts' do.
//!
//! The fonts live in the data folder (`fonts/<key>/`). A frontend tells
//! this crate where with [`set_folder`]; from then on
//! [`resolve_family`](crate::resolve_family) finds a downloaded family as
//! it finds an installed one, so the PDF writer uses it by name, the EPUB
//! writer embeds it ([`find_loaded`]), and the GUI registers its files
//! ([`DownloadableFont::load_from`]).
//!
//! Nothing here asks the reader: the app says the size and license and
//! waits for a yes before calling [`DownloadableFont::install`]. Tests use
//! a fake [`Fetcher`]; only the shared HTTP fetcher (cargo feature
//! `download`) goes to the network, with a neutral User-Agent.

use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;

use sha2::Digest as _;
#[cfg(feature = "download")]
pub use textweaver_components::HttpFetcher;
use textweaver_components::{Component, ComponentError, FilePin, Progress, Sources};
pub use textweaver_components::{Fetched, Fetcher, USER_AGENT};

use crate::Style;
use crate::system::{FaceRef, FamilyFaces};

/// One pinned font file.
#[derive(Debug, PartialEq, Eq)]
pub struct PinnedFile {
    /// Which style it is.
    pub style: Style,
    /// The upstream file name, kept in the data folder.
    pub file_name: &'static str,
    /// Where it is downloaded from, pinned to a commit.
    pub url: &'static str,
    /// Its size in bytes.
    pub size: u64,
    /// Its SHA-256, lowercase hex.
    pub sha256: &'static str,
}

impl PinnedFile {
    /// The media type: `font/ttf` or `font/otf`.
    pub fn media_type(&self) -> &'static str {
        if self.file_name.ends_with(".otf") {
            "font/otf"
        } else {
            "font/ttf"
        }
    }
}

/// A reading font that is downloaded, not bundled.
#[derive(Debug, PartialEq, Eq)]
pub struct DownloadableFont {
    /// Settings key and folder name (`lexend`).
    pub key: &'static str,
    /// The family name the files register under.
    pub name: &'static str,
    /// Other names that choose this family.
    pub aliases: &'static [&'static str],
    /// Who made it and why it helps, in one sentence.
    pub about: &'static str,
    /// SPDX license identifier.
    pub license: &'static str,
    /// The license's name, as said in the question ("SIL Open Font
    /// License").
    pub license_name: &'static str,
    /// The license file, as published upstream (shipped with textweaver).
    pub license_text: &'static str,
    /// The copyright line.
    pub copyright: &'static str,
    /// Home page.
    pub homepage: &'static str,
    /// Upstream version (the pinned commit).
    pub version: &'static str,
    /// The files, regular first.
    pub files: &'static [PinnedFile],
}

/// A downloaded face, read from the data folder and checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedFace {
    /// Which style it is.
    pub style: Style,
    /// The file name.
    pub file_name: &'static str,
    /// Its media type.
    pub media_type: &'static str,
    /// The font file.
    pub data: Vec<u8>,
}

/// Why a font could not be downloaded or installed. The texts are for
/// logs and are put into the app's own message ("Lexend not downloaded:
/// ...").
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// The fetcher failed (no network, a server error, cancelled).
    #[error("{file}: {reason}")]
    Fetch {
        /// The file.
        file: &'static str,
        /// What went wrong.
        reason: String,
    },
    /// The file arrived with the wrong size.
    #[error("{file} is {got} bytes, not {expected}")]
    Size {
        /// The file.
        file: &'static str,
        /// Bytes received.
        got: u64,
        /// Bytes expected.
        expected: u64,
    },
    /// The file does not match its pinned SHA-256.
    #[error("{file} does not match its published hash")]
    Hash {
        /// The file.
        file: &'static str,
    },
    /// Writing to the data folder failed.
    #[error("{}: {source}", path.display())]
    Io {
        /// Where.
        path: PathBuf,
        /// The error.
        source: io::Error,
    },
}

/// SHA-256 of `data`, lowercase hex.
pub fn sha256_hex(data: &[u8]) -> String {
    sha2::Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

impl DownloadableFont {
    /// Bytes to download.
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    /// The size to say, in whole kilobytes of 1,000 bytes, as the voice
    /// downloads say theirs (206 for Lexend).
    pub fn kilobytes(&self) -> u64 {
        (self.total_bytes() + 500) / 1000
    }

    /// True when `name` chooses this font: its key, name, or an alias,
    /// ignoring case, spaces, hyphens, and underscores.
    pub fn matches(&self, name: &str) -> bool {
        let n = squash(name);
        !n.is_empty()
            && (n == squash(self.key)
                || n == squash(self.name)
                || self.aliases.iter().any(|a| squash(a) == n))
    }

    /// The font's folder in `fonts_dir` (the data folder's `fonts`).
    pub fn dir_in(&self, fonts_dir: &Path) -> PathBuf {
        fonts_dir.join(self.key)
    }

    /// True when every file is in `fonts_dir` with its pinned size (the
    /// hashes were checked when they were installed; [`load_from`]
    /// checks them again).
    ///
    /// [`load_from`]: Self::load_from
    pub fn is_installed_in(&self, fonts_dir: &Path) -> bool {
        let dir = self.dir_in(fonts_dir);
        self.files.iter().all(|f| {
            std::fs::metadata(dir.join(f.file_name)).is_ok_and(|m| m.is_file() && m.len() == f.size)
        })
    }

    /// The installed files as a family, by style (Lexend has no italics:
    /// they fall back to the upright faces). `None` when not installed.
    pub fn faces_in(&self, fonts_dir: &Path) -> Option<FamilyFaces> {
        if !self.is_installed_in(fonts_dir) {
            return None;
        }
        let dir = self.dir_in(fonts_dir);
        let face = |s: Style| {
            self.files.iter().find(|f| f.style == s).map(|f| FaceRef {
                path: dir.join(f.file_name),
                index: 0,
            })
        };
        Some(FamilyFaces {
            name: self.name.to_owned(),
            regular: face(Style::Regular)?,
            bold: face(Style::Bold),
            italic: face(Style::Italic),
            bold_italic: face(Style::BoldItalic),
            monospace: false,
        })
    }

    /// Reads the installed files and checks each against its pinned size
    /// and hash. `None` when one is missing or does not match.
    pub fn load_from(&self, fonts_dir: &Path) -> Option<Vec<LoadedFace>> {
        let dir = self.dir_in(fonts_dir);
        self.files
            .iter()
            .map(|f| {
                let data = std::fs::read(dir.join(f.file_name)).ok()?;
                (data.len() as u64 == f.size && sha256_hex(&data) == f.sha256).then_some(
                    LoadedFace {
                        style: f.style,
                        file_name: f.file_name,
                        media_type: f.media_type(),
                        data,
                    },
                )
            })
            .collect()
    }

    /// This font as an optional component (its key as the id, the
    /// `fonts/<key>` folder, and its license file as the notice), for the
    /// components registry and the shared downloader.
    pub fn component(&self) -> Component {
        Component {
            id: Cow::Borrowed(self.key),
            title: Cow::Owned(format!("the {} reading font", self.name)),
            license: Cow::Borrowed(self.license_name),
            credit: Cow::Owned(format!("{}, {}", self.copyright, self.homepage)),
            features: Cow::Borrowed(&[Cow::Borrowed("reading-font")]),
            folder: Cow::Owned(format!("fonts/{}", self.key)),
            files: Cow::Owned(
                self.files
                    .iter()
                    .map(|f| FilePin::sha256(f.file_name, f.url, f.size, f.sha256))
                    .collect(),
            ),
            notice: Some((Cow::Borrowed("OFL.txt"), Cow::Borrowed(self.license_text))),
        }
    }

    /// Downloads every file through `fetcher`, checks its size and hash,
    /// and installs the font with its license in `fonts_dir`, returning
    /// the font's folder. Nothing is left in the font's folder when a file
    /// fails: the files wait in a staging folder until all of them
    /// matched.
    pub fn install(
        &self,
        fonts_dir: &Path,
        fetcher: &dyn Fetcher,
    ) -> Result<PathBuf, DownloadError> {
        self.install_with(
            fonts_dir,
            fetcher,
            &Sources::public(),
            &mut |_| {},
            &AtomicBool::new(false),
        )
    }

    /// [`install`](Self::install) from `sources` (a mirror first, when
    /// set), with progress and a way to cancel.
    pub fn install_with(
        &self,
        fonts_dir: &Path,
        fetcher: &dyn Fetcher,
        sources: &Sources,
        progress: &mut dyn FnMut(Progress),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, DownloadError> {
        let dest = self.dir_in(fonts_dir);
        textweaver_components::download(
            &self.component(),
            &dest,
            sources,
            fetcher,
            progress,
            cancel,
        )
        .map_err(|e| self.download_error(e))?;
        Ok(dest)
    }

    /// The pinned file named `name`, with its `'static` name.
    fn file_named(&self, name: &str) -> &'static str {
        self.files
            .iter()
            .find(|f| f.file_name == name)
            .map_or(self.key, |f| f.file_name)
    }

    fn download_error(&self, e: ComponentError) -> DownloadError {
        match e {
            ComponentError::Fetch { file, reason } => DownloadError::Fetch {
                file: self.file_named(&file),
                reason,
            },
            ComponentError::Size {
                file,
                got,
                expected,
            } => DownloadError::Size {
                file: self.file_named(&file),
                got,
                expected,
            },
            ComponentError::Hash { file } => DownloadError::Hash {
                file: self.file_named(&file),
            },
            ComponentError::Io { path, source } => DownloadError::Io { path, source },
            other => DownloadError::Fetch {
                file: self.key,
                reason: other.to_string(),
            },
        }
    }
}

/// Lexend, pinned to the Lexend project's commit `cd26b9c` (the last one
/// to change its static fonts; Star pinned the same).
pub static LEXEND: DownloadableFont = DownloadableFont {
    key: "lexend",
    name: "Lexend",
    aliases: &["lexend deca"],
    about: "Widely spaced letters, designed to reduce visual stress and improve reading fluency.",
    license: "OFL-1.1",
    license_name: "SIL Open Font License",
    license_text: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../third_party/fonts/lexend/OFL.txt"
    )),
    copyright: "Copyright 2018 The Lexend Project Authors, with Reserved Font Name \u{201c}RevReading Lexend\u{201d}",
    homepage: "https://www.lexend.com/",
    version: "cd26b9c2538d758138c20c3d2f10362ed613854b",
    files: &[
        PinnedFile {
            style: Style::Regular,
            file_name: "Lexend-Regular.ttf",
            url: "https://raw.githubusercontent.com/googlefonts/lexend/cd26b9c2538d758138c20c3d2f10362ed613854b/fonts/lexend/ttf/Lexend-Regular.ttf",
            size: 100_264,
            sha256: "e2082c28389e9871d5d77ae9163d5c0c1c259fe39bdac91f73d56a600b61f60f",
        },
        PinnedFile {
            style: Style::Bold,
            file_name: "Lexend-Bold.ttf",
            url: "https://raw.githubusercontent.com/googlefonts/lexend/cd26b9c2538d758138c20c3d2f10362ed613854b/fonts/lexend/ttf/Lexend-Bold.ttf",
            size: 105_564,
            sha256: "4509838acc21f2c066e9874c9c5eb52b9b8cdac1771a9c1fe9e7e2cda41ce082",
        },
    ],
};

/// Every font textweaver downloads on first choice.
pub static DOWNLOADABLE: [&DownloadableFont; 1] = [&LEXEND];

/// The downloadable font chosen by `name` (key, family name, or alias).
pub fn downloadable(name: &str) -> Option<&'static DownloadableFont> {
    DOWNLOADABLE.iter().copied().find(|f| f.matches(name))
}

static FOLDER: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Tells this process where downloaded fonts live (the data folder's
/// `fonts`), or that there is no such folder. The app sets it when it
/// starts; `tw` sets it before converting.
pub fn set_folder(dir: Option<PathBuf>) {
    if let Ok(mut f) = FOLDER.write() {
        *f = dir;
    }
}

/// Where downloaded fonts live, as [`set_folder`] last said.
pub fn folder() -> Option<PathBuf> {
    FOLDER.read().ok().and_then(|f| f.clone())
}

/// The downloaded family `name` in the [`folder`], as installed faces
/// (for the PDF writer). `None` when it is not a downloadable font or not
/// downloaded.
pub fn find(name: &str) -> Option<FamilyFaces> {
    downloadable(name)?.faces_in(&folder()?)
}

/// The downloaded font `name` in the [`folder`], read and checked (for
/// the EPUB writer, which embeds the files, and the GUI).
pub fn find_loaded(name: &str) -> Option<(&'static DownloadableFont, Vec<LoadedFace>)> {
    let font = downloadable(name)?;
    let faces = font.load_from(&folder()?)?;
    Some((font, faces))
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_components::fake::FakeFetcher;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/w7l")
                .join(name),
        )
        .unwrap()
    }

    /// Hands out the fixtures for the pinned URLs, and records each URL.
    struct Fake;

    impl Fake {
        fn lexend() -> FakeFetcher {
            let mut fake = FakeFetcher::new();
            for f in LEXEND.files {
                fake.insert(f.url, fixture(f.file_name));
            }
            fake
        }
    }

    #[test]
    fn lexend_is_pinned_and_measured() {
        assert_eq!(LEXEND.total_bytes(), 205_828);
        assert_eq!(LEXEND.kilobytes(), 206);
        assert_eq!(LEXEND.license, "OFL-1.1");
        assert!(
            LEXEND
                .license_text
                .contains("SIL OPEN FONT LICENSE Version 1.1")
        );
        assert!(LEXEND.license_text.contains("RevReading Lexend"));
        for f in LEXEND.files {
            assert!(f.url.contains(LEXEND.version), "{}", f.url);
            assert!(f.url.ends_with(f.file_name));
            assert_eq!(f.sha256.len(), 64);
            assert_eq!(sha256_hex(&fixture(f.file_name)), f.sha256);
            assert_eq!(fixture(f.file_name).len() as u64, f.size);
        }
        // The same files the reading font's record names.
        let urls: Vec<&str> = LEXEND.files.iter().map(|f| f.url).collect();
        assert_eq!(urls, crate::READING_FONTS[2].files);
        assert!(downloadable("Lexend").is_some());
        assert!(downloadable("lexend").is_some());
        assert!(downloadable("Lexend Deca").is_some());
        assert!(downloadable("OpenDyslexic").is_none());
    }

    #[test]
    fn the_fixtures_are_lexend() {
        for f in LEXEND.files {
            let info = crate::sfnt::info(&fixture(f.file_name), 0).unwrap();
            assert_eq!(info.family, "Lexend");
            assert_eq!(info.weight >= 600, f.style.is_bold());
            assert!(!info.italic && info.embeddable);
        }
    }

    #[test]
    fn install_checks_and_writes_the_font_and_its_licence() {
        let tmp = tempfile::tempdir().unwrap();
        let fonts = tmp.path().join("fonts");
        assert!(!LEXEND.is_installed_in(&fonts));
        assert!(LEXEND.faces_in(&fonts).is_none());
        let fake = Fake::lexend();
        let dir = LEXEND.install(&fonts, &fake).unwrap();
        assert_eq!(dir, fonts.join("lexend"));
        assert_eq!(fake.request_count(), 2);
        assert!(LEXEND.is_installed_in(&fonts));
        assert_eq!(
            std::fs::read_to_string(dir.join("OFL.txt")).unwrap(),
            LEXEND.license_text
        );
        let faces = LEXEND.faces_in(&fonts).unwrap();
        assert_eq!(faces.name, "Lexend");
        assert!(faces.regular.path.ends_with("Lexend-Regular.ttf"));
        assert!(
            faces
                .face(Style::BoldItalic)
                .path
                .ends_with("Lexend-Bold.ttf")
        );
        assert!(
            faces
                .face(Style::Italic)
                .path
                .ends_with("Lexend-Regular.ttf")
        );
        let loaded = LEXEND.load_from(&fonts).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].media_type, "font/ttf");
        // Nothing temporary is left.
        let names: Vec<String> = std::fs::read_dir(&fonts)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["lexend"]);
        // Installing again replaces the folder cleanly.
        LEXEND.install(&fonts, &fake).unwrap();
        assert!(LEXEND.is_installed_in(&fonts));
    }

    #[test]
    fn a_wrong_file_is_refused_and_nothing_is_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let fonts = tmp.path().join("fonts");
        // A changed byte: same size, wrong hash.
        let mut fake = Fake::lexend();
        let url = LEXEND.files[1].url.to_owned();
        fake.bytes_mut(&url).unwrap()[100] ^= 1;
        let e = LEXEND.install(&fonts, &fake).unwrap_err();
        assert_eq!(
            e.to_string(),
            "Lexend-Bold.ttf does not match its published hash"
        );
        assert!(!LEXEND.is_installed_in(&fonts));
        assert!(!fonts.join("lexend").exists());
        // Cut short.
        fake.bytes_mut(&url).unwrap().truncate(10);
        let e = LEXEND.install(&fonts, &fake).unwrap_err();
        assert_eq!(e.to_string(), "Lexend-Bold.ttf is 10 bytes, not 105564");
        // No network. The regular face, checked on the first try, waits
        // in the staging folder, so only the bold one is asked for.
        let e = LEXEND.install(&fonts, &FakeFetcher::new()).unwrap_err();
        assert_eq!(e.to_string(), "Lexend-Bold.ttf: not found");
        assert!(!fonts.join("lexend").exists());
    }

    #[test]
    fn a_damaged_install_is_not_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let fonts = tmp.path().join("fonts");
        LEXEND.install(&fonts, &Fake::lexend()).unwrap();
        let regular = fonts.join("lexend").join("Lexend-Regular.ttf");
        let mut data = std::fs::read(&regular).unwrap();
        data[200] ^= 1;
        std::fs::write(&regular, data).unwrap();
        assert!(LEXEND.is_installed_in(&fonts), "sizes still match");
        assert!(LEXEND.load_from(&fonts).is_none(), "the hash does not");
    }

    #[test]
    fn a_stale_partial_folder_is_cleared() {
        let tmp = tempfile::tempdir().unwrap();
        let fonts = tmp.path().join("fonts");
        let part = fonts.join("lexend.partial");
        std::fs::create_dir_all(&part).unwrap();
        std::fs::write(part.join("Lexend-Regular.ttf"), b"half").unwrap();
        LEXEND.install(&fonts, &Fake::lexend()).unwrap();
        assert!(!part.exists());
        assert!(LEXEND.load_from(&fonts).is_some());
    }

    #[test]
    fn lexend_is_a_component_with_its_licence() {
        let c = LEXEND.component();
        assert!(c.check_names().is_ok());
        assert_eq!(
            (c.id.as_ref(), c.folder.as_ref()),
            ("lexend", "fonts/lexend")
        );
        assert_eq!(c.size(), LEXEND.total_bytes());
        assert_eq!(c.notice.as_ref().unwrap().0, "OFL.txt");
    }

    #[test]
    fn the_folder_is_found_by_family_name() {
        // The only test that sets the process-wide folder.
        let tmp = tempfile::tempdir().unwrap();
        let fonts = tmp.path().join("fonts");
        set_folder(Some(fonts.clone()));
        assert!(find("Lexend").is_none());
        LEXEND.install(&fonts, &Fake::lexend()).unwrap();
        assert_eq!(find("lexend").unwrap().name, "Lexend");
        assert!(matches!(
            crate::resolve_family("Lexend", &[]),
            Some(crate::FamilySource::Installed(f)) if f.name == "Lexend"
        ));
        let (font, faces) = find_loaded("Lexend").unwrap();
        assert_eq!((font.key, faces.len()), ("lexend", 2));
        set_folder(None);
        assert!(find("Lexend").is_none());
        assert_eq!(folder(), None);
    }
}
