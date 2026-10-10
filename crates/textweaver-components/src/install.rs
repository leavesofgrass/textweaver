//! Install from a file: a downloaded zip or a folder (a clone of a
//! mirror, a memory stick), checked against the same pins as a download.

use std::io::{Read, Write};
use std::path::{Component as PathPart, Path, PathBuf};

use crate::component::Component;
use crate::download::{Claim, Outcome, finish};
use crate::error::ComponentError;
use crate::pin::{FilePin, is_plain_name};

/// What [`install_from`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallReport {
    /// The files installed and kept.
    pub outcome: Outcome,
    /// Files in the zip or folder that were not used, each with the
    /// reason in words ("not one of this component's files").
    pub refused: Vec<(String, String)>,
}

/// Where a candidate file is.
enum Place {
    File(PathBuf),
    Zip(usize),
}

struct Candidate {
    name: String,
    size: u64,
    place: Place,
    used: bool,
}

/// The deepest a folder is searched.
const MAX_DEPTH: usize = 4;

/// The most entries looked at, in a folder or a zip.
const MAX_ENTRIES: usize = 10_000;

/// Installs `component` into `dest` from `from`: a zip or a folder holding
/// its files (at the top or in subfolders). Each pinned file is found by
/// its name, or by its size and hash when it was renamed; it is checked
/// before anything is installed, and nothing is installed unless every
/// file matched. Files the component does not pin are refused and listed
/// with the reason. Names from a zip must be plain: a name with a parent
/// step, an absolute path, or an odd character is refused.
pub fn install_from(
    component: &Component,
    from: &Path,
    dest: &Path,
) -> Result<InstallReport, ComponentError> {
    component.check_names()?;
    let mut report = InstallReport::default();
    let mut zip = None;
    let mut candidates = if from.is_dir() {
        folder_candidates(from, &mut report.refused)
    } else {
        let f = std::fs::File::open(from).map_err(|e| ComponentError::io(from, e))?;
        let mut archive = zip::ZipArchive::new(f)
            .map_err(|e| ComponentError::Manifest(format!("{}: {e}", from.display())))?;
        let c = zip_candidates(&mut archive, &mut report.refused);
        zip = Some(archive);
        c
    };
    let mut claim = Claim::take(&component.id, dest)?;
    for pin in component.files.iter() {
        if pin.matches_file(&dest.join(pin.name.as_ref()))
            || crate::unpack::receipt_matches(dest, pin)
        {
            report.outcome.kept.push(pin.name.to_string());
            if let Some(c) = candidates.iter_mut().find(|c| c.name == pin.name) {
                c.used = true;
            }
            continue;
        }
        let staging = claim.stage(&component.id)?.to_owned();
        let part = staging.join(format!("{}.part", pin.name));
        let found = pick(pin, &mut candidates, zip.as_mut(), &part, from)?;
        if !found {
            let _ = std::fs::remove_file(&part);
            return Err(ComponentError::Missing {
                file: pin.name.to_string(),
                from: from.to_owned(),
            });
        }
        let staged = staging.join(pin.name.as_ref());
        std::fs::rename(&part, &staged).map_err(|e| ComponentError::io(&staged, e))?;
        report.outcome.fetched.push(pin.name.to_string());
    }
    for c in candidates.iter().filter(|c| !c.used) {
        report.refused.push((
            c.name.clone(),
            "not one of this component's files".to_owned(),
        ));
    }
    report.outcome.left_out = finish(component, dest, &claim.staging, &report.outcome.fetched)?;
    Ok(report)
}

/// Copies the candidate for `pin` into `part` and checks it: by name
/// first (a wrong size or hash is an error), then any file of the same
/// size and hash. False when there is none.
fn pick(
    pin: &FilePin,
    candidates: &mut [Candidate],
    mut zip: Option<&mut zip::ZipArchive<std::fs::File>>,
    part: &Path,
    from: &Path,
) -> Result<bool, ComponentError> {
    if let Some(i) = candidates.iter().position(|c| c.name == pin.name) {
        candidates[i].used = true;
        if candidates[i].size != pin.size {
            return Err(ComponentError::Size {
                file: pin.name.to_string(),
                got: candidates[i].size,
                expected: pin.size,
            });
        }
        copy_out(&candidates[i].place, zip.as_deref_mut(), part, from)?;
        if !pin.matches_file(part) {
            let _ = std::fs::remove_file(part);
            return Err(ComponentError::Hash {
                file: pin.name.to_string(),
            });
        }
        return Ok(true);
    }
    for c in candidates.iter_mut() {
        if c.used || c.size != pin.size {
            continue;
        }
        copy_out(&c.place, zip.as_deref_mut(), part, from)?;
        if pin.matches_file(part) {
            c.used = true;
            return Ok(true);
        }
    }
    Ok(false)
}

fn copy_out(
    place: &Place,
    zip: Option<&mut zip::ZipArchive<std::fs::File>>,
    part: &Path,
    from: &Path,
) -> Result<(), ComponentError> {
    let mut out = std::fs::File::create(part).map_err(|e| ComponentError::io(part, e))?;
    match (place, zip) {
        (Place::File(p), _) => {
            let mut f = std::fs::File::open(p).map_err(|e| ComponentError::io(p, e))?;
            std::io::copy(&mut f, &mut out).map_err(|e| ComponentError::io(part, e))?;
        }
        (Place::Zip(i), Some(archive)) => {
            let mut entry = archive
                .by_index(*i)
                .map_err(|e| ComponentError::Manifest(format!("{}: {e}", from.display())))?;
            let mut buf = vec![0u8; 1 << 16];
            loop {
                let n = entry
                    .read(&mut buf)
                    .map_err(|e| ComponentError::io(from, e))?;
                if n == 0 {
                    break;
                }
                out.write_all(&buf[..n])
                    .map_err(|e| ComponentError::io(part, e))?;
            }
        }
        (Place::Zip(_), None) => {
            return Err(ComponentError::Missing {
                file: part.display().to_string(),
                from: from.to_owned(),
            });
        }
    }
    out.flush().map_err(|e| ComponentError::io(part, e))
}

/// The files in a folder and its subfolders (not following links).
fn folder_candidates(root: &Path, refused: &mut Vec<(String, String)>) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_owned(), 0usize)];
    let mut seen = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return out;
            }
            let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
                continue;
            };
            let name = e.file_name().to_string_lossy().into_owned();
            if meta.is_dir() {
                if depth < MAX_DEPTH {
                    stack.push((e.path(), depth + 1));
                }
            } else if meta.is_file() {
                if is_plain_name(&name) {
                    out.push(Candidate {
                        name,
                        size: meta.len(),
                        place: Place::File(e.path()),
                        used: false,
                    });
                } else {
                    refused.push((name, "not a plain file name".to_owned()));
                }
            }
        }
    }
    out
}

/// The files in a zip. A member whose name has a parent step or an
/// absolute path, or whose file name is not plain, is refused.
fn zip_candidates(
    archive: &mut zip::ZipArchive<std::fs::File>,
    refused: &mut Vec<(String, String)>,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    for i in 0..archive.len().min(MAX_ENTRIES) {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let raw = entry.name().to_owned();
        let path = Path::new(&raw);
        let unsafe_path = raw.contains('\\')
            || path.components().any(|p| {
                matches!(
                    p,
                    PathPart::ParentDir | PathPart::RootDir | PathPart::Prefix(_)
                )
            })
            || raw.starts_with('/');
        if unsafe_path {
            refused.push((raw, "has a parent step or an absolute path".to_owned()));
            continue;
        }
        let name = raw.rsplit('/').next().unwrap_or("").to_owned();
        let parts_plain = raw.split('/').filter(|p| !p.is_empty()).all(is_plain_name);
        if !parts_plain {
            refused.push((raw, "not a plain file name".to_owned()));
            continue;
        }
        out.push(Candidate {
            name,
            size: entry.size(),
            place: Place::Zip(i),
            used: false,
        });
    }
    out
}
