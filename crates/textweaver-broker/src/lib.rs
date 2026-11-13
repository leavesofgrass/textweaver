//! textweaver's own small MQTT 3.1.1 broker, for live documents and net
//! logs: `tw broker` runs it headless, and the app can run the same broker
//! inside itself.
//!
//! - **MQTT 3.1.1**, the version textweaver's receiver and Mosquitto's
//!   clients speak, on rumqttc's packet codec: QoS 0 and 1 (a QoS 2
//!   publish is taken and delivered at QoS 1), retained messages, keepalive,
//!   clean sessions, and `+` and `#` in subscriptions. MQTT 5 clients are
//!   refused with the "unacceptable protocol version" code. Last will
//!   messages are not sent.
//! - **No async runtime.** One listener thread per address and one thread
//!   per client, each reading with a short timeout so it can check the stop
//!   flag and pass on what other clients publish. [`Broker::stop`] ends
//!   every thread and frees every port before it returns.
//! - **Access.** Named accounts in `broker.toml` ([`Accounts`]): readers
//!   subscribe to their read topics and may never publish; loggers may
//!   also publish to their publish topics. A client that publishes where
//!   it may not is dropped. With a join code ([`BrokerConfig::join_code`]),
//!   anyone who has the code may read the join topics, and nothing more.
//! - **TLS** with textweaver's rustls and ring. In accounts mode the key
//!   and self-signed certificate live in the data folder, so readers' pins
//!   stay valid across restarts ([`stable_pin`]); with a join code, the
//!   session has its own key, in memory only. Plain TCP only when asked,
//!   and the broker says in words whether each address is loopback,
//!   Tailscale, or something else, warning for the last. A 100.64.0.0/10
//!   address counts as Tailscale only when this computer has a Tailscale
//!   network interface, since carriers use the same range.
//! - **Caps** ([`BrokerLimits`]): clients, message size, topic length,
//!   subscriptions per client, retained topics, and each reader's queue (a
//!   reader that falls that far behind is dropped), connections per
//!   address before and after login, and readers on the join code. Five
//!   failed logins a minute from one address lock it out for five minutes,
//!   counting logins still being checked; after 200 wrong join codes in a
//!   session, the join code turns off.
//! - **Status in words**: every [`BrokerEvent`] is one sentence, meaning
//!   first, for a status line or a Braille display.

mod accounts;
mod conn;
mod identity;
mod limiter;
mod router;

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use bytes::Bytes;
use rumqttc::mqttbytes::QoS;
use textweaver_mqtt::topic::{filter_within, matches, name_problem};
use textweaver_mqtt::{JoinCode, Pin, Secret, clean_text, is_tailscale_addr};

pub use accounts::{
    ACCOUNTS_FILE, Account, Accounts, DEFAULT_ITERATIONS, Role, name_problem as user_name_problem,
};
pub use identity::{CERT_FILE, KEY_FILE, stable_pin};

use limiter::Limiter;
use router::Router;

/// The conventional plain MQTT port.
pub const PLAIN_PORT: u16 = 1883;
/// The conventional MQTT over TLS port.
pub const TLS_PORT: u16 = 8883;

/// Caps that keep one client, or many, from using up the broker.
#[derive(Clone, Debug)]
pub struct BrokerLimits {
    /// Connections at once, including ones still logging in (200).
    pub max_clients: usize,
    /// The largest message payload, in bytes (256 KiB, as the receiver's
    /// own cap).
    pub max_payload: usize,
    /// The longest topic or filter, in bytes (1024).
    pub max_topic: usize,
    /// Subscriptions one client may hold (64).
    pub max_subscriptions: usize,
    /// Topics with a retained message (1000).
    pub max_retained: usize,
    /// Messages waiting for one reader before it is dropped as too slow
    /// (1024).
    pub queue: usize,
    /// Time for the TLS handshake and the CONNECT packet (10 s).
    pub connect_timeout: Duration,
    /// Failed logins from one address within `fail_window` that lock it
    /// out (5).
    pub max_fails: u32,
    /// The window failed logins are counted in (60 s).
    pub fail_window: Duration,
    /// How long an address stays locked out (300 s).
    pub lockout: Duration,
    /// Connections from one address still logging in (8). IPv6 addresses
    /// count by their /64.
    pub max_waiting_per_addr: usize,
    /// Logged-in connections from one address (16).
    pub max_per_addr: usize,
    /// Readers logged in with the join code at once (50).
    pub max_join: usize,
    /// Wrong join codes in one broker session before the join code turns
    /// off (200). Accounts keep working.
    pub join_budget: u32,
}

impl Default for BrokerLimits {
    fn default() -> Self {
        BrokerLimits {
            max_clients: 200,
            max_payload: 256 * 1024,
            max_topic: 1024,
            max_subscriptions: 64,
            max_retained: 1000,
            queue: 1024,
            connect_timeout: Duration::from_secs(10),
            max_fails: 5,
            fail_window: Duration::from_secs(60),
            lockout: Duration::from_secs(300),
            max_waiting_per_addr: 8,
            max_per_addr: 16,
            max_join: 50,
            join_budget: 200,
        }
    }
}

/// How to run the broker.
#[derive(Clone, Debug)]
pub struct BrokerConfig {
    /// Addresses to listen on. Port 0 picks a free port; read the result
    /// with [`Broker::local_addrs`]. Empty means `127.0.0.1` on the
    /// conventional port.
    pub listen: Vec<SocketAddr>,
    /// TLS on every address (true), or plain TCP (false).
    pub tls: bool,
    /// The folder for the accounts mode's key and certificate: textweaver's
    /// data folder.
    pub data_dir: PathBuf,
    /// The named accounts.
    pub accounts: Accounts,
    /// Make a join code for this session: a fresh key and certificate, and
    /// a code whose holders may read [`BrokerConfig::join_topics`]. Needs
    /// TLS.
    pub join_code: bool,
    /// What a join code may read (`#`: everything but `$` topics).
    pub join_topics: Vec<String>,
    /// The caps.
    pub limits: BrokerLimits,
}

impl BrokerConfig {
    /// TLS on 127.0.0.1, the conventional port, no accounts, no join code.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        BrokerConfig {
            listen: Vec::new(),
            tls: true,
            data_dir: data_dir.into(),
            accounts: Accounts::default(),
            join_code: false,
            join_topics: vec!["#".into()],
            limits: BrokerLimits::default(),
        }
    }
}

/// What kind of network an address is on, for the listening line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    /// This computer only.
    Loopback,
    /// A Tailscale address on a computer with a Tailscale interface, whose
    /// WireGuard tunnel encrypts traffic.
    Tailscale,
    /// Every network this computer is on (0.0.0.0 or ::).
    Every,
    /// Another network, such as the local Wi-Fi.
    Other,
}

impl Network {
    /// The kind of network `ip` is on. An address in Tailscale's ranges is
    /// [`Network::Tailscale`] only when [`tailscale_interface`] finds a
    /// Tailscale interface; otherwise it is [`Network::Other`], because
    /// 100.64.0.0/10 is also carrier-grade NAT.
    pub fn of(ip: IpAddr) -> Network {
        Network::classify(ip, || tailscale_interface() == Some(true))
    }

    /// [`Network::of`], with the interface check given.
    fn classify(ip: IpAddr, has_tailscale: impl FnOnce() -> bool) -> Network {
        if ip.is_loopback() {
            Network::Loopback
        } else if is_tailscale_addr(ip) && has_tailscale() {
            Network::Tailscale
        } else if ip.is_unspecified() {
            Network::Every
        } else {
            Network::Other
        }
    }
}

/// Whether this computer has a Tailscale network interface: `Some(true)`
/// or `Some(false)` where the broker can tell without a new dependency
/// (Linux, from `/sys/class/net`, where Tailscale's interface is
/// `tailscale0` or another name starting with `tailscale`), and `None`
/// where it cannot (Windows and macOS, whose interface lists need system
/// calls the broker does not make). Unknown counts as no interface.
pub fn tailscale_interface() -> Option<bool> {
    if cfg!(target_os = "linux") {
        let entries = std::fs::read_dir("/sys/class/net").ok()?;
        Some(entries.flatten().any(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("tailscale"))
        }))
    } else {
        None
    }
}

/// Something the broker did or refused. `Display` gives one sentence,
/// meaning first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrokerEvent {
    /// Listening on an address.
    Listening {
        /// The address, with the port actually bound.
        addr: SocketAddr,
        /// TLS or plain.
        tls: bool,
        /// What kind of network it is on.
        network: Network,
    },
    /// A client logged in.
    Connected {
        /// The account, or `join` for a join code.
        user: String,
        /// Where from.
        addr: SocketAddr,
    },
    /// A logged-in client left or was dropped.
    Left {
        /// The account.
        user: String,
        /// Where from.
        addr: SocketAddr,
        /// Why, in words, without a final period; empty when the client
        /// said goodbye.
        reason: String,
    },
    /// A login was refused.
    LoginRefused {
        /// Where from.
        addr: SocketAddr,
        /// The user name tried, cleaned.
        user: String,
    },
    /// An address failed too often and is locked out.
    LockedOut {
        /// The address.
        ip: IpAddr,
        /// For how long.
        minutes: u64,
    },
    /// A client published where it may not, and was dropped.
    PublishRefused {
        /// The account.
        user: String,
        /// The topic, cleaned.
        topic: String,
    },
    /// A connection was turned away because the broker is full.
    Full {
        /// The cap.
        max: usize,
    },
    /// A connection or a login was turned away because its address has as
    /// many as it may. Said once each time the address reaches the cap.
    AddressFull {
        /// The address (an IPv6 address by its /64).
        ip: IpAddr,
        /// The cap.
        max: usize,
        /// True for logged-in connections, false for ones still logging in.
        logged_in: bool,
    },
    /// A join login was turned away: as many readers have joined as may.
    JoinFull {
        /// The cap.
        max: usize,
    },
    /// Too many wrong join codes this session: the join code is off, and
    /// only accounts can log in until the broker starts again.
    JoinCodeOff {
        /// The wrong codes counted.
        wrong: u32,
    },
    /// A retained message was not kept: the store is full.
    RetainedFull {
        /// The topic, cleaned.
        topic: String,
    },
    /// A problem that does not stop the broker, in words.
    Problem(String),
    /// The broker stopped; every port is free.
    Stopped,
}

impl fmt::Display for BrokerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerEvent::Listening { addr, tls, network } => {
                let how = if *tls { "TLS" } else { "plain" };
                match (network, tls) {
                    (Network::Loopback, _) => write!(
                        f,
                        "Listening: {addr}, {how}, loopback only. Other computers cannot connect."
                    ),
                    (Network::Tailscale, true) => {
                        write!(f, "Listening: {addr}, TLS, on Tailscale.")
                    }
                    (Network::Tailscale, false) => write!(
                        f,
                        "Listening: {addr}, plain, on Tailscale, which encrypts the link."
                    ),
                    (Network::Every, true) => write!(
                        f,
                        "Listening: {addr}, TLS, on every network of this computer."
                    ),
                    (Network::Other, true) => write!(f, "Listening: {addr}, TLS."),
                    (Network::Every | Network::Other, false) if is_tailscale_addr(addr.ip()) => {
                        write!(
                            f,
                            "Warning: {addr} is plain, and no Tailscale interface was found, so the link may not be encrypted. Anyone on that network can read the messages and the passwords."
                        )
                    }
                    (Network::Every | Network::Other, false) => write!(
                        f,
                        "Warning: {addr} is plain and not on Tailscale. Anyone on that network can read the messages and the passwords."
                    ),
                }
            }
            BrokerEvent::Connected { user, addr } => {
                write!(f, "Connected: {user} from {}.", addr.ip())
            }
            BrokerEvent::Left { user, addr, reason } if reason.is_empty() => {
                write!(f, "Left: {user} from {}.", addr.ip())
            }
            BrokerEvent::Left { user, addr, reason } => {
                write!(f, "Left: {user} from {}, {reason}.", addr.ip())
            }
            BrokerEvent::LoginRefused { addr, user } => {
                if user.is_empty() {
                    write!(f, "Login refused: no user name, from {}.", addr.ip())
                } else {
                    write!(
                        f,
                        "Login refused: wrong user name or password for {user}, from {}.",
                        addr.ip()
                    )
                }
            }
            BrokerEvent::LockedOut { ip, minutes } => write!(
                f,
                "Locked out: {ip} for {minutes} minutes, after too many failed logins."
            ),
            BrokerEvent::PublishRefused { user, topic } => write!(
                f,
                "Publish refused: {user} may not publish to {topic}. Disconnected."
            ),
            BrokerEvent::Full { max } => write!(
                f,
                "Refused a connection: the broker has {max} clients, its limit."
            ),
            BrokerEvent::AddressFull {
                ip,
                max,
                logged_in: false,
            } => write!(
                f,
                "Refused connections: {ip} has {max} still logging in, its limit."
            ),
            BrokerEvent::AddressFull {
                ip,
                max,
                logged_in: true,
            } => write!(f, "Refused a login: {ip} has {max} connections, its limit."),
            BrokerEvent::JoinFull { max } => write!(
                f,
                "Refused a join login: {max} readers have joined with the code, its limit."
            ),
            BrokerEvent::JoinCodeOff { .. } => f.write_str(
                "Join code off: too many wrong codes. Start the broker again for a new code.",
            ),
            BrokerEvent::RetainedFull { topic } => write!(
                f,
                "Not kept: the retained message on {topic}, because the broker holds as many as it may."
            ),
            BrokerEvent::Problem(text) => f.write_str(text),
            BrokerEvent::Stopped => f.write_str("Stopped: the broker is closed."),
        }
    }
}

/// What one client may read and publish.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Perms {
    pub(crate) read: Vec<String>,
    pub(crate) publish: Vec<String>,
}

impl Perms {
    pub(crate) fn can_read(&self, topic: &str) -> bool {
        self.read.iter().any(|f| matches(f, topic))
    }

    pub(crate) fn can_publish(&self, topic: &str) -> bool {
        self.publish.iter().any(|f| matches(f, topic))
    }

    pub(crate) fn can_subscribe(&self, filter: &str) -> bool {
        self.read.iter().any(|f| filter_within(filter, f))
    }
}

type EventFn = Box<dyn Fn(BrokerEvent) + Send + Sync>;

/// Connections from one address still logging in, and whether the broker
/// has said the address reached its cap.
#[derive(Debug, Default)]
struct Waiting {
    count: usize,
    warned: bool,
}

/// State every thread shares.
pub(crate) struct Shared {
    pub(crate) stop: AtomicBool,
    pub(crate) limits: BrokerLimits,
    pub(crate) accounts: Accounts,
    pub(crate) join: Option<(Secret, Perms)>,
    pub(crate) tls: Option<Arc<rustls::ServerConfig>>,
    pub(crate) router: Mutex<Router>,
    pub(crate) limiter: Mutex<Limiter>,
    /// Connections still logging in, per address key.
    waiting: Mutex<HashMap<IpAddr, Waiting>>,
    /// Wrong join codes this session.
    pub(crate) join_wrong: AtomicU32,
    /// Set when the join code turned off.
    pub(crate) join_off: AtomicBool,
    pub(crate) active: AtomicUsize,
    pub(crate) next_id: AtomicU64,
    conns: Mutex<Vec<JoinHandle<()>>>,
    events: EventFn,
}

impl Shared {
    pub(crate) fn emit(&self, event: BrokerEvent) {
        (self.events)(event);
    }

    pub(crate) fn router(&self) -> MutexGuard<'_, Router> {
        self.router.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub(crate) fn limiter(&self) -> MutexGuard<'_, Limiter> {
        self.limiter.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Counts a failure from `ip`, and says so when it locks it out.
    pub(crate) fn fail(&self, ip: IpAddr) {
        let locked = self.limiter().fail(ip);
        if locked {
            self.locked_out(ip);
        }
    }

    /// Ends a login the limiter let through, and says so when its failure
    /// locks the address out.
    pub(crate) fn end_login(&self, ip: IpAddr, slot: limiter::Slot, ok: bool) {
        let locked = self.limiter().end(slot, ok);
        if locked {
            self.locked_out(ip);
        }
    }

    fn locked_out(&self, ip: IpAddr) {
        self.emit(BrokerEvent::LockedOut {
            ip,
            minutes: self.limits.lockout.as_secs().div_ceil(60),
        });
    }

    /// Counts a wrong join code, and turns the join code off, saying so
    /// once, when the session's budget is spent.
    pub(crate) fn join_wrong(&self) {
        let wrong = self
            .join_wrong
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        if wrong >= self.limits.join_budget && !self.join_off.swap(true, Ordering::SeqCst) {
            self.emit(BrokerEvent::JoinCodeOff { wrong });
        }
    }

    /// Takes a place for a connection from `ip` that has not logged in
    /// yet, or refuses when its address has as many as it may. Says so the
    /// first time the address reaches the cap.
    pub(crate) fn take_waiting(&self, ip: IpAddr) -> bool {
        let k = limiter::key(ip);
        let max = self.limits.max_waiting_per_addr;
        let warn = {
            let mut w = self.waiting.lock().unwrap_or_else(|p| p.into_inner());
            let e = w.entry(k).or_default();
            if e.count < max {
                e.count += 1;
                return true;
            }
            !std::mem::replace(&mut e.warned, true)
        };
        if warn {
            self.emit(BrokerEvent::AddressFull {
                ip: k,
                max,
                logged_in: false,
            });
        }
        false
    }

    /// Gives back a place [`Shared::take_waiting`] took.
    pub(crate) fn release_waiting(&self, ip: IpAddr) {
        let k = limiter::key(ip);
        let mut w = self.waiting.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(e) = w.get_mut(&k) {
            e.count = e.count.saturating_sub(1);
            if e.count == 0 {
                w.remove(&k);
            } else if e.count < self.limits.max_waiting_per_addr {
                e.warned = false;
            }
        }
    }

    /// Routes a message from a client or from the app.
    pub(crate) fn route(&self, topic: &str, payload: Bytes, qos: QoS, retain: bool) {
        let outcome = self
            .router()
            .publish(topic, payload, qos, retain, self.limits.max_retained);
        if outcome.not_retained {
            self.emit(BrokerEvent::RetainedFull {
                topic: clean_text(topic, 200),
            });
        }
        for user in outcome.dropped {
            self.emit(BrokerEvent::Problem(format!(
                "Dropped: {user} fell more than {} messages behind.",
                self.limits.queue
            )));
        }
    }
}

impl Shared {
    /// The state [`fuzz_connection`] runs against: small caps, one logger
    /// account (`fuzz`, password `fuzz`, on `f/#`) and a join password
    /// (`fuzz-join`, reading everything).
    fn for_fuzz() -> Arc<Shared> {
        let mut accounts = Accounts::default();
        if let Ok(a) = Account::with_iterations(
            "fuzz",
            Role::Logger,
            vec!["f/#".into()],
            vec!["f/#".into()],
            "fuzz",
            1_000,
        ) {
            let _ = accounts.add(a, false);
        }
        let limits = BrokerLimits {
            max_payload: 4096,
            max_topic: 128,
            max_subscriptions: 8,
            max_retained: 64,
            queue: 16,
            // Fuzz input is all from one address and never ends: no
            // lockout and no join budget, so every input reaches the code
            // after the login.
            max_fails: u32::MAX,
            join_budget: u32::MAX,
            ..BrokerLimits::default()
        };
        Arc::new(Shared {
            stop: AtomicBool::new(false),
            limiter: Mutex::new(Limiter::new(
                limits.max_fails,
                limits.fail_window,
                limits.lockout,
            )),
            waiting: Mutex::new(HashMap::new()),
            join_wrong: AtomicU32::new(0),
            join_off: AtomicBool::new(false),
            limits,
            accounts,
            join: Some((
                Secret::new("fuzz-join"),
                Perms {
                    read: vec!["#".into()],
                    publish: Vec::new(),
                },
            )),
            tls: None,
            router: Mutex::new(Router::default()),
            active: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            conns: Mutex::new(Vec::new()),
            events: Box::new(|_| {}),
        })
    }
}

/// A running broker. [`Broker::stop`] ends it; dropping it asks every
/// thread to stop without waiting.
pub struct Broker {
    shared: Arc<Shared>,
    listeners: Vec<JoinHandle<()>>,
    addrs: Vec<SocketAddr>,
    pin: Option<Pin>,
    join_code: Option<JoinCode>,
    made_identity: bool,
}

impl Broker {
    /// Binds every address and starts serving. `events` is called from the
    /// broker's threads with each [`BrokerEvent`]; it must be quick. The
    /// first events are one [`BrokerEvent::Listening`] per address.
    pub fn start(
        config: BrokerConfig,
        events: impl Fn(BrokerEvent) + Send + Sync + 'static,
    ) -> Result<Broker, String> {
        if config.join_code && !config.tls {
            return Err(
                "Broker error: a join code needs TLS, so readers can check the broker. Leave out --plain."
                    .into(),
            );
        }
        for t in &config.join_topics {
            if let Some(why) = textweaver_mqtt::topic::filter_problem(t) {
                return Err(format!(
                    "Broker error: a join topic is not valid: {why}: {}.",
                    clean_text(t, 200)
                ));
            }
        }
        if config.join_code && config.join_topics.is_empty() {
            return Err("Broker error: a join code needs at least one topic to read.".into());
        }
        let (tls, pin, join_code, made_identity) = if !config.tls {
            (None, None, None, false)
        } else if config.join_code {
            let id = identity::fresh()?;
            let code = JoinCode::new(&id.pin)?;
            (Some(id.config), Some(id.pin), Some(code), false)
        } else {
            let (id, made) = identity::stable(&config.data_dir)?;
            (Some(id.config), Some(id.pin), None, made)
        };
        let listen = if config.listen.is_empty() {
            let port = if config.tls { TLS_PORT } else { PLAIN_PORT };
            vec![SocketAddr::from(([127, 0, 0, 1], port))]
        } else {
            config.listen.clone()
        };
        let mut bound = Vec::new();
        for addr in &listen {
            let l = TcpListener::bind(addr).map_err(|e| {
                format!("Broker error: cannot listen on {addr}: {}.", bind_words(&e))
            })?;
            l.set_nonblocking(true)
                .map_err(|e| format!("Broker error: cannot listen on {addr}: {e}."))?;
            let local = l
                .local_addr()
                .map_err(|e| format!("Broker error: cannot listen on {addr}: {e}."))?;
            bound.push((l, local));
        }
        let limits = config.limits.clone();
        let join = join_code.as_ref().map(|c| {
            (
                c.password(),
                Perms {
                    read: config.join_topics.clone(),
                    publish: Vec::new(),
                },
            )
        });
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            limiter: Mutex::new(Limiter::new(
                limits.max_fails,
                limits.fail_window,
                limits.lockout,
            )),
            waiting: Mutex::new(HashMap::new()),
            join_wrong: AtomicU32::new(0),
            join_off: AtomicBool::new(false),
            limits,
            accounts: config.accounts,
            join,
            tls,
            router: Mutex::new(Router::default()),
            active: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            conns: Mutex::new(Vec::new()),
            events: Box::new(events),
        });
        let mut listeners = Vec::new();
        let mut addrs = Vec::new();
        for (l, local) in bound {
            shared.emit(BrokerEvent::Listening {
                addr: local,
                tls: config.tls,
                network: Network::of(local.ip()),
            });
            let s = shared.clone();
            let t = std::thread::Builder::new()
                .name("broker-listen".into())
                .spawn(move || accept_loop(&s, &l))
                .map_err(|e| format!("Broker error: cannot start a thread: {e}."))?;
            listeners.push(t);
            addrs.push(local);
        }
        Ok(Broker {
            shared,
            listeners,
            addrs,
            pin,
            join_code,
            made_identity,
        })
    }

    /// The addresses bound, with their real ports.
    pub fn local_addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }

    /// The certificate pin readers check, when TLS is on: the full pin in
    /// accounts mode, the session's in join code mode.
    pub fn pin(&self) -> Option<&Pin> {
        self.pin.as_ref()
    }

    /// The session's join code, when one was asked for.
    pub fn join_code(&self) -> Option<&JoinCode> {
        self.join_code.as_ref()
    }

    /// True when this start made the data folder's key and certificate.
    pub fn made_new_identity(&self) -> bool {
        self.made_identity
    }

    /// Clients logged in now.
    pub fn clients(&self) -> usize {
        self.shared.router().count()
    }

    /// Publishes from inside the program, as the app's sharing and
    /// check-in commands will: no login, no network. QoS 1 to readers who
    /// asked for it.
    pub fn publish(&self, topic: &str, payload: &[u8], retain: bool) -> Result<(), String> {
        if let Some(why) = name_problem(topic) {
            return Err(format!("Publish error: {why}."));
        }
        if topic.len() > self.shared.limits.max_topic {
            return Err(format!(
                "Publish error: the topic is longer than {} bytes.",
                self.shared.limits.max_topic
            ));
        }
        if payload.len() > self.shared.limits.max_payload {
            return Err(format!(
                "Too large: the message is {} bytes; the limit is {}.",
                payload.len(),
                self.shared.limits.max_payload
            ));
        }
        self.shared.route(
            topic,
            Bytes::copy_from_slice(payload),
            QoS::AtLeastOnce,
            retain,
        );
        Ok(())
    }

    /// Stops listening, ends every connection, and waits for every thread.
    /// Every port is free when it returns.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        for t in self.listeners.drain(..) {
            let _ = t.join();
        }
        let conns =
            std::mem::take(&mut *self.shared.conns.lock().unwrap_or_else(|p| p.into_inner()));
        for t in conns {
            let _ = t.join();
        }
        self.shared.emit(BrokerEvent::Stopped);
    }
}

impl Drop for Broker {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
    }
}

fn bind_words(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::AddrInUse => "the port is in use by another program".into(),
        std::io::ErrorKind::AddrNotAvailable => "that address is not one of this computer's".into(),
        std::io::ErrorKind::PermissionDenied => "permission denied for that port".into(),
        _ => e.to_string(),
    }
}

/// How often the listener checks the stop flag when nobody connects.
const ACCEPT_TICK: Duration = Duration::from_millis(50);

fn accept_loop(shared: &Arc<Shared>, listener: &TcpListener) {
    loop {
        if shared.stopped() {
            return;
        }
        match listener.accept() {
            Ok((tcp, peer)) => {
                // Windows hands a non-blocking listener's sockets out
                // non-blocking; the connection threads block with timeouts.
                if tcp.set_nonblocking(false).is_err() || shared.limiter().locked(peer.ip()) {
                    continue;
                }
                if shared.active.load(Ordering::SeqCst) >= shared.limits.max_clients {
                    shared.emit(BrokerEvent::Full {
                        max: shared.limits.max_clients,
                    });
                    continue;
                }
                if !shared.take_waiting(peer.ip()) {
                    continue;
                }
                shared.active.fetch_add(1, Ordering::SeqCst);
                let id = shared.next_id.fetch_add(1, Ordering::SeqCst);
                let s = shared.clone();
                let spawned = std::thread::Builder::new()
                    .name("broker-client".into())
                    .spawn(move || {
                        conn::serve(&s, tcp, peer, id);
                        s.active.fetch_sub(1, Ordering::SeqCst);
                    });
                let mut conns = shared.conns.lock().unwrap_or_else(|p| p.into_inner());
                conns.retain(|t| !t.is_finished());
                match spawned {
                    Ok(t) => conns.push(t),
                    Err(e) => {
                        shared.active.fetch_sub(1, Ordering::SeqCst);
                        shared.release_waiting(peer.ip());
                        shared.emit(BrokerEvent::Problem(format!(
                            "Problem: cannot start a thread for a client: {e}."
                        )));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_TICK);
            }
            Err(e) => {
                if !matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::ConnectionReset
                ) {
                    shared.emit(BrokerEvent::Problem(format!(
                        "Problem: a connection could not be taken: {e}."
                    )));
                }
                std::thread::sleep(ACCEPT_TICK);
            }
        }
    }
}

/// Runs one connection's packet handling over arbitrary bytes, as if a
/// client had sent them, with a test account (`fuzz`, password `fuzz`,
/// logger on `f/#`), for the fuzz target. Not part of the API.
#[doc(hidden)]
pub fn fuzz_connection(data: &[u8]) {
    conn::fuzz(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn networks_are_named() {
        let of = |ip: &str, ts: bool| Network::classify(ip.parse().unwrap(), || ts);
        assert_eq!(of("127.0.0.1", true), Network::Loopback);
        assert_eq!(of("::1", true), Network::Loopback);
        assert_eq!(of("100.101.1.2", true), Network::Tailscale);
        assert_eq!(of("fd7a:115c:a1e0::5", true), Network::Tailscale);
        assert_eq!(
            of("100.101.1.2", false),
            Network::Other,
            "a carrier's 100.64 address with no Tailscale interface"
        );
        assert_eq!(of("0.0.0.0", true), Network::Every);
        assert_eq!(of("192.168.1.20", true), Network::Other);
    }

    #[test]
    fn listening_lines_put_meaning_first() {
        let line_with = |ip: &str, tls, ts: bool| {
            BrokerEvent::Listening {
                addr: SocketAddr::new(ip.parse().unwrap(), 1883),
                tls,
                network: Network::classify(ip.parse().unwrap(), || ts),
            }
            .to_string()
        };
        let line = |ip: &str, tls| line_with(ip, tls, true);
        assert_eq!(
            line("127.0.0.1", true),
            "Listening: 127.0.0.1:1883, TLS, loopback only. Other computers cannot connect."
        );
        assert!(line("192.168.1.20", false).starts_with("Warning: 192.168.1.20:1883 is plain"));
        assert!(line("0.0.0.0", false).starts_with("Warning:"));
        assert_eq!(
            line("100.101.1.2", false),
            "Listening: 100.101.1.2:1883, plain, on Tailscale, which encrypts the link."
        );
        // No Tailscale interface: a 100.64 address keeps the warning.
        assert_eq!(
            line_with("100.64.1.1", false, false),
            "Warning: 100.64.1.1:1883 is plain, and no Tailscale interface was found, so the link may not be encrypted. Anyone on that network can read the messages and the passwords."
        );
        assert_eq!(
            line_with("100.64.1.1", true, false),
            "Listening: 100.64.1.1:1883, TLS."
        );
    }

    #[test]
    fn new_refusals_put_meaning_first() {
        let ip: IpAddr = "192.0.2.9".parse().unwrap();
        assert_eq!(
            BrokerEvent::AddressFull {
                ip,
                max: 8,
                logged_in: false
            }
            .to_string(),
            "Refused connections: 192.0.2.9 has 8 still logging in, its limit."
        );
        assert_eq!(
            BrokerEvent::AddressFull {
                ip,
                max: 16,
                logged_in: true
            }
            .to_string(),
            "Refused a login: 192.0.2.9 has 16 connections, its limit."
        );
        assert_eq!(
            BrokerEvent::JoinFull { max: 50 }.to_string(),
            "Refused a join login: 50 readers have joined with the code, its limit."
        );
        assert_eq!(
            BrokerEvent::JoinCodeOff { wrong: 200 }.to_string(),
            "Join code off: too many wrong codes. Start the broker again for a new code."
        );
    }

    #[test]
    fn perms_follow_filters() {
        let p = Perms {
            read: vec!["netlog/#".into()],
            publish: vec!["netlog/checkins".into()],
        };
        assert!(p.can_read("netlog/checkins"));
        assert!(!p.can_read("other"));
        assert!(p.can_publish("netlog/checkins"));
        assert!(!p.can_publish("netlog/other"));
        assert!(p.can_subscribe("netlog/+"));
        assert!(!p.can_subscribe("#"));
    }

    #[test]
    fn join_code_needs_tls() {
        let mut c = BrokerConfig::new("never-written");
        c.tls = false;
        c.join_code = true;
        let err = Broker::start(c, |_| {}).err().unwrap();
        assert!(
            err.starts_with("Broker error: a join code needs TLS"),
            "{err}"
        );
    }
}
