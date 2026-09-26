//! Planning: turning files and folders into (source, output) jobs.
//!
//! - A file converts to `<out>/<stem>.<ext>`, or beside itself without
//!   `--out`.
//! - A folder is walked recursively (hidden files and folders skipped, and
//!   the output folder when it lies inside the input) and mirrored:
//!   `in/a/b.md` becomes `<out>/a/b.<ext>`. Without `--out` each output
//!   lands beside its source, and files that already have the output
//!   extension are treated as earlier outputs, not sources.
//! - Only files with an extension some loader reads are taken from folders;
//!   files named explicitly are always tried.
//! - A job whose output would overwrite its source, or an output another
//!   source already claims, is rejected with a reason instead of run.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::{ConvertError, Converter, FileResult, Status, extension};

/// One file to convert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// The source file.
    pub source: PathBuf,
    /// Where the output goes.
    pub output: PathBuf,
    /// The input folder the source was found under (its parent for a file
    /// named directly); notes for inline embeds are looked up here.
    pub root: PathBuf,
}

/// Planned jobs, and inputs rejected before running.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    /// Jobs to run, in a stable order.
    pub jobs: Vec<Job>,
    /// Files that cannot be converted as planned, with the reason.
    pub rejected: Vec<FileResult>,
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy();
    n.starts_with('.') && n != "." && n != ".."
}

pub(crate) fn plan(conv: &Converter, inputs: &[PathBuf]) -> Result<Plan, ConvertError> {
    let opts = conv.options();
    let out_ext = opts.to.extension();
    let exts = conv.source_extensions();
    let out_canonical = opts.out_dir.as_ref().and_then(|d| d.canonicalize().ok());
    let mut candidates: Vec<Job> = Vec::new();
    for input in inputs {
        let meta = std::fs::metadata(input).map_err(|_| ConvertError::Missing(input.clone()))?;
        if meta.is_file() {
            let root = input
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map_or_else(|| PathBuf::from("."), Path::to_owned);
            let name = with_ext(input.file_name().map(Path::new).unwrap_or(input), out_ext);
            let output = match &opts.out_dir {
                Some(d) => d.join(name),
                None => input.with_extension(out_ext),
            };
            candidates.push(Job {
                source: input.clone(),
                output,
                root,
            });
            continue;
        }
        let out_root = opts.out_dir.clone().unwrap_or_else(|| input.clone());
        let walker = WalkDir::new(input)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                if is_hidden(e.file_name()) {
                    return false;
                }
                // Never descend into the output folder.
                !(e.file_type().is_dir()
                    && out_canonical
                        .as_ref()
                        .is_some_and(|o| e.path().canonicalize().is_ok_and(|p| &p == o)))
            });
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("skipping unreadable entry: {e}");
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let ext = extension(path);
            if !exts.contains(&ext.as_str()) {
                continue;
            }
            if opts.out_dir.is_none() && ext == out_ext {
                continue;
            }
            let rel = path.strip_prefix(input).unwrap_or(path);
            candidates.push(Job {
                source: path.to_owned(),
                output: out_root.join(with_ext(rel, out_ext)),
                root: input.clone(),
            });
        }
    }
    let mut plan = Plan::default();
    let mut claimed: HashMap<PathBuf, PathBuf> = HashMap::new();
    for job in candidates {
        let key = normalize(&job.output);
        if key == normalize(&job.source) {
            plan.rejected.push(FileResult::failed(
                &job,
                "the output would overwrite the source; choose an output folder with --out",
            ));
            continue;
        }
        if let Some(other) = claimed.get(&key) {
            if normalize(other) == normalize(&job.source) {
                continue; // the same file named twice
            }
            let reason = format!(
                "{} converts to the same output; rename one of them",
                other.display()
            );
            plan.rejected.push(FileResult {
                status: Status::Failed(reason),
                ..FileResult::failed(&job, "")
            });
            continue;
        }
        claimed.insert(key, job.source.clone());
        plan.jobs.push(job);
    }
    Ok(plan)
}

/// `path` with its extension replaced (or added) by `ext`.
fn with_ext(path: &Path, ext: &str) -> PathBuf {
    path.with_extension(ext)
}

/// A path for comparison: canonical when it exists, else made absolute
/// lexically, with case folded on Windows.
fn normalize(p: &Path) -> PathBuf {
    let abs = p.canonicalize().unwrap_or_else(|_| {
        if p.is_absolute() {
            p.to_owned()
        } else {
            std::env::current_dir().map_or_else(|_| p.to_owned(), |d| d.join(p))
        }
    });
    if cfg!(windows) {
        PathBuf::from(abs.to_string_lossy().to_lowercase())
    } else {
        abs
    }
}
