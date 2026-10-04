//! `textweaver-xilem`: the textweaver GUI on Masonry, Vello, Parley,
//! AccessKit, and winit (ADR-0027). It becomes `textweaver-gui` once it
//! passes the UI Automation report the wxDragon spike passes.
//!
//! On Windows it is a GUI-subsystem program, so no console window opens
//! beside it (the owner's session 2, ADR-0033). Started from a terminal,
//! it attaches to that terminal to print `--help`, `--version`, and
//! errors ([`textweaver_xilem::console`]).

#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use clap::error::ErrorKind;
use textweaver_xilem::console;
use textweaver_xilem::graphics::{self, GraphicsBackend};
use textweaver_xilem::gui::{self, GuiOptions};
use textweaver_xilem::safety::{self, StartupMessages};
use textweaver_xilem::setup::Options;
use textweaver_xilem::widgets::AnnounceMode;

fn parse_graphics(s: &str) -> Result<GraphicsBackend, String> {
    GraphicsBackend::parse(s)
        .ok_or_else(|| format!("use one of {}", GraphicsBackend::NAMES.join(", ")))
}

fn parse_announce(s: &str) -> Result<AnnounceMode, String> {
    AnnounceMode::parse(s).ok_or_else(|| format!("use {}", AnnounceMode::NAMES.join(" or ")))
}

/// Read documents aloud in a window.
#[derive(Parser, Debug)]
#[command(name = "textweaver-gui", version, about)]
struct Args {
    /// Document to open.
    file: Option<PathBuf>,
    /// Do not speak: the null speech backend.
    #[arg(long)]
    no_speech: bool,
    /// Speech backend id (see `tw backends`); `paced` is a silent backend
    /// that times words like a real engine, for automated checks.
    #[arg(long)]
    backend: Option<String>,
    /// Voice id or name for this run.
    #[arg(long)]
    voice: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    home: Option<PathBuf>,
    /// Speak textweaver's messages with its voice for this run, as the
    /// "Speak textweaver's messages" setting does ([gui] speak_messages).
    #[arg(long)]
    self_voicing: bool,
    /// Start reading once the document is open.
    #[arg(long)]
    read: bool,
    /// Close the window after this many seconds (automated checks).
    #[arg(long, value_name = "SECONDS", hide = true)]
    exit_after: Option<f64>,
    /// Log announcements, commands, keys, and timings.
    #[arg(long)]
    log: bool,
    /// Write the --log lines to this file.
    #[arg(long, value_name = "PATH")]
    log_file: Option<PathBuf>,
    /// For automated checks: never activate the window, keep it off
    /// screen, and give it no taskbar button. On Windows, if UI Automation
    /// activates it all the same, it gives the foreground straight back.
    #[arg(long, hide = true)]
    background: bool,
    /// While reading, select the spoken word instead of placing the caret
    /// at its start. The default, kept after the first screen reader
    /// session, is a background color on the word (ADR-0028).
    #[arg(long, hide = true)]
    select_spoken: bool,
    /// Expose the document as a read-only multi-line edit instead of a
    /// Document (an experiment for listening sessions).
    #[arg(long, hide = true)]
    edit_role: bool,
    /// Let textweaver announce each list item as well as the screen reader
    /// (an experiment for listening sessions).
    #[arg(long, hide = true)]
    app_list_announcements: bool,
    /// Settings opens the app's settings list, as the terminal reader shows
    /// it, instead of the settings dialog (for comparison).
    #[arg(long, hide = true)]
    settings_list: bool,
    /// Show the menus as a list inside the window (F10), as on Linux,
    /// instead of the system's menu bar.
    #[arg(long)]
    list_menus: bool,
    /// How announcements reach the screen reader: `live` (a live region;
    /// the default) or `uia` (UI Automation Notification events, Windows
    /// only). Overrides the `[gui] announce` setting.
    #[arg(long, value_name = "HOW", value_parser = parse_announce)]
    announce: Option<AnnounceMode>,
    /// The graphics API the window draws with: `auto` (the default, every
    /// one wgpu finds), `vulkan`, `dx12`, `metal`, or `gl`. Overrides the
    /// `[gui] graphics` setting; `WGPU_BACKEND`, when set, wins over both.
    #[arg(long, value_name = "API", value_parser = parse_graphics)]
    graphics: Option<GraphicsBackend>,
    /// Use this theme instead of the saved one.
    #[arg(long)]
    theme: Option<String>,
    /// Draw the window into this PNG with the CPU renderer and exit; no
    /// window opens.
    #[arg(long, value_name = "PATH", hide = true)]
    screenshot: Option<PathBuf>,
    /// Scale for --screenshot (1 is 100%, 2 is 200%).
    #[arg(long, default_value_t = 1.0, hide = true)]
    scale: f64,
    /// For --screenshot: show the spoken word at this character.
    #[arg(long, value_name = "CHAR", hide = true)]
    highlight_at: Option<usize>,
    /// Write the review screenshots (three themes, 100% and 200%, a dialog)
    /// into this folder and exit.
    #[arg(long, value_name = "DIR", hide = true)]
    review_screenshots: Option<PathBuf>,
    /// Read the document with no window on screen, on the silent paced
    /// backend, and measure this many moves of the spoken word through the
    /// whole widget tree; then exit. Needs --home. --scale sets the scale.
    #[arg(long, value_name = "MOVES", hide = true)]
    measure_frames: Option<usize>,
    /// For --measure-frames: every reading aid on first.
    #[arg(long, hide = true)]
    frames_aids: bool,
    /// For --measure-frames: also write the report as JSON to this file.
    #[arg(long, value_name = "PATH", hide = true)]
    frames_json: Option<PathBuf>,
}

/// Parses the command line. Help and the version go to the terminal the
/// program was started from; a mistake goes there too, or to a message
/// box when there is none.
fn parse_args() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(e) => match e.kind() {
            ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => e.exit(),
            _ => {
                let background = std::env::args_os().any(|a| a == "--background");
                console::report_error(e.to_string().trim_end(), background);
                std::process::exit(2);
            }
        },
    }
}

/// The first flag given that only the `screenshot` feature runs (the
/// review screenshots, one screenshot, the frame-time probe), if any.
fn harness_flag(args: &Args) -> Option<&'static str> {
    if args.review_screenshots.is_some() {
        Some("--review-screenshots")
    } else if args.measure_frames.is_some() {
        Some("--measure-frames")
    } else if args.screenshot.is_some() {
        Some("--screenshot")
    } else {
        None
    }
}

/// The error for `flag` in a build without the screenshot harness: what
/// failed first, then why, then that no window opened.
fn unbuilt_message(flag: &str) -> String {
    format!(
        "{flag} is not in this build: textweaver-gui was built without the screenshot \
         feature. No window was opened."
    )
}

fn main() {
    // Before anything is printed: connect to the terminal, if any.
    console::attach();
    let args = parse_args();
    // Before any thread starts: it may set an environment variable.
    let gpu = graphics::apply(graphics::wanted(args.graphics, args.home.as_deref()));
    let low_power = graphics::prefer_low_power();
    // textweaver.log in the state folder, as the terminal reader writes it.
    let log_message = safety::start_log(args.home.as_deref());
    let messages = StartupMessages::load(args.home.as_deref());
    safety::install_panic_hook(messages.clone(), args.background);
    if let Some(path) = &args.log_file
        && let Err(e) = textweaver_xilem::log::to_file(path)
    {
        console::report_error(
            &format!("cannot write {}: {e}", path.display()),
            args.background,
        );
    }
    #[cfg(feature = "screenshot")]
    {
        use textweaver_xilem::screenshot::{ShotOptions, review_set, screenshot};
        if let Some(dir) = &args.review_screenshots {
            let file = args
                .file
                .clone()
                .unwrap_or_else(|| PathBuf::from("fixtures/sample.md"));
            match review_set(dir, &file) {
                Ok(files) => {
                    for f in files {
                        println!("{}", f.display());
                    }
                    return;
                }
                Err(e) => {
                    console::report_error(&e.to_string(), true);
                    std::process::exit(1);
                }
            }
        }
        if let Some(moves) = args.measure_frames {
            let (Some(file), Some(home)) = (args.file.clone(), args.home.clone()) else {
                console::report_error(
                    "--measure-frames needs a document and --home, so no settings are saved into yours",
                    true,
                );
                std::process::exit(2);
            };
            let mut o = textweaver_xilem::frames::FrameOptions::new(file, home);
            o.moves = moves.clamp(1, 100_000);
            o.scale = args.scale;
            o.aids = args.frames_aids;
            o.theme = args.theme.clone();
            if let Err(e) = textweaver_xilem::frames::run(&o, args.frames_json.as_deref()) {
                console::report_error(&e, true);
                std::process::exit(1);
            }
            return;
        }
        if let Some(path) = &args.screenshot {
            let o = ShotOptions {
                path: path.clone(),
                file: args.file.clone(),
                size: (1100, 780),
                scale: args.scale,
                theme: args.theme.clone(),
                highlight_at: args.highlight_at,
                list: None,
                settings: false,
                home: args.home.clone(),
                aids: false,
                colors: false,
                voices: false,
                edit: false,
                panel: None,
                ruler: false,
                reading: false,
                settings_filter: None,
            };
            if let Err(e) = screenshot(&o) {
                console::report_error(&e.to_string(), true);
                std::process::exit(1);
            }
            return;
        }
    }
    // Built without the screenshot harness (the separate GUI package,
    // W8b-r): its flags end with a clear error instead of opening a window.
    if !cfg!(feature = "screenshot")
        && let Some(flag) = harness_flag(&args)
    {
        console::report_error(&unbuilt_message(flag), args.background);
        std::process::exit(2);
    }
    let background = args.background;
    if let Some(name) = gpu
        && (args.log || args.log_file.is_some())
    {
        textweaver_xilem::log::line(&format!("graphics: {name}"));
    }
    if low_power && (args.log || args.log_file.is_some()) {
        textweaver_xilem::log::line("graphics: the integrated GPU asked for (WGPU_POWER_PREF=low)");
    }
    // `--log` with no file writes to the terminal for the whole run; else
    // let go of the terminal, so Control C there leaves the window open.
    if !(args.log && args.log_file.is_none()) {
        console::detach();
    }
    let opts = GuiOptions {
        app: Options {
            no_speech: args.no_speech,
            backend: args.backend,
            home: args.home,
            self_voicing: args.self_voicing,
            voice: args.voice,
        },
        file: args.file,
        read_on_start: args.read,
        exit_after: args
            .exit_after
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(Duration::from_secs_f64),
        log: args.log || args.log_file.is_some(),
        background: args.background,
        experiments: textweaver_xilem::gui::Experiments {
            select_spoken: args.select_spoken,
            edit_role: args.edit_role,
            app_list_announcements: args.app_list_announcements,
            settings_list: args.settings_list,
            list_menus: args.list_menus,
            announce: args.announce,
        },
        theme: args.theme,
    };
    if let Some(m) = log_message {
        console::report_error(&m, background);
    }
    // A panic that unwinds out of the window (after the window saved what
    // it could, as it is dropped) ends with a sentence in words.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gui::run(opts))) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            console::report_error(&e, background);
            std::process::exit(1);
        }
        Err(_) => {
            if !safety::graphics_failure_reported() {
                let said = messages.after_crash(safety::saved_after_trouble());
                log::error!("{said}");
                console::report_error(&said, background);
            }
            std::process::exit(101);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("textweaver-gui").chain(args.iter().copied()))
            .expect("the arguments parse")
    }

    /// The harness's flags are found whichever the build, so a build
    /// without the screenshot feature can refuse them (W8c-w).
    #[test]
    fn the_harness_flags_are_named() {
        assert_eq!(harness_flag(&parse(&[])), None);
        assert_eq!(
            harness_flag(&parse(&["notes.md", "--theme", "galaxy"])),
            None
        );
        assert_eq!(
            harness_flag(&parse(&["--screenshot", "shot.png"])),
            Some("--screenshot")
        );
        assert_eq!(
            harness_flag(&parse(&["--review-screenshots", "shots"])),
            Some("--review-screenshots")
        );
        assert_eq!(
            harness_flag(&parse(&["--measure-frames", "20", "--home", "h"])),
            Some("--measure-frames")
        );
    }

    /// The window's command line is textweaver-gui's, and the flags for
    /// automated checks and listening experiments stay out of `--help`.
    #[test]
    fn help_names_textweaver_gui_and_hides_the_test_flags() {
        use clap::CommandFactory;
        let help = Args::command().render_long_help().to_string();
        assert!(help.contains("textweaver-gui"), "{help}");
        for hidden in [
            "--exit-after",
            "--background",
            "--select-spoken",
            "--edit-role",
            "--app-list-announcements",
            "--settings-list",
            "--screenshot",
            "--scale",
            "--highlight-at",
            "--review-screenshots",
            "--measure-frames",
            "--frames-aids",
            "--frames-json",
        ] {
            assert!(!help.contains(hidden), "{hidden} is in --help");
        }
        for shown in ["--no-speech", "--home", "--graphics", "--log"] {
            assert!(help.contains(shown), "{shown} is missing from --help");
        }
        // Hidden flags still parse.
        assert!(parse(&["--background", "--exit-after", "2"]).background);
    }

    /// The error leads with the flag and says no window opened.
    #[test]
    fn the_unbuilt_message_leads_with_the_flag() {
        let m = unbuilt_message("--screenshot");
        assert!(m.starts_with("--screenshot is not in this build"), "{m}");
        assert!(m.ends_with("No window was opened."), "{m}");
    }
}
