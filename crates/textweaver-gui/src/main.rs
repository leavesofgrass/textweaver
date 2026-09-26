//! `textweaver-gui`: the native reader window on wxDragon (wxWidgets), the
//! Wave 2 feasibility spike for the Wave 3 GUI (ADR-0014).
//!
//! A menu bar with the keymap's accelerators, the document in a read-only
//! multi-line native text control whose caret follows the spoken word,
//! Play/Pause and Stop buttons, a status bar, and announcements through the
//! `live-region` crate (UI Automation notifications on Windows). The app core
//! (`textweaver-app`) does the work; this crate only translates.
//!
//! Owner: Agent K (spike).

mod announce;
mod keys;
mod log;
mod positions;
mod setup;
mod window;

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, ValueEnum};

use crate::window::{GuiOptions, NameMode};

/// Read documents aloud in a native window.
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
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    home: Option<PathBuf>,
    /// Also speak announcements with the reading voice (for use without a
    /// screen reader).
    #[arg(long)]
    self_voicing: bool,
    /// Start reading from the saved position once the document is open.
    #[arg(long)]
    read: bool,
    /// Close the window after this many seconds (automated checks).
    #[arg(long, value_name = "SECONDS")]
    exit_after: Option<f64>,
    /// Log announcements, commands, keys, and timings (to standard error, or
    /// to --log-file).
    #[arg(long)]
    log: bool,
    /// Write the --log lines to this file.
    #[arg(long, value_name = "PATH")]
    log_file: Option<PathBuf>,
    /// For automated checks: start minimized and inactive with no taskbar
    /// button, so the window never takes focus from the person at the
    /// machine.
    #[arg(long)]
    background: bool,
    /// How the document control is named for screen readers (experiment).
    #[arg(long, value_enum, default_value_t = NameArg::Label)]
    accessible_name: NameArg,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum NameArg {
    Label,
    Accessible,
    Both,
}

fn main() {
    let args = Args::parse();
    if let Some(path) = &args.log_file
        && let Err(e) = log::to_file(path)
    {
        eprintln!("textweaver-gui: cannot write {}: {e}", path.display());
    }
    let opts = GuiOptions {
        app: setup::Options {
            no_speech: args.no_speech,
            backend: args.backend,
            home: args.home,
            self_voicing: args.self_voicing,
        },
        file: args.file,
        read_on_start: args.read,
        exit_after: args
            .exit_after
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(Duration::from_secs_f64),
        log: args.log || args.log_file.is_some(),
        background: args.background,
        name_mode: match args.accessible_name {
            NameArg::Label => NameMode::Label,
            NameArg::Accessible => NameMode::Accessible,
            NameArg::Both => NameMode::Both,
        },
    };
    if let Err(e) = wxdragon::main(move |_app| window::build(opts)) {
        eprintln!("textweaver-gui: {e}");
        std::process::exit(1);
    }
}
