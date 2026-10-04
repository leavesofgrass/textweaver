//! `tw eloquence`: which ETI-Eloquence engines this computer has, which one
//! textweaver will use, and how to get one (docs/eloquence.md).

use std::path::Path;

use serde::Serialize;
use textweaver_engines::eci::discovery;
use textweaver_engines::eci::{CODE_FACTORY_ENV, EciConfig};

/// The guide, compiled in so it is available wherever `tw` is installed.
const GUIDE: &str = include_str!("../../../../docs/eloquence.md");

/// Code Factory's default install location.
const CODE_FACTORY_DLL: &str = r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll";

/// Arguments for `tw eloquence`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Print the full guide to getting Eloquence.
    #[arg(long)]
    pub guide: bool,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// What `tw eloquence` found.
#[derive(Debug, Serialize)]
pub struct Report {
    /// True when textweaver can speak with Eloquence now.
    pub available: bool,
    /// The engine it will use, or why none can be used.
    pub status: String,
    /// Every place searched, found or not.
    pub searched: Vec<Searched>,
    /// A Code Factory installation exists but is not enabled.
    pub code_factory_not_enabled: bool,
    /// What to do next on this platform.
    pub next_steps: Vec<String>,
}

/// One searched location.
#[derive(Debug, Serialize)]
pub struct Searched {
    /// The file.
    pub path: String,
    /// The product expected there.
    pub product: String,
    /// Whether it exists.
    pub found: bool,
}

/// Builds the report for this computer.
pub fn report() -> Report {
    let d = discovery::diagnose(&EciConfig::default());
    let available = d.library.is_ok() && !d.hosts.is_empty();
    let status = match (&d.library, d.hosts.is_empty()) {
        (Ok(c), false) => format!("Eloquence is available: {}.", c.reason),
        (Ok(c), true) => format!(
            "Found {}, but the helper program that runs it is missing; reinstall textweaver (developers: cargo xtask hosts).",
            c.reason
        ),
        (Err(e), _) => format!("No Eloquence engine found ({e})."),
    };
    let code_factory_not_enabled = cfg!(windows)
        && Path::new(CODE_FACTORY_DLL).is_file()
        && std::env::var(CODE_FACTORY_ENV).map_or(true, |v| v != "1");
    let searched = d
        .candidates
        .iter()
        .map(|c| Searched {
            path: c.path.display().to_string(),
            product: c.product.name().to_owned(),
            found: c.exists,
        })
        .collect();
    let mut next_steps = Vec::new();
    if code_factory_not_enabled {
        next_steps.push(format!(
            "Code Factory's Eloquence for Windows is installed but not enabled. If you have bought it, run: setx {CODE_FACTORY_ENV} 1, then open a new terminal."
        ));
    }
    if !available {
        let step = if cfg!(target_os = "macos") {
            "On a Mac, Eloquence is built in: use the Apple speech backend with --voice Reed."
        } else if cfg!(windows) {
            "On Windows, buy Code Factory's Eloquence for Windows (codefactoryglobal.com), then enable it as above."
        } else {
            "On Linux, buy Voxin from voxin.oralux.net and run its installer, or set TEXTWEAVER_ECI_LIBRARY to your libibmeci.so."
        };
        next_steps.push(step.to_owned());
        next_steps.push("Run tw eloquence --guide for every option.".to_owned());
    } else {
        next_steps.push("Try it: tw speak --voice Reed \"Eloquence is working.\"".to_owned());
    }
    Report {
        available,
        status,
        searched,
        code_factory_not_enabled,
        next_steps,
    }
}

/// Runs `tw eloquence`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if args.guide {
        crate::cmd::out!("{GUIDE}");
        return Ok(());
    }
    let r = report();
    if args.json {
        crate::cmd::outln!("{}", serde_json::to_string_pretty(&r)?);
        return Ok(());
    }
    crate::cmd::outln!("{}", r.status);
    crate::cmd::outln!("Searched:");
    for s in &r.searched {
        let state = if s.found { "found" } else { "not found" };
        crate::cmd::outln!("  {} ({}): {state}", s.path, s.product);
    }
    for step in &r.next_steps {
        crate::cmd::outln!("{step}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_is_compiled_in() {
        assert!(GUIDE.starts_with("# Getting ETI-Eloquence"));
    }

    #[test]
    fn report_always_has_a_next_step() {
        let r = report();
        assert!(!r.status.is_empty());
        assert!(!r.next_steps.is_empty());
    }
}
