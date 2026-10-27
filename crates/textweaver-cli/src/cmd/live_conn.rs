//! What `tw send` and `tw share` share: the options that say how to reach
//! a broker, and the password prompt, which `tw broker account add` uses
//! too. Passwords are never taken on the command line: they are asked
//! without echo, read from `TEXTWEAVER_MQTT_PASSWORD`, or derived from a
//! join code.

use std::io::{BufRead as _, IsTerminal as _, Write as _};
use std::path::PathBuf;

use anyhow::{Result, bail};
use textweaver_mqtt::{JOIN_USER, JoinCode, MqttConfig, Pin, Secret, Tls, default_port};

use crate::cmd::Plain;

/// The environment variable a script can give the password in.
pub(crate) const PASSWORD_VAR: &str = "TEXTWEAVER_MQTT_PASSWORD";

/// How to reach the broker. With none of --plain, --pin, --ca-file and
/// --join-code, TLS is checked against textweaver's bundled root
/// certificates, as for a public broker.
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
    #[arg(long, conflicts_with_all = ["pin", "ca_file", "join_code"])]
    pub plain: bool,
    /// Trust exactly the broker key with this pin, as tw broker prints it
    /// after "Certificate pin:".
    #[arg(long, value_name = "PIN", conflicts_with_all = ["ca_file", "join_code"])]
    pub pin: Option<String>,
    /// Trust the certificates in this PEM file (a private CA, or the
    /// broker's own certificate).
    #[arg(long, value_name = "FILE", conflicts_with = "join_code")]
    pub ca_file: Option<PathBuf>,
    /// Join with the code tw broker --join-code printed: four groups of
    /// four characters. It checks the broker and, without --user, logs in.
    #[arg(long, value_name = "CODE")]
    pub join_code: Option<String>,
    /// The user name to log in with. The password is asked without echo,
    /// or read from TEXTWEAVER_MQTT_PASSWORD.
    #[arg(long, value_name = "NAME")]
    pub user: Option<String>,
}

impl Conn {
    /// The client settings, asking for the password when a user name is
    /// given and the environment holds none.
    pub(crate) fn config(&self) -> Result<MqttConfig> {
        let join = match &self.join_code {
            Some(text) => Some(JoinCode::parse(text).map_err(Plain)?),
            None => None,
        };
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
        let port = self.port.unwrap_or_else(|| default_port(&tls));
        let mut config = MqttConfig::new(self.host.trim(), port, tls);
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
