//! The one downloader: sources in order, a `.part` file, the size and
//! hash, then a rename; files that already check out are kept; a stopped
//! download goes on from its `.part` file.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use crate::component::Component;
use crate::error::ComponentError;
use crate::fetch::Fetcher;
use crate::pin::FilePin;

/// Where files may come from, in order: a mirror (when set), then each
/// file's public address.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    /// The mirror's base address: an `https:` address or a folder on this
    /// computer. A component's files are under `<mirror>/<id>/<file name>`
    /// (on GitHub, a release tagged with the component's id), and the
    /// mirror's own list of components at `<mirror>/manifest/components.toml`.
    pub mirror: Option<String>,
}

impl Sources {
    /// Public addresses only.
    pub fn public() -> Self {
        Sources::default()
    }

    /// `mirror` first, then public addresses. An empty mirror is none.
    pub fn with_mirror(mirror: &str) -> Self {
        let m = mirror.trim();
        Sources {
            mirror: (!m.is_empty()).then(|| m.to_owned()),
        }
    }

    /// The mirror from `TEXTWEAVER_COMPONENTS_MIRROR` when set, else
    /// `setting` (the `[components] mirror` setting; empty for none).
    pub fn from_env_or(setting: &str) -> Self {
        match std::env::var(crate::MIRROR_ENV) {
            Ok(v) if !v.trim().is_empty() => Sources::with_mirror(&v),
            _ => Sources::with_mirror(setting),
        }
    }

    /// The addresses to try for `file` of `component`, in order.
    pub fn addresses(&self, component: &Component, file: &FilePin) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(m) = &self.mirror {
            out.push(join(m, &[&component.id, &file.name]));
        }
        if !file.url.is_empty() {
            out.push(file.url.to_string());
        }
        out
    }

    /// Where the mirror's list of extra components is, when a mirror is
    /// set.
    pub fn manifest_address(&self) -> Option<String> {
        self.mirror
            .as_deref()
            .map(|m| join(m, &["manifest", "components.toml"]))
    }
}

fn join(base: &str, parts: &[&str]) -> String {
    let mut s = base.trim_end_matches(['/', '\\']).to_owned();
    for p in parts {
        s.push('/');
        s.push_str(p);
    }
    s
}

/// How far a download has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Bytes so far, across the component (files kept count as done).
    pub done: u64,
    /// Bytes in the whole component.
    pub total: u64,
}

impl Progress {
    /// Whole percent done, 0 to 100.
    pub fn percent(&self) -> u64 {
        if self.total == 0 {
            return 100;
        }
        (self.done.min(self.total) * 100) / self.total
    }
}

/// Says progress every 10 percent, never more often (no spinner):
/// [`Tenths::step`] gives 10, 20, ... 100, each once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tenths {
    said: u64,
}

impl Tenths {
    /// The percent to say now, when `p` reached a new tenth.
    pub fn step(&mut self, p: Progress) -> Option<u64> {
        let tenth = p.percent() / 10 * 10;
        (tenth > self.said).then(|| {
            self.said = tenth;
            tenth
        })
    }
}

/// What a download or install did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Files fetched (or copied) and checked.
    pub fetched: Vec<String>,
    /// Files already in place that checked out, kept as they were (a
    /// folder placed by hand is adopted, not downloaded again).
    pub kept: Vec<String>,
}

/// Components being downloaded or installed in this program, by folder.
static BUSY: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

/// How long another program's lock is honored without it being renewed.
const LOCK_FRESH: Duration = Duration::from_secs(60);

/// How often a long download renews its lock.
const LOCK_RENEW: Duration = Duration::from_secs(10);

/// The lock file's name, in the staging folder.
const LOCK_NAME: &str = "download.lock";

/// The claim on one component's folder: in this program (a set of
/// folders), and across programs (a lock file in the staging folder,
/// renewed while the download runs and honored for a minute after).
pub(crate) struct Claim {
    dest: PathBuf,
    pub(crate) staging: PathBuf,
    lock: Option<PathBuf>,
    renewed: Instant,
}

impl Claim {
    /// Claims `dest` for `id`.
    pub(crate) fn take(id: &str, dest: &Path) -> Result<Claim, ComponentError> {
        let staging =
            staging_dir(dest).ok_or_else(|| ComponentError::BadName(dest.display().to_string()))?;
        let mut busy = BUSY.lock().unwrap_or_else(|p| p.into_inner());
        let set = busy.get_or_insert_with(HashSet::new);
        if !set.insert(dest.to_owned()) {
            return Err(ComponentError::Busy(id.to_owned()));
        }
        Ok(Claim {
            dest: dest.to_owned(),
            staging,
            lock: None,
            renewed: Instant::now(),
        })
    }

    /// Makes the staging folder and takes its lock file, once.
    pub(crate) fn stage(&mut self, id: &str) -> Result<&Path, ComponentError> {
        if self.lock.is_none() {
            std::fs::create_dir_all(&self.staging)
                .map_err(|e| ComponentError::io(&self.staging, e))?;
            let lock = self.staging.join(LOCK_NAME);
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock)
            {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let fresh = std::fs::metadata(&lock)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok())
                        .is_some_and(|age| age < LOCK_FRESH);
                    if fresh {
                        return Err(ComponentError::Busy(id.to_owned()));
                    }
                    // Left by a program that stopped: taken over.
                    std::fs::write(&lock, b"").map_err(|e| ComponentError::io(&lock, e))?;
                }
                Err(e) => return Err(ComponentError::io(&lock, e)),
            }
            self.lock = Some(lock);
            self.renewed = Instant::now();
        }
        Ok(&self.staging)
    }

    /// Renews the lock file now and then during a long download.
    fn renew(&mut self) {
        if let Some(lock) = &self.lock
            && self.renewed.elapsed() >= LOCK_RENEW
        {
            let _ = std::fs::write(lock, b"");
            self.renewed = Instant::now();
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            let _ = std::fs::remove_file(&lock);
            // Only an empty staging folder is removed; `.part` files of a
            // stopped download stay for the next one.
            let _ = std::fs::remove_dir(&self.staging);
        }
        let mut busy = BUSY.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(set) = busy.as_mut() {
            set.remove(&self.dest);
        }
    }
}

/// The staging folder beside `dest`: `<name>.partial`.
pub(crate) fn staging_dir(dest: &Path) -> Option<PathBuf> {
    let leaf = dest.file_name()?.to_str()?;
    Some(dest.with_file_name(format!("{leaf}.partial")))
}

/// Downloads every file of `component` that is not already in `dest` and
/// checked, through `fetcher`, trying `sources` in order for each file.
/// `progress` hears bytes done; setting `cancel` stops the download, and
/// a later download goes on from the `.part` file. Only call this after
/// the reader said yes (the component's size and license were said).
///
/// Each file is written to `<file>.part` in a staging folder beside
/// `dest`, checked by size and hash, and renamed; a file that fails its
/// check is deleted and the next source is tried. When every file is
/// checked, the new files are moved into `dest` together, with the
/// component's notice, so `dest` never holds a file that did not match.
pub fn download(
    component: &Component,
    dest: &Path,
    sources: &Sources,
    fetcher: &dyn Fetcher,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<Outcome, ComponentError> {
    component.check_names()?;
    let total = component.size();
    let mut claim = Claim::take(&component.id, dest)?;
    let mut outcome = Outcome::default();
    let mut done = 0u64;
    for f in component.files.iter() {
        if f.matches_file(&dest.join(f.name.as_ref())) {
            done += f.size;
            outcome.kept.push(f.name.to_string());
            progress(Progress { done, total });
            continue;
        }
        let staging = claim.stage(&component.id)?.to_owned();
        let staged = staging.join(f.name.as_ref());
        if staged.exists() {
            if f.matches_file(&staged) {
                done += f.size;
                outcome.fetched.push(f.name.to_string());
                progress(Progress { done, total });
                continue;
            }
            std::fs::remove_file(&staged).map_err(|e| ComponentError::io(&staged, e))?;
        }
        let part = staging.join(format!("{}.part", f.name));
        let addresses = sources.addresses(component, f);
        if addresses.is_empty() {
            return Err(ComponentError::NoSource {
                file: f.name.to_string(),
            });
        }
        let mut last = None;
        for address in &addresses {
            if cancel.load(Ordering::Relaxed) {
                return Err(ComponentError::Cancelled);
            }
            let fetched = fetch_one(
                fetcher,
                address,
                f,
                &part,
                &mut |n| {
                    progress(Progress {
                        done: done + n,
                        total,
                    })
                },
                cancel,
                &mut claim,
            );
            match fetched {
                Ok(()) if f.matches_file(&part) => {
                    std::fs::rename(&part, &staged).map_err(|e| ComponentError::io(&staged, e))?;
                    last = None;
                    break;
                }
                Ok(()) => {
                    log::warn!("{} from {address} does not match its pin", f.name);
                    remove_if_there(&part)?;
                    last = Some(ComponentError::Hash {
                        file: f.name.to_string(),
                    });
                }
                Err(ComponentError::Cancelled) => return Err(ComponentError::Cancelled),
                Err(e @ ComponentError::Size { .. }) => {
                    remove_if_there(&part)?;
                    last = Some(e);
                }
                Err(e) => {
                    log::warn!("{} from {address}: {e}", f.name);
                    last = Some(e);
                }
            }
        }
        if let Some(e) = last {
            return Err(e);
        }
        done += f.size;
        outcome.fetched.push(f.name.to_string());
        progress(Progress { done, total });
    }
    finish(component, dest, &claim.staging, &outcome.fetched)?;
    Ok(outcome)
}

fn remove_if_there(path: &Path) -> Result<(), ComponentError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ComponentError::io(path, e)),
    }
}

/// Fetches one file from one address into `part`, going on from what
/// `part` already holds when the source allows it.
fn fetch_one(
    fetcher: &dyn Fetcher,
    address: &str,
    file: &FilePin,
    part: &Path,
    progress: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
    claim: &mut Claim,
) -> Result<(), ComponentError> {
    let fetch_err = |reason: String| ComponentError::Fetch {
        file: file.name.to_string(),
        reason,
    };
    let have = match std::fs::metadata(part) {
        Ok(m) if m.len() < file.size => m.len(),
        Ok(_) => {
            remove_if_there(part)?;
            0
        }
        Err(_) => 0,
    };
    let fetched = fetcher.open(address, have).map_err(fetch_err)?;
    let mut got = fetched.start;
    let out = if got > 0 {
        std::fs::OpenOptions::new().append(true).open(part)
    } else {
        std::fs::File::create(part)
    }
    .map_err(|e| ComponentError::io(part, e))?;
    let mut out = std::io::BufWriter::new(out);
    progress(got);
    let mut reader = fetched.reader;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        if cancel.load(Ordering::Relaxed) {
            out.flush().map_err(|e| ComponentError::io(part, e))?;
            return Err(ComponentError::Cancelled);
        }
        let n = match reader.read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                // What arrived is kept, so the next try goes on from it.
                out.flush().map_err(|e| ComponentError::io(part, e))?;
                return Err(fetch_err(e.to_string()));
            }
        };
        if n == 0 {
            break;
        }
        if got + n as u64 > file.size {
            drop(out);
            return Err(ComponentError::Size {
                file: file.name.to_string(),
                got: got + n as u64,
                expected: file.size,
            });
        }
        out.write_all(&buf[..n])
            .map_err(|e| ComponentError::io(part, e))?;
        got += n as u64;
        progress(got);
        claim.renew();
    }
    out.flush().map_err(|e| ComponentError::io(part, e))?;
    drop(out);
    if got != file.size {
        return Err(ComponentError::Size {
            file: file.name.to_string(),
            got,
            expected: file.size,
        });
    }
    Ok(())
}

/// Moves the checked files `names` from `staging` into `dest`, writes the
/// component's notice, and leaves `dest` complete.
pub(crate) fn finish(
    component: &Component,
    dest: &Path,
    staging: &Path,
    names: &[String],
) -> Result<(), ComponentError> {
    std::fs::create_dir_all(dest).map_err(|e| ComponentError::io(dest, e))?;
    for name in names {
        let to = dest.join(name);
        remove_if_there(&to)?;
        std::fs::rename(staging.join(name), &to).map_err(|e| ComponentError::io(&to, e))?;
    }
    if let Some((name, text)) = &component.notice {
        let path = dest.join(name.as_ref());
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_ref()) {
            std::fs::write(&path, text.as_bytes()).map_err(|e| ComponentError::io(&path, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_first_then_public() {
        let c = crate::component::tests_support::sample();
        let f = &c.files[0];
        assert_eq!(Sources::public().addresses(&c, f), vec![f.url.to_string()]);
        let s = Sources::with_mirror("D:/mirror/");
        assert_eq!(
            s.addresses(&c, f),
            vec!["D:/mirror/sample/a.bin".to_owned(), f.url.to_string()]
        );
        assert_eq!(
            s.manifest_address().unwrap(),
            "D:/mirror/manifest/components.toml"
        );
        assert_eq!(Sources::with_mirror("  "), Sources::public());
    }

    #[test]
    fn tenths_are_said_once_each() {
        let mut t = Tenths::default();
        let p = |done| Progress { done, total: 1000 };
        assert_eq!(t.step(p(50)), None);
        assert_eq!(t.step(p(100)), Some(10));
        assert_eq!(t.step(p(150)), None);
        assert_eq!(t.step(p(390)), Some(30));
        assert_eq!(t.step(p(1000)), Some(100));
        assert_eq!(t.step(p(1000)), None);
    }
}
