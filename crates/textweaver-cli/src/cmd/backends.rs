//! `tw backends`. Owner: Agent B.
//!
//! Lists every speech backend compiled into this build, highest priority
//! first, with availability and which one automatic selection picks. Each
//! backend is one sentence, so the list reads well aloud.

use serde::Serialize;
use textweaver_app::speech::{BackendInfo, BackendRegistry};

/// Arguments for `tw backends`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// What `tw backends --json` prints.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The backend automatic selection chooses.
    pub auto: &'static str,
    /// Every backend, highest priority first.
    pub backends: Vec<BackendInfo>,
}

/// Builds the report.
pub fn report(registry: &BackendRegistry) -> Report {
    Report {
        auto: registry.select(None).backend.id,
        backends: registry.list(),
    }
}

/// One sentence describing a backend.
pub fn describe(b: &BackendInfo, auto: &str) -> String {
    let mut s = format!(
        "{}: {}. {}.",
        b.id,
        b.name,
        if b.available {
            "Available"
        } else {
            "Not available"
        }
    );
    if b.priority != i32::MIN {
        s.push_str(&format!(" Priority {}.", b.priority));
    } else {
        s.push_str(" Last resort.");
    }
    if b.opt_in {
        s.push_str(" Only when chosen by name.");
    }
    if b.id == auto {
        s.push_str(" Chosen automatically.");
    }
    s
}

/// Runs `tw backends`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let r = report(&BackendRegistry::with_builtins());
    if args.json {
        println!("{}", serde_json::to_string_pretty(&r)?);
        return Ok(());
    }
    for b in &r.backends {
        println!("{}", describe(b, r.auto));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_listed_with_a_choice() {
        let r = report(&BackendRegistry::with_builtins());
        assert!(r.backends.iter().any(|b| b.id == "null"));
        let null = r.backends.iter().find(|b| b.id == "null").unwrap();
        let line = describe(null, "null");
        assert_eq!(
            line,
            "null: Silent (no audio). Available. Last resort. Chosen automatically."
        );
        let rec = r.backends.iter().find(|b| b.id == "recording").unwrap();
        assert!(describe(rec, r.auto).contains("Only when chosen by name."));
    }
}
