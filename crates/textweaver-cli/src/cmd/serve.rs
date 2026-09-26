//! `tw serve`. Owner: Agent D.
//!
//! Serves the app over JSON-RPC 2.0 on stdin and stdout
//! (`textweaver_app::rpc`, ADR-0015): one JSON message per line, or
//! `Content-Length` framed. Speech is self-voiced like the terminal reader
//! unless `--no-speech` is given; every announcement is also sent to the
//! client as an `announcement` notification. Nothing but protocol messages
//! is written to stdout; startup messages go to the client as
//! announcements.

use std::io::BufReader;
use std::path::PathBuf;

use textweaver_app::a11y::Priority;
use textweaver_app::rpc;

/// Arguments for `tw serve`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Serve JSON-RPC over stdin and stdout.
    #[arg(long)]
    pub stdio: bool,
    /// Do not speak; the client gets announcements and positions only.
    #[arg(long)]
    pub no_speech: bool,
    /// Speech backend id (see `tw backends`).
    #[arg(long)]
    pub backend: Option<String>,
    /// Keep settings and reading positions under this directory.
    #[arg(long)]
    pub home: Option<PathBuf>,
    /// Open this document before serving.
    #[arg(long)]
    pub open: Option<PathBuf>,
}

/// Runs `tw serve`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if !args.stdio {
        anyhow::bail!("`tw serve` needs a transport; use --stdio");
    }
    let opts = textweaver_tui::Options {
        no_speech: args.no_speech,
        // The client decides what to do with announcements; reading and
        // announcing stay as they were (self-voicing, or silent with
        // --no-speech), whatever mode the terminal reader uses.
        mode: Some(textweaver_tui::AccessMode::SelfVoicing),
        backend: args.backend,
        home: args.home,
        theme: None,
        log: None,
    };
    // The log file, at the level TEXTWEAVER_LOG asks for (stdout is the
    // protocol, so nothing is printed).
    let log_message = textweaver_tui::setup::start_log(&opts);
    let (announcer, queue) = rpc::announcer();
    let (mut app, mut messages) = textweaver_tui::build_app_with(&opts, announcer);
    messages.extend(log_message);
    for m in messages {
        app.announce(&m, Priority::Polite);
    }
    if let Some(file) = &args.open
        && let Err(e) = app.open(file)
    {
        app.announce(
            &format!("Could not open {}: {e}", file.display()),
            Priority::Assertive,
        );
    }
    let server = rpc::Server::new(app, queue);
    let stdin = BufReader::new(std::io::stdin());
    rpc::serve(server, stdin, std::io::stdout().lock())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_a_transport() {
        let args = Args {
            stdio: false,
            no_speech: true,
            backend: None,
            home: None,
            open: None,
        };
        assert!(run(args).unwrap_err().to_string().contains("--stdio"));
    }
}
