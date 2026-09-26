//! Installed fonts: where each platform keeps them, what families they
//! hold, and the files for a family's four styles.
//!
//! [`scan`] reads only the table directory and a few small tables of each
//! file (see [`crate::sfnt`]), so scanning a Windows font folder of several
//! hundred files takes a fraction of a second. Callers that scan more than
//! once should keep the result.

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::Style;
use crate::sfnt::{self, FaceInfo};

/// One face of an installed font file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFace {
    /// The font file.
    pub path: PathBuf,
    /// The face's index in a collection (`.ttc`), else 0.
    pub index: u32,
    /// What the face says about itself.
    pub info: FaceInfo,
}

/// A face to load: a file and a face index in it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FaceRef {
    /// The font file.
    pub path: PathBuf,
    /// Face index (0 unless the file is a collection).
    pub index: u32,
}

/// The files of one installed family, by style. Missing styles are `None`;
/// callers use the regular face in their place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyFaces {
    /// The family name as the fonts give it.
    pub name: String,
    /// Upright, normal weight (always present).
    pub regular: FaceRef,
    /// Upright bold.
    pub bold: Option<FaceRef>,
    /// Italic.
    pub italic: Option<FaceRef>,
    /// Bold italic.
    pub bold_italic: Option<FaceRef>,
    /// The family is monospaced.
    pub monospace: bool,
}

impl FamilyFaces {
    /// The face for `style`, falling back to the nearest style present.
    pub fn face(&self, style: Style) -> &FaceRef {
        match style {
            Style::Regular => &self.regular,
            Style::Bold => self.bold.as_ref().unwrap_or(&self.regular),
            Style::Italic => self.italic.as_ref().unwrap_or(&self.regular),
            Style::BoldItalic => self
                .bold_italic
                .as_ref()
                .or(self.bold.as_ref())
                .or(self.italic.as_ref())
                .unwrap_or(&self.regular),
        }
    }
}

/// The folders fonts are installed in on this platform, including the
/// user's own.
pub fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        dirs.push(PathBuf::from(windir).join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push("/System/Library/Fonts".into());
        dirs.push("/Library/Fonts".into());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else {
        dirs.push("/usr/share/fonts".into());
        dirs.push("/usr/local/share/fonts".into());
        if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(data).join("fonts"));
        } else if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join(".fonts"));
        }
    }
    dirs
}

fn is_font_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "ttf" | "otf" | "ttc" | "otc"
        )
    })
}

fn collect(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_dir() && depth > 0 {
            collect(&path, depth - 1, out);
        } else if is_font_file(&path) {
            out.push(path);
        }
    }
}

/// Reads every face of the font files under `dirs` (and their
/// subfolders, four levels deep). Unreadable files are skipped.
pub fn scan_dirs(dirs: &[PathBuf]) -> Vec<SystemFace> {
    let mut files = Vec::new();
    for d in dirs {
        collect(d, 4, &mut files);
    }
    files.sort();
    files.dedup();
    let mut faces = Vec::new();
    for path in files {
        let Ok(mut f) = File::open(&path) else {
            continue;
        };
        let count = sfnt::face_count(&mut f);
        for index in 0..count {
            if let Some(info) = sfnt::info_from(&mut f, index) {
                faces.push(SystemFace {
                    path: path.clone(),
                    index,
                    info,
                });
            }
        }
    }
    faces
}

/// Reads every installed face ([`font_dirs`]).
pub fn scan() -> Vec<SystemFace> {
    scan_dirs(&font_dirs())
}

fn usable(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('@') && !name.starts_with('.')
}

/// Family names to offer in a font chooser, sorted without regard to case,
/// each once. Faces whose older family name differs from their typographic
/// family (such as "Segoe UI Semibold" in "Segoe UI") are listed under the
/// typographic family only, since choosing a weight selects them.
pub fn family_names(faces: &[SystemFace]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for f in faces {
        let name = &f.info.family;
        if usable(name) && !names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            names.push(name.clone());
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// The installed faces of family `name` (typographic or older family
/// name, ignoring case), by style: the upright face nearest weight 400 is
/// regular, the upright face nearest 700 (at least 600) is bold, and the
/// same for italics. `None` when no face has that name.
pub fn find_family(faces: &[SystemFace], name: &str) -> Option<FamilyFaces> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let members: Vec<&SystemFace> = faces
        .iter()
        .filter(|f| {
            f.info.family.eq_ignore_ascii_case(name)
                || f.info.legacy_family.eq_ignore_ascii_case(name)
        })
        .collect();
    let pick = |italic: bool, target: u16, min: u16| -> Option<FaceRef> {
        members
            .iter()
            .filter(|f| f.info.italic == italic && f.info.weight >= min)
            .min_by_key(|f| (f.info.weight.abs_diff(target), f.path.clone(), f.index))
            .map(|f| FaceRef {
                path: f.path.clone(),
                index: f.index,
            })
    };
    let regular = pick(false, 400, 0).or_else(|| pick(true, 400, 0))?;
    let first = members.first()?;
    let bold = pick(false, 700, 600).filter(|b| *b != regular);
    let italic = pick(true, 400, 0).filter(|i| *i != regular);
    let bold_italic = pick(true, 700, 600).filter(|b| Some(b) != italic.as_ref());
    Some(FamilyFaces {
        name: first.info.family.clone(),
        regular,
        bold,
        italic,
        bold_italic,
        monospace: members.iter().all(|f| f.info.monospace),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(family: &str, legacy: &str, weight: u16, italic: bool, file: &str) -> SystemFace {
        SystemFace {
            path: PathBuf::from(file),
            index: 0,
            info: FaceInfo {
                family: family.into(),
                legacy_family: legacy.into(),
                subfamily: String::new(),
                weight,
                italic,
                monospace: false,
                embeddable: true,
            },
        }
    }

    fn faces() -> Vec<SystemFace> {
        vec![
            face("Segoe UI", "Segoe UI", 400, false, "segoeui.ttf"),
            face("Segoe UI", "Segoe UI", 700, false, "segoeuib.ttf"),
            face("Segoe UI", "Segoe UI Semibold", 600, false, "seguisb.ttf"),
            face("Segoe UI", "Segoe UI Light", 300, false, "segoeuil.ttf"),
            face("Segoe UI", "Segoe UI", 400, true, "segoeuii.ttf"),
            face("Arial", "Arial", 400, false, "arial.ttf"),
            face("@MS Gothic", "@MS Gothic", 400, false, "msgothic.ttc"),
        ]
    }

    #[test]
    fn families_are_listed_once_by_typographic_name() {
        assert_eq!(family_names(&faces()), ["Arial", "Segoe UI"]);
    }

    #[test]
    fn styles_are_matched_by_weight_and_slope() {
        let f = find_family(&faces(), "segoe ui").unwrap();
        assert_eq!(f.name, "Segoe UI");
        assert_eq!(f.regular.path, Path::new("segoeui.ttf"));
        assert_eq!(f.bold.as_ref().unwrap().path, Path::new("segoeuib.ttf"));
        assert_eq!(f.italic.as_ref().unwrap().path, Path::new("segoeuii.ttf"));
        assert_eq!(f.bold_italic, None);
        // Bold italic falls back to bold.
        assert_eq!(f.face(Style::BoldItalic).path, Path::new("segoeuib.ttf"));
        // The older family name finds the family too.
        assert!(find_family(&faces(), "Segoe UI Semibold").is_some());
        // Arial has only a regular face.
        let a = find_family(&faces(), "Arial").unwrap();
        assert_eq!(a.face(Style::Bold).path, Path::new("arial.ttf"));
        assert!(find_family(&faces(), "Nope").is_none());
        assert!(find_family(&faces(), " ").is_none());
    }

    #[test]
    fn scans_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.txt"), "not a font").unwrap();
        std::fs::write(dir.path().join("broken.ttf"), "not a font either").unwrap();
        #[cfg(feature = "bundled-fonts")]
        crate::bundled::default_text()
            .unwrap()
            .extract_to(&dir.path().join("sub"))
            .unwrap();
        let faces = scan_dirs(&[dir.path().to_path_buf()]);
        #[cfg(feature = "bundled-fonts")]
        {
            assert_eq!(faces.len(), 4);
            let f = find_family(&faces, "Atkinson Hyperlegible Next").unwrap();
            assert!(
                f.regular
                    .path
                    .ends_with("AtkinsonHyperlegibleNext-Regular.ttf")
            );
            assert!(
                f.bold_italic
                    .unwrap()
                    .path
                    .ends_with("AtkinsonHyperlegibleNext-BoldItalic.ttf")
            );
        }
        #[cfg(not(feature = "bundled-fonts"))]
        assert!(faces.is_empty());
    }

    #[test]
    fn installed_fonts_scan_without_error() {
        // Whatever is installed: every face found has a family name.
        let started = std::time::Instant::now();
        let faces = scan();
        eprintln!(
            "{} installed faces, {} families, scanned in {:?}",
            faces.len(),
            family_names(&faces).len(),
            started.elapsed()
        );
        assert!(faces.iter().all(|f| !f.info.family.is_empty()));
    }
}
