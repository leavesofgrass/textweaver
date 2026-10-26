//! The test broker, from the beta 5 MQTT client spike: a minimal MQTT
//! 3.1.1 broker with topic filters, QoS 1 delivery, retained messages per
//! topic, an optional login with a chosen refusal code, refused topics, a
//! kick of every connection, optional TLS, and counters. Loopback only.
//! Test code: it runs a tokio runtime, the client does not.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use rumqttc::mqttbytes::v4::{
    ConnAck, ConnectReturnCode, Packet, Publish, SubAck, SubscribeReasonCode, UnsubAck,
};
use rumqttc::mqttbytes::{Error as CodecError, QoS};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

pub const MAX_PACKET: usize = 1 << 20;

#[derive(Default)]
pub struct Opts {
    pub login: Option<(String, String)>,
    /// The CONNACK code for a wrong login (default: bad user name or
    /// password, 4; Mosquitto 2 sends not authorized, 5).
    pub refuse_code: Option<ConnectReturnCode>,
    /// Subscriptions to filters starting with this are refused.
    pub refuse_prefix: Option<String>,
    pub tls: Option<Arc<rustls::ServerConfig>>,
}

#[derive(Default)]
pub struct Stats {
    pub connects: AtomicU64,
    pub refused_logins: AtomicU64,
    pub subscribes: AtomicU64,
    pub pingreqs: AtomicU64,
    pub pubacks: AtomicU64,
    pub client_publishes: AtomicU64,
    pub client_ids: Mutex<Vec<String>>,
    pub keep_alives: Mutex<Vec<u16>>,
}

impl Stats {
    pub fn get(&self, c: &AtomicU64) -> u64 {
        c.load(Ordering::SeqCst)
    }
}

struct Shared {
    opts: Opts,
    stats: Stats,
    retained: Mutex<HashMap<String, Bytes>>,
    lines: broadcast::Sender<(String, Bytes)>,
    kick: broadcast::Sender<()>,
}

pub struct TestBroker {
    pub port: u16,
    shared: Arc<Shared>,
    rt: Option<tokio::runtime::Runtime>,
}

impl TestBroker {
    pub fn start(opts: Opts) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let (lines, _) = broadcast::channel(1024);
        let (kick, _) = broadcast::channel(4);
        let shared = Arc::new(Shared {
            opts,
            stats: Stats::default(),
            retained: Mutex::new(HashMap::new()),
            lines,
            kick,
        });
        let listener = rt
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("bind loopback");
        let port = listener.local_addr().unwrap().port();
        let s = shared.clone();
        rt.spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    break;
                };
                let s = s.clone();
                tokio::spawn(async move {
                    if let Some(cfg) = s.opts.tls.clone() {
                        let acceptor = tokio_rustls::TlsAcceptor::from(cfg);
                        if let Ok(tls) = acceptor.accept(tcp).await {
                            handle(s, tls).await;
                        }
                    } else {
                        handle(s, tcp).await;
                    }
                });
            }
        });
        TestBroker {
            port,
            shared,
            rt: Some(rt),
        }
    }

    pub fn stats(&self) -> &Stats {
        &self.shared.stats
    }

    /// Publishes as the broker itself (the logger's side in these tests).
    pub fn publish(&self, topic: &str, payload: &[u8], retain: bool) {
        let payload = Bytes::copy_from_slice(payload);
        if retain {
            self.shared
                .retained
                .lock()
                .unwrap()
                .insert(topic.to_string(), payload.clone());
        }
        let _ = self.shared.lines.send((topic.to_string(), payload));
    }

    /// Closes every client connection, as a broker restart would.
    pub fn kick_all(&self) {
        let _ = self.shared.kick.send(());
    }
}

impl Drop for TestBroker {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

pub fn matches(filter: &str, topic: &str) -> bool {
    let mut f = filter.split('/');
    let mut t = topic.split('/');
    loop {
        match (f.next(), t.next()) {
            (Some("#"), _) => return true,
            (Some("+"), Some(_)) => {}
            (Some(a), Some(b)) if a == b => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

async fn handle<S: AsyncRead + AsyncWrite + Unpin>(s: Arc<Shared>, stream: S) {
    let (mut r, mut w) = tokio::io::split(stream);
    let mut buf = BytesMut::with_capacity(1024);
    let connect = match tokio::time::timeout(Duration::from_secs(10), next(&mut r, &mut buf)).await
    {
        Ok(Some(Packet::Connect(c))) => c,
        _ => return,
    };
    s.stats
        .client_ids
        .lock()
        .unwrap()
        .push(connect.client_id.clone());
    s.stats.keep_alives.lock().unwrap().push(connect.keep_alive);
    if let Some((u, p)) = &s.opts.login {
        let ok = connect
            .login
            .as_ref()
            .is_some_and(|l| &l.username == u && &l.password == p);
        if !ok {
            s.stats.refused_logins.fetch_add(1, Ordering::SeqCst);
            let _ = send(
                &mut w,
                Packet::ConnAck(ConnAck::new(
                    s.opts
                        .refuse_code
                        .unwrap_or(ConnectReturnCode::BadUserNamePassword),
                    false,
                )),
            )
            .await;
            return;
        }
    }
    s.stats.connects.fetch_add(1, Ordering::SeqCst);
    if send(
        &mut w,
        Packet::ConnAck(ConnAck::new(ConnectReturnCode::Success, false)),
    )
    .await
    .is_err()
    {
        return;
    }
    let keep_alive = Duration::from_secs(u64::from(connect.keep_alive.max(1)) * 3 / 2 + 1);
    let mut subs: Vec<(String, QoS)> = Vec::new();
    let mut lines = s.lines.subscribe();
    let mut kick = s.kick.subscribe();
    let mut pkid: u16 = 0;

    loop {
        tokio::select! {
            _ = kick.recv() => break,
            pkt = tokio::time::timeout(keep_alive, next(&mut r, &mut buf)) => {
                let Ok(Some(pkt)) = pkt else { break };
                match pkt {
                    Packet::Subscribe(sub) => {
                        s.stats.subscribes.fetch_add(1, Ordering::SeqCst);
                        let mut codes = Vec::new();
                        let mut granted = Vec::new();
                        for f in &sub.filters {
                            let refused = s.opts.refuse_prefix.as_ref().is_some_and(|p| f.path.starts_with(p.as_str()));
                            if refused {
                                codes.push(SubscribeReasonCode::Failure);
                            } else {
                                let q = if f.qos == QoS::AtMostOnce { QoS::AtMostOnce } else { QoS::AtLeastOnce };
                                codes.push(SubscribeReasonCode::Success(q));
                                granted.push((f.path.clone(), q));
                            }
                        }
                        if send(&mut w, Packet::SubAck(SubAck::new(sub.pkid, codes))).await.is_err() { break; }
                        let retained: Vec<(String, Bytes)> = s.retained.lock().unwrap()
                            .iter().map(|(t, p)| (t.clone(), p.clone())).collect();
                        for (f, q) in &granted {
                            for (t, p) in &retained {
                                if matches(f, t) {
                                    let mut out = Publish::from_bytes(t.clone(), *q, p.clone());
                                    out.retain = true;
                                    if *q == QoS::AtLeastOnce { pkid = pkid % u16::MAX + 1; out.pkid = pkid; }
                                    if send(&mut w, Packet::Publish(out)).await.is_err() { return; }
                                }
                            }
                        }
                        subs.extend(granted);
                    }
                    Packet::Publish(_) => {
                        // The client must never publish.
                        s.stats.client_publishes.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                    Packet::PingReq => {
                        s.stats.pingreqs.fetch_add(1, Ordering::SeqCst);
                        if send(&mut w, Packet::PingResp).await.is_err() { break; }
                    }
                    Packet::PubAck(_) => { s.stats.pubacks.fetch_add(1, Ordering::SeqCst); }
                    Packet::Unsubscribe(u) => {
                        subs.clear();
                        if send(&mut w, Packet::UnsubAck(UnsubAck::new(u.pkid))).await.is_err() { break; }
                    }
                    _ => break,
                }
            }
            line = lines.recv() => {
                let Ok((topic, payload)) = line else { continue };
                let q = subs.iter().filter(|(f, _)| matches(f, &topic)).map(|(_, q)| *q).max_by_key(|q| *q as u8);
                if let Some(q) = q {
                    let mut out = Publish::from_bytes(topic, q, payload);
                    if q == QoS::AtLeastOnce { pkid = pkid % u16::MAX + 1; out.pkid = pkid; }
                    if send(&mut w, Packet::Publish(out)).await.is_err() { break; }
                }
            }
        }
    }
    let _ = w.shutdown().await;
}

async fn next<R: AsyncRead + Unpin>(r: &mut R, buf: &mut BytesMut) -> Option<Packet> {
    loop {
        match Packet::read(buf, MAX_PACKET) {
            Ok(p) => return Some(p),
            Err(CodecError::InsufficientBytes(_)) => match r.read_buf(buf).await {
                Ok(0) | Err(_) => return None,
                Ok(_) => {}
            },
            Err(_) => return None,
        }
    }
}

async fn send<W: AsyncWrite + Unpin>(w: &mut W, p: Packet) -> std::io::Result<()> {
    let mut b = BytesMut::new();
    p.write(&mut b, MAX_PACKET + 16)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    w.write_all(&b).await
}

/// A test CA and a broker certificate signed by it, made at run time.
/// The loopback tests write only the CA's public certificate; the
/// Mosquitto test also writes the broker's key, into the target folder.
pub struct TestPki {
    pub server: Arc<rustls::ServerConfig>,
    pub ca_pem: String,
    pub server_cert_pem: String,
    pub server_key_pem: String,
}

pub fn pki(names: &[&str]) -> TestPki {
    use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "textweaver test CA");
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);

    let leaf_key = KeyPair::generate().unwrap();
    let leaf_params =
        CertificateParams::new(names.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
    let leaf = leaf_params.signed_by(&leaf_key, &issuer).unwrap();

    let key = rustls::pki_types::PrivateKeyDer::try_from(leaf_key.serialize_der()).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![leaf.der().clone(), ca_cert.der().clone()], key)
        .unwrap();
    TestPki {
        server: Arc::new(server),
        ca_pem: ca_cert.pem(),
        server_cert_pem: leaf.pem(),
        server_key_pem: leaf_key.serialize_pem(),
    }
}

/// Writes the CA certificate (public, no key) to the test's own folder in
/// the target directory, never to drive C.
pub fn write_ca(name: &str, pem: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let path = dir.join(name);
    std::fs::write(&path, pem).unwrap();
    path
}

/// A started source with its wake signal, and the events seen so far.
pub struct Harness {
    pub source: Option<textweaver_mqtt::MqttSource>,
    woken: std::sync::mpsc::Receiver<()>,
    pub wakes: Arc<AtomicU64>,
}

impl Harness {
    pub fn start(config: textweaver_mqtt::MqttConfig) -> Self {
        let (tx, woken) = std::sync::mpsc::channel();
        let wakes = Arc::new(AtomicU64::new(0));
        let w = wakes.clone();
        let source = textweaver_mqtt::MqttSource::start(
            config,
            Box::new(move || {
                w.fetch_add(1, Ordering::SeqCst);
                let _ = tx.send(());
            }),
        );
        Harness {
            source: Some(source),
            woken,
            wakes,
        }
    }

    pub fn source(&self) -> &textweaver_mqtt::MqttSource {
        self.source.as_ref().unwrap()
    }

    /// Polls until an event matches, returning every event seen up to and
    /// including it. Panics with the events so far after `secs`.
    pub fn wait_for(
        &mut self,
        secs: u64,
        mut pred: impl FnMut(&textweaver_mqtt::MqttEvent) -> bool,
    ) -> Vec<textweaver_mqtt::MqttEvent> {
        let deadline = std::time::Instant::now() + Duration::from_secs(secs);
        let mut seen = Vec::new();
        loop {
            for ev in self.source().poll() {
                let hit = pred(&ev);
                seen.push(ev);
                if hit {
                    return seen;
                }
            }
            if std::time::Instant::now() >= deadline {
                panic!("timed out; events so far: {seen:#?}");
            }
            let _ = self.woken.recv_timeout(Duration::from_millis(50));
        }
    }

    /// Every event that arrives within `ms` milliseconds.
    pub fn collect(&mut self, ms: u64) -> Vec<textweaver_mqtt::MqttEvent> {
        let deadline = std::time::Instant::now() + Duration::from_millis(ms);
        let mut seen = Vec::new();
        while std::time::Instant::now() < deadline {
            seen.extend(self.source().poll());
            let _ = self.woken.recv_timeout(Duration::from_millis(50));
        }
        seen.extend(self.source().poll());
        seen
    }

    pub fn stop(mut self) -> Duration {
        let t = std::time::Instant::now();
        self.source.take().unwrap().stop();
        t.elapsed()
    }
}
