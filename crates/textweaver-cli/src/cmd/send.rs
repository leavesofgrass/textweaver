//! `tw send` and `tw share`: the sending side of live documents, through
//! textweaver_mqtt's small publisher (QoS 1, acknowledged).
//!
//! - `tw send --topic T --field name=value ...` publishes one entry, a
//!   JSON object with the fields in the order given; with no `--field`,
//!   the entry is a JSON object read from standard input. Not retained
//!   unless `--retain`.
//! - `tw share FILE --topic T` publishes the document as one retained live
//!   document message, then again each time the file is saved, until
//!   Ctrl+C (or once, with `--once`). The message is
//!   `{"type": "textweaver.document", "rev": N, "title": "...", "format":
//!   "markdown", "text": "..."}`; `docs/live-broker.md` describes it.
//!
//! Connection options are shared ([`crate::cmd::live_conn::Conn`]); the
//! password is never on the command line.

use std::io::{IsTerminal as _, Read as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result, bail};
use textweaver_mqtt::{MqttConfig, Publisher, document_message, format_for_name, publish_once};

use crate::cmd::live_conn::{Conn, on_ctrl_c};
use crate::cmd::{Plain, outln};

/// The largest message: the receiver's own cap.
const MAX_MESSAGE: usize = 256 * 1024;

/// Arguments for `tw send`.
#[derive(clap::Args, Debug)]
pub struct SendArgs {
    /// The topic to publish to, such as netlog/checkins (no + or #).
    #[arg(long, value_name = "TOPIC")]
    pub topic: String,
    /// A field of the entry, as name=value, such as callsign=N0CALL.
    /// Repeat for more; they are sent in this order. With none, a JSON
    /// object is read from standard input.
    #[arg(long, value_name = "NAME=VALUE")]
    pub field: Vec<String>,
    /// Ask the broker to keep the entry for readers who connect later.
    #[arg(long)]
    pub retain: bool,
    /// How to reach the broker.
    #[command(flatten)]
    pub conn: Conn,
}

/// Arguments for `tw share`.
#[derive(clap::Args, Debug)]
pub struct ShareArgs {
    /// The document to share: Markdown or plain text.
    pub file: PathBuf,
    /// The topic to publish to, such as docs/agenda (no + or #).
    #[arg(long, value_name = "TOPIC")]
    pub topic: String,
    /// The title readers see. Default: the file's name without its
    /// extension.
    #[arg(long, value_name = "TEXT")]
    pub title: Option<String>,
    /// Publish once and stop, instead of again on each save.
    #[arg(long)]
    pub once: bool,
    /// How often to look for a save, in seconds.
    #[arg(long, value_name = "SECONDS", default_value_t = 1)]
    pub every: u64,
    /// How to reach the broker.
    #[command(flatten)]
    pub conn: Conn,
}

/// Runs `tw send`.
pub fn send(args: SendArgs) -> Result<()> {
    let payload = if args.field.is_empty() {
        if std::io::stdin().is_terminal() {
            bail!(Plain(
                "No entry to send. Give fields with --field NAME=VALUE, or pipe a JSON object in."
                    .into()
            ));
        }
        entry_from_stdin()?
    } else {
        entry_from_fields(&args.field)?
    };
    let config = args.conn.config()?;
    publish_once(&config, &args.topic, payload.as_bytes(), args.retain)
        .map_err(|e| Plain(format!("Not sent: {e}")))?;
    outln!(
        "Sent: one entry to {} on {}{}.",
        args.topic,
        config.target(),
        if args.retain { ", retained" } else { "" }
    );
    Ok(())
}

/// The fields as a JSON object, in the order given.
fn entry_from_fields(fields: &[String]) -> Result<String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut parts = Vec::new();
    for f in fields {
        let Some((name, value)) = f.split_once('=') else {
            bail!(Plain(format!(
                "The field {f} has no equals sign. Give it as name=value, such as callsign=N0CALL."
            )));
        };
        let name = name.trim();
        if name.is_empty() {
            bail!(Plain(format!(
                "The field {f} has no name before the equals sign."
            )));
        }
        if seen.contains(&name) {
            bail!(Plain(format!("The field {name} is given twice.")));
        }
        seen.push(name);
        parts.push(format!(
            "{}:{}",
            serde_json::to_string(name)?,
            serde_json::to_string(value)?
        ));
    }
    Ok(format!("{{{}}}", parts.join(",")))
}

/// A JSON object from standard input, sent as it was written.
fn entry_from_stdin() -> Result<String> {
    let mut text = String::new();
    std::io::stdin()
        .lock()
        .take(MAX_MESSAGE as u64 + 1)
        .read_to_string(&mut text)
        .context("Could not read the entry from standard input")?;
    if text.len() > MAX_MESSAGE {
        bail!(Plain(format!(
            "The entry is larger than {MAX_MESSAGE} bytes, the limit."
        )));
    }
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| Plain(format!("The entry is not JSON: {e}.")))?;
    if !value.is_object() {
        bail!(Plain(
            "The entry must be one JSON object, such as {\"callsign\": \"N0CALL\"}.".into()
        ));
    }
    Ok(text.trim().to_string())
}

/// "markdown" for .md and .markdown files, "text" for the rest.
fn format_of(path: &Path) -> &'static str {
    format_for_name(
        &path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
    )
}

/// The file's size and time, to notice a save.
fn stamp(path: &Path) -> Option<(u64, SystemTime)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()?))
}

/// Reads the file and builds its message.
fn message(path: &Path, rev: u64, title: &str) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
    let text = String::from_utf8(bytes).map_err(|_| {
        Plain(format!(
            "{} is not UTF-8 text. Share Markdown or plain text files.",
            path.display()
        ))
    })?;
    let msg = document_message(rev, title, format_of(path), &text);
    if msg.len() > MAX_MESSAGE {
        bail!(Plain(format!(
            "{} is too large to share: the message would be {} bytes; the limit is {MAX_MESSAGE}.",
            path.display(),
            msg.len()
        )));
    }
    Ok(msg)
}

fn publish(config: &MqttConfig, topic: &str, msg: &str) -> Result<(), String> {
    let mut p = Publisher::connect(config).map_err(|e| e.to_string())?;
    p.publish(topic, msg.as_bytes(), true)
        .map_err(|e| e.to_string())?;
    p.disconnect();
    Ok(())
}

/// Runs `tw share`.
pub fn share(args: ShareArgs) -> Result<()> {
    let path = args.file.clone();
    let title = args.title.clone().unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    });
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let config = args.conn.config()?;
    let mut rev = 1;
    let msg = message(&path, rev, &title)?;
    publish(&config, &args.topic, &msg).map_err(|e| Plain(format!("Not shared: {e}")))?;
    outln!(
        "Shared: revision {rev} of {name}, {} bytes, on {}.",
        msg.len(),
        args.topic
    );
    if args.once {
        return Ok(());
    }
    let stop = on_ctrl_c()?;
    outln!("Watching: each save is shared. Press Ctrl+C to stop.");
    let every = Duration::from_secs(args.every.max(1));
    let mut last = stamp(&path);
    loop {
        if stop.recv_timeout(every).is_ok() {
            outln!("Stopped sharing. Readers keep revision {rev}.");
            return Ok(());
        }
        let now = stamp(&path);
        if now == last || now.is_none() {
            continue;
        }
        last = now;
        let next = rev + 1;
        let msg = match message(&path, next, &title) {
            Ok(m) => m,
            Err(e) => {
                let why = e.to_string();
                outln!(
                    "Not shared: {}. Trying again at the next save.",
                    why.trim_end_matches('.')
                );
                continue;
            }
        };
        match publish(&config, &args.topic, &msg) {
            Ok(()) => {
                rev = next;
                outln!("Shared: revision {rev}, {} bytes.", msg.len());
            }
            Err(e) => {
                outln!(
                    "Not shared: {}. Trying again at the next save.",
                    e.trim_end_matches('.')
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_keep_their_order_and_escape() {
        let e = entry_from_fields(&[
            "callsign=N0CALL".into(),
            "name=Ada Example".into(),
            "note=a \"quote\" = sign".into(),
        ])
        .unwrap();
        assert_eq!(
            e,
            r#"{"callsign":"N0CALL","name":"Ada Example","note":"a \"quote\" = sign"}"#
        );
        assert!(entry_from_fields(&["nothing".into()]).is_err());
        assert!(entry_from_fields(&["=x".into()]).is_err());
        assert!(entry_from_fields(&["a=1".into(), "a=2".into()]).is_err());
    }

    #[test]
    fn the_document_message_is_typed_and_escaped() {
        let m = document_message(3, "Agenda", "markdown", "# Net\n\"hi\"");
        assert_eq!(
            m,
            r##"{"type":"textweaver.document","rev":3,"title":"Agenda","format":"markdown","text":"# Net\n\"hi\""}"##
        );
        let v: serde_json::Value = serde_json::from_str(&m).unwrap();
        assert_eq!(v["rev"], 3);
        assert_eq!(format_of(Path::new("a.MD")), "markdown");
        assert_eq!(format_of(Path::new("a.txt")), "text");
    }
}
