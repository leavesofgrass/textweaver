//! The client thread. It owns the connection and sends [`MqttEvent`]s over
//! a bounded std channel; [`MqttSource::poll`] drains it with `try_recv`,
//! so the reader never blocks. It never sends PUBLISH: the only packets it
//! writes are CONNECT, SUBSCRIBE, PUBACK, PINGREQ and DISCONNECT.

use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use rumqttc::mqttbytes::v4::{
    Connect, ConnectReturnCode, Packet, PubAck, Subscribe, SubscribeFilter, SubscribeReasonCode,
};
use rumqttc::mqttbytes::{Error as CodecError, QoS, check};

use crate::error::{Failure, Kind, Stage};
use crate::sanitize::{KeyOrder, clean_text, decode_payload_ordered};
use crate::{MqttConfig, MqttEvent, Payload, new_client_id, tls, topic};

/// The read timeout: how often the thread checks its stop flag, the
/// keepalive and the quiet hint.
const TICK: Duration = Duration::from_millis(200);

/// Room for the topic, packet id and header around the largest payload.
const PACKET_OVERHEAD: usize = 65_535 + 16;

/// A running live source. Dropping it asks the thread to stop without
/// waiting; [`MqttSource::stop`] also waits for it.
pub struct MqttSource {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    events: Receiver<MqttEvent>,
    client_id: String,
}

impl MqttSource {
    /// Starts the client thread. `wake` is called from that thread after
    /// each event is queued (textweaver's wake slot); it must be cheap and
    /// never block. Settings errors are reported as the first events, not
    /// returned.
    pub fn start(mut config: MqttConfig, wake: Box<dyn Fn() + Send>) -> MqttSource {
        if config.client_id.is_empty() {
            config.client_id = new_client_id();
        }
        let client_id = config.client_id.clone();
        let (tx, events) = sync_channel(config.queue.max(16));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = Worker {
            config,
            stop: stop.clone(),
            tx: tx.clone(),
            wake,
            dropped: 0,
            status: Status::Idle,
        };
        let thread = match std::thread::Builder::new()
            .name("mqtt-receiver".into())
            .spawn(move || worker.run())
        {
            Ok(t) => Some(t),
            Err(e) => {
                let _ = tx.try_send(MqttEvent::Disconnected {
                    reason: format!("Failed: cannot start the connection thread: {e}."),
                    retrying: false,
                });
                None
            }
        };
        MqttSource {
            stop,
            thread,
            events,
            client_id,
        }
    }

    /// Every event queued since the last call, oldest first. Never blocks.
    pub fn poll(&self) -> Vec<MqttEvent> {
        self.events.try_iter().collect()
    }

    /// The client id this source logs in with (`tw-` and 16 hex digits
    /// unless the configuration named one).
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Stops the thread and waits for it: a fifth of a second once
    /// connected, a little more while a connection is being made. Sends
    /// DISCONNECT when connected.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for MqttSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

/// What was last reported, so a status is reported once per change.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Status {
    Idle,
    Connecting,
    Connected,
    Down(String),
}

struct Worker {
    config: MqttConfig,
    stop: Arc<AtomicBool>,
    tx: SyncSender<MqttEvent>,
    wake: Box<dyn Fn() + Send>,
    /// Messages dropped while the channel was full, not yet reported.
    dropped: u64,
    status: Status,
}

enum End {
    Stopped,
    /// The session failed; the flag says whether it reached CONNACK.
    Failed(Failure, bool),
}

/// The quiet hint's record for one topic filter.
struct Watch {
    filter: String,
    last: Option<Instant>,
    reported: bool,
}

impl Worker {
    fn run(mut self) {
        if let Some(f) = self.check_settings() {
            self.down(&f, false);
            return;
        }
        let mut watches: Vec<Watch> = self
            .config
            .topics
            .iter()
            .map(|t| Watch {
                filter: t.clone(),
                last: None,
                reported: false,
            })
            .collect();
        let mut attempt: u32 = 0;
        loop {
            if self.stopped() {
                return;
            }
            attempt += 1;
            if matches!(self.status, Status::Idle) {
                self.status = Status::Connecting;
                self.emit(MqttEvent::Connecting { attempt });
            }
            match self.session(&mut watches) {
                End::Stopped => return,
                End::Failed(failure, was_connected) => {
                    let retrying = failure.kind.retry_helps();
                    self.down(&failure, retrying);
                    if !retrying {
                        return;
                    }
                    if was_connected {
                        attempt = 1; // a working session resets the backoff
                    }
                    let until = Instant::now() + backoff(&self.config, attempt);
                    while Instant::now() < until {
                        if self.stopped() {
                            return;
                        }
                        std::thread::sleep(TICK.min(until - Instant::now()));
                    }
                }
            }
        }
    }

    /// Settings that cannot work, found before any connection is tried.
    fn check_settings(&self) -> Option<Failure> {
        let cfg = &self.config;
        let bad = |why: String| {
            Some(Failure::new(
                Kind::Config,
                format!("Settings error: {why}."),
            ))
        };
        if cfg.host.trim().is_empty() {
            return bad("no host is set".into());
        }
        if cfg.port == 0 {
            return bad("the port is 0".into());
        }
        if cfg.topics.is_empty() {
            return bad("no topic is set".into());
        }
        for t in &cfg.topics {
            if let Some(why) = topic::filter_problem(t) {
                return bad(format!("{why}: {}", clean_text(t, 200)));
            }
        }
        if cfg.password.is_some() && cfg.username.is_none() {
            return bad("a password needs a user name".into());
        }
        if cfg.tls.is_on() {
            if let Err(f) = tls::client_config(&cfg.tls) {
                return Some(f);
            }
            if let Err(f) = tls::server_name(&cfg.host) {
                return Some(f);
            }
        }
        None
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Reports a failure, unless the same reason is already reported.
    fn down(&mut self, failure: &Failure, retrying: bool) {
        let text = clean_text(&failure.text, 2000);
        if self.status == Status::Down(text.clone()) && retrying {
            return;
        }
        self.status = Status::Down(text.clone());
        self.emit(MqttEvent::Disconnected {
            reason: text,
            retrying,
        });
    }

    /// Queues an event without blocking on messages: a message that finds
    /// the channel full is counted, and the count is reported once there
    /// is room again. A status event waits for room, checking for a stop.
    fn emit(&mut self, event: MqttEvent) {
        if self.dropped > 0 {
            let note = MqttEvent::Skipped {
                count: self.dropped,
            };
            if self.tx.try_send(note).is_ok() {
                self.dropped = 0;
            }
        }
        match self.tx.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(MqttEvent::Message { .. })) => {
                self.dropped += 1;
                return;
            }
            Err(TrySendError::Full(other)) => {
                let mut ev = other;
                loop {
                    if self.stopped() {
                        return;
                    }
                    match self.tx.try_send(ev) {
                        Ok(()) => break,
                        Err(TrySendError::Full(back)) => {
                            ev = back;
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        Err(TrySendError::Disconnected(_)) => return,
                    }
                }
            }
            Err(TrySendError::Disconnected(_)) => {
                // The reader is gone: nothing more to do.
                self.stop.store(true, Ordering::SeqCst);
                return;
            }
        }
        (self.wake)();
    }

    fn session(&mut self, watches: &mut [Watch]) -> End {
        let cfg = self.config.clone();
        let target = cfg.target();

        // TCP, every address at once.
        let (tcp, peer) =
            match crate::net::connect(&cfg.host, cfg.port, cfg.connect_timeout, &self.stop) {
                Ok(Some(x)) => x,
                Ok(None) => return End::Stopped,
                Err(f) => return End::Failed(f, false),
            };
        let _ = tcp.set_nodelay(true);
        if let Err(e) = tcp
            .set_read_timeout(Some(TICK))
            .and_then(|_| tcp.set_write_timeout(Some(cfg.connect_timeout)))
        {
            return End::Failed(Failure::from_io(&e, Stage::Connect, &target), false);
        }
        let tailscale = tls::is_tailscale_host(&cfg.host) || tls::is_tailscale_addr(peer.ip());

        // TLS, with the handshake finished here so its errors are named.
        let mut stream: Box<dyn Stream> = if cfg.tls.is_on() {
            let (config, name) = match tls::client_config(&cfg.tls)
                .and_then(|c| tls::server_name(&cfg.host).map(|n| (c, n)))
            {
                Ok(x) => x,
                Err(f) => return End::Failed(f, false),
            };
            let conn = match rustls::ClientConnection::new(config, name) {
                Ok(c) => c,
                Err(e) => return End::Failed(Failure::new(Kind::Tls, tls::describe(&e)), false),
            };
            let mut s = rustls::StreamOwned::new(conn, tcp);
            let deadline = Instant::now() + cfg.connect_timeout;
            while s.conn.is_handshaking() {
                if self.stopped() {
                    return End::Stopped;
                }
                match s.conn.complete_io(&mut s.sock) {
                    Ok(_) => {}
                    Err(e) if is_tick(&e) && Instant::now() < deadline => {}
                    Err(e) => {
                        return End::Failed(Failure::from_io(&e, Stage::Handshake, &target), false);
                    }
                }
            }
            Box::new(s)
        } else {
            Box::new(tcp)
        };

        // CONNECT, then wait for CONNACK.
        let mut connect = Connect::new(cfg.client_id.clone());
        connect.keep_alive = cfg.keep_alive.as_secs().min(u64::from(u16::MAX)) as u16;
        connect.clean_session = true;
        if let Some(user) = &cfg.username {
            let password = cfg
                .password
                .as_ref()
                .map(|p| p.expose().to_string())
                .unwrap_or_default();
            connect.set_login(user.clone(), password);
        }
        if let Err(e) = send(&mut stream, Packet::Connect(connect)) {
            return End::Failed(Failure::from_io(&e, Stage::Login, &target), false);
        }
        let max_packet = cfg.limits.max_payload.saturating_add(PACKET_OVERHEAD);
        let mut buf = BytesMut::with_capacity(4096);
        let deadline = Instant::now() + cfg.connect_timeout;
        let connack = loop {
            if self.stopped() {
                return End::Stopped;
            }
            match read_packet(&mut stream, &mut buf, max_packet) {
                Ok(Some(Frame::Packet(Packet::ConnAck(a)))) => break a,
                Ok(Some(_)) => {
                    return End::Failed(
                        Failure::new(
                            Kind::Protocol,
                            "Protocol error: the broker did not answer the login with CONNACK.",
                        ),
                        false,
                    );
                }
                Ok(None) if Instant::now() < deadline => {}
                Ok(None) => {
                    return End::Failed(
                        Failure::new(
                            Kind::Unreachable,
                            format!(
                                "Unreachable: {target} accepted the connection but did not answer the login in time."
                            ),
                        ),
                        false,
                    );
                }
                Err(e) => {
                    let mut f = Failure::from_io(&e, Stage::Login, &target);
                    // Mosquitto closes a TLS port on a plain CONNECT
                    // without a TLS alert (a reset on Windows), so a hint
                    // is all we can give.
                    if !cfg.tls.is_on()
                        && matches!(
                            e.kind(),
                            io::ErrorKind::UnexpectedEof
                                | io::ErrorKind::ConnectionReset
                                | io::ErrorKind::ConnectionAborted
                        )
                    {
                        f.text
                            .push_str(" If this port expects TLS, turn TLS on for this source.");
                    }
                    return End::Failed(f, false);
                }
            }
        };
        if connack.code != ConnectReturnCode::Success {
            return End::Failed(Failure::from_connack(connack.code), false);
        }
        self.status = Status::Connected;
        self.emit(MqttEvent::Connected {
            encrypted: cfg.tls.is_on(),
            tailscale,
        });

        // SUBSCRIBE on every CONNACK: a clean session forgets them.
        let filters: Vec<SubscribeFilter> = cfg
            .topics
            .iter()
            .map(|t| SubscribeFilter::new(t.clone(), QoS::AtLeastOnce))
            .collect();
        if let Err(e) = send(
            &mut stream,
            Packet::Subscribe(Subscribe { pkid: 1, filters }),
        ) {
            return End::Failed(Failure::from_io(&e, Stage::Running, &target), true);
        }
        let connected_at = Instant::now();
        let mut last_sent = connected_at;
        let mut ping_sent: Option<Instant> = None;
        let keep_alive = cfg.keep_alive;
        let ping_wait = (keep_alive / 2)
            .max(Duration::from_secs(5))
            .min(cfg.connect_timeout.max(Duration::from_secs(5)));
        loop {
            if self.stopped() {
                let _ = send(&mut stream, Packet::Disconnect);
                return End::Stopped;
            }
            match read_packet(&mut stream, &mut buf, max_packet) {
                Ok(None) => {}
                Ok(Some(Frame::Oversize(o))) => {
                    if o.qos == QoS::AtLeastOnce {
                        if let Err(e) = send(&mut stream, Packet::PubAck(PubAck::new(o.pkid))) {
                            return End::Failed(
                                Failure::from_io(&e, Stage::Running, &target),
                                true,
                            );
                        }
                        last_sent = Instant::now();
                    }
                    let topic = clean_text(&o.topic, 2000);
                    heard(watches, &topic);
                    self.emit(MqttEvent::Message {
                        topic,
                        payload: Payload::TooLarge { len: o.len },
                        retained: o.retain,
                        key_order: KeyOrder::new(),
                    });
                }
                Ok(Some(Frame::Packet(p))) => match p {
                    Packet::Publish(p) => {
                        if p.qos == QoS::ExactlyOnce {
                            return End::Failed(
                                Failure::new(
                                    Kind::Protocol,
                                    "Protocol error: the broker sent QoS 2, which was not asked for.",
                                ),
                                true,
                            );
                        }
                        if p.qos == QoS::AtLeastOnce {
                            if let Err(e) = send(&mut stream, Packet::PubAck(PubAck::new(p.pkid))) {
                                return End::Failed(
                                    Failure::from_io(&e, Stage::Running, &target),
                                    true,
                                );
                            }
                            last_sent = Instant::now();
                        }
                        let topic = clean_text(&p.topic, 2000);
                        heard(watches, &topic);
                        let (payload, key_order) = decode_payload_ordered(&p.payload, &cfg.limits);
                        self.emit(MqttEvent::Message {
                            topic,
                            payload,
                            retained: p.retain,
                            key_order,
                        });
                    }
                    Packet::SubAck(ack) => {
                        let mut granted = 0;
                        for (filter, code) in cfg.topics.iter().zip(ack.return_codes) {
                            match code {
                                SubscribeReasonCode::Success(_) => granted += 1,
                                SubscribeReasonCode::Failure => {
                                    self.emit(MqttEvent::TopicRefused {
                                        topic: clean_text(filter, 2000),
                                    });
                                }
                            }
                        }
                        if granted == 0 {
                            let _ = send(&mut stream, Packet::Disconnect);
                            return End::Failed(
                                Failure::new(
                                    Kind::SubscriptionRefused,
                                    "Not authorized: the broker refused every topic this source asked for.",
                                ),
                                true,
                            );
                        }
                    }
                    Packet::PingResp => ping_sent = None,
                    Packet::Disconnect => {
                        return End::Failed(
                            Failure::new(
                                Kind::Lost,
                                format!(
                                    "Connection lost to {target}: the broker ended the session."
                                ),
                            ),
                            true,
                        );
                    }
                    other => {
                        return End::Failed(
                            Failure::new(
                                Kind::Protocol,
                                format!(
                                    "Protocol error: unexpected {} packet from the broker.",
                                    kind(&other)
                                ),
                            ),
                            true,
                        );
                    }
                },
                Err(e) => return End::Failed(Failure::from_io(&e, Stage::Running, &target), true),
            }

            // The quiet hint: once per quiet spell, timed from this
            // session's login or the topic's last message.
            if !cfg.quiet_after.is_zero() {
                let now = Instant::now();
                let mut quiet = Vec::new();
                for w in watches.iter_mut() {
                    let since = w.last.map_or(connected_at, |l| l.max(connected_at));
                    if !w.reported && now.duration_since(since) >= cfg.quiet_after {
                        w.reported = true;
                        quiet.push(clean_text(&w.filter, 2000));
                    }
                }
                for topic in quiet {
                    self.emit(MqttEvent::Quiet {
                        topic,
                        seconds: cfg.quiet_after.as_secs(),
                    });
                }
            }

            // Keepalive: a PINGREQ once nothing has been sent for the
            // interval; the broker must answer within `ping_wait`.
            if !keep_alive.is_zero() {
                if let Some(sent) = ping_sent {
                    if sent.elapsed() > ping_wait {
                        return End::Failed(
                            Failure::new(
                                Kind::Lost,
                                format!(
                                    "Connection lost to {target}: the broker stopped answering."
                                ),
                            ),
                            true,
                        );
                    }
                } else if last_sent.elapsed() >= keep_alive {
                    if let Err(e) = send(&mut stream, Packet::PingReq) {
                        return End::Failed(Failure::from_io(&e, Stage::Running, &target), true);
                    }
                    last_sent = Instant::now();
                    ping_sent = Some(last_sent);
                }
            }
        }
    }
}

/// A message arrived on `topic`: every filter it matches is heard.
fn heard(watches: &mut [Watch], topic: &str) {
    let now = Instant::now();
    for w in watches
        .iter_mut()
        .filter(|w| topic::matches(&w.filter, topic))
    {
        w.last = Some(now);
        w.reported = false;
    }
}

fn backoff(cfg: &MqttConfig, attempt: u32) -> Duration {
    let shift = attempt.saturating_sub(1).min(16);
    let base = cfg
        .min_backoff
        .saturating_mul(1u32 << shift)
        .min(cfg.max_backoff);
    // Up to a quarter more, so many readers do not return at the same
    // moment.
    use ring::rand::{SecureRandom, SystemRandom};
    let mut b = [0u8; 1];
    let _ = SystemRandom::new().fill(&mut b);
    base + base / 4 * u32::from(b[0]) / 255
}

fn is_tick(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

fn send(stream: &mut Box<dyn Stream>, p: Packet) -> io::Result<()> {
    let mut b = BytesMut::new();
    p.write(&mut b, usize::MAX)
        .map_err(|e| io::Error::other(e.to_string()))?;
    stream.write_all(&b)?;
    stream.flush()
}

/// A PUBLISH over the size limit, skipped without keeping its payload.
#[derive(Debug)]
struct Oversize {
    topic: String,
    len: usize,
    qos: QoS,
    pkid: u16,
    retain: bool,
}

#[derive(Debug)]
enum Frame {
    Packet(Packet),
    Oversize(Oversize),
}

/// Reads at most one packet. `Ok(None)` means nothing complete arrived
/// within one tick. A PUBLISH over `max` keeps only its topic and packet
/// id; the payload is discarded as it streams in, so it is never held in
/// memory, and the connection stays up.
fn read_packet(
    stream: &mut Box<dyn Stream>,
    buf: &mut BytesMut,
    max: usize,
) -> io::Result<Option<Frame>> {
    loop {
        // A TLS record (handshake or alert, version 3.x) where an MQTT
        // packet should be: the port wants TLS.
        if buf.len() >= 2 && matches!(buf[0], 0x15 | 0x16) && buf[1] == 0x03 {
            return Err(io::Error::other(crate::error::TlsExpected));
        }
        match check(buf.iter(), max) {
            Ok(_) => {
                let p = Packet::read(buf, max).map_err(codec_err)?;
                return Ok(Some(Frame::Packet(p)));
            }
            Err(CodecError::PayloadSizeLimitExceeded(_)) => {
                return skip_oversize(stream, buf).map(|o| Some(Frame::Oversize(o)));
            }
            Err(CodecError::InsufficientBytes(_)) => match fill(stream, buf)? {
                true => {}
                false => return Ok(None),
            },
            Err(e) => return Err(codec_err(e)),
        }
    }
}

/// Reads more bytes into `buf`. False when the tick passed with none.
fn fill(stream: &mut Box<dyn Stream>, buf: &mut BytesMut) -> io::Result<bool> {
    let mut chunk = [0u8; 8192];
    match stream.read(&mut chunk) {
        Ok(0) => Err(io::ErrorKind::UnexpectedEof.into()),
        Ok(n) => {
            buf.extend_from_slice(&chunk[..n]);
            Ok(true)
        }
        Err(e) if is_tick(&e) => Ok(false),
        Err(e) => Err(e),
    }
}

/// The fixed header of the packet at the start of `buf`: its first byte,
/// its length, and the remaining length. `None` until enough has arrived.
fn fixed_header(buf: &[u8]) -> io::Result<Option<(u8, usize, usize)>> {
    let Some(&first) = buf.first() else {
        return Ok(None);
    };
    let mut remaining = 0usize;
    for i in 0..4 {
        let Some(&b) = buf.get(1 + i) else {
            return Ok(None);
        };
        remaining |= usize::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(Some((first, 2 + i, remaining)));
        }
    }
    Err(codec_err(CodecError::MalformedRemainingLength))
}

/// Skips an oversize packet. Only PUBLISH may be that large; its topic and
/// packet id are kept so the message can be named and acknowledged.
fn skip_oversize(stream: &mut Box<dyn Stream>, buf: &mut BytesMut) -> io::Result<Oversize> {
    let (first, header, remaining) = loop {
        match fixed_header(buf)? {
            Some(h) => break h,
            None => {
                fill(stream, buf)?;
            }
        }
    };
    if first >> 4 != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed MQTT packet (an oversize packet that is not PUBLISH)",
        ));
    }
    let qos = match (first >> 1) & 0b11 {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        _ => QoS::ExactlyOnce,
    };
    // The topic length, then the topic and the packet id: at most 65,541
    // bytes, held while the rest is thrown away.
    while buf.len() < header + 2 {
        fill(stream, buf)?;
    }
    let topic_len = usize::from(u16::from_be_bytes([buf[header], buf[header + 1]]));
    let id_len = if qos == QoS::AtMostOnce { 0 } else { 2 };
    let variable = 2 + topic_len + id_len;
    if variable > remaining {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed MQTT packet (PUBLISH shorter than its topic)",
        ));
    }
    while buf.len() < header + variable {
        fill(stream, buf)?;
    }
    let topic = String::from_utf8_lossy(&buf[header + 2..header + 2 + topic_len]).into_owned();
    let pkid = if id_len == 2 {
        u16::from_be_bytes([buf[header + 2 + topic_len], buf[header + 3 + topic_len]])
    } else {
        0
    };
    let frame = header + remaining;
    let take = frame.min(buf.len());
    let _ = buf.split_to(take);
    let mut left = frame - take;
    let mut sink = [0u8; 8192];
    while left > 0 {
        match stream.read(&mut sink[..left.min(8192)]) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => left -= n,
            Err(e) if is_tick(&e) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(Oversize {
        topic,
        len: remaining - variable,
        qos,
        pkid,
        retain: first & 1 == 1,
    })
}

fn codec_err(e: CodecError) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("malformed MQTT packet ({e})"),
    )
}

fn kind(p: &Packet) -> &'static str {
    match p {
        Packet::Connect(_) => "CONNECT",
        Packet::ConnAck(_) => "CONNACK",
        Packet::Publish(_) => "PUBLISH",
        Packet::PubAck(_) => "PUBACK",
        Packet::PubRec(_) => "PUBREC",
        Packet::PubRel(_) => "PUBREL",
        Packet::PubComp(_) => "PUBCOMP",
        Packet::Subscribe(_) => "SUBSCRIBE",
        Packet::SubAck(_) => "SUBACK",
        Packet::Unsubscribe(_) => "UNSUBSCRIBE",
        Packet::UnsubAck(_) => "UNSUBACK",
        Packet::PingReq => "PINGREQ",
        Packet::PingResp => "PINGRESP",
        Packet::Disconnect => "DISCONNECT",
    }
}

/// The fuzz target's entry: reads packets from `data` as from a broker,
/// with a small size limit so the oversize path runs too, until the bytes
/// run out or a packet is malformed.
pub(crate) fn fuzz_read(data: &[u8]) {
    let mut stream: Box<dyn Stream> = Box::new(io::Cursor::new(data.to_vec()));
    let mut buf = BytesMut::new();
    for _ in 0..10_000 {
        match read_packet(&mut stream, &mut buf, 1024) {
            Ok(Some(Frame::Packet(Packet::Publish(p)))) => {
                let _ = clean_text(&p.topic, 64);
            }
            Ok(Some(Frame::Oversize(o))) => assert!(o.len <= data.len()),
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tls;

    #[test]
    fn backoff_doubles_to_the_cap() {
        let c = MqttConfig::new("x", 1, Tls::Off);
        let d1 = backoff(&c, 1);
        assert!(d1 >= Duration::from_secs(1) && d1 <= Duration::from_millis(1250));
        let d3 = backoff(&c, 3);
        assert!(d3 >= Duration::from_secs(4) && d3 <= Duration::from_secs(5));
        let d20 = backoff(&c, 20);
        assert!(d20 >= Duration::from_secs(60) && d20 <= Duration::from_secs(75));
    }

    fn publish_bytes(topic: &str, payload: &[u8], qos: QoS, pkid: u16) -> Vec<u8> {
        let mut p = rumqttc::mqttbytes::v4::Publish::new(topic, qos, payload.to_vec());
        p.pkid = pkid;
        p.retain = true;
        let mut b = BytesMut::new();
        Packet::Publish(p).write(&mut b, usize::MAX).unwrap();
        b.to_vec()
    }

    #[test]
    fn oversize_publish_keeps_topic_and_id_and_the_next_packet() {
        let mut bytes = publish_bytes("netlog/big", &vec![b'x'; 5000], QoS::AtLeastOnce, 7);
        bytes.extend(publish_bytes("netlog/small", b"ok", QoS::AtMostOnce, 0));
        let mut stream: Box<dyn Stream> = Box::new(io::Cursor::new(bytes));
        let mut buf = BytesMut::new();
        let Some(Frame::Oversize(o)) = read_packet(&mut stream, &mut buf, 1024).unwrap() else {
            panic!("not skipped")
        };
        assert_eq!(o.topic, "netlog/big");
        assert_eq!(o.len, 5000);
        assert_eq!(o.pkid, 7);
        assert!(o.retain);
        let Some(Frame::Packet(Packet::Publish(p))) =
            read_packet(&mut stream, &mut buf, 1024).unwrap()
        else {
            panic!("next packet lost")
        };
        assert_eq!(p.payload.as_ref(), b"ok");
    }

    #[test]
    fn fuzz_entry_survives_garbage() {
        fuzz_read(&[0x30, 0xff, 0xff, 0xff, 0xff, 0x01]);
        fuzz_read(&[0x32, 0x80, 0x10, 0xff, 0xff]);
        fuzz_read(&[0x16, 0x03, 0x01]);
        fuzz_read(&[]);
    }
}
