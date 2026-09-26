//! `cargo xtask soak [--minutes N] [--seed S]`: a long, randomized reading
//! session that must stay correct and level (docs/roadmap.md, "Soak test").
//!
//! Like `bench`, it rebuilds xtask in release mode with the `bench` feature
//! (for the counting allocator) and runs `soak-run`. Two workloads run at
//! once for about `N` minutes (default 10):
//!
//! - **Reading.** The app reads the 10 MB bench corpus with the recording
//!   backend (`textweaver_app::testing`, silent and instant), while random
//!   actions arrive every few tens of milliseconds: next and previous
//!   sentence, paragraph, and heading, go to a random percentage, pause
//!   and resume, rate up and down, and an edit (enter edit mode, type,
//!   leave and discard). Between two actions the highlight must only move
//!   forward (every highlight step is checked). For the last 30% of the
//!   run the actions stop, and the whole document is read from the top:
//!   reading must reach the end and stop by itself.
//! - **Host kills.** A second app reads the 1 MB corpus through the
//!   Eloquence backend with the real engine host running its fake engine
//!   (`textweaver-eci-host --engine fake`, no audio), and the host process
//!   is killed at random every few seconds. After each kill, reading must
//!   go on: the highlight moves again within a deadline (the app restarts
//!   the host; if it gave up, reading is started again, as a user would).
//!
//! Throughout, live heap is sampled once a second. After a warm-up, the
//! median of the last third of the samples may be at most 25% (plus
//! 16 MB) above the median of the first third: memory stays level. When
//! both apps are shut down, no engine host started by this process may be
//! left running.

// Without the feature only `run` and the helpers' tests use this module.
#![cfg_attr(not(feature = "bench"), allow(dead_code))]

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, bail};

/// Parsed options.
#[derive(Clone, Debug, PartialEq)]
struct Args {
    minutes: f64,
    seed: u64,
    host: Option<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            minutes: 10.0,
            seed: 0x5eed,
            host: None,
        }
    }
}

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--minutes" => {
                let v = it.next().context("--minutes needs a number")?;
                out.minutes = v
                    .parse()
                    .ok()
                    .filter(|m: &f64| *m > 0.0)
                    .with_context(|| format!("--minutes {v}: give a positive number"))?;
            }
            "--seed" => {
                let v = it.next().context("--seed needs a number")?;
                out.seed = v
                    .parse()
                    .with_context(|| format!("--seed {v}: give a whole number"))?;
            }
            "--host" => out.host = Some(it.next().context("--host needs a path")?.into()),
            other => bail!("unknown option {other} (cargo xtask soak [--minutes N] [--seed S])"),
        }
    }
    Ok(out)
}

/// The engine host's executable name.
const HOST_NAME: &str = "textweaver-eci-host";

/// `cargo xtask soak`: builds the ECI host, then re-runs xtask in release
/// mode with the bench feature.
pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let parsed = parse(&args)?;
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut extra = Vec::new();
    if parsed.host.is_none() {
        let status = Command::new(&cargo)
            .current_dir(crate::bench::root())
            .args([
                "build",
                "--quiet",
                "--release",
                "--no-default-features",
                "-p",
                "textweaver-eci",
                "--bin",
                HOST_NAME,
            ])
            .status()
            .context("running cargo")?;
        if !status.success() {
            bail!("building the engine host failed: {status}");
        }
        let host = crate::eci::target_dir(&crate::bench::root())
            .join("release")
            .join(format!("{HOST_NAME}{}", std::env::consts::EXE_SUFFIX));
        extra.push("--host".to_owned());
        extra.push(host.display().to_string());
    }
    let status = Command::new(&cargo)
        .args([
            "run",
            "--quiet",
            "--release",
            "--package",
            "xtask",
            "--features",
            "bench",
            "--",
            "soak-run",
        ])
        .args(&args)
        .args(&extra)
        .status()?;
    if !status.success() {
        bail!("the soak test failed: {status}");
    }
    Ok(())
}

/// Engine hosts started by this process: (pid, name).
fn child_hosts() -> Vec<(u32, String)> {
    let me = std::process::id();
    let mut out = Vec::new();
    if cfg!(target_os = "linux") {
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return out;
        };
        for e in entries.flatten() {
            let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            let Ok(stat) = std::fs::read_to_string(e.path().join("stat")) else {
                continue;
            };
            // "pid (comm) state ppid ...": comm may hold spaces, so split
            // after its closing parenthesis.
            let Some(rest) = stat.rfind(')').map(|i| &stat[i + 1..]) else {
                continue;
            };
            let ppid = rest
                .split_whitespace()
                .nth(1)
                .and_then(|p| p.parse::<u32>().ok());
            if ppid != Some(me) {
                continue;
            }
            let exe = std::fs::read_link(e.path().join("exe"))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_default();
            if exe.starts_with(HOST_NAME) {
                out.push((pid, exe));
            }
        }
    } else if cfg!(windows) {
        let script = format!(
            "Get-CimInstance Win32_Process -Filter \"ParentProcessId={me}\" | ForEach-Object {{ \"$($_.ProcessId) $($_.Name)\" }}"
        );
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
        {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let mut parts = line.split_whitespace();
                if let (Some(pid), Some(name)) = (parts.next(), parts.next())
                    && let Ok(pid) = pid.parse()
                    && name.starts_with(HOST_NAME)
                {
                    out.push((pid, name.to_owned()));
                }
            }
        }
    } else if let Ok(o) = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,comm="])
        .output()
    {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let mut parts = line.split_whitespace();
            if let (Some(pid), Some(ppid), Some(comm)) = (parts.next(), parts.next(), parts.next())
                && ppid.parse::<u32>().ok() == Some(me)
                && let Ok(pid) = pid.parse()
            {
                let name = comm.rsplit('/').next().unwrap_or(comm);
                if name.starts_with(HOST_NAME) {
                    out.push((pid, name.to_owned()));
                }
            }
        }
    }
    out
}

/// Kills one process at once.
fn kill(pid: u32) -> bool {
    let status = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
    } else {
        Command::new("kill").args(["-9", &pid.to_string()]).output()
    };
    status.is_ok_and(|o| o.status.success())
}

#[cfg(feature = "bench")]
pub use inner::run_inner;

#[cfg(feature = "bench")]
mod inner {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use anyhow::{Context, bail};
    use textweaver_app::a11y::LogAnnouncer;
    use textweaver_app::core::CharRange;
    use textweaver_app::eci::{AudioOutput, EciBackend, EciConfig};
    use textweaver_app::keymap::{ActionId, Frontend, Keymap, Platform};
    use textweaver_app::speech::{ServiceConfig, SpeechService};
    use textweaver_app::store::{Paths, Settings};
    use textweaver_app::{App, AppConfig, Command, Playback, parse_go_to};

    use crate::bench::{Lcg, alloc, markdown_corpus};

    fn new_app(home: &Path, speech: SpeechService, backend: &str) -> App {
        let mut app = App::new(AppConfig {
            settings: Settings::default(),
            keymap: Keymap::defaults(Platform::current(), Frontend::Terminal),
            speech,
            paths: Some(Paths::under(home)),
            announcer: Box::new(LogAnnouncer::default()),
            // Announcements stay on the status line, so the highlight is
            // only ever the document's.
            self_voicing: false,
            backend_name: backend.into(),
        });
        app.dispatch(Command::Resize {
            width: 100,
            height: 30,
        });
        app
    }

    fn corpus(name: &str, bytes: usize, seed: u64) -> anyhow::Result<PathBuf> {
        let dir = crate::bench::corpus_dir();
        std::fs::create_dir_all(&dir)?;
        let p = dir.join(name);
        let text = markdown_corpus(bytes, seed);
        if std::fs::read_to_string(&p).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&p, &text)?;
        }
        Ok(p)
    }

    fn spoken(app: &App) -> Option<CharRange> {
        app.session().and_then(|s| s.spoken)
    }

    fn doc_len(app: &App) -> usize {
        app.session().map_or(0, |s| s.doc.len_chars())
    }

    /// Follows the highlight between actions: it may only move forward.
    #[derive(Default)]
    struct Order {
        last: Option<usize>,
        /// The furthest highlight end seen.
        furthest: usize,
        checked: u64,
        backwards: Vec<String>,
    }

    impl Order {
        /// A new stretch starts (an action moved the reading position).
        fn reset(&mut self) {
            self.last = None;
        }

        fn see(&mut self, app: &App, what: &str) {
            if app.playback() != Playback::Reading {
                return;
            }
            let Some(r) = spoken(app) else {
                return;
            };
            let at = r.start.0;
            if let Some(last) = self.last
                && at < last
                && self.backwards.len() < 20
            {
                self.backwards.push(format!(
                    "{what}: the highlight went back from {last} to {at}"
                ));
            }
            self.last = Some(at);
            self.furthest = self.furthest.max(r.end.0);
            self.checked += 1;
        }
    }

    /// Polls speech until `until`, applying one status at a time so every
    /// highlight step is seen (a window that ends clears the highlight
    /// before the next one starts, so sampling after a whole batch would
    /// miss them).
    fn pump(app: &mut App, order: &mut Order, what: &str, until: Instant) {
        while Instant::now() < until {
            let mut applied = 0;
            while app.poll_speech_step().is_some() {
                order.see(app, what);
                applied += 1;
                if applied >= 10_000 {
                    break;
                }
            }
            // Notices a speech thread that died.
            app.poll_speech();
            if applied == 0 {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }

    /// What the reading workload saw.
    #[derive(Debug, Default)]
    struct ReadingReport {
        actions: u64,
        edits: u64,
        checked: u64,
        backwards: Vec<String>,
        finished: bool,
        end: usize,
        len: usize,
    }

    /// Reading the 10 MB corpus with the recording backend and random
    /// actions until `actions_until`, then reading to the end by
    /// `deadline`.
    fn reading(
        path: &Path,
        home: &Path,
        seed: u64,
        actions_until: Instant,
        deadline: Instant,
    ) -> anyhow::Result<ReadingReport> {
        let (speech, log) =
            textweaver_app::testing::recording_service().context("the recording backend")?;
        let mut app = new_app(home, speech, "recording");
        app.open(path).context("opening the corpus")?;
        let mut rep = ReadingReport {
            len: doc_len(&app),
            ..ReadingReport::default()
        };
        let mut rng = Lcg(seed);
        let mut order = Order::default();
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        let debug = std::env::var_os("TW_SOAK_DEBUG").is_some();
        while Instant::now() < actions_until {
            if debug {
                println!(
                    "    reading: {:?}, highlight {:?}, {} utterances, {:?}",
                    app.playback(),
                    spoken(&app),
                    log.utterances().len(),
                    app.status_text()
                );
            }
            // The recording log keeps every utterance; the app does not.
            log.clear();
            let pause = Duration::from_millis(20 + rng.below(80) as u64);
            pump(&mut app, &mut order, "reading", Instant::now() + pause);
            order.reset();
            rep.actions += 1;
            let what = rng.below(100);
            match what {
                0..=14 => {
                    app.dispatch(Command::Action(ActionId::NextSentence));
                }
                15..=19 => {
                    app.dispatch(Command::Action(ActionId::PreviousSentence));
                }
                20..=29 => {
                    app.dispatch(Command::Action(ActionId::NextParagraph));
                }
                30..=32 => {
                    app.dispatch(Command::Action(ActionId::PreviousParagraph));
                }
                33..=40 => {
                    app.dispatch(Command::Action(ActionId::NextHeading));
                }
                41..=43 => {
                    app.dispatch(Command::Action(ActionId::PreviousHeading));
                }
                44..=47 => {
                    let pct = rng.below(90);
                    if let Some(to) = parse_go_to(&format!("{pct}%")) {
                        app.dispatch(Command::GoTo(to));
                    }
                }
                48..=59 => {
                    // Pause, wait, resume.
                    app.dispatch(Command::Action(ActionId::PlayPause));
                    pump(
                        &mut app,
                        &mut order,
                        "paused",
                        Instant::now() + Duration::from_millis(rng.below(60) as u64),
                    );
                    order.reset();
                    app.dispatch(Command::Action(ActionId::PlayPause));
                }
                60..=69 => {
                    app.dispatch(Command::Action(ActionId::RateUp));
                }
                70..=79 => {
                    app.dispatch(Command::Action(ActionId::RateDown));
                }
                80..=83 => {
                    rep.edits += 1;
                    app.dispatch(Command::Action(ActionId::ToggleEditMode));
                    if app.is_editing() {
                        for c in "soak ".chars() {
                            app.dispatch(Command::Insert(c.to_string()));
                        }
                        app.dispatch(Command::DeleteBack);
                        app.dispatch(Command::Action(ActionId::ToggleEditMode));
                        // Leaving with changes asks what to do: discard.
                        if app.is_editing() {
                            app.dispatch(Command::Choose(1));
                        }
                    }
                }
                _ => {}
            }
            app.poll_speech();
            if app.playback() != Playback::Reading {
                app.dispatch(Command::Action(ActionId::ReadFromCursor));
            }
        }

        // Read the whole document from the top to the end, with no more
        // actions.
        app.dispatch(Command::Action(ActionId::Stop));
        app.dispatch(Command::GoTo(parse_go_to("start").expect("start")));
        app.poll_speech();
        order.reset();
        order.furthest = 0;
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        while Instant::now() < deadline {
            log.clear();
            pump(
                &mut app,
                &mut order,
                "reading to the end",
                Instant::now() + Duration::from_millis(200),
            );
            if app.playback() == Playback::Idle {
                break;
            }
        }
        let last_end = order.furthest;
        rep.end = last_end;
        // Finished: reading stopped by itself, with the last highlight in
        // the last 1% of the document.
        rep.finished = app.playback() == Playback::Idle && last_end + rep.len / 100 >= rep.len;
        rep.checked = order.checked;
        rep.backwards = order.backwards;
        app.shutdown();
        Ok(rep)
    }

    /// What the host-kill workload saw.
    #[derive(Debug, Default)]
    struct KillReport {
        kills: u64,
        recovered: u64,
        restarts: u64,
        stuck: Vec<String>,
        checked: u64,
        backwards: Vec<String>,
    }

    fn eci_service(host: &Path) -> anyhow::Result<SpeechService> {
        let config = EciConfig {
            host: Some(host.to_owned()),
            // Silent, eight times faster than real time.
            output: AudioOutput::Null { speed: 8.0 },
            fake_engine: true,
            ..EciConfig::default()
        };
        SpeechService::spawn(
            Box::new(move || Ok(Box::new(EciBackend::new(config.clone())?) as _)),
            ServiceConfig::default(),
        )
        .map_err(|e| anyhow::anyhow!("starting the fake engine host: {e}"))
    }

    /// Reads the 1 MB corpus through the fake engine host, killing the
    /// host every few seconds, until `stop` is set.
    fn host_kills(
        path: &Path,
        home: &Path,
        host: &Path,
        seed: u64,
        stop: &AtomicBool,
    ) -> anyhow::Result<KillReport> {
        let mut app = new_app(home, eci_service(host)?, "eci-fake");
        app.open(path).context("opening the corpus")?;
        let mut rng = Lcg(seed ^ 0x6b69_6c6c);
        let mut order = Order::default();
        let mut rep = KillReport::default();
        app.dispatch(Command::Action(ActionId::ReadFromCursor));
        while !stop.load(Ordering::Relaxed) {
            let wait = Duration::from_millis(1500 + rng.below(4000) as u64);
            pump(&mut app, &mut order, "host kills", Instant::now() + wait);
            if rng.below(3) == 0 {
                app.dispatch(Command::Action(if rng.below(2) == 0 {
                    ActionId::RateUp
                } else {
                    ActionId::RateDown
                }));
            }
            let hosts = super::child_hosts();
            let Some((pid, _)) = hosts.first() else {
                continue;
            };
            if !super::kill(*pid) {
                continue;
            }
            rep.kills += 1;
            order.reset();
            // Reading must go on: the highlight moves again soon.
            let before = spoken(&app).map(|r| r.start.0);
            let deadline = Instant::now() + Duration::from_secs(20);
            let mut moved = false;
            while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
                pump(
                    &mut app,
                    &mut order,
                    "after a host kill",
                    Instant::now() + Duration::from_millis(50),
                );
                if spoken(&app).map(|r| r.start.0) != before {
                    moved = true;
                    break;
                }
                if app.playback() == Playback::Idle {
                    // The app gave up on this sentence; start again, as a
                    // user would.
                    rep.restarts += 1;
                    if app
                        .session()
                        .is_some_and(|s| s.cursor.0 + 10 >= s.doc.len_chars())
                    {
                        app.dispatch(Command::GoTo(parse_go_to("start").expect("start")));
                    }
                    app.dispatch(Command::Action(ActionId::ReadFromCursor));
                }
            }
            if moved {
                rep.recovered += 1;
            } else if !stop.load(Ordering::Relaxed) && rep.stuck.len() < 20 {
                rep.stuck.push(format!(
                    "kill {}: the highlight did not move within 20 s ({}; {})",
                    rep.kills,
                    match app.playback() {
                        Playback::Idle => "idle",
                        Playback::Reading => "reading",
                        Playback::Paused { .. } => "paused",
                    },
                    app.status_text()
                ));
            }
            if app.playback() == Playback::Idle {
                app.dispatch(Command::Action(ActionId::ReadFromCursor));
            }
        }
        rep.checked = order.checked;
        rep.backwards = order.backwards;
        app.shutdown();
        Ok(rep)
    }

    fn median(v: &mut [usize]) -> usize {
        v.sort_unstable();
        v.get(v.len() / 2).copied().unwrap_or(0)
    }

    fn mb(b: usize) -> f64 {
        b as f64 / 1_048_576.0
    }

    /// Runs the soak test (the `soak-run` task).
    pub fn run_inner() -> anyhow::Result<()> {
        let args = super::parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
        let host = args
            .host
            .clone()
            .context("soak-run needs --host (cargo xtask soak builds it)")?;
        let total = Duration::from_secs_f64(args.minutes * 60.0);
        let start = Instant::now();
        // Random actions for 70% of the time; then reading to the end,
        // which may run a little over.
        let actions_until = start + total.mul_f64(0.7);
        let deadline = start + total + Duration::from_secs(300);
        let big = corpus("md-10mb.md", 10 << 20, 4)?;
        let small = corpus("md-1mb.md", 1 << 20, 1)?;
        let base = crate::bench::corpus_dir().join(format!("soak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        println!(
            "soak: {:.1} minutes, seed {}, {} and {}",
            args.minutes,
            args.seed,
            big.display(),
            small.display()
        );

        let stop = Arc::new(AtomicBool::new(false));
        let kills = {
            let stop = Arc::clone(&stop);
            let (small, home, host) = (small.clone(), base.join("kills"), host.clone());
            let seed = args.seed;
            std::thread::spawn(move || host_kills(&small, &home, &host, seed, &stop))
        };
        let reader = {
            let (big, home) = (big.clone(), base.join("reading"));
            let seed = args.seed;
            std::thread::spawn(move || reading(&big, &home, seed, actions_until, deadline))
        };

        // Live heap once a second while the reader runs.
        let mut samples = Vec::new();
        let mut next_report = Instant::now() + Duration::from_secs(60);
        while !reader.is_finished() {
            std::thread::sleep(Duration::from_secs(1));
            samples.push(alloc::current());
            if Instant::now() >= next_report {
                next_report += Duration::from_secs(60);
                println!(
                    "  {:.0} s: {:.1} MB live heap, {} engine hosts running",
                    start.elapsed().as_secs_f64(),
                    mb(alloc::current()),
                    super::child_hosts().len()
                );
            }
        }
        stop.store(true, Ordering::Relaxed);
        let reading = reader
            .join()
            .map_err(|_| anyhow::anyhow!("the reading workload panicked"))??;
        let kills = kills
            .join()
            .map_err(|_| anyhow::anyhow!("the host-kill workload panicked"))??;

        // No host may outlive its app.
        let wait = Instant::now() + Duration::from_secs(5);
        let mut left = super::child_hosts();
        while !left.is_empty() && Instant::now() < wait {
            std::thread::sleep(Duration::from_millis(100));
            left = super::child_hosts();
        }
        let _ = std::fs::remove_dir_all(&base);

        let mut failures = Vec::new();
        println!(
            "Reading: {} random actions ({} edits); the highlight was checked {} times and reached character {} of {}.",
            reading.actions, reading.edits, reading.checked, reading.end, reading.len
        );
        if !reading.finished {
            failures.push(format!(
                "reading did not finish: the last highlight ended at character {} of {}",
                reading.end, reading.len
            ));
        }
        failures.extend(reading.backwards.iter().cloned());
        println!(
            "Host kills: {} kills, {} recovered, {} restarts by hand; the highlight was checked {} times.",
            kills.kills, kills.recovered, kills.restarts, kills.checked
        );
        if kills.kills == 0 {
            failures.push("no engine host was killed (none was found running)".into());
        }
        failures.extend(kills.stuck.iter().cloned());
        failures.extend(kills.backwards.iter().cloned());

        // Memory stays level: skip the first 10% (warm-up), then compare
        // the medians of the first and last thirds.
        let warm = samples.len() / 10;
        let rest = &samples[warm.min(samples.len())..];
        if rest.len() >= 6 {
            let third = rest.len() / 3;
            let first = median(&mut rest[..third].to_vec());
            let last = median(&mut rest[rest.len() - third..].to_vec());
            let limit = first + first / 4 + (16 << 20);
            println!(
                "Memory: live heap {:.1} MB early, {:.1} MB late (limit {:.1} MB), {} samples.",
                mb(first),
                mb(last),
                mb(limit),
                samples.len()
            );
            if last > limit {
                failures.push(format!(
                    "live heap grew from {:.1} MB to {:.1} MB",
                    mb(first),
                    mb(last)
                ));
            }
        } else {
            println!("Memory: too few samples to compare ({}).", samples.len());
        }
        if !left.is_empty() {
            failures.push(format!(
                "engine hosts were left running: {}",
                left.iter()
                    .map(|(p, n)| format!("{n} ({p})"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if failures.is_empty() {
            println!("Soak test passed.");
            return Ok(());
        }
        for f in &failures {
            println!("Failed: {f}");
        }
        bail!("the soak test found {} problems", failures.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse() {
        let a = parse(&[
            "--minutes".into(),
            "0.5".into(),
            "--seed".into(),
            "7".into(),
        ])
        .unwrap();
        assert!((a.minutes - 0.5).abs() < f64::EPSILON);
        assert_eq!(a.seed, 7);
        assert_eq!(parse(&[]).unwrap(), Args::default());
        assert!(parse(&["--minutes".into(), "0".into()]).is_err());
        assert!(parse(&["--minutes".into()]).is_err());
        assert!(parse(&["--seed".into(), "x".into()]).is_err());
        assert!(parse(&["--bogus".into()]).is_err());
    }

    #[test]
    fn no_hosts_are_running_under_the_test_process() {
        // The test harness starts no engine hosts.
        assert!(child_hosts().is_empty());
    }
}
