//! One client's connection, on its own thread: the TLS handshake, the
//! login, then packets in and queued messages out, until the client leaves,
//! breaks a rule, falls too far behind, or the broker stops.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, sync_channel};
use std::time::{Duration, Instant};

use bytes::BytesMut;
use rumqttc::mqttbytes::v4::{
    ConnAck, Connect, ConnectReturnCode, Packet, PubAck, PubComp, PubRec, Publish, SubAck,
    Subscribe, SubscribeReasonCode, UnsubAck,
};
use rumqttc::mqttbytes::{Error as CodecError, Protocol, QoS};
use rustls::{ServerConnection, StreamOwned};
use textweaver_mqtt::topic::{filter_problem, name_problem};
use textweaver_mqtt::{JOIN_USER, clean_text, ct_eq};

use crate::router::{Client, Outgoing, min_qos};
use crate::{BrokerEvent, Perms, Shared};

/// How long a read waits before the thread checks the stop flag, its
/// queue and the keepalive: the most a queued message waits to go out.
const TICK: Duration = Duration::from_millis(25);

/// How long a write to a stuck client may block before it is dropped.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Queued messages written per pass, so reading is never starved.
const BATCH: usize = 256;

pub(crate) trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

/// Serves one accepted connection to its end.
pub(crate) fn serve(shared: &Arc<Shared>, tcp: TcpStream, peer: SocketAddr, id: u64) {
    let _ = tcp.set_nodelay(true);
    if tcp
        .set_read_timeout(Some(TICK))
        .and_then(|_| tcp.set_write_timeout(Some(WRITE_TIMEOUT)))
        .is_err()
    {
        return;
    }
    let stream: Box<dyn Stream> = match &shared.tls {
        Some(config) => match handshake(shared, config.clone(), tcp) {
            Some(s) => Box::new(s),
            None => {
                if !shared.stopped() {
                    shared.fail(peer.ip());
                }
                return;
            }
        },
        None => Box::new(tcp),
    };
    session(shared, stream, peer, id);
}

fn handshake(
    shared: &Shared,
    config: Arc<rustls::ServerConfig>,
    tcp: TcpStream,
) -> Option<StreamOwned<ServerConnection, TcpStream>> {
    let conn = ServerConnection::new(config).ok()?;
    let mut s = StreamOwned::new(conn, tcp);
    let deadline = Instant::now() + shared.limits.connect_timeout;
    while s.conn.is_handshaking() {
        if shared.stopped() || Instant::now() >= deadline {
            return None;
        }
        match s.conn.complete_io(&mut s.sock) {
            Ok((0, 0)) => std::thread::sleep(TICK),
            Ok(_) => {}
            Err(e) if is_tick(&e) => {}
            Err(_) => return None,
        }
    }
    Some(s)
}

/// Removes the client from the router however the session ends.
struct Registered<'a> {
    shared: &'a Shared,
    id: u64,
}

impl Drop for Registered<'_> {
    fn drop(&mut self) {
        self.shared.router().remove(self.id);
    }
}

fn session(shared: &Shared, mut stream: Box<dyn Stream>, peer: SocketAddr, id: u64) {
    let limits = &shared.limits;
    let max_packet = limits.max_payload + limits.max_topic + 16;
    let mut buf = BytesMut::with_capacity(1024);

    // CONNECT first, within the time limit.
    let deadline = Instant::now() + limits.connect_timeout;
    let connect = loop {
        if shared.stopped() || Instant::now() >= deadline {
            return;
        }
        match read_packet(&mut stream, &mut buf, max_packet) {
            Ok(Some(Packet::Connect(c))) => break c,
            Ok(None) => {}
            Err(ReadErr::Version) => {
                let _ = connack(&mut stream, ConnectReturnCode::RefusedProtocolVersion);
                return;
            }
            Err(ReadErr::Io(_)) => return,
            Ok(Some(_)) | Err(ReadErr::Malformed | ReadErr::TooLarge) => {
                shared.fail(peer.ip());
                return;
            }
        }
    };
    if connect.protocol != Protocol::V4 {
        let _ = connack(&mut stream, ConnectReturnCode::RefusedProtocolVersion);
        return;
    }
    let Some((user, perms)) = login(shared, &connect) else {
        let tried = connect
            .login
            .as_ref()
            .map(|l| clean_text(&l.username, 64))
            .unwrap_or_default();
        shared.emit(BrokerEvent::LoginRefused {
            addr: peer,
            user: tried,
        });
        let _ = connack(&mut stream, ConnectReturnCode::NotAuthorized);
        shared.fail(peer.ip());
        return;
    };
    let mut client_id = connect.client_id.clone();
    if client_id.is_empty() {
        if !connect.clean_session {
            let _ = connack(&mut stream, ConnectReturnCode::BadClientId);
            return;
        }
        client_id = format!("tw-broker-{id}");
    }
    if client_id.len() > 256 {
        let _ = connack(&mut stream, ConnectReturnCode::BadClientId);
        return;
    }

    let (tx, rx) = sync_channel(limits.queue.max(1));
    let kill = Arc::new(AtomicBool::new(false));
    let perms = Arc::new(perms);
    let added = shared.router().add(
        id,
        Client {
            client_id: client_id.clone(),
            user: user.clone(),
            perms: perms.clone(),
            subs: Vec::new(),
            tx,
            kill: kill.clone(),
        },
    );
    if added.is_err() {
        shared.emit(BrokerEvent::Problem(format!(
            "Refused: {user} asked for client id {}, which another account is using.",
            clean_text(&client_id, 64)
        )));
        let _ = connack(&mut stream, ConnectReturnCode::BadClientId);
        return;
    }
    let _registered = Registered { shared, id };
    if connack(&mut stream, ConnectReturnCode::Success).is_err() {
        return;
    }
    shared.emit(BrokerEvent::Connected {
        user: user.clone(),
        addr: peer,
    });
    let mut s = Session {
        shared,
        stream,
        buf,
        rx,
        kill,
        perms,
        user: user.clone(),
        id,
        max_packet,
        next_pkid: 0,
    };
    let reason = s.run(connect.keep_alive);
    shared.emit(BrokerEvent::Left {
        user,
        addr: peer,
        reason,
    });
}

/// The account a CONNECT logs in as, and what it may do.
fn login(shared: &Shared, c: &Connect) -> Option<(String, Perms)> {
    let l = c.login.as_ref()?;
    if l.username == JOIN_USER {
        let (password, perms) = shared.join.as_ref()?;
        return ct_eq(l.password.as_bytes(), password.expose().as_bytes())
            .then(|| (JOIN_USER.to_string(), perms.clone()));
    }
    shared
        .accounts
        .login(&l.username, &l.password)
        .map(|p| (l.username.clone(), p))
}

struct Session<'a> {
    shared: &'a Shared,
    stream: Box<dyn Stream>,
    buf: BytesMut,
    rx: Receiver<Outgoing>,
    kill: Arc<AtomicBool>,
    perms: Arc<Perms>,
    user: String,
    id: u64,
    max_packet: usize,
    next_pkid: u16,
}

/// What handling a packet decided.
enum Next {
    Go,
    End(String),
}

impl Session<'_> {
    /// Serves the logged-in client. Returns why it ended, in words.
    fn run(&mut self, keep_alive: u16) -> String {
        // MQTT allows one and a half keepalive intervals of silence.
        let silence = (keep_alive > 0).then(|| Duration::from_millis(u64::from(keep_alive) * 1500));
        let mut last_heard = Instant::now();
        loop {
            if self.shared.stopped() {
                return "the broker stopped".into();
            }
            if self.kill.load(Ordering::SeqCst) {
                return "ended by the broker: a newer connection with its client id, or too far behind".into();
            }
            match read_packet(&mut self.stream, &mut self.buf, self.max_packet) {
                Ok(None) => {}
                Ok(Some(p)) => {
                    last_heard = Instant::now();
                    if let Next::End(why) = self.handle(p) {
                        return why;
                    }
                }
                Err(ReadErr::TooLarge) => {
                    return format!(
                        "dropped for a message over {} bytes",
                        self.shared.limits.max_payload
                    );
                }
                Err(ReadErr::Malformed | ReadErr::Version) => {
                    return "dropped for a malformed packet".into();
                }
                Err(ReadErr::Io(e)) => return closed_words(&e),
            }
            if silence.is_some_and(|s| last_heard.elapsed() > s) {
                return "timed out: nothing heard within its keepalive".into();
            }
            if let Err(e) = self.deliver() {
                return closed_words(&e);
            }
        }
    }

    /// Writes what is waiting in the queue.
    fn deliver(&mut self) -> io::Result<()> {
        for _ in 0..BATCH {
            match self.rx.try_recv() {
                Ok(out) => self.write_out(out)?,
                Err(_) => break,
            }
        }
        Ok(())
    }

    fn write_out(&mut self, out: Outgoing) -> io::Result<()> {
        let mut p = Publish::from_bytes(out.topic.to_string(), out.qos, out.payload);
        p.retain = out.retain;
        if out.qos != QoS::AtMostOnce {
            self.next_pkid = self.next_pkid.checked_add(1).unwrap_or(1);
            p.pkid = self.next_pkid;
        }
        send(&mut self.stream, Packet::Publish(p))
    }

    fn handle(&mut self, packet: Packet) -> Next {
        let io = |r: io::Result<()>| match r {
            Ok(()) => Next::Go,
            Err(e) => Next::End(closed_words(&e)),
        };
        match packet {
            Packet::Publish(p) => self.publish(p),
            Packet::PubRel(r) => io(send(
                &mut self.stream,
                Packet::PubComp(PubComp::new(r.pkid)),
            )),
            Packet::PubAck(_) | Packet::PubRec(_) | Packet::PubComp(_) => Next::Go,
            Packet::Subscribe(s) => self.subscribe(s),
            Packet::Unsubscribe(u) => {
                {
                    let mut router = self.shared.router();
                    for t in &u.topics {
                        router.unsubscribe(self.id, t);
                    }
                }
                io(send(
                    &mut self.stream,
                    Packet::UnsubAck(UnsubAck::new(u.pkid)),
                ))
            }
            Packet::PingReq => io(send(&mut self.stream, Packet::PingResp)),
            Packet::Disconnect => Next::End("left".into()),
            Packet::Connect(_) => Next::End("dropped for a second CONNECT".into()),
            Packet::ConnAck(_) | Packet::SubAck(_) | Packet::UnsubAck(_) | Packet::PingResp => {
                Next::End("dropped for a packet only a broker sends".into())
            }
        }
    }

    fn publish(&mut self, p: Publish) -> Next {
        let limits = &self.shared.limits;
        if p.topic.len() > limits.max_topic || name_problem(&p.topic).is_some() {
            return Next::End("dropped for publishing to an invalid topic".into());
        }
        if !self.perms.can_publish(&p.topic) {
            self.shared.emit(BrokerEvent::PublishRefused {
                user: self.user.clone(),
                topic: clean_text(&p.topic, 200),
            });
            return Next::End("may not publish there".into());
        }
        let qos = p.qos;
        self.shared.route(
            &p.topic,
            p.payload,
            min_qos(qos, QoS::AtLeastOnce),
            p.retain,
        );
        let ack = match qos {
            QoS::AtMostOnce => return Next::Go,
            QoS::AtLeastOnce => Packet::PubAck(PubAck::new(p.pkid)),
            QoS::ExactlyOnce => Packet::PubRec(PubRec::new(p.pkid)),
        };
        match send(&mut self.stream, ack) {
            Ok(()) => Next::Go,
            Err(e) => Next::End(closed_words(&e)),
        }
    }

    fn subscribe(&mut self, s: Subscribe) -> Next {
        let limits = &self.shared.limits;
        let mut codes = Vec::with_capacity(s.filters.len());
        let mut granted = Vec::new();
        {
            let mut router = self.shared.router();
            for f in &s.filters {
                let room = router.subscriptions(self.id) < limits.max_subscriptions;
                let ok = f.path.len() <= limits.max_topic
                    && filter_problem(&f.path).is_none()
                    && self.perms.can_subscribe(&f.path)
                    && room;
                if ok {
                    let qos = min_qos(f.qos, QoS::AtLeastOnce);
                    router.subscribe(self.id, &f.path, qos);
                    codes.push(SubscribeReasonCode::Success(qos));
                    granted.push((f.path.clone(), qos));
                } else {
                    codes.push(SubscribeReasonCode::Failure);
                }
            }
        }
        if let Err(e) = send(&mut self.stream, Packet::SubAck(SubAck::new(s.pkid, codes))) {
            return Next::End(closed_words(&e));
        }
        // The retained messages, after the SUBACK, as MQTT says.
        for (filter, qos) in granted {
            let retained = self.shared.router().retained_for(&filter, qos, &self.perms);
            for out in retained {
                if let Err(e) = self.write_out(out) {
                    return Next::End(closed_words(&e));
                }
            }
        }
        Next::Go
    }
}

fn connack(stream: &mut Box<dyn Stream>, code: ConnectReturnCode) -> io::Result<()> {
    send(stream, Packet::ConnAck(ConnAck::new(code, false)))
}

fn send(stream: &mut Box<dyn Stream>, p: Packet) -> io::Result<()> {
    let mut b = BytesMut::new();
    p.write(&mut b, usize::MAX)
        .map_err(|e| io::Error::other(e.to_string()))?;
    stream.write_all(&b)?;
    stream.flush()
}

enum ReadErr {
    Io(io::Error),
    TooLarge,
    Version,
    Malformed,
}

/// Reads at most one packet. `Ok(None)` means nothing complete arrived
/// within one tick. A packet over `max` is refused from its header, before
/// its body is read.
fn read_packet(
    stream: &mut Box<dyn Stream>,
    buf: &mut BytesMut,
    max: usize,
) -> Result<Option<Packet>, ReadErr> {
    loop {
        match Packet::read(buf, max) {
            Ok(p) => return Ok(Some(p)),
            Err(CodecError::InsufficientBytes(_)) => {
                let mut chunk = [0u8; 8192];
                match stream.read(&mut chunk) {
                    Ok(0) => return Err(ReadErr::Io(io::ErrorKind::UnexpectedEof.into())),
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    Err(e) if is_tick(&e) => return Ok(None),
                    Err(e) => return Err(ReadErr::Io(e)),
                }
            }
            Err(CodecError::PayloadSizeLimitExceeded(_)) => return Err(ReadErr::TooLarge),
            Err(CodecError::InvalidProtocol | CodecError::InvalidProtocolLevel(_)) => {
                return Err(ReadErr::Version);
            }
            Err(_) => return Err(ReadErr::Malformed),
        }
    }
}

fn is_tick(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

fn closed_words(e: &io::Error) -> String {
    use io::ErrorKind as K;
    match e.kind() {
        K::UnexpectedEof | K::ConnectionReset | K::ConnectionAborted | K::BrokenPipe => {
            "the connection closed".into()
        }
        K::TimedOut | K::WouldBlock => "dropped: it stopped taking messages".into(),
        _ => format!("the connection failed: {}", clean_text(&e.to_string(), 200)),
    }
}

/// A client made of fuzz bytes: reads them, and throws away what the
/// broker writes.
struct FuzzStream(io::Cursor<Vec<u8>>);

impl Read for FuzzStream {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.0.read(out)
    }
}

impl Write for FuzzStream {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        Ok(b.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn fuzz(data: &[u8]) {
    static SHARED: std::sync::OnceLock<Arc<Shared>> = std::sync::OnceLock::new();
    let shared = SHARED.get_or_init(Shared::for_fuzz);
    let id = shared.next_id.fetch_add(1, Ordering::SeqCst);
    let peer = SocketAddr::from(([192, 0, 2, 1], 50_000));
    session(
        shared,
        Box::new(FuzzStream(io::Cursor::new(data.to_vec()))),
        peer,
        id,
    );
}
