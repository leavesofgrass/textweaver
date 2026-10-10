//! Installing a checked update (B1-u1): unpacking the package, finding the
//! install folder, and putting the new files in place so that either every
//! file is new or every file is as it was.
//!
//! - [`unpack`] reads the zip or tarball into a folder, refusing absolute
//!   paths, `..`, links, and names that are not plain, and drops the
//!   package's top folder (`textweaver-VERSION-PLATFORM/`).
//! - [`install_dir`] finds the folder the running program came from and
//!   checks it is a release package (its `NOTICE` file, or the app bundle
//!   on macOS); a build from source or a system package is refused.
//! - [`swap_in`] copies each of the package's top-level entries beside its
//!   old one (`NAME.update-new`), then renames the old one aside
//!   (`NAME.update-old`) and the new one into place; if any step fails,
//!   what was done is undone. Only the package's own entries are touched:
//!   settings, notes, the library, state, and components live in the data
//!   and configuration folders, never in the install folder.
//! - Windows cannot replace a running program, so [`start_finisher`]
//!   starts the new package's `tw update --finish`, which waits for
//!   textweaver to close ([`wait_until_closed`]), swaps, and starts it
//!   again; under Program Files it asks for administrator rights once.

use std::path::{Component as PathPart, Path, PathBuf};
use std::time::{Duration, Instant};

use crate::pin::is_plain_name;
use crate::update::{PackageKind, Target, UpdateError};

/// The suffix of a new entry copied beside the old one.
const NEW: &str = "update-new";

/// The suffix of an old entry renamed aside.
const OLD: &str = "update-old";

/// The marker of a release package's folder.
const MARKER: &str = "NOTICE";

/// The relative path of an archive member without the package's top
/// folder, when every part is plain; `Ok(None)` for the top folder itself.
fn member_path(raw: &Path) -> Result<Option<PathBuf>, UpdateError> {
    let mut parts = Vec::new();
    for p in raw.components() {
        match p {
            PathPart::Normal(s) => {
                let s = s.to_str().unwrap_or("");
                if !is_plain_name(s) {
                    return Err(UpdateError::Unpack(format!(
                        "{} has a name that is not plain",
                        raw.display()
                    )));
                }
                parts.push(s.to_owned());
            }
            PathPart::CurDir => {}
            _ => {
                return Err(UpdateError::Unpack(format!(
                    "{} points outside the package",
                    raw.display()
                )));
            }
        }
    }
    Ok((parts.len() > 1).then(|| parts[1..].iter().collect()))
}

/// Unpacks `package` (a `.zip` or `.tar.gz`) into `into`, without the
/// package's top folder. Returns `into`.
pub fn unpack(package: &Path, into: &Path) -> Result<PathBuf, UpdateError> {
    std::fs::create_dir_all(into).map_err(|e| UpdateError::install(into, e))?;
    let name = package
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let file = std::fs::File::open(package).map_err(|e| UpdateError::install(package, e))?;
    if name.ends_with(".zip") {
        unpack_zip(file, into)?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        unpack_tar(file, into)?;
    } else {
        return Err(UpdateError::Unpack(format!(
            "{name} is not a zip or tarball"
        )));
    }
    Ok(into.to_owned())
}

fn unpack_zip(file: std::fs::File, into: &Path) -> Result<(), UpdateError> {
    let bad = |e: zip::result::ZipError| UpdateError::Unpack(e.to_string());
    let mut zip = zip::ZipArchive::new(file).map_err(bad)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(bad)?;
        if entry.is_symlink() {
            return Err(UpdateError::Unpack(format!("{} is a link", entry.name())));
        }
        let raw = PathBuf::from(entry.name());
        let Some(rel) = member_path(&raw)? else {
            continue;
        };
        let dest = into.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|e| UpdateError::install(&dest, e))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| UpdateError::install(parent, e))?;
        }
        let mut out = std::fs::File::create(&dest).map_err(|e| UpdateError::install(&dest, e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| UpdateError::install(&dest, e))?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode & 0o777))
                .map_err(|e| UpdateError::install(&dest, e))?;
        }
    }
    Ok(())
}

fn unpack_tar(file: std::fs::File, into: &Path) -> Result<(), UpdateError> {
    let bad = |e: std::io::Error| UpdateError::Unpack(e.to_string());
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in tar.entries().map_err(bad)? {
        let mut entry = entry.map_err(bad)?;
        let kind = entry.header().entry_type();
        let raw = entry.path().map_err(bad)?.into_owned();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(UpdateError::Unpack(format!(
                "{} is not a file or a folder",
                raw.display()
            )));
        }
        let Some(rel) = member_path(&raw)? else {
            continue;
        };
        let dest = into.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| UpdateError::install(parent, e))?;
        }
        entry
            .unpack(&dest)
            .map_err(|e| UpdateError::install(&dest, e))?;
    }
    Ok(())
}

/// The install folder of the program at `exe`: the folder holding it, or
/// on macOS the folder holding its app bundle. It must be a release
/// package: its `NOTICE` file is there, or on macOS `textweaver.app` is.
/// A build from source or a system package is refused.
pub fn install_dir(exe: &Path) -> Result<PathBuf, UpdateError> {
    // Symbolic links (a tarball install linked from ~/.local/bin) lead to
    // the package; Windows keeps the path as it is.
    let exe = if cfg!(windows) {
        exe.to_owned()
    } else {
        std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_owned())
    };
    let bundle = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"));
    let dir = match bundle {
        Some(b) => b.parent(),
        None => exe.parent(),
    }
    .ok_or_else(|| UpdateError::NotPackage(exe.clone()))?
    .to_owned();
    if dir.join(MARKER).is_file() || bundle.is_some() {
        Ok(dir)
    } else {
        Err(UpdateError::NotPackage(dir))
    }
}

/// `name` with `suffix` added (`tw.exe.update-new`).
fn beside(dir: &Path, name: &str, suffix: &str) -> PathBuf {
    dir.join(format!("{name}.{suffix}"))
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)? {
            let e = e?;
            copy_tree(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

fn remove_any(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Removes what an earlier swap left behind in `install`: `*.update-new`
/// and `*.update-old` entries, when they could not be removed then (a
/// program still had one open). Best effort.
fn clear_leftovers(install: &Path) {
    let Ok(entries) = std::fs::read_dir(install) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(&format!(".{NEW}")) || name.ends_with(&format!(".{OLD}")) {
            let _ = remove_any(&e.path());
        }
    }
}

/// Undoes the renames of a failed swap: each entry put in place goes, and
/// its old one comes back; the new copies go.
fn undo(install: &Path, done: &[(&String, bool)], names: &[String]) {
    for (m, had) in done.iter().rev() {
        let cur = install.join(m);
        let _ = remove_any(&cur);
        if *had {
            let _ = std::fs::rename(beside(install, m, OLD), &cur);
        }
    }
    for m in names {
        let _ = remove_any(&beside(install, m, NEW));
    }
}

/// Puts the unpacked package `new_root` in place in `install`: every
/// top-level entry of the package, or with `only_existing` (macOS, where
/// the bundle and `tw` may sit in a folder of other programs) only those
/// already there. Either every entry is replaced or none is. Returns how
/// many entries were put in place.
pub fn swap_in(new_root: &Path, install: &Path, only_existing: bool) -> Result<usize, UpdateError> {
    clear_leftovers(install);
    let mut names: Vec<String> = std::fs::read_dir(new_root)
        .map_err(|e| UpdateError::install(new_root, e))?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| is_plain_name(n))
        .filter(|n| !only_existing || install.join(n).exists())
        .collect();
    names.sort();
    // 1. Copy each new entry beside the old one; nothing is replaced yet.
    for n in &names {
        let to = beside(install, n, NEW);
        if let Err(e) = remove_any(&to).and_then(|()| copy_tree(&new_root.join(n), &to)) {
            undo(install, &[], &names);
            return Err(UpdateError::install(&to, e));
        }
    }
    // 2. Rename each old entry aside and the new one in; undo on failure.
    let mut done: Vec<(&String, bool)> = Vec::new();
    for n in &names {
        let (cur, new, old) = (
            install.join(n),
            beside(install, n, NEW),
            beside(install, n, OLD),
        );
        let had = cur.exists();
        let step = if had {
            std::fs::rename(&cur, &old)
        } else {
            Ok(())
        }
        .and_then(|()| {
            std::fs::rename(&new, &cur).inspect_err(|_| {
                if had {
                    let _ = std::fs::rename(&old, &cur);
                }
            })
        });
        if let Err(e) = step {
            undo(install, &done, &names);
            return Err(UpdateError::install(&cur, e));
        }
        done.push((n, had));
    }
    // 3. The old entries go; one still open stays until the next update.
    clear_leftovers(install);
    Ok(names.len())
}

/// Replaces one file (an AppImage) with `new`: copied beside it, made
/// runnable, then renamed over it, so the file is either old or new.
pub fn replace_file(new: &Path, file: &Path) -> Result<(), UpdateError> {
    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let dir = file.parent().unwrap_or(Path::new("."));
    let tmp = beside(dir, name, NEW);
    std::fs::copy(new, &tmp).map_err(|e| UpdateError::install(&tmp, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| UpdateError::install(&tmp, e))?;
    }
    std::fs::rename(&tmp, file).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        UpdateError::install(file, e)
    })
}

/// Where an update goes on this computer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// A package's folder ([`install_dir`]).
    Folder(PathBuf),
    /// One AppImage file, replaced whole.
    Image(PathBuf),
}

impl Place {
    /// Where the running textweaver came from: the AppImage file the
    /// runtime names in `APPIMAGE`, or the package folder of this program.
    /// A build from source or a system package is refused, so nothing is
    /// downloaded that cannot be installed.
    pub fn find(target: &Target) -> Result<Place, UpdateError> {
        if target.kind == PackageKind::AppImage {
            let image = std::env::var_os("APPIMAGE").map(PathBuf::from);
            return match image {
                Some(p) if p.is_file() => Ok(Place::Image(p)),
                other => Err(UpdateError::NotPackage(other.unwrap_or_default())),
            };
        }
        let exe = std::env::current_exe().map_err(|e| UpdateError::install(Path::new("."), e))?;
        install_dir(&exe).map(Place::Folder)
    }
}

/// What installing did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Installed {
    /// The new files are in place; the next start uses them.
    Done,
    /// Windows: unpacked into `new_root`, to be swapped into `install` by
    /// [`start_finisher`] once textweaver closes.
    OnClose {
        /// The unpacked package.
        new_root: PathBuf,
        /// The install folder.
        install: PathBuf,
    },
}

/// Installs the checked `package` at `place`, unpacking into `work` (a
/// folder of its own, cleared first). Linux and macOS replace the files
/// now (a running program there keeps its old copy until it closes);
/// Windows unpacks and waits for textweaver to close ([`Installed::OnClose`]).
pub fn install(
    package: &Path,
    target: &Target,
    place: &Place,
    work: &Path,
) -> Result<Installed, UpdateError> {
    let dir = match place {
        Place::Image(file) => return replace_file(package, file).map(|()| Installed::Done),
        Place::Folder(dir) => dir,
    };
    let new_root = work.join("new");
    remove_any(&new_root).map_err(|e| UpdateError::install(&new_root, e))?;
    unpack(package, &new_root)?;
    match target.kind {
        PackageKind::WindowsZip => Ok(Installed::OnClose {
            new_root,
            install: dir.clone(),
        }),
        kind => {
            swap_in(&new_root, dir, kind == PackageKind::MacZip)?;
            Ok(Installed::Done)
        }
    }
}

/// Waits until the program at `exe` has closed, at most `limit`: on
/// Windows a running program cannot be opened for writing. True when it
/// closed (at once elsewhere, where a running program can be replaced).
pub fn wait_until_closed(exe: &Path, limit: Duration) -> bool {
    if !cfg!(windows) {
        return true;
    }
    let start = Instant::now();
    loop {
        if std::fs::OpenOptions::new().append(true).open(exe).is_ok() {
            return true;
        }
        if start.elapsed() >= limit {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// True when this program may write in `dir` (a probe file is made and
/// removed).
pub fn can_write(dir: &Path) -> bool {
    let probe = dir.join("textweaver-update-probe.tmp");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// The arguments of `tw update --finish`.
pub fn finish_args(
    new_root: &Path,
    install: &Path,
    wait_for: &Path,
    start: Option<&Path>,
) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "update".into(),
        "--finish".into(),
        new_root.display().to_string(),
        "--into".into(),
        install.display().to_string(),
        "--wait-for".into(),
        wait_for.display().to_string(),
    ];
    if let Some(s) = start {
        a.push("--start".into());
        a.push(s.display().to_string());
    }
    a
}

/// A PowerShell string in single quotes (a quote inside is doubled).
fn ps_quote(s: &str) -> String {
    const Q: char = '\u{27}';
    let doubled = s.replace(Q, "\u{27}\u{27}");
    format!("{Q}{doubled}{Q}")
}

/// Starts `tw` from the unpacked package (`new_root`) to finish the update
/// after this program closes: it waits for `wait_for` to close, swaps the
/// files into `install`, and starts `start` again when given. When this
/// program may not write in `install` (Program Files), it asks Windows for
/// administrator rights once, through PowerShell's `Start-Process -Verb
/// RunAs`.
pub fn start_finisher(
    new_root: &Path,
    install: &Path,
    wait_for: &Path,
    start: Option<&Path>,
) -> Result<(), UpdateError> {
    let tw = new_root.join(format!("tw{}", std::env::consts::EXE_SUFFIX));
    if !tw.is_file() {
        return Err(UpdateError::Unpack("the package has no tw".into()));
    }
    let args = finish_args(new_root, install, wait_for, start);
    let spawned = if can_write(install) {
        textweaver_core::process::command(&tw).args(&args).spawn()
    } else {
        let list: Vec<String> = args.iter().map(|a| ps_quote(&format!("\"{a}\""))).collect();
        let script = format!(
            "Start-Process -FilePath {} -ArgumentList {} -Verb RunAs -WindowStyle Hidden",
            ps_quote(&tw.display().to_string()),
            list.join(",")
        );
        textweaver_core::process::command("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .spawn()
    };
    spawned
        .map(|_| ())
        .map_err(|e| UpdateError::install(&tw, e))
}

#[cfg(test)]
mod tests;
