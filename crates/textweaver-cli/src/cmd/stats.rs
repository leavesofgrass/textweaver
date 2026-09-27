//! `tw stats`: reading statistics from `stats.json` (time read aloud, the
//! furthest point, and sessions, per document), as the reader's list shows
//! them, or as JSON; `--clear` removes them. Owner: Agent W3e.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use textweaver_app::lexicon::args;
use textweaver_app::lexicon::i18n::{Catalog, duration};
use textweaver_app::store::{Paths, ReadingStats, SettingsStore};

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
    if args.json {
        serde_json::to_writer_pretty(&mut *out, &stats)?;
        writeln!(out)?;
        return Ok(());
    }
    if stats.documents.is_empty() {
        writeln!(out, "{}", c.tr("stats-empty"))?;
    } else {
        writeln!(
            out,
            "{}",
            c.fmt(
                "stats-total",
                &args![
                    "time" => duration(&c, stats.total_seconds()),
                    "sessions" => stats.total_sessions(),
                    "docs" => stats.documents.len()
                ]
            )
        )?;
        for (rank, (_, d)) in stats.most_read(args.top).into_iter().enumerate() {
            let title = if d.title.is_empty() {
                d.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            } else {
                d.title.clone()
            };
            writeln!(
                out,
                "{}",
                c.fmt(
                    "stats-most-read",
                    &args![
                        "rank" => rank + 1,
                        "title" => title,
                        "time" => duration(&c, d.seconds),
                        "pct" => d.furthest_percent
                    ]
                )
            )?;
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
        assert!(run_with(&paths, &args(false, true, false), "n\n").ends_with("Cancelled.\n"));
        assert!(paths.stats_file().exists());
        let cleared = run_with(&paths, &args(false, true, false), "y\n");
        assert!(
            cleared.starts_with("Remove the reading statistics of 1 document? y or n "),
            "{cleared}"
        );
        assert!(!paths.stats_file().exists());
    }
}
