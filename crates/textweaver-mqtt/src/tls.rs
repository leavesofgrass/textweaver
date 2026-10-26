//! TLS for the client: textweaver's own rustls with the ring provider,
//! trusting the bundled Mozilla roots (`webpki-roots`) or a CA file the
//! user names. There is no way to turn certificate checking off.

use std::net::IpAddr;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};

use crate::Tls;
use crate::error::{Failure, Kind};

/// Builds the rustls client configuration. Uses the ring provider
/// directly, so it never depends on a process-wide default provider.
pub(crate) fn client_config(tls: &Tls) -> Result<Arc<ClientConfig>, Failure> {
    let mut roots = RootCertStore::empty();
    match tls {
        Tls::Off => {
            return Err(Failure::new(
                Kind::Config,
                "Settings error: TLS was asked for on a plain connection.",
            ));
        }
        Tls::System => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
        Tls::CaFile(path) => {
            let shown = path.display();
            let iter = CertificateDer::pem_file_iter(path).map_err(|e| {
                Failure::new(
                    Kind::Config,
                    format!("Settings error: cannot read the CA file {shown}: {e}."),
                )
            })?;
            let mut found = 0;
            for cert in iter {
                let cert = cert.map_err(|e| {
                    Failure::new(
                        Kind::Config,
                        format!("Settings error: the CA file {shown} is not valid PEM: {e}."),
                    )
                })?;
                roots.add(cert).map_err(|e| {
                    Failure::new(
                        Kind::Config,
                        format!("Settings error: a certificate in {shown} was refused: {e}."),
                    )
                })?;
                found += 1;
            }
            if found == 0 {
                return Err(Failure::new(
                    Kind::Config,
                    format!("Settings error: the CA file {shown} holds no certificate."),
                ));
            }
        }
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| Failure::new(Kind::Tls, format!("TLS failure: {e}.")))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// The name the certificate must carry: a DNS name, or an IP address
/// (rustls checks IP address entries in the certificate too).
pub(crate) fn server_name(host: &str) -> Result<ServerName<'static>, Failure> {
    ServerName::try_from(host.to_string()).map_err(|_| {
        Failure::new(
            Kind::Config,
            format!("Settings error: {host} is not a valid host name for TLS."),
        )
    })
}

/// Words for a rustls error, meaning first.
pub(crate) fn describe(err: &rustls::Error) -> String {
    use rustls::{CertificateError, Error};
    match err {
        Error::InvalidCertificate(c) => match c {
            CertificateError::UnknownIssuer => "TLS failure: the broker's certificate is not trusted. Its issuer is unknown; set a CA file if the broker uses a private certificate.".into(),
            CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => "TLS failure: the broker's certificate is for a different host name.".into(),
            CertificateError::Expired | CertificateError::ExpiredContext { .. } => "TLS failure: the broker's certificate has expired.".into(),
            CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => "TLS failure: the broker's certificate is not valid yet. Check this computer's clock.".into(),
            CertificateError::Revoked => "TLS failure: the broker's certificate was revoked.".into(),
            other => format!("TLS failure: the broker's certificate was refused ({other:?})."),
        },
        Error::AlertReceived(alert) => format!("TLS failure: the broker ended the handshake ({alert:?})."),
        Error::NoCertificatesPresented => "TLS failure: the broker sent no certificate.".into(),
        Error::PeerIncompatible(p) => format!("TLS failure: the broker's TLS settings are not compatible ({p:?})."),
        Error::InvalidMessage(m) => format!("TLS failure: the broker did not answer with TLS ({m:?}). Is this a plain port?"),
        other => format!("TLS failure: {other}."),
    }
}

/// True when the host is on a Tailscale network: an address in
/// 100.64.0.0/10 (Tailscale's IPv4 range, which is also the carrier-grade
/// NAT range, so this is a strong hint rather than proof), Tailscale's
/// IPv6 prefix fd7a:115c:a1e0::/48, or a MagicDNS name ending in
/// `.ts.net`.
pub fn is_tailscale_host(host: &str) -> bool {
    let h = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if h.ends_with(".ts.net") {
        return true;
    }
    h.parse::<IpAddr>().is_ok_and(is_tailscale_addr)
}

/// True for an address in Tailscale's ranges, 100.64.0.0/10 or
/// fd7a:115c:a1e0::/48. Used on the resolved peer too, so a MagicDNS short
/// name that resolves to 100.x is recognized.
pub fn is_tailscale_addr(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 100 && (o[1] & 0xC0) == 64
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_tailscale_addr(IpAddr::V4(v4));
            }
            let s = v6.segments();
            s[0] == 0xfd7a && s[1] == 0x115c && s[2] == 0xa1e0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailscale_hosts() {
        assert!(is_tailscale_host("100.64.0.1"));
        assert!(is_tailscale_host("100.127.255.254"));
        assert!(!is_tailscale_host("100.128.0.1"));
        assert!(!is_tailscale_host("100.63.255.255"));
        assert!(is_tailscale_host("logger.tail1234.ts.net"));
        assert!(is_tailscale_host("LOGGER.tail1234.TS.NET."));
        assert!(is_tailscale_host("fd7a:115c:a1e0::1"));
        assert!(is_tailscale_host("[fd7a:115c:a1e0::1]"));
        assert!(!is_tailscale_host("fd7a:115c:a1e1::1"));
        assert!(!is_tailscale_host("44.0.0.1"));
        assert!(!is_tailscale_host("example.net"));
        assert!(!is_tailscale_host("ts.net.example.com"));
        assert!(is_tailscale_addr("::ffff:100.100.1.1".parse().unwrap()));
    }

    #[test]
    fn missing_ca_file_is_a_settings_error_in_words() {
        let err = client_config(&Tls::CaFile("no-such-ca.pem".into())).unwrap_err();
        assert_eq!(err.kind, Kind::Config);
        assert!(
            err.text
                .starts_with("Settings error: cannot read the CA file")
        );
    }

    #[test]
    fn bundled_roots_build() {
        assert!(client_config(&Tls::System).is_ok());
    }
}
