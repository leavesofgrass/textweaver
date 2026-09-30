//! `tw sync setup|status|now`: sync notes, highlights, bookmarks, places,
//! reading statistics, portable settings, profiles, key overrides, the word
//! list, the glossary and pronunciations, and favorite voices with other
//! computers through a folder the owner chooses (ADR-0049), from the
//! command line. Each takes `--json`.
//!
//! - `tw sync setup --folder DIR [--name NAME] [--groups places,notes,...]`
//!   turns sync on with that folder, names this computer ("Computer 1",
//!   "Computer 2" and so on when no name is given, never the computer's own
//!   name), and chooses what syncs.
//! - `tw sync status` says how sync stands, the line the reader says:
//!   "Sync: up to date", "Sync: folder missing, saving here".
//! - `tw sync now` merges every document this computer knows, and the
//!   settings and word lists, with the other computers' files, and
//!   publishes its own.
//!
//! Owner: Agent S4 (the sync wave).

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::bail;
use serde::Serialize;
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::{Paths, Settings, SettingsStore};
use textweaver_app::sync_engine::{
    AllOutcome, EngineConfig, EngineStatus, Groups, Notice, StatusKind, SyncEngine, notice_text,
    status_line,
};
use textweaver_app::sync_groups::{GroupsRequest, KeySystem, apply_groups};

/// Arguments for `tw sync`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub command: SyncCommand,
}

/// Options every `tw sync` command takes.
#[derive(clap::Args, Debug, Clone)]
pub struct Common {
    /// Print the result as JSON.
    #[arg(long)]
    pub json: bool,
    /// Use the files under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// `tw sync` commands.
#[derive(clap::Subcommand, Debug)]
pub enum SyncCommand {
    /// Turn sync on with a folder, name this computer, and choose what syncs.
    Setup {
        /// The sync folder: one kept in step by Syncthing, a cloud folder, or a USB stick.
        #[arg(long, value_name = "DIR")]
        folder: PathBuf,
        /// This computer's name, such as laptop or lab (default: Computer 1, Computer 2, ...).
        #[arg(long)]
        name: Option<String>,
        /// What syncs, separated by commas: places, notes, highlights, bookmarks, statistics, settings, profiles, key_overrides, words, glossary, favorite_voices (default: all).
        #[arg(long, value_delimiter = ',', value_name = "GROUPS")]
        groups: Option<Vec<String>>,
        /// JSON and the home folder.
        #[command(flatten)]
        common: Common,
    },
    /// Say how sync stands, and name the other computers.
    Status {
        /// JSON and the home folder.
        #[command(flatten)]
        common: Common,
    },
    /// Merge every document with the other computers now.
    Now {
        /// JSON and the home folder.
        #[command(flatten)]
        common: Common,
    },
}

/// Runs `tw sync`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    let home = match &args.command {
        SyncCommand::Setup { common, .. }
        | SyncCommand::Status { common }
        | SyncCommand::Now { common } => common.home.clone(),
    };
    let paths = match &home {
        Some(h) => Paths::under(h),
        None => Paths::platform()?,
    };
    execute(&paths, &args.command, &mut out)
}

/// What `--json` prints.
#[derive(Serialize, Debug, PartialEq)]
struct Report {
    /// Sync is set up and on.
    enabled: bool,
    /// The status line, as the reader says it.
    status: String,
    /// The state, as a word: off, starting, ready, folder_missing,
    /// read_only, failed.
    state: &'static str,
    /// This computer's name.
    this_computer: String,
    /// The other computers' names.
    other_computers: Vec<String>,
    /// Computers whose clocks are more than a day ahead.
    clocks_ahead: Vec<String>,
    /// Damaged files skipped.
    damaged_files: usize,
    /// Why writing or opening failed, if it did.
    problem: Option<String>,
    /// What was said once (damaged files, a newer format).
    messages: Vec<String>,
    /// For `now`: documents looked at, and those that took changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    documents: Option<usize>,
    /// For `now`.
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<usize>,
    /// For `now`: changes to settings and word lists taken from the other
    /// computers.
    #[serde(skip_serializing_if = "Option::is_none")]
    settings_changes: Option<usize>,
}

fn state_word(k: StatusKind) -> &'static str {
    match k {
        StatusKind::Off => "off",
        StatusKind::Starting => "starting",
        StatusKind::Ready => "ready",
        StatusKind::FolderMissing => "folder_missing",
        StatusKind::ReadOnly => "read_only",
        StatusKind::Failed => "failed",
    }
}

fn report(
    c: &Catalog,
    settings: &Settings,
    on: bool,
    st: &EngineStatus,
    notices: &[Notice],
) -> Report {
    Report {
        enabled: on,
        status: status_line(c, &settings.sync, on, st),
        state: if on { state_word(st.kind) } else { "off" },
        this_computer: st.label.clone(),
        other_computers: st.others.clone(),
        clocks_ahead: st.clocks_ahead.clone(),
        damaged_files: st.damaged,
        problem: st.write_error.clone().or_else(|| st.failure.clone()),
        messages: notices.iter().map(|n| notice_text(c, n).0).collect(),
        documents: None,
        changed: None,
        settings_changes: None,
    }
}

fn print(c: &Catalog, r: &Report, json: bool, out: &mut dyn Write) -> anyhow::Result<()> {
    if json {
        serde_json::to_writer_pretty(&mut *out, r)?;
        writeln!(out)?;
        return Ok(());
    }
    for m in &r.messages {
        writeln!(out, "{m}")?;
    }
    writeln!(out, "{}", r.status)?;
    if r.enabled {
        if !r.this_computer.is_empty() {
            writeln!(
                out,
                "{}",
                c.fmt(
                    "sync-status-this-computer",
                    &args!["name" => r.this_computer.as_str()]
                )
            )?;
        }
        if r.other_computers.is_empty() {
            writeln!(out, "{}", c.tr("sync-status-no-others"))?;
        } else {
            let names = r.other_computers.join(", ");
            writeln!(
                out,
                "{}",
                c.fmt("sync-status-others", &args!["names" => names])
            )?;
        }
        if let Some(p) = &r.problem {
            writeln!(
                out,
                "{}",
                c.fmt("sync-status-error", &args!["error" => p.as_str()])
            )?;
        }
    } else {
        writeln!(out, "{}", c.tr("sync-how-to-set-up"))?;
    }
    Ok(())
}

/// The groups named on the command line.
fn parse_groups(names: &[String]) -> anyhow::Result<Groups> {
    let mut g = Groups::NONE;
    for n in names {
        let name = n.trim().to_lowercase().replace('-', "_");
        let name = match name.as_str() {
            "stats" => "statistics",
            "keys" | "keymap" => "key_overrides",
            "voices" => "favorite_voices",
            other => other,
        };
        if name.is_empty() {
            continue;
        }
        match g.get_mut(name) {
            Some(v) => *v = true,
            None => bail!("unknown group {name}: use {}", Groups::NAMES.join(", ")),
        }
    }
    Ok(g)
}

/// Opens the folder `settings` name, for this computer.
fn engine(settings: &Settings, paths: &Paths) -> (SyncEngine, Vec<Notice>, bool) {
    let mut e = SyncEngine::new();
    let config = EngineConfig::from_settings(&settings.sync, paths);
    let on = config.is_some();
    let notices = e.configure(config);
    (e, notices, on)
}

fn execute(paths: &Paths, command: &SyncCommand, out: &mut dyn Write) -> anyhow::Result<()> {
    let store = SettingsStore::new(paths.clone());
    let mut settings = store.load().0;
    let (c, _) = Catalog::for_language(&settings.interface.language, Some(&paths.locales_dir()));
    match command {
        SyncCommand::Setup {
            folder,
            name,
            groups,
            common,
        } => {
            if !folder.is_dir() {
                bail!("{} is not a folder", folder.display());
            }
            let name = match name {
                Some(n) => match textweaver_app::sync_folder::check_label(n) {
                    Ok(n) => n,
                    Err(_) => bail!("{}", c.tr("sync-name-refused")),
                },
                None => default_name(folder, &settings),
            };
            let g = match groups {
                Some(list) => parse_groups(list)?,
                None => Groups::ALL,
            };
            let y = &mut settings.sync;
            y.enabled = true;
            y.folder = Some(absolute(folder));
            y.device_name = name;
            g.write_to(y);
            store.save(&settings)?;
            let (e, notices, on) = engine(&settings, paths);
            let r = report(&c, &settings, on, e.status(), &notices);
            print(&c, &r, common.json, out)
        }
        SyncCommand::Status { common } => {
            let (e, notices, on) = engine(&settings, paths);
            let r = report(&c, &settings, on, e.status(), &notices);
            print(&c, &r, common.json, out)
        }
        SyncCommand::Now { common } => {
            let (mut e, mut notices, on) = engine(&settings, paths);
            let mut all = AllOutcome::default();
            let mut settings_changes = 0;
            let mut settings_from = Vec::new();
            if on {
                all = e.sync_all(None);
                notices.append(&mut all.notices);
                // Settings and word lists: merged, then applied to the
                // files here (the settings saved only when they changed).
                let saved_ms = std::fs::metadata(paths.settings_file())
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                let mut groups = e.groups_cycle(&GroupsRequest {
                    settings: Box::new(settings.clone()),
                    settings_ms: saved_ms,
                    force: true,
                });
                notices.append(&mut groups.notices);
                let applied =
                    apply_groups(paths, &mut settings, &groups.arrivals, KeySystem::current());
                if !applied.settings_paths.is_empty() {
                    store.save(&settings)?;
                }
                settings_changes = applied.changes;
                settings_from = applied.from;
            }
            let mut r = report(&c, &settings, on, e.status(), &notices);
            if on && matches!(e.status().kind, StatusKind::Ready | StatusKind::ReadOnly) {
                r.status = if all.changed == 0 {
                    c.fmt("sync-now-done", &args!["n" => all.documents])
                } else {
                    c.fmt("sync-now-changed", &args!["n" => all.changed])
                };
                r.documents = Some(all.documents);
                r.changed = Some(all.changed);
                r.settings_changes = Some(settings_changes);
                if settings_changes > 0 {
                    let device = settings_from
                        .first()
                        .cloned()
                        .unwrap_or_else(|| c.tr("sync-another-computer"));
                    r.messages.push(c.fmt(
                        "sync-settings-arrived",
                        &args!["n" => settings_changes, "device" => device],
                    ));
                }
            }
            print(&c, &r, common.json, out)
        }
    }
}

/// The first "Computer N" no other computer in the folder uses, unless
/// this computer already has a name.
fn default_name(folder: &Path, settings: &Settings) -> String {
    let current = settings.sync.device_name.trim();
    if !current.is_empty() {
        return current.to_owned();
    }
    let taken = textweaver_app::sync_engine::labels_elsewhere(folder);
    textweaver_app::sync_folder::default_label(taken.iter().map(String::as_str))
}

fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_in(home: &Path, command: &SyncCommand) -> String {
        let mut out = Vec::new();
        execute(&Paths::under(home), command, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn common(json: bool) -> Common {
        Common { json, home: None }
    }

    #[test]
    fn setup_status_and_now() {
        let home = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let said = run_in(
            home.path(),
            &SyncCommand::Status {
                common: common(false),
            },
        );
        assert!(said.starts_with("Sync: not set up"), "{said}");

        let said = run_in(
            home.path(),
            &SyncCommand::Setup {
                folder: folder.path().to_owned(),
                name: Some("lab".into()),
                groups: Some(vec!["notes".into(), "places".into()]),
                common: common(false),
            },
        );
        assert!(said.contains("Sync: up to date"), "{said}");
        assert!(said.contains("This computer: lab."), "{said}");
        let settings = SettingsStore::new(Paths::under(home.path())).load().0;
        assert!(settings.sync.enabled && settings.sync.notes && !settings.sync.bookmarks);

        let json = run_in(
            home.path(),
            &SyncCommand::Status {
                common: common(true),
            },
        );
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["state"], "ready");
        assert_eq!(v["this_computer"], "lab");

        let json = run_in(
            home.path(),
            &SyncCommand::Now {
                common: common(true),
            },
        );
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["documents"], 0);
        assert!(
            v["status"]
                .as_str()
                .unwrap()
                .starts_with("Sync: up to date")
        );
    }

    #[test]
    fn a_second_computer_gets_the_next_default_name() {
        let folder = tempfile::tempdir().unwrap();
        for expected in ["Computer 1", "Computer 2"] {
            let home = tempfile::tempdir().unwrap();
            let said = run_in(
                home.path(),
                &SyncCommand::Setup {
                    folder: folder.path().to_owned(),
                    name: None,
                    groups: None,
                    common: common(false),
                },
            );
            assert!(
                said.contains(&format!("This computer: {expected}.")),
                "{said}"
            );
        }
    }

    #[test]
    fn a_missing_folder_is_said_first() {
        let home = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        run_in(
            home.path(),
            &SyncCommand::Setup {
                folder: folder.path().to_owned(),
                name: Some("laptop".into()),
                groups: None,
                common: common(false),
            },
        );
        let mut s = SettingsStore::new(Paths::under(home.path())).load().0;
        s.sync.folder = Some(folder.path().join("stick-not-plugged-in"));
        SettingsStore::new(Paths::under(home.path()))
            .save(&s)
            .unwrap();
        let said = run_in(
            home.path(),
            &SyncCommand::Status {
                common: common(false),
            },
        );
        assert!(
            said.starts_with("Sync: folder missing, saving here"),
            "{said}"
        );
    }

    #[test]
    fn bad_groups_and_names_are_refused() {
        assert!(parse_groups(&["notes".into(), "cats".into()]).is_err());
        let home = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        let r = execute(
            &Paths::under(home.path()),
            &SyncCommand::Setup {
                folder: folder.path().to_owned(),
                name: Some("x".repeat(41)),
                groups: None,
                common: common(false),
            },
            &mut out,
        );
        assert!(r.is_err());
    }
}
