//! TLS for the client: textweaver's own rustls with the ring provider,
//! trusting the bundled Mozilla roots (`webpki-roots`), a CA file the user
//! names, or exactly one public key by its pin. There is no way to turn
//! certificate checking off.

use std::net::IpAddr;
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};

use crate::Tls;
use crate::error::{Failure, Kind};
use crate::pin::{Pin, spki_of_certificate};
use crate::reason::DisconnectReason;

/// Builds the rustls client configuration. Uses the ring provider
/// directly, so it never depends on a process-wide default provider.
pub(crate) fn client_config(tls: &Tls) -> Result<Arc<ClientConfig>, Failure> {
    let mut roots = RootCertStore::empty();
    match tls {
        Tls::Off => {
            return Err(Failure::new(Kind::Config, DisconnectReason::TlsOnPlain));
        }
        Tls::System => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
        Tls::Pinned(pin) => return pinned_config(pin),
        Tls::CaFile(path) => {
            let shown = || path.display().to_string();
            let iter = CertificateDer::pem_file_iter(path).map_err(|e| {
                Failure::new(
                    Kind::Config,
                    DisconnectReason::CaUnreadable {
                        path: shown(),
                        error: e.to_string(),
                    },
                )
            })?;
            let mut found = 0;
            for cert in iter {
                let cert = cert.map_err(|e| {
                    Failure::new(
                        Kind::Config,
                        DisconnectReason::CaNotPem {
                            path: shown(),
                            error: e.to_string(),
                        },
                    )
                })?;
                roots.add(cert).map_err(|e| {
                    Failure::new(
                        Kind::Config,
                        DisconnectReason::CaCertificateRefused {
                            path: shown(),
                            error: e.to_string(),
                        },
                    )
                })?;
                found += 1;
            }
            if found == 0 {
                return Err(Failure::new(
                    Kind::Config,
                    DisconnectReason::CaEmpty { path: shown() },
                ));
            }
        }
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(tls_setup_failure)?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// A TLS library error while setting up.
fn tls_setup_failure(e: rustls::Error) -> Failure {
    Failure::new(
        Kind::Tls,
        DisconnectReason::Tls {
            error: e.to_string(),
        },
    )
}

/// A configuration that trusts exactly the key with this pin, whatever
/// names or dates its certificate carries: the broker proves it holds the
/// key by signing the handshake, which rustls checks with that key.
fn pinned_config(pin: &Pin) -> Result<Arc<ClientConfig>, Failure> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = Arc::new(PinVerifier {
        pin: pin.clone(),
        algs: provider.signature_verification_algorithms,
    });
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(tls_setup_failure)?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Accepts the one certificate whose public key has the pin, and checks
/// the handshake signature with that key, so the broker holds the key.
#[derive(Debug)]
struct PinVerifier {
    pin: Pin,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let spki = spki_of_certificate(end_entity).ok_or(rustls::Error::InvalidCertificate(
            rustls::CertificateError::BadEncoding,
        ))?;
        if self.pin.matches_spki(spki) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

/// The name the certificate must carry: a DNS name, or an IP address
/// (rustls checks IP address entries in the certificate too).
pub(crate) fn server_name(host: &str) -> Result<ServerName<'static>, Failure> {
    ServerName::try_from(host.to_string()).map_err(|_| {
        Failure::new(
            Kind::Config,
            DisconnectReason::BadTlsHost {
                host: host.to_owned(),
            },
        )
    })
}

/// Why, for a rustls error, meaning first.
pub(crate) fn describe(err: &rustls::Error) -> DisconnectReason {
    use DisconnectReason as R;
    use rustls::{CertificateError, Error};
    match err {
        Error::InvalidCertificate(c) => match c {
            CertificateError::UnknownIssuer => R::UnknownIssuer,
            CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. } => {
                R::WrongHostName
            }
            CertificateError::Expired | CertificateError::ExpiredContext { .. } => R::Expired,
            CertificateError::NotValidYet | CertificateError::NotValidYetContext { .. } => {
                R::NotValidYet
            }
            CertificateError::Revoked => R::Revoked,
            // Only the pinned verifier gives this.
            CertificateError::ApplicationVerificationFailure => R::PinMismatch,
            other => R::CertificateRefused {
                detail: format!("{other:?}"),
            },
        },
        Error::AlertReceived(alert) => R::HandshakeAlert {
            detail: format!("{alert:?}"),
        },
        Error::NoCertificatesPresented => R::NoCertificate,
        Error::PeerIncompatible(p) => R::Incompatible {
            detail: format!("{p:?}"),
        },
        Error::InvalidMessage(m) => R::NotTls {
            detail: format!("{m:?}"),
        },
        other => R::Tls {
            error: other.to_string(),
        },
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
            err.reason
                .to_string()
                .starts_with("Settings error: cannot read the CA file")
        );
    }

    #[test]
    fn bundled_roots_build() {
        assert!(client_config(&Tls::System).is_ok());
    }
}
