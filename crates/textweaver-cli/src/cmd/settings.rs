//! `tw settings`: export settings and key overrides to JSON or TOML, import
//! them (validated, backed up, merged or replaced), show where the files
//! are, and reset them to the defaults. Owner: Agent U.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use textweaver_app::keymap::{Frontend, Keymap, Platform};
use textweaver_app::store::{
    ExportFormat, ExportOptions, ImportMode, ImportPlan, Paths, SettingsStore, atomic_write,
};

/// Arguments for `tw settings`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
    /// Use the settings under this folder instead of the usual place (a
    /// portable install, or trying things out).
    #[arg(long, global = true, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// `tw settings` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Write settings and key overrides to FILE as JSON (or TOML), or print
    /// them when no FILE is given.
    Export(ExportArgs),
    /// Import settings from a JSON or TOML file. Everything is checked first;
    /// the old files are backed up.
    Import(ImportArgs),
    /// Show where the settings files are.
    Path,
    /// Return settings to their defaults: all of them, or one section.
    Reset(ResetArgs),
}

/// Arguments for `tw settings export`.
#[derive(clap::Args, Debug)]
pub struct ExportArgs {
    /// The file to write. Without it, the export is printed.
    pub file: Option<PathBuf>,
    /// Only the values that differ from the defaults: short and easy to
    /// read and share.
    #[arg(long)]
    pub changed_only: bool,
    /// json (the default) or toml. A FILE ending in .toml means toml.
    #[arg(long, value_enum)]
    pub format: Option<Format>,
}

/// Export file format.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// JSON with 2-space indentation and sorted keys.
    Json,
    /// TOML, the format of settings.toml.
    Toml,
}

/// Arguments for `tw settings import`.
#[derive(clap::Args, Debug)]
pub struct ImportArgs {
    /// The file to import: a textweaver export (JSON or TOML) or a
    /// settings.toml. A single dash reads standard input.
    pub file: PathBuf,
    /// Make the settings exactly the file's. Without it, only the settings
    /// the file names change.
    #[arg(long)]
    pub replace: bool,
    /// Show what would change, one line per setting, without writing
    /// anything.
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `tw settings reset`.
#[derive(clap::Args, Debug)]
pub struct ResetArgs {
    /// Reset only this section, for example speech, display, or keymap (the
    /// key overrides).
    #[arg(long, value_name = "NAME")]
    pub section: Option<String>,
    /// Reset without asking first.
    #[arg(long, short)]
    pub yes: bool,
}

/// Runs `tw settings`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = match &args.home {
        Some(home) => Paths::under(home),
        None => Paths::platform()?,
    };
    let store = SettingsStore::new(paths);
    let mut input = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    execute(&store, args.command, &mut input, &mut out)
}

/// Runs a subcommand against `store`, reading answers and `-` from
/// `input` and writing to `out`.
fn execute(
    store: &SettingsStore,
    command: Command,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match command {
        Command::Export(a) => export(store, &a, out),
        Command::Import(a) => import(store, &a, input, out),
        Command::Path => show_paths(store, out),
        Command::Reset(a) => reset(store, &a, input, out),
    }
}

fn export(store: &SettingsStore, a: &ExportArgs, out: &mut dyn Write) -> anyhow::Result<()> {
    let format = match (a.format, &a.file) {
        (Some(Format::Json), _) => ExportFormat::Json,
        (Some(Format::Toml), _) => ExportFormat::Toml,
        (None, Some(file)) => ExportFormat::for_path(file),
        (None, None) => ExportFormat::Json,
    };
    let text = store.export(ExportOptions {
        changed_only: a.changed_only,
        format,
    })?;
    match &a.file {
        None => out.write_all(text.as_bytes())?,
        Some(file) => {
            atomic_write(file, text.as_bytes())?;
            let what = if a.changed_only {
                "Changed settings"
            } else {
                "Settings"
            };
            writeln!(
                out,
                "{what} and key overrides exported to {} as {}.",
                file.display(),
                format.name().to_uppercase()
            )?;
        }
    }
    Ok(())
}

fn read_input(file: &Path, input: &mut dyn BufRead) -> anyhow::Result<String> {
    if file.as_os_str() == "-" {
        let mut text = String::new();
        input
            .read_to_string(&mut text)
            .context("Cannot read settings from standard input")?;
        return Ok(text);
    }
    std::fs::read_to_string(file).with_context(|| format!("Cannot read {}", file.display()))
}

/// Warnings about the key overrides a plan would install: unknown actions,
/// keys that cannot be read, conflicts.
fn key_warnings(plan: &ImportPlan) -> Vec<String> {
    if !plan.keymap_changed {
        return Vec::new();
    }
    Keymap::with_overrides(Platform::current(), Frontend::Terminal, &plan.keymap).1
}

fn print_notes(plan: &ImportPlan, out: &mut dyn Write) -> anyhow::Result<()> {
    for w in plan.warnings.iter().chain(key_warnings(plan).iter()) {
        writeln!(out, "Note: {w}")?;
    }
    Ok(())
}

fn print_changes(plan: &ImportPlan, out: &mut dyn Write) -> anyhow::Result<()> {
    for line in plan.lines() {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn print_backups(
    applied: &textweaver_app::store::Applied,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    for b in &applied.backups {
        writeln!(out, "The old file is saved as {}.", b.display())?;
    }
    Ok(())
}

fn import(
    store: &SettingsStore,
    a: &ImportArgs,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let text = read_input(&a.file, input)?;
    let mode = if a.replace {
        ImportMode::Replace
    } else {
        ImportMode::Merge
    };
    let plan = store.plan_import(&text, mode)?;
    let name = a.file.display();
    print_notes(&plan, out)?;
    if plan.is_empty() {
        writeln!(
            out,
            "Nothing to import: your settings already match {name}."
        )?;
        return Ok(());
    }
    if a.dry_run {
        writeln!(
            out,
            "Dry run: nothing was written. Importing {name} would make these changes. {}",
            plan.summary()
        )?;
        print_changes(&plan, out)?;
        return Ok(());
    }
    let applied = store.apply(&plan)?;
    writeln!(out, "Imported {name}. {}", plan.summary())?;
    print_changes(&plan, out)?;
    print_backups(&applied, out)?;
    Ok(())
}

fn show_paths(store: &SettingsStore, out: &mut dyn Write) -> anyhow::Result<()> {
    let paths = store.paths();
    let state = |p: &Path, missing: &str| {
        if p.exists() {
            "exists".to_owned()
        } else {
            missing.to_owned()
        }
    };
    let settings = paths.settings_file();
    let keymap = paths.keymap_file();
    writeln!(out, "Settings folder: {}", paths.config_dir.display())?;
    writeln!(
        out,
        "Settings file: {} ({})",
        settings.display(),
        state(
            &settings,
            "not created yet; every setting is at its default"
        )
    )?;
    writeln!(
        out,
        "Key overrides file: {} ({})",
        keymap.display(),
        state(&keymap, "not created yet; every key is at its default")
    )?;
    Ok(())
}

fn reset(
    store: &SettingsStore,
    a: &ResetArgs,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let section = a.section.as_deref().map(str::trim);
    let what = match section {
        None => "all settings and key overrides".to_owned(),
        Some("keymap" | "keys") => "the key overrides".to_owned(),
        Some(s) => format!("the {s} settings"),
    };
    let plan = store.plan_reset(section)?;
    print_notes(&plan, out)?;
    if plan.is_empty() {
        writeln!(
            out,
            "Nothing to reset: {what} are already at their defaults."
        )?;
        return Ok(());
    }
    if !a.yes {
        write!(
            out,
            "Reset {what} to their defaults? {} The current files are backed up first. Type y and press Enter to reset, or just press Enter to cancel: ",
            plan.summary()
        )?;
        out.flush()?;
        let mut answer = String::new();
        input.read_line(&mut answer)?;
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            writeln!(out, "Cancelled. Nothing was changed.")?;
            return Ok(());
        }
    }
    let applied = store.apply(&plan)?;
    writeln!(out, "Reset {what} to their defaults. {}", plan.summary())?;
    print_changes(&plan, out)?;
    print_backups(&applied, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use textweaver_app::store::Settings;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let p = std::env::temp_dir()
                .join(format!("tw-settings-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }

        fn store(&self) -> SettingsStore {
            SettingsStore::new(Paths::under(&self.0.join("home")))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[derive(clap::Parser, Debug)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }

    fn parse(argv: &[&str]) -> Args {
        Cli::try_parse_from(std::iter::once("settings").chain(argv.iter().copied()))
            .unwrap()
            .args
    }

    /// Runs `tw settings ARGV` with `answer` on standard input; returns the
    /// output.
    fn tw(store: &SettingsStore, argv: &[&str], answer: &str) -> anyhow::Result<String> {
        let mut out = Vec::new();
        let mut input = answer.as_bytes();
        execute(store, parse(argv).command, &mut input, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn config_files(store: &SettingsStore) -> Vec<String> {
        let mut v: Vec<String> = match std::fs::read_dir(&store.paths().config_dir) {
            Ok(d) => d
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => Vec::new(),
        };
        v.sort();
        v
    }

    #[test]
    fn arguments_parse() {
        let a = parse(&["export", "out.json", "--changed-only", "--format", "toml"]);
        let Command::Export(e) = a.command else {
            panic!()
        };
        assert!(e.changed_only);
        assert_eq!(e.format, Some(Format::Toml));
        assert_eq!(e.file, Some(PathBuf::from("out.json")));
        let a = parse(&["import", "in.json", "--replace", "--dry-run", "--home", "h"]);
        assert_eq!(a.home, Some(PathBuf::from("h")));
        let Command::Import(i) = a.command else {
            panic!()
        };
        assert!(i.replace && i.dry_run);
        let a = parse(&["reset", "--section", "speech", "-y"]);
        let Command::Reset(r) = a.command else {
            panic!()
        };
        assert_eq!(r.section.as_deref(), Some("speech"));
        assert!(r.yes);
        assert!(matches!(parse(&["path"]).command, Command::Path));
    }

    #[test]
    fn export_prints_json_or_writes_a_file() {
        let dir = TempDir::new("export");
        let store = dir.store();
        let mut s = Settings::default();
        s.display.theme = "nord".into();
        store.save(&s).unwrap();

        let printed = tw(&store, &["export"], "").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(doc["textweaver_settings"], 1);
        assert_eq!(doc["settings"]["display"]["theme"], "nord");
        assert_eq!(doc["settings"]["speech"]["rate"], 265, "full by default");

        let printed = tw(&store, &["export", "--changed-only"], "").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(
            doc["settings"],
            serde_json::json!({"display": {"theme": "nord"}})
        );

        let file = dir.0.join("mine.toml");
        let said = tw(
            &store,
            &["export", file.to_str().unwrap(), "--changed-only"],
            "",
        )
        .unwrap();
        assert!(
            said.contains("exported to") && said.contains("as TOML."),
            "{said}"
        );
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            text.contains(
                "[settings.display]
theme = \"nord\""
            ),
            "{text}"
        );
    }

    #[test]
    fn dry_run_lists_changes_and_writes_nothing() {
        let dir = TempDir::new("dry");
        let store = dir.store();
        let file = dir.0.join("in.json");
        std::fs::write(
            &file,
            r#"{"textweaver_settings": 1, "settings": {"speech": {"rate": 300, "auto_play": true}}, "keymap": {"stop": ["F5"]}}"#,
        )
        .unwrap();
        let said = tw(&store, &["import", file.to_str().unwrap(), "--dry-run"], "").unwrap();
        assert!(said.contains("Dry run: nothing was written."), "{said}");
        // F5 already reads the next theme: the conflict is a note.
        assert!(
            said.contains("Note: keymap.toml: F5 is bound to stop"),
            "{said}"
        );
        assert!(said.contains("3 settings change."), "{said}");
        assert!(
            said.contains("\nspeech.rate changes from 265 to 300.\n"),
            "{said}"
        );
        assert!(
            said.contains("\nspeech.auto_play changes from off to on.\n"),
            "{said}"
        );
        assert!(
            said.contains("\nKeys for stop change from the default keys to F5.\n"),
            "{said}"
        );
        assert!(config_files(&store).is_empty());

        let said = tw(&store, &["import", file.to_str().unwrap()], "").unwrap();
        assert!(said.contains("\nImported "), "{said}");
        assert_eq!(store.load().0.speech.rate.wpm(), 300);
        assert_eq!(store.load_keymap().unwrap()["stop"], vec!["F5".to_owned()]);

        // The same file again: nothing to do, nothing written.
        let said = tw(&store, &["import", file.to_str().unwrap()], "").unwrap();
        assert!(said.contains("already match"), "{said}");
        assert_eq!(config_files(&store).len(), 2);
    }

    #[test]
    fn import_backs_up_and_reads_standard_input() {
        let dir = TempDir::new("stdin");
        let store = dir.store();
        let mut s = Settings::default();
        s.display.theme = "nord".into();
        store.save(&s).unwrap();
        let said = tw(
            &store,
            &["import", "-"],
            r#"{"textweaver_settings": 1, "settings": {"display": {"theme": "sepia"}}}"#,
        )
        .unwrap();
        assert!(
            said.contains("display.theme changes from nord to sepia."),
            "{said}"
        );
        assert!(said.contains("The old file is saved as "), "{said}");
        assert!(
            config_files(&store)
                .iter()
                .any(|f| f.starts_with("settings.toml.bak-")),
            "{:?}",
            config_files(&store)
        );
    }

    #[test]
    fn import_reports_notes_and_refuses_bad_files() {
        let dir = TempDir::new("bad");
        let store = dir.store();
        let err = tw(&store, &["import", "-"], r#"{"tts_rate": 300}"#).unwrap_err();
        assert!(err.to_string().contains("tw migrate-star"), "{err}");
        let err = tw(
            &store,
            &["import", "-"],
            r#"{"textweaver_settings": 1, "settings": {"speech": {"rate": "fast"}}}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("settings.speech.rate"), "{err}");
        assert!(config_files(&store).is_empty());
        let err = tw(&store, &["import", "no-such-file.json"], "").unwrap_err();
        assert!(err.to_string().contains("Cannot read"), "{err}");

        let said = tw(
            &store,
            &["import", "-", "--dry-run"],
            r#"{"textweaver_settings": 1, "keymap": {"no_such_action": ["F5"]}, "settings": {"speech": {"shiny": 1}}}"#,
        )
        .unwrap();
        assert!(
            said.contains("Note: settings.speech.shiny is not a textweaver setting; it is kept."),
            "{said}"
        );
        assert!(
            said.contains("Note: unknown action \"no_such_action\""),
            "{said}"
        );
    }

    #[test]
    fn reset_asks_first_unless_yes() {
        let dir = TempDir::new("reset");
        let store = dir.store();
        let mut s = Settings::default();
        s.display.theme = "nord".into();
        s.speech.rate = textweaver_app::core::Rate::Wpm(300);
        store.save(&s).unwrap();

        let said = tw(&store, &["reset"], "\n").unwrap();
        assert!(
            said.contains(
                "Reset all settings and key overrides to their defaults? 2 settings change."
            ),
            "{said}"
        );
        assert!(
            said.ends_with("Cancelled. Nothing was changed.\n"),
            "{said}"
        );
        assert_eq!(store.load().0, s);

        let said = tw(&store, &["reset", "--section", "display"], "y\n").unwrap();
        assert!(
            said.contains("Reset the display settings to their defaults. 1 setting changes."),
            "{said}"
        );
        assert_eq!(store.load().0.display.theme, "galaxy");
        assert_eq!(store.load().0.speech.rate.wpm(), 300);

        let said = tw(&store, &["reset", "--yes"], "").unwrap();
        assert!(!said.contains('?'), "{said}");
        assert_eq!(store.load().0, Settings::default());

        let said = tw(&store, &["reset", "-y"], "").unwrap();
        assert!(said.contains("Nothing to reset"), "{said}");

        let err = tw(&store, &["reset", "--section", "sound", "-y"], "").unwrap_err();
        assert!(err.to_string().contains("Sections: speech"), "{err}");
    }

    #[test]
    fn path_names_the_files() {
        let dir = TempDir::new("path");
        let store = dir.store();
        let said = tw(&store, &["path"], "").unwrap();
        assert!(said.contains("Settings folder: "), "{said}");
        assert!(said.contains("settings.toml (not created yet"), "{said}");
        store.save(&Settings::default()).unwrap();
        let said = tw(&store, &["path"], "").unwrap();
        assert!(said.contains("settings.toml (exists)"), "{said}");
    }
}
