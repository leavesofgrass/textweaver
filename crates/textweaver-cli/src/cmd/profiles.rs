//! `tw settings profile`: settings profiles from the command line, the same
//! `profiles.toml` the reader's profiles list uses. Owner: Agent W3e.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::Context;
use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::Catalog;
use textweaver_app::store::profiles::{ProfileError, value_text};
use textweaver_app::store::{Profiles, SettingsStore, atomic_write};

/// `tw settings profile` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum ProfileCommand {
    /// List the profiles, with their voice, rate, theme, and access mode.
    List,
    /// Save the current settings as a profile (replacing one of that name).
    Save {
        /// The profile's name.
        name: String,
    },
    /// Switch to a profile: its settings are written into settings.toml.
    Switch {
        /// The profile's name.
        name: String,
    },
    /// Rename a profile.
    Rename {
        /// The current name.
        from: String,
        /// The new name.
        to: String,
    },
    /// Delete a profile, after a yes or no.
    Delete {
        /// The profile's name.
        name: String,
        /// Do not ask.
        #[arg(long, short)]
        yes: bool,
    },
    /// Export profiles to FILE (JSON, or TOML for a .toml name), or print
    /// them.
    Export {
        /// The file to write; without it, the export is printed.
        file: Option<PathBuf>,
        /// Only this profile (give it more than once for several).
        #[arg(long = "name", value_name = "NAME")]
        names: Vec<String>,
    },
    /// Import profiles from an export; profiles of the same name are
    /// replaced.
    Import {
        /// The export file.
        file: PathBuf,
    },
}

fn catalog(store: &SettingsStore) -> std::sync::Arc<Catalog> {
    let settings = store.load().0;
    Catalog::for_language(
        &settings.interface.language,
        Some(&store.paths().locales_dir()),
    )
    .0
}

fn problem(c: &Catalog, e: &ProfileError) -> anyhow::Error {
    let text = match e {
        ProfileError::NotFound(n) => c.fmt("profile-not-found", &args!["name" => n]),
        ProfileError::EmptyName => c.tr("profile-needs-name"),
        ProfileError::Exists(n) => c.fmt("profile-exists", &args!["name" => n]),
        ProfileError::NotAnExport(d) => c.fmt("profiles-not-an-export", &args!["detail" => d]),
        ProfileError::Store(s) => c.fmt("profiles-save-failed", &args!["error" => s.to_string()]),
    };
    anyhow::anyhow!(text)
}

/// Runs a profile subcommand against the settings in `store`.
pub fn execute(
    store: &SettingsStore,
    command: ProfileCommand,
    input: &mut dyn BufRead,
    terminal: bool,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let c = catalog(store);
    let paths = store.paths().clone();
    let mut profiles = Profiles::load(&paths).context("reading profiles.toml")?;
    match command {
        ProfileCommand::List => {
            let n = profiles.profiles.len();
            writeln!(out, "{}", c.fmt("profiles-title", &args!["n" => n]))?;
            for (name, p) in &profiles.profiles {
                let mut parts = Vec::new();
                for (key, id, var) in [
                    ("speech.voice", "profile-summary-voice", "voice"),
                    ("speech.rate", "profile-summary-rate", "rate"),
                    ("display.theme", "profile-summary-theme", "theme"),
                    ("accessibility.mode", "profile-summary-access", "mode"),
                ] {
                    if let Some(v) = value_text(p, key) {
                        parts.push(c.fmt(id, &[(var, v.replace('-', " ").into())]));
                    }
                }
                let summary = if parts.is_empty() {
                    c.tr("profile-summary-empty")
                } else {
                    parts.join(", ")
                };
                let id = if profiles.active.as_deref() == Some(name.as_str()) {
                    "profiles-item-active"
                } else {
                    "profiles-item"
                };
                writeln!(
                    out,
                    "- {}",
                    c.fmt(id, &args!["name" => name, "summary" => summary])
                )?;
            }
        }
        ProfileCommand::Save { name } => {
            let settings = store.load().0;
            let replaced = profiles
                .save_current(&name, &settings)
                .map_err(|e| problem(&c, &e))?;
            let saved = profiles.active.clone().unwrap_or_default();
            profiles.save(&paths)?;
            let id = if replaced {
                "profile-replaced"
            } else {
                "profile-saved"
            };
            writeln!(out, "{}", c.fmt(id, &args!["name" => saved]))?;
        }
        ProfileCommand::Switch { name } => {
            let settings = store.load().0;
            let (new, dropped) = profiles
                .apply(&name, &settings)
                .map_err(|e| problem(&c, &e))?;
            store.save(&new)?;
            profiles.save(&paths)?;
            writeln!(
                out,
                "{}",
                c.fmt("profile-switched", &args!["name" => &name])
            )?;
            if !dropped.is_empty() {
                writeln!(
                    out,
                    "{}",
                    c.fmt(
                        "profile-dropped",
                        &args!["n" => dropped.len(), "keys" => dropped.join(", ")]
                    )
                )?;
            }
        }
        ProfileCommand::Rename { from, to } => {
            profiles.rename(&from, &to).map_err(|e| problem(&c, &e))?;
            profiles.save(&paths)?;
            writeln!(
                out,
                "{}",
                c.fmt(
                    "profile-renamed",
                    &args!["old" => &from, "new" => to.trim()]
                )
            )?;
        }
        ProfileCommand::Delete { name, yes } => {
            if !profiles.profiles.contains_key(&name) {
                return Err(problem(&c, &ProfileError::NotFound(name)));
            }
            let question = c.fmt("profile-delete-question", &args!["name" => &name]);
            if !yes && !super::confirm(&question, input, terminal, "--yes")? {
                writeln!(out, "{}", c.tr("profile-kept"))?;
                return Ok(());
            }
            profiles.delete(&name).map_err(|e| problem(&c, &e))?;
            profiles.save(&paths)?;
            writeln!(out, "{}", c.fmt("profile-deleted", &args!["name" => &name]))?;
        }
        ProfileCommand::Export { file, names } => {
            if profiles.profiles.is_empty() {
                writeln!(out, "{}", c.tr("profiles-none-to-export"))?;
                return Ok(());
            }
            let toml = file
                .as_ref()
                .and_then(|f| f.extension())
                .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
            let chosen = (!names.is_empty()).then_some(names.as_slice());
            let text = profiles.export(chosen, toml).map_err(|e| problem(&c, &e))?;
            match file {
                None => out.write_all(text.as_bytes())?,
                Some(f) => {
                    atomic_write(&f, text.as_bytes())?;
                    let n = chosen.map_or(profiles.profiles.len(), <[String]>::len);
                    writeln!(
                        out,
                        "{}",
                        c.fmt(
                            "profiles-exported",
                            &args!["n" => n, "file" => f.display().to_string()]
                        )
                    )?;
                }
            }
        }
        ProfileCommand::Import { file } => {
            let r = profiles.import_file(&file).map_err(|e| problem(&c, &e))?;
            if !r.imported.is_empty() {
                profiles.save(&paths)?;
            }
            writeln!(
                out,
                "{}",
                c.fmt(
                    "profiles-imported",
                    &args![
                        "n" => r.imported.len(),
                        "file" => file.display().to_string(),
                        "names" => r.imported.join(", ")
                    ]
                )
            )?;
            if !r.dropped.is_empty() {
                writeln!(
                    out,
                    "{}",
                    c.fmt(
                        "profile-dropped",
                        &args!["n" => r.dropped.len(), "keys" => r.dropped.join(", ")]
                    )
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use textweaver_app::store::Paths;

    use super::*;

    fn run(store: &SettingsStore, cmd: ProfileCommand, input: &str) -> anyhow::Result<String> {
        let mut out = Vec::new();
        execute(store, cmd, &mut input.as_bytes(), true, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn save_list_switch_rename_export_import_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(Paths::under(&dir.path().join("home")));
        let mut s = store.load().0;
        s.speech.rate = textweaver_app::core::Rate::Wpm(180);
        store.save(&s).unwrap();
        assert_eq!(
            run(
                &store,
                ProfileCommand::Save {
                    name: "Study".into()
                },
                ""
            )
            .unwrap(),
            "Saved the current settings as Study.\n"
        );
        assert_eq!(
            run(&store, ProfileCommand::List, "").unwrap(),
            "Settings profiles, 1 profile\n- Study, in use: rate 180, theme galaxy, self voicing mode\n"
        );
        s.speech.rate = textweaver_app::core::Rate::Wpm(300);
        store.save(&s).unwrap();
        run(
            &store,
            ProfileCommand::Switch {
                name: "Study".into(),
            },
            "",
        )
        .unwrap();
        assert_eq!(store.load().0.speech.rate.wpm(), 180);
        let err = run(
            &store,
            ProfileCommand::Switch {
                name: "Nope".into(),
            },
            "",
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "There is no profile named Nope.");
        run(
            &store,
            ProfileCommand::Rename {
                from: "Study".into(),
                to: "Exam".into(),
            },
            "",
        )
        .unwrap();
        let file = dir.path().join("p.toml");
        let out = run(
            &store,
            ProfileCommand::Export {
                file: Some(file.clone()),
                names: vec![],
            },
            "",
        )
        .unwrap();
        assert!(out.starts_with("Exported 1 profile to "), "{out}");
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("textweaver_profiles = 1")
        );
        assert_eq!(
            run(
                &store,
                ProfileCommand::Delete {
                    name: "Exam".into(),
                    yes: false
                },
                "n\n"
            )
            .unwrap(),
            "Kept.\n"
        );
        run(
            &store,
            ProfileCommand::Delete {
                name: "Exam".into(),
                yes: true,
            },
            "",
        )
        .unwrap();
        assert_eq!(
            run(&store, ProfileCommand::List, "").unwrap(),
            "Settings profiles, none saved yet\n"
        );
        let out = run(&store, ProfileCommand::Import { file }, "").unwrap();
        assert!(out.starts_with("Imported 1 profile from "), "{out}");
    }
}
