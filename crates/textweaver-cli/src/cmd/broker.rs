//! `tw broker`: textweaver's own MQTT broker for live documents
//! (textweaver_broker), in the foreground until Ctrl+C, and its accounts.
//!
//! - `tw broker` listens on 127.0.0.1 with TLS (port 8883) unless
//!   `--listen` names addresses; `--plain` turns TLS off (port 1883) and
//!   the broker says in words whether each address is loopback, Tailscale
//!   or something else. `--join-code` makes a session key and a join code
//!   whose holders may read the `--join-topic` filters, and prints the
//!   join link (`Join link: twjoin://...`) to copy to readers.
//! - `tw broker account add|remove|list` change `broker.toml` in the data
//!   folder. The password is asked without echo (twice), or read as one
//!   line from standard input when it is piped; never on the command line.
//! - `tw broker pin` prints the certificate pin readers check, making the
//!   key and certificate if there are none yet.
//!
//! Status lines are words, meaning first, one per line, with no spinner.
//! The user guide is `docs/live-broker.md`.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Result, bail};
use textweaver_broker::{
    ACCOUNTS_FILE, Account, Accounts, Broker, BrokerConfig, PLAIN_PORT, Role, TLS_PORT, stable_pin,
};

use crate::cmd::live_conn::{ask_secret, on_ctrl_c};
use crate::cmd::{Plain, outln};

/// Arguments for `tw broker`.
#[derive(clap::Args, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Args {
    /// Account and pin commands; with none, the broker runs.
    #[command(subcommand)]
    pub command: Option<Command>,
    /// How to run the broker.
    #[command(flatten)]
    pub run: RunArgs,
}

/// `tw broker` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Add, remove or list the broker's accounts (broker.toml in the data folder).
    Account(AccountArgs),
    /// Print the certificate pin readers check, making the key and certificate if needed.
    Pin(PinArgs),
}

/// How to run the broker.
#[derive(clap::Args, Debug)]
pub struct RunArgs {
    /// An address to listen on, with or without a port, such as 0.0.0.0,
    /// 100.101.1.2 or [::1]:8883. Repeat for more. Default: 127.0.0.1.
    #[arg(long, value_name = "ADDR")]
    pub listen: Vec<String>,
    /// The port for addresses given without one: 8883 with TLS, 1883 with
    /// --plain, unless given.
    #[arg(long, value_name = "N")]
    pub port: Option<u16>,
    /// Plain TCP without TLS: only on this computer or across Tailscale.
    #[arg(long, conflicts_with = "join_code")]
    pub plain: bool,
    /// Make a join code for this session, with its own key: anyone with
    /// the code may read the --join-topic filters, and nothing else.
    #[arg(long)]
    pub join_code: bool,
    /// A topic filter a join code may read. Repeat for more. Default: #.
    #[arg(long, value_name = "FILTER", requires = "join_code")]
    pub join_topic: Vec<String>,
    /// Use this folder for textweaver's data (broker.toml and the key)
    /// instead of the usual one.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw broker account`.
#[derive(clap::Args, Debug)]
pub struct AccountArgs {
    /// What to do.
    #[command(subcommand)]
    pub command: AccountCommand,
}

/// `tw broker account` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum AccountCommand {
    /// Add an account, or change one with --replace. The password is asked without echo.
    Add(AddArgs),
    /// Remove an account.
    Remove(RemoveArgs),
    /// List the accounts: name, role and topics. Passwords are never shown.
    List(ListArgs),
}

/// Arguments for `tw broker account add`.
#[derive(clap::Args, Debug)]
pub struct AddArgs {
    /// The user name: letters, digits, dot, dash and underscore.
    pub name: String,
    /// reader (may only read) or logger (may also publish).
    #[arg(long, value_name = "ROLE")]
    pub role: String,
    /// A topic filter the account may read. Repeat for more. Default: #.
    #[arg(long, value_name = "FILTER")]
    pub read: Vec<String>,
    /// A topic filter a logger may publish to. Repeat for more. A logger
    /// needs at least one.
    #[arg(long, value_name = "FILTER")]
    pub publish: Vec<String>,
    /// Change the account if it exists.
    #[arg(long)]
    pub replace: bool,
    /// Use this folder for textweaver's data instead of the usual one.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw broker account remove`.
#[derive(clap::Args, Debug)]
pub struct RemoveArgs {
    /// The user name.
    pub name: String,
    /// Use this folder for textweaver's data instead of the usual one.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw broker account list`.
#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Print JSON: one object per account, without passwords.
    #[arg(long)]
    pub json: bool,
    /// Use this folder for textweaver's data instead of the usual one.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Arguments for `tw broker pin`.
#[derive(clap::Args, Debug)]
pub struct PinArgs {
    /// Print JSON: {"pin": "...", "made": true or false}.
    #[arg(long)]
    pub json: bool,
    /// Use this folder for textweaver's data instead of the usual one.
    #[arg(long, value_name = "DIR")]
    pub home: Option<PathBuf>,
}

/// Runs `tw broker`.
pub fn run(args: Args) -> Result<()> {
    match args.command {
        None => serve(args.run),
        Some(Command::Pin(a)) => pin(a),
        Some(Command::Account(a)) => match a.command {
            AccountCommand::Add(a) => add(a),
            AccountCommand::Remove(a) => remove(a),
            AccountCommand::List(a) => list(a),
        },
    }
}

fn data_dir(home: Option<&std::path::Path>) -> Result<PathBuf> {
    Ok(crate::cmd::paths(home)?.data_dir)
}

/// Reads `--listen` values: an address with a port, or an address alone.
fn listen_addrs(values: &[String], port: u16) -> Result<Vec<SocketAddr>> {
    let mut out = Vec::new();
    for v in values {
        let v = v.trim();
        if let Ok(a) = v.parse::<SocketAddr>() {
            out.push(a);
        } else if let Ok(ip) = v
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
        {
            out.push(SocketAddr::new(ip, port));
        } else {
            bail!(Plain(format!(
                "{v} is not an address to listen on. Give an IP address, such as 0.0.0.0 or 100.101.1.2, with or without a port."
            )));
        }
    }
    Ok(out)
}

fn serve(args: RunArgs) -> Result<()> {
    let dir = data_dir(args.home.as_deref())?;
    let accounts = Accounts::load(&dir.join(ACCOUNTS_FILE)).map_err(Plain)?;
    if accounts.is_empty() && !args.join_code {
        bail!(Plain(
            "No accounts and no join code, so nobody could log in. Add an account with tw broker account add, or start with --join-code.".into()
        ));
    }
    let tls = !args.plain;
    let port = args.port.unwrap_or(if tls { TLS_PORT } else { PLAIN_PORT });
    let mut config = BrokerConfig::new(&dir);
    config.tls = tls;
    config.listen = if args.listen.is_empty() {
        vec![SocketAddr::from(([127, 0, 0, 1], port))]
    } else {
        listen_addrs(&args.listen, port)?
    };
    config.join_code = args.join_code;
    if !args.join_topic.is_empty() {
        config.join_topics = args.join_topic.clone();
    }
    let join_topics = config.join_topics.join(", ");
    let count = accounts.list().len();
    config.accounts = accounts;
    let stop = on_ctrl_c()?;
    outln!(
        "Starting: textweaver's broker, {count} account{}.",
        if count == 1 { "" } else { "s" }
    );
    let broker = Broker::start(config, |event| {
        crate::cmd::write_out(format_args!("{event}\n"))
    })
    .map_err(Plain)?;
    if let Some(pin) = broker.pin() {
        if broker.made_new_identity() {
            outln!(
                "New key: a key and certificate were made in {}.",
                dir.display()
            );
        }
        outln!("Certificate pin: {}", pin.grouped());
    }
    if let Some(code) = broker.join_code() {
        outln!("Join code: {}", code.grouped());
        if let Some(link) = join_link(broker.local_addrs(), code) {
            outln!("Join link: {link}");
        }
        outln!("The join code may read: {join_topics}.");
    }
    outln!("Running: press Ctrl+C to stop.");
    let _ = stop.recv();
    broker.stop();
    Ok(())
}

/// The join link for a broker on `addrs`: the first address that is not
/// loopback (else the first), as readers reach it.
fn join_link(
    addrs: &[SocketAddr],
    code: &textweaver_mqtt::JoinCode,
) -> Option<textweaver_mqtt::JoinLink> {
    let addr = addrs
        .iter()
        .find(|a| !a.ip().is_loopback())
        .or_else(|| addrs.first())?;
    let host = textweaver_mqtt::JoinLink::host_for(addr.ip());
    textweaver_mqtt::JoinLink::new(&host, addr.port(), code.clone(), None).ok()
}

fn pin(args: PinArgs) -> Result<()> {
    let dir = data_dir(args.home.as_deref())?;
    let (pin, made) = stable_pin(&dir).map_err(Plain)?;
    if args.json {
        outln!(
            "{}",
            serde_json::json!({ "pin": pin.grouped(), "made": made })
        );
    } else {
        if made {
            outln!(
                "New key: a key and certificate were made in {}.",
                dir.display()
            );
        }
        outln!("Certificate pin: {}", pin.grouped());
    }
    Ok(())
}

fn add(args: AddArgs) -> Result<()> {
    let role = Role::parse(&args.role).ok_or_else(|| {
        Plain(format!(
            "{} is not a role. Give --role reader or --role logger.",
            args.role.trim()
        ))
    })?;
    if role == Role::Reader && !args.publish.is_empty() {
        bail!(Plain(
            "A reader may not publish. Leave out --publish, or give --role logger.".into()
        ));
    }
    if role == Role::Logger && args.publish.is_empty() {
        bail!(Plain(
            "A logger needs at least one --publish topic, such as --publish netlog/#.".into()
        ));
    }
    if let Some(why) = textweaver_broker::user_name_problem(&args.name) {
        bail!(Plain(format!("Account error: {why}.")));
    }
    let read = if args.read.is_empty() {
        vec!["#".to_string()]
    } else {
        args.read.clone()
    };
    let dir = data_dir(args.home.as_deref())?;
    let path = dir.join(ACCOUNTS_FILE);
    let mut accounts = Accounts::load(&path).map_err(Plain)?;
    if accounts.get(&args.name).is_some() && !args.replace {
        bail!(Plain(format!(
            "Account error: {} already exists. Give --replace to change it.",
            args.name
        )));
    }
    let password = read_new_password(&args.name)?;
    let account =
        Account::new(&args.name, role, read, args.publish.clone(), &password).map_err(Plain)?;
    let replaced = accounts.add(account, args.replace).map_err(Plain)?;
    accounts.save(&path).map_err(Plain)?;
    let a = accounts.get(&args.name).map(describe).unwrap_or_default();
    outln!("{}: {a}", if replaced { "Changed" } else { "Added" });
    Ok(())
}

/// Asks for a new password twice on a terminal, or reads one line when
/// standard input is piped.
fn read_new_password(name: &str) -> Result<String> {
    use std::io::IsTerminal as _;
    let first = ask_secret(&format!("New password for {name}, not shown as you type: "))?;
    if first.is_empty() {
        bail!(Plain("The password is empty. Nothing was changed.".into()));
    }
    if std::io::stdin().is_terminal() {
        let again = ask_secret("The same password again: ")?;
        if again != first {
            bail!(Plain(
                "The two passwords differ. Nothing was changed.".into()
            ));
        }
    }
    Ok(first)
}

fn remove(args: RemoveArgs) -> Result<()> {
    let dir = data_dir(args.home.as_deref())?;
    let path = dir.join(ACCOUNTS_FILE);
    let mut accounts = Accounts::load(&path).map_err(Plain)?;
    if !accounts.remove(&args.name) {
        bail!(Plain(format!(
            "No account named {}. tw broker account list shows them.",
            args.name
        )));
    }
    accounts.save(&path).map_err(Plain)?;
    outln!("Removed: {}.", args.name);
    Ok(())
}

fn list(args: ListArgs) -> Result<()> {
    let dir = data_dir(args.home.as_deref())?;
    let accounts = Accounts::load(&dir.join(ACCOUNTS_FILE)).map_err(Plain)?;
    if args.json {
        let rows: Vec<serde_json::Value> = accounts
            .list()
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "role": a.role.name(),
                    "read": a.read,
                    "publish": a.publish,
                })
            })
            .collect();
        outln!("{}", serde_json::Value::Array(rows));
        return Ok(());
    }
    if accounts.is_empty() {
        outln!("No accounts yet. Add one with tw broker account add.");
    }
    for a in accounts.list() {
        outln!("{}", describe(a));
    }
    Ok(())
}

/// "netcontrol, logger, reads netlog/#, publishes netlog/#."
fn describe(a: &Account) -> String {
    let mut line = format!(
        "{}, {}, reads {}",
        a.name,
        a.role.name(),
        a.read.join(" and ")
    );
    if a.role == Role::Logger {
        line.push_str(&format!(", publishes {}", a.publish.join(" and ")));
    }
    line.push('.');
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listen_values() {
        let a = listen_addrs(
            &["0.0.0.0".into(), "[::1]:1884".into(), "100.101.1.2".into()],
            8883,
        )
        .unwrap();
        assert_eq!(a[0].to_string(), "0.0.0.0:8883");
        assert_eq!(a[1].to_string(), "[::1]:1884");
        assert_eq!(a[2].to_string(), "100.101.1.2:8883");
        assert!(listen_addrs(&["my-laptop".into()], 8883).is_err());
    }
}
