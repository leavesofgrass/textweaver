//! Memory attribution for the GUI (ADR-0028, W4s): the app built exactly as
//! the GUI builds it, with no window and no renderer, a document opened,
//! and optionally read with the silent paced backend. It then waits, so an
//! outside tool (PowerShell's `Get-Process`) can measure the app's share of
//! the GUI's working set and private bytes.
//!
//! It never plays audio: speech is the null backend, or `paced`, which
//! times words and outputs nothing.
//!
//! ```text
//! cargo run --release -p textweaver-xilem --example memory_probe -- \
//!     --home SCRATCH [--file PATH] [--read] [--seconds 20]
//! ```

use std::path::PathBuf;
use std::time::{Duration, Instant};

use textweaver_app::Command;
use textweaver_app::a11y::LogAnnouncer;
use textweaver_app::keymap::ActionId;
use textweaver_xilem::setup::{self, Options, PACED_BACKEND};

fn main() {
    let mut file: Option<PathBuf> = None;
    let mut home: Option<PathBuf> = None;
    let mut read = false;
    let mut seconds = 20.0_f64;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--file" => file = args.next().map(PathBuf::from),
            "--home" => home = args.next().map(PathBuf::from),
            "--read" => read = true,
            "--seconds" => {
                seconds = args.next().and_then(|s| s.parse().ok()).unwrap_or(seconds);
            }
            other => {
                eprintln!("memory_probe: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let Some(home) = home else {
        eprintln!("memory_probe: --home SCRATCH is required (never the real settings)");
        std::process::exit(2);
    };
    let opts = Options {
        no_speech: !read,
        backend: read.then(|| PACED_BACKEND.to_owned()),
        home: Some(home),
        ..Options::default()
    };
    let started = Instant::now();
    let (mut app, _) = setup::build_app(&opts, Box::new(LogAnnouncer::default()));
    println!(
        "app built in {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    if let Some(path) = &file {
        let t = Instant::now();
        match app.open(path) {
            Ok(_) => println!("opened in {:.1} ms", t.elapsed().as_secs_f64() * 1000.0),
            Err(e) => {
                eprintln!("memory_probe: cannot open {}: {e}", path.display());
                std::process::exit(1);
            }
        }
        if read {
            let _ = app.dispatch(Command::Action(ActionId::ReadFromCursor));
        }
    }
    println!("ready");
    let end = Instant::now() + Duration::from_secs_f64(seconds.max(0.0));
    while Instant::now() < end {
        let _ = app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(50));
    }
}
