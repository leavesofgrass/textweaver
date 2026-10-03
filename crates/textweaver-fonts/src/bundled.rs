//! The fonts that ship with textweaver (`third_party/fonts/`).
//!
//! With the `bundled-fonts` feature (the default), [`BUNDLED`] holds three
//! families, each with its four styles embedded; without it, [`BUNDLED`] is
//! empty and every lookup returns `None`.

use std::io;
use std::path::{Path, PathBuf};

use crate::Style;

/// One embedded font file.
#[derive(Debug, PartialEq, Eq)]
pub struct BundledFace {
    /// Which style it is.
    pub style: Style,
    /// The upstream file name (`AtkinsonHyperlegibleNext-Bold.ttf`).
    pub file_name: &'static str,
    /// The font file, unchanged from upstream.
    pub data: &'static [u8],
}

impl BundledFace {
    /// The media type: `font/ttf` or `font/otf`.
    pub fn media_type(&self) -> &'static str {
        if self.file_name.ends_with(".otf") {
            "font/otf"
        } else {
            "font/ttf"
        }
    }
}

/// A bundled family and everything its licence asks to travel with it.
#[derive(Debug, PartialEq, Eq)]
pub struct BundledFamily {
    /// Settings key (`atkinson-hyperlegible-next`).
    pub key: &'static str,
    /// The family name the files register under.
    pub name: &'static str,
    /// Other names that choose this family (`atkinson`, `mono`).
    pub aliases: &'static [&'static str],
    /// Every character the same width.
    pub monospace: bool,
    /// Who made it and why it helps, in one sentence to show and speak.
    pub about: &'static str,
    /// SPDX licence identifier.
    pub license: &'static str,
    /// The licence file, as published upstream.
    pub license_text: &'static str,
    /// The copyright line.
    pub copyright: &'static str,
    /// Home page.
    pub homepage: &'static str,
    /// Upstream version.
    pub version: &'static str,
    /// Regular, bold, italic, bold italic, in that order.
    pub faces: [BundledFace; 4],
}

impl BundledFamily {
    /// The face for `style`.
    pub fn face(&self, style: Style) -> &BundledFace {
        match style {
            Style::Regular => &self.faces[0],
            Style::Bold => &self.faces[1],
            Style::Italic => &self.faces[2],
            Style::BoldItalic => &self.faces[3],
        }
    }

    /// True when `name` chooses this family: its key, name, or an alias,
    /// ignoring case, spaces, hyphens, and underscores.
    pub fn matches(&self, name: &str) -> bool {
        let n = squash(name);
        !n.is_empty()
            && (n == squash(self.key)
                || n == squash(self.name)
                || self.aliases.iter().any(|a| squash(a) == n))
    }

    /// Size of the four font files in bytes.
    pub fn total_bytes(&self) -> usize {
        self.faces.iter().map(|f| f.data.len()).sum()
    }

    /// Writes the four font files and the licence (as `OFL.txt`) into
    /// `dir`, creating it, and returns the font file paths in face order.
    /// Files already there with the right size are left alone, so this is
    /// cheap to call at every start (the GUI registers the files from here).
    pub fn extract_to(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dir)?;
        let mut paths = Vec::with_capacity(4);
        for face in &self.faces {
            let path = dir.join(face.file_name);
            write_if_changed(&path, face.data)?;
            paths.push(path);
        }
        write_if_changed(&dir.join("OFL.txt"), self.license_text.as_bytes())?;
        Ok(paths)
    }
}

fn write_if_changed(path: &Path, data: &[u8]) -> io::Result<()> {
    let same = std::fs::metadata(path).is_ok_and(|m| m.len() == data.len() as u64);
    if !same {
        // Write aside and rename, so a crash never leaves half a font.
        textweaver_core::fs::write_atomic(path, data)?;
    }
    Ok(())
}

fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Key of the default text family (Atkinson Hyperlegible Next).
pub const TEXT_FAMILY: &str = "atkinson-hyperlegible-next";
/// Key of the default monospaced family (Atkinson Hyperlegible Mono).
pub const MONO_FAMILY: &str = "atkinson-hyperlegible-mono";
/// Key of OpenDyslexic.
pub const DYSLEXIC_FAMILY: &str = "opendyslexic";

/// The bundled family chosen by `name` (key, family name, or alias).
pub fn family(name: &str) -> Option<&'static BundledFamily> {
    BUNDLED.iter().find(|f| f.matches(name))
}

/// The default text family, when fonts are bundled.
pub fn default_text() -> Option<&'static BundledFamily> {
    family(TEXT_FAMILY)
}

/// The default monospaced family, when fonts are bundled.
pub fn default_mono() -> Option<&'static BundledFamily> {
    family(MONO_FAMILY)
}

/// True when `name` is a bundled family.
pub fn is_bundled(name: &str) -> bool {
    family(name).is_some()
}

#[cfg(feature = "bundled-fonts")]
macro_rules! font_file {
    ($dir:literal, $file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../third_party/fonts/",
            $dir,
            "/",
            $file
        ))
    };
}

#[cfg(feature = "bundled-fonts")]
macro_rules! face {
    ($style:ident, $dir:literal, $file:literal) => {
        BundledFace {
            style: Style::$style,
            file_name: $file,
            data: font_file!($dir, $file),
        }
    };
}

#[cfg(feature = "bundled-fonts")]
macro_rules! licence {
    ($dir:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../third_party/fonts/",
            $dir,
            "/OFL.txt"
        ))
    };
}

/// The bundled families: Atkinson Hyperlegible Next, Atkinson Hyperlegible
/// Mono, and OpenDyslexic (empty without the `bundled-fonts` feature).
#[cfg(feature = "bundled-fonts")]
pub static BUNDLED: &[BundledFamily] = &[
    BundledFamily {
        key: TEXT_FAMILY,
        name: "Atkinson Hyperlegible Next",
        aliases: &["atkinson", "atkinson hyperlegible", "hyperlegible"],
        monospace: false,
        about: "From the Braille Institute, for readers with low vision: letters that look alike are made different.",
        license: "OFL-1.1",
        license_text: licence!("atkinson-hyperlegible-next"),
        copyright: "Copyright 2020-2024 The Atkinson Hyperlegible Next Project Authors",
        homepage: "https://www.brailleinstitute.org/freefont/",
        version: "2.001",
        faces: [
            face!(
                Regular,
                "atkinson-hyperlegible-next",
                "AtkinsonHyperlegibleNext-Regular.ttf"
            ),
            face!(
                Bold,
                "atkinson-hyperlegible-next",
                "AtkinsonHyperlegibleNext-Bold.ttf"
            ),
            face!(
                Italic,
                "atkinson-hyperlegible-next",
                "AtkinsonHyperlegibleNext-Italic.ttf"
            ),
            face!(
                BoldItalic,
                "atkinson-hyperlegible-next",
                "AtkinsonHyperlegibleNext-BoldItalic.ttf"
            ),
        ],
    },
    BundledFamily {
        key: MONO_FAMILY,
        name: "Atkinson Hyperlegible Mono",
        aliases: &["mono", "atkinson mono", "hyperlegible mono"],
        monospace: true,
        about: "The monospaced Atkinson Hyperlegible, for code: every letter the same width, and letters that look alike are made different.",
        license: "OFL-1.1",
        license_text: licence!("atkinson-hyperlegible-mono"),
        copyright: "Copyright 2020-2024 The Atkinson Hyperlegible Mono Project Authors",
        homepage: "https://www.brailleinstitute.org/freefont/",
        version: "2.001",
        faces: [
            face!(
                Regular,
                "atkinson-hyperlegible-mono",
                "AtkinsonHyperlegibleMono-Regular.ttf"
            ),
            face!(
                Bold,
                "atkinson-hyperlegible-mono",
                "AtkinsonHyperlegibleMono-Bold.ttf"
            ),
            face!(
                Italic,
                "atkinson-hyperlegible-mono",
                "AtkinsonHyperlegibleMono-Italic.ttf"
            ),
            face!(
                BoldItalic,
                "atkinson-hyperlegible-mono",
                "AtkinsonHyperlegibleMono-BoldItalic.ttf"
            ),
        ],
    },
    BundledFamily {
        key: DYSLEXIC_FAMILY,
        name: "OpenDyslexic",
        aliases: &["open dyslexic", "dyslexic"],
        monospace: false,
        about: "For readers with dyslexia: heavier letter bottoms and distinct letter shapes.",
        license: "OFL-1.1",
        license_text: licence!("opendyslexic"),
        copyright: "Copyright (c) 2019-07-29, Abbie Gonzalez, with Reserved Font Name OpenDyslexic",
        homepage: "https://opendyslexic.org/",
        version: "0.91.12",
        faces: [
            face!(Regular, "opendyslexic", "OpenDyslexic-Regular.otf"),
            face!(Bold, "opendyslexic", "OpenDyslexic-Bold.otf"),
            face!(Italic, "opendyslexic", "OpenDyslexic-Italic.otf"),
            face!(BoldItalic, "opendyslexic", "OpenDyslexic-Bold-Italic.otf"),
        ],
    },
];

/// The bundled families (empty: built without the `bundled-fonts` feature).
#[cfg(not(feature = "bundled-fonts"))]
pub static BUNDLED: &[BundledFamily] = &[];

#[cfg(all(test, feature = "bundled-fonts"))]
mod tests {
    use super::*;
    use crate::sfnt;

    #[test]
    fn every_face_parses_with_the_right_names() {
        assert_eq!(BUNDLED.len(), 3);
        let mut total = 0;
        for fam in BUNDLED {
            assert_eq!(fam.license, "OFL-1.1");
            assert!(
                fam.license_text
                    .contains("SIL OPEN FONT LICENSE Version 1.1")
            );
            for (face, style) in fam.faces.iter().zip(Style::ALL) {
                assert_eq!(face.style, style);
                let info = sfnt::info(face.data, 0).expect(face.file_name);
                assert_eq!(info.family, fam.name, "{}", face.file_name);
                assert_eq!(info.italic, style.is_italic(), "{}", face.file_name);
                assert_eq!(info.weight >= 600, style.is_bold(), "{}", face.file_name);
                assert_eq!(info.monospace, fam.monospace, "{}", face.file_name);
                assert!(info.embeddable, "{}", face.file_name);
                total += face.data.len();
            }
        }
        // The size third_party/fonts/README.md reports.
        assert_eq!(total, 1_354_240);
    }

    #[test]
    fn lookup_by_key_name_and_alias() {
        assert_eq!(
            family("Atkinson Hyperlegible Next").unwrap().key,
            TEXT_FAMILY
        );
        assert_eq!(
            family("atkinson-hyperlegible-next").unwrap().key,
            TEXT_FAMILY
        );
        assert_eq!(family("ATKINSON").unwrap().key, TEXT_FAMILY);
        assert_eq!(family("mono").unwrap().key, MONO_FAMILY);
        assert_eq!(family("Open Dyslexic").unwrap().key, DYSLEXIC_FAMILY);
        assert!(family("").is_none());
        assert!(family("Arial").is_none());
        assert!(default_text().is_some() && default_mono().unwrap().monospace);
        assert_eq!(
            family("opendyslexic")
                .unwrap()
                .face(Style::Bold)
                .media_type(),
            "font/otf"
        );
    }

    #[test]
    fn extract_writes_fonts_and_licence_once() {
        let dir = tempfile::tempdir().unwrap();
        let fam = default_text().unwrap();
        let paths = fam.extract_to(dir.path()).unwrap();
        assert_eq!(paths.len(), 4);
        for (p, f) in paths.iter().zip(&fam.faces) {
            assert_eq!(std::fs::read(p).unwrap(), f.data);
        }
        let ofl = std::fs::read_to_string(dir.path().join("OFL.txt")).unwrap();
        assert_eq!(ofl, fam.license_text);
        // A second call leaves the files alone.
        let before = std::fs::metadata(&paths[0]).unwrap().modified().unwrap();
        fam.extract_to(dir.path()).unwrap();
        assert_eq!(
            std::fs::metadata(&paths[0]).unwrap().modified().unwrap(),
            before
        );
    }
}
