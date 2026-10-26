//! Name lookup and TCP connection, both watching the stop flag.
//!
//! Every resolved address is tried at once, each on its own short-lived
//! thread, and the first to connect wins. Windows tries `::1` first for
//! `localhost`, and a refused connection there takes about two seconds;
//! a broker listening on IPv4 only would cost that on every connect if the
//! addresses were tried one by one.

use std::io;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use crate::error::{Failure, Kind, Stage};

/// The most addresses tried at once.
const MAX_PARALLEL: usize = 8;

/// How often the waits check the stop flag.
const TICK: Duration = Duration::from_millis(100);

/// Connects to `host:port`. `Ok(None)` means the stop flag was set.
pub(crate) fn connect(
    host: &str,
    port: u16,
    timeout: Duration,
    stop: &AtomicBool,
) -> Result<Option<(TcpStream, SocketAddr)>, Failure> {
    let target = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let Some(addrs) = resolve(host, port, timeout, stop)? else {
        return Ok(None);
    };
    if addrs.is_empty() {
        return Err(Failure::new(
            Kind::Unreachable,
            format!("Unreachable: the host {host} has no address."),
        ));
    }
    let addrs: Vec<SocketAddr> = addrs.into_iter().take(MAX_PARALLEL).collect();
    let (tx, rx) = channel();
    let mut errors: Vec<Option<io::Error>> = Vec::new();
    for (i, addr) in addrs.iter().copied().enumerate() {
        let tx = tx.clone();
        let spawned = std::thread::Builder::new()
            .name("mqtt-connect".into())
            .spawn(move || {
                let r = TcpStream::connect_timeout(&addr, timeout);
                // The receiver may be gone: a later stream is then dropped.
                let _ = tx.send((i, addr, r));
            });
        errors.push(spawned.err());
    }
    drop(tx);
    loop {
        match rx.recv_timeout(TICK) {
            Ok((_, addr, Ok(stream))) => return Ok(Some((stream, addr))),
            Ok((i, _, Err(e))) => errors[i] = Some(e),
            Err(RecvTimeoutError::Timeout) => {
                if stop.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    // Every address failed: report the first in the resolver's order.
    let first = errors.into_iter().flatten().next();
    Err(match first {
        Some(e) => Failure::from_io(&e, Stage::Connect, &target),
        None => Failure::new(
            Kind::Unreachable,
            format!("Unreachable: cannot connect to {target}."),
        ),
    })
}

/// Resolves the host on a helper thread, so a slow lookup neither delays a
/// stop nor runs past the timeout. `Ok(None)` means the stop flag was set.
fn resolve(
    host: &str,
    port: u16,
    timeout: Duration,
    stop: &AtomicBool,
) -> Result<Option<Vec<SocketAddr>>, Failure> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return Ok(Some(vec![SocketAddr::new(ip, port)]));
    }
    let not_found = |why: String| {
        Failure::new(
            Kind::Unreachable,
            format!("Unreachable: cannot find the host {host}: {why}."),
        )
    };
    let (tx, rx) = channel();
    let name = host.to_string();
    std::thread::Builder::new()
        .name("mqtt-resolve".into())
        .spawn(move || {
            let r = (name.as_str(), port)
                .to_socket_addrs()
                .map(|it| it.collect::<Vec<_>>());
            let _ = tx.send(r);
        })
        .map_err(|e| not_found(e.to_string()))?;
    let deadline = Instant::now() + timeout;
    loop {
        match rx.recv_timeout(TICK) {
            Ok(Ok(addrs)) => return Ok(Some(addrs)),
            Ok(Err(e)) => {
                return Err(not_found(crate::sanitize::clean_text(&e.to_string(), 300)));
            }
            Err(RecvTimeoutError::Timeout) => {
                if stop.load(Ordering::SeqCst) {
                    return Ok(None);
                }
                if Instant::now() >= deadline {
                    return Err(not_found("no answer in time".into()));
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(not_found("the lookup failed".into()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_connects_quickly_to_an_ipv4_only_listener() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = AtomicBool::new(false);
        let t = Instant::now();
        let (_, addr) = connect("localhost", port, Duration::from_secs(5), &stop)
            .unwrap()
            .unwrap();
        assert!(addr.ip().is_loopback());
        // Sequential tries cost about 2 s on Windows; generous for slow CI.
        assert!(
            t.elapsed() < Duration::from_millis(1500),
            "{:?}",
            t.elapsed()
        );
    }

    #[test]
    fn stop_ends_a_connect() {
        let stop = AtomicBool::new(true);
        // A documentation address (TEST-NET-1) that never answers.
        let r = connect("192.0.2.1", 1883, Duration::from_secs(10), &stop);
        assert!(matches!(r, Ok(None)));
    }
}
