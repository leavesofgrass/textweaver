//! `tw components`: optional components (W8a-d), the models, fonts, and
//! voices textweaver can download, from the command line.
//!
//! - `tw components list` names each one: installed or not, its size, its
//!   license, and the features that need it.
//! - `tw components download ID` says what it is (size, license, source)
//!   and asks; `--yes` answers for you. Progress is said every 10 percent
//!   on standard error. A mirror (`[components] mirror` or
//!   `TEXTWEAVER_COMPONENTS_MIRROR`) is tried first.
//! - `tw components verify [ID]` checks each installed file by size and
//!   SHA-256.
//! - `tw components remove ID` removes the component's own files, printing
//!   each path, after asking (or `--yes`).
//! - `tw components install ID PATH` installs from a downloaded zip or a
//!   folder, checked against the same pins; anything else is refused with
//!   the reason.
//!
//! `tw` never shows the first-run list; that is the GUI's and the terminal
//! reader's.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use anyhow::{Context, bail};
use textweaver_app::components::{
    Component, FileState, Progress, Registry, StandardFetcher, Status, Tenths, component_dir,
    features_text, sources,
};
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::SettingsStore;

/// Arguments for `tw components`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Use the files under this folder instead of the usual place.
    #[arg(long, global = true, value_name = "DIR")]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: Sub,
}

#[derive(clap::Subcommand, Debug)]
enum Sub {
    /// List the optional components: installed or not, size, license, and what each is for.
    List {
        /// Print the list as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Download a component after saying its size and license.
    Download {
        /// The component's id (see tw components list).
        id: String,
        /// Download without asking.
        #[arg(long, short)]
        yes: bool,
    },
    /// Check installed components' files by size and SHA-256.
    Verify {
        /// One component; all installed ones when not given.
        id: Option<String>,
    },
    /// Remove a component's files, printing each path removed.
    Remove {
        /// The component's id.
        id: String,
        /// Remove without asking.
        #[arg(long, short)]
        yes: bool,
    },
    /// Install a component from a downloaded zip or a folder, checked against its pins.
    Install {
        /// The component's id.
        id: String,
        /// The zip file or folder.
        path: PathBuf,
    },
}

/// Runs `tw components`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = super::paths(args.home.as_deref())?;
    let settings = SettingsStore::new(paths.clone()).load().0;
    let registry = registry_with_mirror(&settings);
    let data = paths.data_dir.clone();
    match args.command {
        Sub::List { json: false } => super::print_all(&list(&registry, &data)),
        Sub::List { json: true } => {
            super::print_all(&format!("{:#}\n", list_json(&registry, &data)))
        }
        Sub::Download { id, yes } => {
            let c = named(&registry, &id)?;
            download(c, &component_dir(c, &data), &settings, yes)
        }
        Sub::Verify { id } => verify(&registry, &data, id.as_deref()),
        Sub::Remove { id, yes } => {
            let c = named(&registry, &id)?;
            remove(c, &component_dir(c, &data), yes)
        }
        Sub::Install { id, path } => {
            let c = named(&registry, &id)?;
            install(c, &component_dir(c, &data), &path)
        }
    }
}

/// The registry, with the mirror's components when a mirror is set and
/// its list can be read.
fn registry_with_mirror(settings: &textweaver_app::store::Settings) -> Registry {
    let mut registry = Registry::builtin();
    if let Some(address) = sources(settings).manifest_address() {
        let text = textweaver_app::components::fetch_text(&StandardFetcher, &address);
        match text.map(|t| registry.add_manifest(&t)) {
            Ok(Ok(refused)) => {
                for (id, why) in refused {
                    eprintln!("The mirror's component {id} was refused: {why}.");
                }
            }
            Ok(Err(e)) => eprintln!("The mirror's list of components was not used: {e}."),
            Err(e) => eprintln!("The mirror has no list of components ({e})."),
        }
    }
    registry
}

fn named<'a>(registry: &'a Registry, id: &str) -> anyhow::Result<&'a Component> {
    registry.get(id.trim()).with_context(|| {
        let ids: Vec<&str> = registry
            .components()
            .iter()
            .map(|c| c.id.as_ref())
            .collect();
        format!(
            "{id} is not a component; the components are {}",
            ids.join(", ")
        )
    })
}

fn state(c: &Component, dir: &Path) -> String {
    match c.status_in(dir) {
        Status::Installed => "installed".to_owned(),
        Status::NotInstalled => "not installed".to_owned(),
        Status::Partial(missing) => format!("partly installed, missing {}", missing.join(", ")),
        Status::Damaged(f) => format!("damaged ({f})"),
    }
}

/// What `tw components list` prints: one line per component, meaning
/// first.
fn list(registry: &Registry, data: &Path) -> String {
    let mut out = String::new();
    for c in registry.components() {
        let dir = component_dir(c, data);
        out.push_str(&format!(
            "{}: {}, {}, license {}, for {}. Id {}.\n",
            c.title,
            state(c, &dir),
            c.size_text(),
            c.license,
            features_text(&Catalog::english(), c),
            c.id
        ));
    }
    let (n, all) = registry.installed_count(data);
    out.push_str(&format!("{n} of {all} installed.\n"));
    out
}

/// `tw components list --json`: one object per component, and the count
/// installed. Keys are English and never translated.
fn list_json(registry: &Registry, data: &Path) -> serde_json::Value {
    let items: Vec<serde_json::Value> = registry
        .components()
        .iter()
        .map(|c| {
            let dir = component_dir(c, data);
            let (status, missing) = match c.status_in(&dir) {
                Status::Installed => ("installed", Vec::new()),
                Status::NotInstalled => ("not-installed", Vec::new()),
                Status::Partial(m) => ("partial", m),
                Status::Damaged(f) => ("damaged", vec![f]),
            };
            serde_json::json!({
                "id": c.id,
                "title": c.title,
                "status": status,
                "missing": missing,
                "size": c.size(),
                "license": c.license,
                "features": c.features,
                "folder": dir,
            })
        })
        .collect();
    let (installed, total) = registry.installed_count(data);
    serde_json::json!({ "installed": installed, "total": total, "components": items })
}

/// A yes or no question on standard error, asked only on a terminal.
fn ask(question: &str) -> anyhow::Result<bool> {
    let mut input = std::io::stdin().lock();
    super::confirm(
        &format!("{question} y or n:"),
        &mut input,
        super::stdin_is_terminal(),
        "--yes",
    )
}

/// Downloads `c` into `dir` after asking (or `yes`), with progress every
/// 10 percent on standard error. Used by `tw dictate download` too.
pub(crate) fn download(
    c: &Component,
    dir: &Path,
    settings: &textweaver_app::store::Settings,
    yes: bool,
) -> anyhow::Result<()> {
    if c.status_in(dir) == Status::Installed {
        crate::cmd::outln!("{} is already installed in {}.", c.title, dir.display());
        return Ok(());
    }
    let sources = sources(settings);
    let from = match &sources.mirror {
        Some(m) => format!("the mirror {m}, then its public source"),
        None => c
            .files
            .first()
            .and_then(|f| f.url.split('/').nth(2))
            .unwrap_or("its public source")
            .to_owned(),
    };
    eprintln!(
        "This downloads {} from {from}: {}, license {}. {}",
        c.title,
        c.size_text(),
        c.license,
        c.credit
    );
    if !yes && !ask("Download it?")? {
        eprintln!("Nothing was downloaded.");
        return Ok(());
    }
    let cancel = AtomicBool::new(false);
    let mut tenths = Tenths::default();
    textweaver_app::components::download_component(
        c,
        dir,
        &sources,
        &StandardFetcher,
        &mut |p: Progress| {
            if let Some(percent) = tenths.step(p) {
                eprintln!("{percent} percent");
            }
        },
        &cancel,
    )
    .with_context(|| format!("{} was not downloaded", c.title))?;
    crate::cmd::outln!(
        "{} is downloaded and checked, in {}.",
        c.title,
        dir.display()
    );
    Ok(())
}

fn verify(registry: &Registry, data: &Path, id: Option<&str>) -> anyhow::Result<()> {
    let chosen: Vec<&Component> = match id {
        Some(id) => vec![named(registry, id)?],
        None => registry
            .components()
            .iter()
            .filter(|c| c.status_in(&component_dir(c, data)) != Status::NotInstalled)
            .collect(),
    };
    if chosen.is_empty() {
        crate::cmd::outln!("No components are installed.");
        return Ok(());
    }
    let mut bad = 0;
    for c in chosen {
        let dir = component_dir(c, data);
        for (name, s) in c.verify_in(&dir) {
            let said = match s {
                FileState::Good => "checks out".to_owned(),
                FileState::Missing => {
                    bad += 1;
                    "missing".to_owned()
                }
                FileState::WrongSize(n) => {
                    bad += 1;
                    format!("the wrong size, {n} bytes")
                }
                FileState::WrongHash => {
                    bad += 1;
                    "does not match its published hash".to_owned()
                }
            };
            crate::cmd::outln!("{}: {name} {said}.", c.title);
        }
    }
    if bad > 0 {
        bail!("{bad} files do not check out; download them again or install from a file");
    }
    Ok(())
}

fn remove(c: &Component, dir: &Path, yes: bool) -> anyhow::Result<()> {
    if c.status_in(dir) == Status::NotInstalled {
        crate::cmd::outln!("{} is not installed.", c.title);
        return Ok(());
    }
    if !yes && !ask(&format!("Remove {} from {}?", c.title, dir.display()))? {
        eprintln!("Nothing was removed.");
        return Ok(());
    }
    // Each path is printed before it is removed; only the component's
    // own files are named.
    let names = c
        .files
        .iter()
        .map(|f| f.name.to_string())
        .chain(c.notice.iter().map(|(n, _)| n.to_string()));
    for name in names {
        let path = dir.join(name);
        if path.is_file() {
            crate::cmd::outln!("Removing {}", path.display());
        }
    }
    let n = c
        .remove_in(dir)
        .with_context(|| format!("{} was not removed", c.title))?;
    crate::cmd::outln!("Removed {n} files of {}.", c.title);
    Ok(())
}

fn install(c: &Component, dir: &Path, from: &Path) -> anyhow::Result<()> {
    let report = textweaver_app::components::install_component(c, from, dir)
        .with_context(|| format!("{} was not installed", c.title))?;
    for (name, why) in &report.refused {
        eprintln!("Left out {name}: {why}.");
    }
    crate::cmd::outln!(
        "{} is installed and checked, in {}: {} files copied, {} already there.",
        c.title,
        dir.display(),
        report.outcome.fetched.len(),
        report.outcome.kept.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_reads_meaning_first_with_none_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let text = list(&Registry::builtin(), tmp.path());
        let first = text.lines().next().unwrap();
        assert!(
            first.starts_with("Whisper base.en, English dictation: not installed, 79.3 MB"),
            "{first}"
        );
        assert!(text.ends_with(" installed.\n"));
        assert!(text.contains("Id lexend."));
    }

    /// `--yes` downloads without asking: here from a mirror that is a
    /// folder, so nothing goes to the network.
    #[test]
    fn yes_downloads_without_asking_and_remove_takes_only_its_files() {
        use std::borrow::Cow;
        use textweaver_app::components::FilePin;
        let tmp = tempfile::tempdir().unwrap();
        let bytes = b"made-up model".to_vec();
        let c = Component {
            id: Cow::Borrowed("made-up"),
            title: Cow::Borrowed("A made-up model"),
            license: Cow::Borrowed("CC0-1.0"),
            credit: Cow::Borrowed(""),
            features: Cow::Borrowed(&[Cow::Borrowed("dictation")]),
            folder: Cow::Borrowed("models/made-up"),
            files: Cow::Owned(vec![FilePin {
                name: Cow::Borrowed("model.onnx"),
                url: Cow::Borrowed(""),
                size: bytes.len() as u64,
                check: textweaver_app::components::Check::Sha256(Cow::Owned(
                    textweaver_app::components::sha256_hex(&bytes),
                )),
            }]),
            notice: None,
        };
        let mirror = tmp.path().join("mirror");
        std::fs::create_dir_all(mirror.join("made-up")).unwrap();
        std::fs::write(mirror.join("made-up").join("model.onnx"), &bytes).unwrap();
        let mut settings = textweaver_app::store::Settings::default();
        settings.components.mirror = mirror.to_string_lossy().into_owned();
        let dir = c.dir_in(&tmp.path().join("data"));
        if std::env::var_os(textweaver_app::components::MIRROR_ENV).is_none() {
            download(&c, &dir, &settings, true).unwrap();
            assert_eq!(c.status_in(&dir), Status::Installed);
            std::fs::write(dir.join("notes.txt"), b"mine").unwrap();
            remove(&c, &dir, true).unwrap();
            assert!(!dir.join("model.onnx").exists());
            assert!(dir.join("notes.txt").exists(), "only its own files go");
        }
    }

    #[test]
    fn tw_dictate_download_parses() {
        use clap::Parser as _;
        #[derive(clap::Parser)]
        struct T {
            #[command(flatten)]
            args: super::super::dictate::Args,
        }
        let t = T::try_parse_from(["t", "download", "--yes"]).unwrap();
        assert!(matches!(
            t.args.action,
            Some(super::super::dictate::Action::Download { yes: true, .. })
        ));
        let t = T::try_parse_from(["t", "--file", "a.wav"]).unwrap();
        assert!(t.args.action.is_none());
    }

    #[test]
    fn an_unknown_id_names_the_known_ones() {
        let e = named(&Registry::builtin(), "gpt").unwrap_err().to_string();
        assert!(e.contains("whisper-base.en"), "{e}");
    }
}
