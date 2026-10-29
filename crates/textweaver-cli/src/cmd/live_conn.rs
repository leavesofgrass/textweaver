//! What `tw publish` and its older names share: the options that say how to reach
//! a broker, and the password prompt, which `tw broker account add` uses
//! too. Passwords are never taken on the command line: they are asked
//! without echo, read from `TEXTWEAVER_MQTT_PASSWORD`, or derived from a
//! join code.

use std::io::{BufRead as _, IsTerminal as _, Write as _};
use std::path::PathBuf;

use anyhow::{Result, bail};
use textweaver_mqtt::{JOIN_USER, JoinCode, JoinLink, MqttConfig, Pin, Secret, Tls, default_port};

use crate::cmd::Plain;

/// The environment variable a script can give the password in.
pub(crate) const PASSWORD_VAR: &str = "TEXTWEAVER_MQTT_PASSWORD";

/// How to reach the broker. With none of --plain, --pin, --ca-file,
/// --join-code and --join-link, TLS is checked against textweaver's
/// bundled root certificates, as for a public broker.
#[derive(clap::Args, Debug, Clone)]
pub struct Conn {
    /// The broker's host name or address.
    #[arg(long, value_name = "HOST", default_value = "127.0.0.1")]
    pub host: String,
    /// The broker's port: 8883 with TLS, 1883 with --plain, unless given.
    #[arg(long, value_name = "N")]
    pub port: Option<u16>,
    /// Plain TCP without TLS: only for a broker on this computer or across
    /// Tailscale.
    #[arg(long, conflicts_with_all = ["pin", "ca_file", "join_code", "join_link"])]
    pub plain: bool,
    /// Trust exactly the broker key with this pin, as tw broker prints it
    /// after "Certificate pin:".
    #[arg(long, value_name = "PIN", conflicts_with_all = ["ca_file", "join_code", "join_link"])]
    pub pin: Option<String>,
    /// Trust the certificates in this PEM file (a private CA, or the
    /// broker's own certificate).
    #[arg(long, value_name = "FILE", conflicts_with_all = ["join_code", "join_link"])]
    pub ca_file: Option<PathBuf>,
    /// Join with the code tw broker --join-code printed: four groups of
    /// four characters. It checks the broker and, without --user, logs in.
    #[arg(long, value_name = "CODE")]
    pub join_code: Option<String>,
    /// Join with the link tw broker --join-code printed after "Join
    /// link:" (twjoin://HOST:PORT/CODE): the host, port and code in one.
    #[arg(long, value_name = "LINK", conflicts_with_all = ["host", "port", "join_code"])]
    pub join_link: Option<String>,
    /// The user name to log in with. The password is asked without echo,
    /// or read from TEXTWEAVER_MQTT_PASSWORD.
    #[arg(long, value_name = "NAME")]
    pub user: Option<String>,
}

impl Conn {
    /// The client settings, asking for the password when a user name is
    /// given and the environment holds none.
    pub(crate) fn config(&self) -> Result<MqttConfig> {
        let link = self.link()?;
        let join = match (&self.join_code, &link) {
            (Some(text), _) => Some(JoinCode::parse(text).map_err(Plain)?),
            (None, Some(l)) => Some(l.code().clone()),
            (None, None) => None,
        };
        let host = link.as_ref().map_or(self.host.trim(), JoinLink::host);
        let tls = if self.plain {
            Tls::Off
        } else if let Some(pin) = &self.pin {
            Tls::Pinned(Pin::parse(pin).map_err(Plain)?)
        } else if let Some(ca) = &self.ca_file {
            Tls::CaFile(ca.clone())
        } else if let Some(code) = &join {
            Tls::Pinned(code.pin())
        } else {
            Tls::System
        };
        let port = link
            .as_ref()
            .map(JoinLink::port)
            .or(self.port)
            .unwrap_or_else(|| default_port(&tls));
        let mut config = MqttConfig::new(host, port, tls);
        match (&self.user, &join) {
            (Some(user), _) => {
                config.username = Some(user.clone());
                config.password = Some(password_for(user)?);
            }
            (None, Some(code)) => {
                config.username = Some(JOIN_USER.into());
                config.password = Some(code.password());
            }
            (None, None) => {}
        }
        Ok(config)
    }
}

impl Conn {
    /// The `--join-link` given, read.
    fn link(&self) -> Result<Option<JoinLink>> {
        match &self.join_link {
            Some(text) => Ok(Some(JoinLink::parse(text).map_err(Plain)?)),
            None => Ok(None),
        }
    }

    /// The join link to hand readers, following `topic`: with a join code
    /// or a join link given, `None` otherwise.
    pub(crate) fn reader_link(&self, topic: &str) -> Result<Option<JoinLink>> {
        let link = self.link()?;
        let (host, port, code) = match (&self.join_code, link) {
            (Some(text), _) => {
                let code = JoinCode::parse(text).map_err(Plain)?;
                let port = self.port.unwrap_or(8883);
                (self.host.trim().to_owned(), port, code)
            }
            (None, Some(l)) => (l.host().to_owned(), l.port(), l.code().clone()),
            (None, None) => return Ok(None),
        };
        Ok(Some(
            JoinLink::new(&host, port, code, Some(topic)).map_err(Plain)?,
        ))
    }
}

/// The password for `user`: from [`PASSWORD_VAR`], else asked without
/// echo on the terminal.
fn password_for(user: &str) -> Result<Secret> {
    if let Some(p) = std::env::var_os(PASSWORD_VAR) {
        let p = p.to_string_lossy().into_owned();
        if !p.is_empty() {
            return Ok(Secret::new(p));
        }
    }
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        bail!(Plain(format!(
            "No password for {user}. Set {PASSWORD_VAR}, or run this in a terminal to be asked."
        )));
    }
    Ok(Secret::new(ask_secret(&format!(
        "Password for {user}, not shown as you type: "
    ))?))
}

/// Reads a secret. On a terminal, the prompt goes to standard error and
/// the keys typed are not shown or spoken: Backspace takes one back,
/// Enter ends, Escape or Ctrl+C cancels. Piped, it reads one line from
/// standard input.
pub(crate) fn ask_secret(prompt: &str) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        return Ok(line.trim_end_matches(['\r', '\n']).to_string());
    }
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use ratatui::crossterm::terminal;
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    terminal::enable_raw_mode()?;
    let mut text = String::new();
    let outcome = loop {
        let ev = match event::read() {
            Ok(ev) => ev,
            Err(e) => break Err(anyhow::Error::new(e)),
        };
        let Event::Key(key) = ev else { continue };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Enter => break Ok(()),
            KeyCode::Esc => {
                break Err(anyhow::Error::new(Plain(
                    "Canceled: nothing was changed.".into(),
                )));
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                break Err(anyhow::Error::new(Plain(
                    "Canceled: nothing was changed.".into(),
                )));
            }
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Char(c) => text.push(c),
            _ => {}
        }
    };
    let _ = terminal::disable_raw_mode();
    eprintln!();
    outcome.map(|()| text)
}

/// A channel that receives once per Ctrl+C (or the terminal closing), for
/// the commands that run until stopped. Set up once per process.
pub(crate) fn on_ctrl_c() -> Result<std::sync::mpsc::Receiver<()>> {
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .map_err(|e| anyhow::anyhow!("Could not watch for Ctrl+C: {e}"))?;
    Ok(rx)
}
