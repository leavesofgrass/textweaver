//! `tw stats`: reading statistics from `stats.json` (time read aloud, the
//! furthest point, and sessions, per document), as the reader's list shows
//! them, or as JSON; `--clear` removes them. Owner: Agent W3e. With sync
//! on, other computers' reading is added document by document, and
//! `--by-computer` lists each computer's share (the sync wave, S6).

use std::io::{BufRead, Write};
use std::path::PathBuf;

use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::{Catalog, duration};
use textweaver_app::store::{Paths, ReadingStats, SettingsStore};
use textweaver_app::synced_library::{CombinedStats, SyncedLibrary, computer_line, stats_title};

/// `--json`: this computer's `stats.json` as before, and the totals over
/// every computer when another computer's reading is included.
#[derive(serde::Serialize)]
struct JsonStats<'a> {
    #[serde(flatten)]
    stats: &'a ReadingStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    combined: Option<&'a CombinedStats>,
}

/// Arguments for `tw stats`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Print the statistics as JSON.
    #[arg(long)]
    pub json: bool,
    /// How many of the most read documents to list.
    #[arg(long, default_value_t = 10, value_name = "N")]
    pub top: usize,
    /// Remove every statistic recorded, after a yes or no.
    #[arg(long)]
    pub clear: bool,
    /// With --clear, do not ask.
    #[arg(long, short)]
    pub yes: bool,
    /// Under each document read on more than one computer, a line per
    /// computer with its share (with sync on).
    #[arg(long)]
    pub by_computer: bool,
    /// Use the files under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Runs `tw stats`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let paths = match &args.home {
        Some(h) => Paths::under(h),
        None => Paths::platform()?,
    };
    let mut input = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    execute(&paths, &args, &mut input, &mut out)
}

fn execute(
    paths: &Paths,
    args: &Args,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let settings = SettingsStore::new(paths.clone()).load().0;
    let (c, _) = Catalog::for_language(&settings.interface.language, Some(&paths.locales_dir()));
    let stats = ReadingStats::load(paths)?;
    if args.clear {
        if stats.documents.is_empty() {
            writeln!(out, "{}", c.tr("stats-empty"))?;
            return Ok(());
        }
        if !args.yes {
            write!(
                out,
                "{} ",
                c.fmt("stats-clear-question", &args!["n" => stats.documents.len()])
            )?;
            out.flush()?;
            let mut answer = String::new();
            input.read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                writeln!(out, "{}", c.tr("common-cancelled"))?;
                return Ok(());
            }
        }
        std::fs::remove_file(paths.stats_file())?;
        writeln!(out, "{}", c.tr("stats-cleared"))?;
        return Ok(());
    }
    // Other computers' reading, with sync and its statistics group on (S6).
    let synced = SyncedLibrary::load(paths, &settings);
    let combined = CombinedStats::build(&stats, synced.as_ref());
    if args.json {
        let out_json = JsonStats {
            stats: &stats,
            combined: combined.has_others().then_some(&combined),
        };
        serde_json::to_writer_pretty(&mut *out, &out_json)?;
        writeln!(out)?;
        return Ok(());
    }
    if combined.documents.is_empty() {
        writeln!(out, "{}", c.tr("stats-empty"))?;
    } else {
        writeln!(
            out,
            "{}",
            c.fmt(
                "stats-total",
                &args![
                    "time" => duration(&c, combined.total_seconds()),
                    "sessions" => combined.total_sessions(),
                    "docs" => combined.documents.len()
                ]
            )
        )?;
        for (rank, d) in combined.documents.iter().take(args.top).enumerate() {
            writeln!(
                out,
                "{}",
                c.fmt(
                    "stats-most-read",
                    &args![
                        "rank" => rank + 1,
                        "title" => stats_title(&c, d),
                        "time" => duration(&c, d.seconds),
                        "pct" => d.furthest_percent
                    ]
                )
            )?;
            if args.by_computer && d.from_others() {
                for share in &d.computers {
                    writeln!(out, "  {}", computer_line(&c, share))?;
                }
            }
        }
    }
    if !settings.stats.enabled {
        writeln!(out, "{}", c.tr("stats-off-cli"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use textweaver_app::store::StatsDelta;

    use super::*;

    fn args(json: bool, clear: bool, yes: bool) -> Args {
        Args {
            json,
            top: 10,
            clear,
            yes,
            by_computer: false,
            home: None,
        }
    }

    fn run_with(paths: &Paths, a: &Args, input: &str) -> String {
        let mut out = Vec::new();
        execute(paths, a, &mut input.as_bytes(), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn lists_clears_and_prints_json() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        assert_eq!(
            run_with(&paths, &args(false, false, false), ""),
            "No reading recorded yet. Time is counted while textweaver reads aloud.\n"
        );
        ReadingStats::add_to_file(
            &paths,
            &[StatsDelta {
                key: "k".into(),
                title: "Biology".into(),
                path: None,
                seconds: 3725.0,
                new_session: true,
                furthest_percent: 40,
                furthest_char: 10,
                at: 1,
            }],
        )
        .unwrap();
        assert_eq!(
            run_with(&paths, &args(false, false, false), ""),
            "1 hour and 2 minutes read in all, in 1 session, over 1 document.\n\
             Most read 1: Biology, 1 hour and 2 minutes, furthest point 40 percent.\n"
        );
        assert!(run_with(&paths, &args(true, false, false), "").contains("\"seconds\": 3725.0"));
        assert!(run_with(&paths, &args(false, true, false), "n\n").ends_with("Canceled.\n"));
        assert!(paths.stats_file().exists());
        let cleared = run_with(&paths, &args(false, true, false), "y\n");
        assert!(
            cleared.starts_with("Remove the reading statistics of 1 document? y or n "),
            "{cleared}"
        );
        assert!(!paths.stats_file().exists());
    }

    /// The sync wave (S6): with sync on, another computer's reading of the
    /// same document is added to this computer's, and `--by-computer`
    /// lists each computer's share.
    #[test]
    fn other_computers_reading_is_summed() {
        use textweaver_app::store::DocKey;
        use textweaver_app::store::sync_ids::{SyncIdEntry, SyncIds};
        use textweaver_app::sync_folder::docid::Details;
        use textweaver_app::sync_folder::{Clock, DocRecord, Identity, SyncFolder, SyncId};

        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("sync");
        std::fs::create_dir_all(&folder).unwrap();
        let doc = dir.path().join("biology.md");
        std::fs::write(&doc, "Cells.").unwrap();
        let id = SyncId::random();

        // The laptop read it for two minutes, in two sessions.
        let laptop = Paths::under(&dir.path().join("laptop"));
        let (mut identity, _) = Identity::load_or_create(&laptop.data_dir).unwrap();
        let sync = SyncFolder::open(&folder, &mut identity, "laptop", "test")
            .unwrap()
            .folder;
        let mut clock = Clock::new(sync.device());
        let mut record = DocRecord::new(id);
        record.publish_identity(
            clock.tick(),
            &Default::default(),
            &Details {
                title: Some("Biology".into()),
                ..Details::default()
            },
        );
        record.stats.seconds.add(sync.device(), 120);
        record.stats.sessions.add(sync.device(), 2);
        sync.write_doc(&record).unwrap();

        // This computer read it for one minute, and knows its sync id.
        let paths = Paths::under(&dir.path().join("lab"));
        let key = DocKey::for_path(&doc);
        ReadingStats::add_to_file(
            &paths,
            &[StatsDelta {
                key: key.0.clone(),
                title: "Biology".into(),
                path: Some(doc.clone()),
                seconds: 60.0,
                new_session: true,
                furthest_percent: 40,
                furthest_char: 10,
                at: 1,
            }],
        )
        .unwrap();
        let mut ids = SyncIds::default();
        ids.set(
            &key,
            SyncIdEntry {
                sync_id: id.to_string(),
                ..SyncIdEntry::default()
            },
        );
        ids.save(&paths.sync_ids_file()).unwrap();
        let store = SettingsStore::new(paths.clone());
        let (mut settings, _) = store.load();
        settings.sync.enabled = true;
        settings.sync.folder = Some(folder);
        settings.sync.device_name = "lab".into();
        store.save(&settings).unwrap();

        let out = run_with(&paths, &args(false, false, false), "");
        assert_eq!(
            out,
            "3 minutes and 0 seconds read in all, in 3 sessions, over 1 document.\n\
             Most read 1: Biology, 3 minutes and 0 seconds, furthest point 40 percent.\n"
        );
        let by = Args {
            by_computer: true,
            ..args(false, false, false)
        };
        let out = run_with(&paths, &by, "");
        assert!(
            out.contains("\n  lab: 1 minute and 0 seconds, 1 session\n"),
            "{out}"
        );
        assert!(
            out.contains("\n  laptop: 2 minutes and 0 seconds, 2 sessions\n"),
            "{out}"
        );
        let json = run_with(&paths, &args(true, false, false), "");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["version"], 1, "stats.json as before");
        assert_eq!(v["combined"]["documents"][0]["seconds"], 180.0);
    }
}
