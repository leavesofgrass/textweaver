//! `textweaver`: the self-voicing terminal reader.
//!
//! Owner: Agent D.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use textweaver_app::a11y::Priority;
use textweaver_tui::{Options, Tui, build_app, run};

/// Read documents aloud in the terminal.
#[derive(Parser, Debug)]
#[command(name = "textweaver", version, about)]
struct Args {
    /// Document to open.
    file: Option<PathBuf>,
    /// Do not speak: no self-voicing and no reading aloud (use with a
    /// screen reader, which reads the status line and follows the cursor).
    #[arg(long)]
    no_speech: bool,
    /// Speech backend id (see `tw backends`); overrides the settings.
    #[arg(long)]
    backend: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    home: Option<PathBuf>,
    /// Color theme for this run: galaxy, light, or high-contrast.
    #[arg(long)]
    theme: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let opts = Options {
        no_speech: args.no_speech,
        backend: args.backend,
        home: args.home,
        theme: args.theme,
    };
    let (app, messages) = build_app(&opts);
    let mut tui = Tui::new(app);
    if let Some(file) = &args.file {
        if let Err(e) = tui.app_mut().open(file) {
            let msg = format!("Could not open {}: {e}", file.display());
            tui.app_mut().announce(&msg, Priority::Assertive);
        }
    } else {
        tui.app_mut().announce(
            "No document is open. Press Control O to open one, or F1 for help.",
            Priority::Polite,
        );
    }
    for m in messages {
        tui.app_mut().announce(&m, Priority::Assertive);
    }
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut tui);
    ratatui::restore();
    tui.app_mut().shutdown();
    result
}
