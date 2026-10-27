//! The broker's key and self-signed certificate, and its TLS settings.
//!
//! - **Accounts mode** keeps one key and certificate in textweaver's data
//!   folder (`broker-key.pem`, `broker-cert.pem`), made by rcgen on the
//!   first start, so the pin readers save stays the same across restarts.
//! - **Join code mode** makes a fresh key and certificate in memory for
//!   each session, so an old join code never works again.
//!
//! The certificate names `localhost`, `127.0.0.1` and `::1`, so a
//! Mosquitto client can also trust it as a CA file on this computer;
//! textweaver's readers pin the key instead and ignore names.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use rustls::ServerConfig;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use textweaver_mqtt::Pin;

/// The certificate's file name in the data folder.
pub const CERT_FILE: &str = "broker-cert.pem";
/// The private key's file name in the data folder.
pub const KEY_FILE: &str = "broker-key.pem";

/// A key, its certificate's pin, and the TLS settings that serve them.
pub(crate) struct Identity {
    pub(crate) config: Arc<ServerConfig>,
    pub(crate) pin: Pin,
}

/// The data folder's key and certificate, made on first use. The flag says
/// whether they were made now.
pub(crate) fn stable(dir: &Path) -> Result<(Identity, bool), String> {
    let cert_path = dir.join(CERT_FILE);
    let key_path = dir.join(KEY_FILE);
    let made = match (cert_path.exists(), key_path.exists()) {
        (true, true) => false,
        (false, false) => {
            let (cert_pem, key_pem) = generate()?;
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("TLS error: cannot make {}: {e}.", dir.display()))?;
            write_private(&key_path, key_pem.as_bytes())
                .map_err(|e| format!("TLS error: cannot write {}: {e}.", key_path.display()))?;
            std::fs::write(&cert_path, cert_pem)
                .map_err(|e| format!("TLS error: cannot write {}: {e}.", cert_path.display()))?;
            true
        }
        (true, false) => {
            return Err(format!(
                "TLS error: the broker's key {} is missing, but its certificate is there. Move the certificate away to make a new pair; readers then need the new pin.",
                key_path.display()
            ));
        }
        (false, true) => {
            return Err(format!(
                "TLS error: the broker's certificate {} is missing, but its key is there. Move the key away to make a new pair; readers then need the new pin.",
                cert_path.display()
            ));
        }
    };
    let cert = CertificateDer::from_pem_file(&cert_path).map_err(|e| {
        format!(
            "TLS error: cannot read the certificate {}: {e}.",
            cert_path.display()
        )
    })?;
    let key = PrivateKeyDer::from_pem_file(&key_path).map_err(|e| {
        format!(
            "TLS error: cannot read the key {}: {e}.",
            key_path.display()
        )
    })?;
    Ok((identity(cert, key)?, made))
}

/// A fresh key and certificate, in memory only.
pub(crate) fn fresh() -> Result<Identity, String> {
    let ck = rcgen::generate_simple_self_signed(names())
        .map_err(|e| format!("TLS error: cannot make a certificate: {e}."))?;
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
    identity(ck.cert.der().clone(), key)
}

/// The pin of the data folder's certificate, making the pair if there is
/// none, and whether it was made now.
pub fn stable_pin(dir: &Path) -> Result<(Pin, bool), String> {
    stable(dir).map(|(id, made)| (id.pin, made))
}

fn names() -> Vec<String> {
    vec![
        "textweaver-broker".into(),
        "localhost".into(),
        "127.0.0.1".into(),
        "::1".into(),
    ]
}

fn generate() -> Result<(String, String), String> {
    let ck = rcgen::generate_simple_self_signed(names())
        .map_err(|e| format!("TLS error: cannot make a certificate: {e}."))?;
    Ok((ck.cert.pem(), ck.signing_key.serialize_pem()))
}

fn identity(
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> Result<Identity, String> {
    let pin = Pin::of_certificate(&cert)
        .ok_or("TLS error: the broker's certificate holds no readable public key.")?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("TLS error: {e}."))?
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| format!("TLS error: the key does not fit the certificate: {e}."))?;
    Ok(Identity {
        config: Arc::new(config),
        pin,
    })
}

/// Writes a file only its owner can read on Unix (mode 0600); on Windows
/// it takes the folder's permissions, which for the data folder are the
/// user's own.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_identity_keeps_its_pin() {
        let dir = tempfile::tempdir().unwrap();
        let (a, made) = stable(dir.path()).unwrap();
        assert!(made);
        let (b, made) = stable(dir.path()).unwrap();
        assert!(!made);
        assert_eq!(a.pin, b.pin);
        assert!(!a.pin.is_short());
        let (c, _) = stable_pin(dir.path()).unwrap();
        assert_eq!(c, a.pin);
        assert_ne!(fresh().unwrap().pin, a.pin);
    }

    #[test]
    fn half_a_pair_is_said_in_words() {
        let dir = tempfile::tempdir().unwrap();
        stable(dir.path()).unwrap();
        let moved = dir.path().join("old-key.pem");
        std::fs::rename(dir.path().join(KEY_FILE), &moved).unwrap();
        let err = stable(dir.path()).err().unwrap();
        assert!(err.starts_with("TLS error: the broker's key"), "{err}");
    }
}
