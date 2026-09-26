//! `textweaver-xilem`: the textweaver GUI on Masonry, Vello, Parley,
//! AccessKit, and winit (ADR-0023). It becomes `textweaver-gui` once it
//! passes the UI Automation report the wxDragon spike passes.

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use textweaver_xilem::gui::{self, GuiOptions};
use textweaver_xilem::setup::Options;

/// Read documents aloud in a window.
#[derive(Parser, Debug)]
#[command(name = "textweaver-xilem", version, about)]
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
    /// Also speak announcements with the reading voice.
    #[arg(long)]
    self_voicing: bool,
    /// Start reading once the document is open.
    #[arg(long)]
    read: bool,
    /// Close the window after this many seconds (automated checks).
    #[arg(long, value_name = "SECONDS")]
    exit_after: Option<f64>,
    /// Log announcements, commands, keys, and timings.
    #[arg(long)]
    log: bool,
    /// Write the --log lines to this file.
    #[arg(long, value_name = "PATH")]
    log_file: Option<PathBuf>,
    /// For automated checks: never activate the window, keep it off
    /// screen, and give it no taskbar button.
    #[arg(long)]
    background: bool,
    /// While reading, select the spoken word instead of placing the caret
    /// on it (an experiment for listening sessions).
    #[arg(long)]
    select_spoken: bool,
    /// Expose the document as a read-only multi-line edit instead of a
    /// Document (an experiment for listening sessions).
    #[arg(long)]
    edit_role: bool,
    /// Use this theme instead of the saved one.
    #[arg(long)]
    theme: Option<String>,
    /// Draw the window into this PNG with the CPU renderer and exit; no
    /// window opens.
    #[arg(long, value_name = "PATH")]
    screenshot: Option<PathBuf>,
    /// Scale for --screenshot (1 is 100%, 2 is 200%).
    #[arg(long, default_value_t = 1.0)]
    scale: f64,
    /// For --screenshot: show the spoken word at this character.
    #[arg(long, value_name = "CHAR")]
    highlight_at: Option<usize>,
    /// Write the review screenshots (three themes, 100% and 200%, a dialog)
    /// into this folder and exit.
    #[arg(long, value_name = "DIR")]
    review_screenshots: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Some(path) = &args.log_file
        && let Err(e) = textweaver_xilem::log::to_file(path)
    {
        eprintln!("textweaver-xilem: cannot write {}: {e}", path.display());
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
                    eprintln!("textweaver-xilem: {e}");
                    std::process::exit(1);
                }
            }
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
                home: args.home.clone(),
            };
            if let Err(e) = screenshot(&o) {
                eprintln!("textweaver-xilem: {e}");
                std::process::exit(1);
            }
            return;
        }
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
        },
        theme: args.theme,
    };
    if let Err(e) = gui::run(opts) {
        eprintln!("textweaver-xilem: {e}");
        std::process::exit(1);
    }
}
