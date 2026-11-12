//! `cargo xtask affected [--base REF] [--args] [--feature PKG/FEAT]
//! [--generated]`: the workspace crates a change can break, for the fast
//! check (`scripts/dev-check.ps1 -Fast`, docs/dev/testing.md).
//!
//! The changed files are those that differ between the merge base of REF
//! (default `main`) and the working tree, staged or not, plus untracked
//! files that git does not ignore. Each file belongs to the workspace crate
//! whose folder holds it. The set is those crates plus every crate that
//! depends on them, directly or through another workspace crate, by any
//! kind of dependency (a dev-dependency counts: its tests can break).
//!
//! A change to the root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
//! `.cargo/`, or `third_party/` can change every crate's build, so it
//! selects the whole workspace, and says so on standard error.
//!
//! Output, one crate per line; with `--args`, one line of `-p NAME`
//! arguments, followed by `--features PKG/FEAT` when `--feature` is given
//! and a selected crate is PKG or depends on it directly (Cargo refuses the
//! flag otherwise). With `--generated`, the one word `yes` or `no`: whether
//! the change touches a source of a generated file or check (the docs,
//! locale catalogs, the settings schema, the keymap, any `Cargo.toml`).
//!
//! The graph comes from `cargo metadata --no-deps`: no network, no build.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};
use serde_json::Value;

/// The usage line.
const USAGE: &str =
    "usage: cargo xtask affected [--base REF] [--args] [--feature PKG/FEAT] [--generated]";

/// Files that can change every crate's build.
const WHOLE_WORKSPACE: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
    "third_party/",
];

/// One workspace crate: its folder, relative to the repository root with
/// forward slashes, and the workspace crates it depends on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Member {
    /// The crate's folder, such as `crates/textweaver-qr`.
    pub dir: String,
    /// Workspace crates it depends on, of any kind.
    pub deps: BTreeSet<String>,
}

/// The workspace: crate name to its folder and dependencies.
pub type Workspace = BTreeMap<String, Member>;

pub fn run() -> anyhow::Result<()> {
    let mut base = "main".to_owned();
    let mut args = false;
    let mut generated = false;
    let mut feature: Option<String> = None;
    let mut it = std::env::args().skip(2);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = it.next().context(USAGE)?,
            "--args" => args = true,
            "--generated" => generated = true,
            "--feature" => feature = Some(it.next().context(USAGE)?),
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => bail!("unknown option {other}. {USAGE}"),
        }
    }
    let root = crate::eci::root();
    let files = changed_files(&root, &base)?;
    if generated {
        println!(
            "{}",
            if touches_generated(&files) {
                "yes"
            } else {
                "no"
            }
        );
        return Ok(());
    }
    let ws = workspace(&root)?;
    if let Some(f) = files.iter().find(|f| is_whole_workspace(f)) {
        eprintln!("{f} changed, so every workspace crate is affected.");
    }
    let set = affected(&ws, &files);
    if args {
        let line = cargo_args(&ws, &set, feature.as_deref());
        if !line.is_empty() {
            println!("{line}");
        }
    } else {
        for name in &set {
            println!("{name}");
        }
    }
    Ok(())
}

/// Runs git in `root` and returns its standard output.
fn git(root: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Files changed since the merge base of `base`, in the working tree, and
/// untracked, relative to the root with forward slashes.
fn changed_files(root: &Path, base: &str) -> anyhow::Result<Vec<String>> {
    let mb = git(root, &["merge-base", base, "HEAD"])
        .with_context(|| format!("finding where this branch left {base}"))?;
    let mb = mb.trim();
    let mut files: BTreeSet<String> = BTreeSet::new();
    for line in git(root, &["diff", "--name-only", "--no-renames", mb])?.lines() {
        files.insert(line.trim().to_owned());
    }
    for line in git(root, &["ls-files", "--others", "--exclude-standard"])?.lines() {
        files.insert(line.trim().to_owned());
    }
    files.remove("");
    Ok(files.into_iter().collect())
}

/// The workspace from `cargo metadata --no-deps`.
fn workspace(root: &Path) -> anyhow::Result<Workspace> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("running cargo metadata")?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let meta: Value = serde_json::from_slice(&out.stdout).context("reading cargo metadata")?;
    workspace_from_metadata(&meta)
}

/// Forward slashes, no trailing slash.
fn slashes(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_owned()
}

/// Builds the workspace from `cargo metadata --no-deps` output.
pub fn workspace_from_metadata(meta: &Value) -> anyhow::Result<Workspace> {
    let root = slashes(
        meta["workspace_root"]
            .as_str()
            .context("cargo metadata has no workspace_root")?,
    );
    let packages = meta["packages"]
        .as_array()
        .context("cargo metadata has no packages")?;
    let names: BTreeSet<&str> = packages.iter().filter_map(|p| p["name"].as_str()).collect();
    let mut ws = Workspace::new();
    for p in packages {
        let name = p["name"].as_str().context("a package has no name")?;
        let manifest = slashes(
            p["manifest_path"]
                .as_str()
                .context("a package has no manifest_path")?,
        );
        let dir = manifest
            .strip_suffix("/Cargo.toml")
            .unwrap_or(&manifest)
            .to_owned();
        let dir = dir
            .strip_prefix(&root)
            .map(|d| d.trim_start_matches('/').to_owned())
            .unwrap_or(dir);
        let deps = p["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d["name"].as_str())
            .filter(|d| names.contains(d) && *d != name)
            .map(str::to_owned)
            .collect();
        ws.insert(name.to_owned(), Member { dir, deps });
    }
    Ok(ws)
}

/// Whether a changed file can change every crate's build.
fn is_whole_workspace(file: &str) -> bool {
    WHOLE_WORKSPACE.iter().any(|w| match w.strip_suffix('/') {
        Some(dir) => file.starts_with(&format!("{dir}/")),
        None => file == *w,
    })
}

/// The crate whose folder holds `file`: the longest matching folder, so a
/// crate nested in another's folder wins. `None` for files outside every
/// crate (docs, scripts, the workflows).
pub fn owner<'a>(ws: &'a Workspace, file: &str) -> Option<&'a str> {
    ws.iter()
        .filter(|(_, m)| {
            m.dir.is_empty() || file.starts_with(&format!("{}/", m.dir)) || file == m.dir
        })
        .max_by_key(|(_, m)| m.dir.len())
        .map(|(n, _)| n.as_str())
}

/// The crates `files` change, and every crate that depends on them.
pub fn affected(ws: &Workspace, files: &[String]) -> BTreeSet<String> {
    if files.iter().any(|f| is_whole_workspace(f)) {
        return ws.keys().cloned().collect();
    }
    let changed: BTreeSet<String> = files
        .iter()
        .filter_map(|f| owner(ws, f))
        .map(str::to_owned)
        .collect();
    with_dependents(ws, &changed)
}

/// `start` and every workspace crate that reaches one of them through its
/// dependencies.
pub fn with_dependents(ws: &Workspace, start: &BTreeSet<String>) -> BTreeSet<String> {
    // The reverse graph: crate to the crates that depend on it.
    let mut users: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, m) in ws {
        for d in &m.deps {
            users.entry(d.as_str()).or_default().push(name.as_str());
        }
    }
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut todo: Vec<&str> = start
        .iter()
        .filter(|s| ws.contains_key(*s))
        .map(String::as_str)
        .collect();
    while let Some(n) = todo.pop() {
        if !out.insert(n.to_owned()) {
            continue;
        }
        for u in users.get(n).into_iter().flatten() {
            if !out.contains(*u) {
                todo.push(u);
            }
        }
    }
    out
}

/// `-p NAME` for each crate, then `--features PKG/FEAT` when a selected
/// crate is PKG or depends on it directly.
pub fn cargo_args(ws: &Workspace, set: &BTreeSet<String>, feature: Option<&str>) -> String {
    let mut parts: Vec<String> = set.iter().map(|n| format!("-p {n}")).collect();
    if let Some(f) = feature
        && !parts.is_empty()
    {
        let pkg = f.split_once('/').map_or(f, |(p, _)| p);
        let applies = set
            .iter()
            .any(|n| n == pkg || ws.get(n).is_some_and(|m| m.deps.contains(pkg)));
        if applies {
            parts.push(format!("--features {f}"));
        }
    }
    parts.join(" ")
}

/// Whether a file is a source of a generated file or a generated check:
/// the docs, the locale catalogs, the settings schema, the keymap, or any
/// `Cargo.toml` (the notices, the dependency rules, the crate counts).
pub fn touches_generated(files: &[String]) -> bool {
    files.iter().any(|f| {
        f.starts_with("docs/")
            || f.contains("/locales/")
            || f.ends_with("settings_schema.rs")
            || f.starts_with("crates/textweaver-keymap/")
            || f == "Cargo.toml"
            || f.ends_with("/Cargo.toml")
            || f == "Cargo.lock"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(list: &[(&str, &str, &[&str])]) -> Workspace {
        list.iter()
            .map(|(n, dir, deps)| {
                (
                    (*n).to_owned(),
                    Member {
                        dir: (*dir).to_owned(),
                        deps: deps.iter().map(|d| (*d).to_owned()).collect(),
                    },
                )
            })
            .collect()
    }

    /// core <- text <- formats <- app <- tui; qr <- app; xtask -> text.
    fn sample() -> Workspace {
        ws(&[
            ("core", "crates/core", &[]),
            ("text", "crates/text", &["core"]),
            ("formats", "crates/formats", &["core", "text"]),
            ("qr", "crates/qr", &[]),
            ("app", "crates/app", &["formats", "qr"]),
            ("tui", "crates/tui", &["app"]),
            ("xtask", "xtask", &["text"]),
        ])
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|f| (*f).to_owned()).collect()
    }

    #[test]
    fn a_leaf_change_selects_its_users_only() {
        let w = sample();
        assert_eq!(
            affected(&w, &files(&["crates/qr/src/lib.rs"])),
            set(&["qr", "app", "tui"])
        );
    }

    #[test]
    fn a_core_change_walks_every_level() {
        let w = sample();
        assert_eq!(
            affected(&w, &files(&["crates/core/src/lib.rs"])),
            set(&["core", "text", "formats", "app", "tui", "xtask"])
        );
    }

    #[test]
    fn a_top_change_selects_itself() {
        let w = sample();
        assert_eq!(
            affected(&w, &files(&["crates/tui/src/main.rs"])),
            set(&["tui"])
        );
    }

    #[test]
    fn docs_and_scripts_select_no_crate() {
        let w = sample();
        assert!(affected(&w, &files(&["docs/gui.md", "scripts/dev-check.sh"])).is_empty());
    }

    #[test]
    fn root_manifest_and_lock_select_everything() {
        let w = sample();
        for f in ["Cargo.lock", "Cargo.toml", "third_party/x/src/lib.rs"] {
            assert_eq!(affected(&w, &files(&[f])).len(), w.len(), "{f}");
        }
    }

    #[test]
    fn a_cycle_through_dev_dependencies_ends() {
        let w = ws(&[("a", "a", &["b"]), ("b", "b", &["a"]), ("c", "c", &[])]);
        assert_eq!(with_dependents(&w, &set(&["a"])), set(&["a", "b"]));
    }

    #[test]
    fn the_longest_folder_owns_a_file() {
        let w = ws(&[("outer", "crates/x", &[]), ("inner", "crates/x/inner", &[])]);
        assert_eq!(owner(&w, "crates/x/inner/src/lib.rs"), Some("inner"));
        assert_eq!(owner(&w, "crates/x/src/lib.rs"), Some("outer"));
        assert_eq!(owner(&w, "crates/xy/src/lib.rs"), None);
    }

    #[test]
    fn args_add_the_feature_only_where_cargo_takes_it() {
        let w = ws(&[
            ("speech", "s", &[]),
            ("app", "a", &["speech"]),
            ("qr", "q", &[]),
        ]);
        assert_eq!(
            cargo_args(&w, &set(&["qr"]), Some("speech/omnivox")),
            "-p qr"
        );
        assert_eq!(
            cargo_args(&w, &set(&["app", "qr"]), Some("speech/omnivox")),
            "-p app -p qr --features speech/omnivox"
        );
        assert_eq!(
            cargo_args(&w, &set(&["speech"]), Some("speech/omnivox")),
            "-p speech --features speech/omnivox"
        );
        assert_eq!(cargo_args(&w, &set(&[]), Some("speech/omnivox")), "");
    }

    #[test]
    fn generated_sources_are_named() {
        assert!(touches_generated(&files(&["docs/gui.md"])));
        assert!(touches_generated(&files(&[
            "crates/textweaver-lexicon/locales/en/main.ftl"
        ])));
        assert!(touches_generated(&files(&[
            "crates/textweaver-qr/Cargo.toml"
        ])));
        assert!(touches_generated(&files(&[
            "crates/textweaver-keymap/src/action.rs"
        ])));
        assert!(!touches_generated(&files(&[
            "crates/textweaver-qr/src/lib.rs"
        ])));
    }

    #[test]
    fn metadata_folders_are_relative() {
        let meta = serde_json::json!({
            "workspace_root": "D:\\w",
            "packages": [
                {"name": "a", "manifest_path": "D:\\w\\crates\\a\\Cargo.toml",
                 "dependencies": [{"name": "b"}, {"name": "serde"}]},
                {"name": "b", "manifest_path": "D:\\w\\crates\\b\\Cargo.toml",
                 "dependencies": []}
            ]
        });
        let w = workspace_from_metadata(&meta).unwrap();
        assert_eq!(w["a"].dir, "crates/a");
        assert_eq!(w["a"].deps, set(&["b"]));
    }

    #[test]
    fn the_workspace_reads() {
        let w = workspace(&crate::eci::root()).unwrap();
        assert!(w.contains_key("textweaver-core"));
        let users = with_dependents(&w, &set(&["textweaver-qr"]));
        assert!(users.contains("textweaver-app"), "{users:?}");
        assert!(!users.contains("textweaver-core"));
    }
}
