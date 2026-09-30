//! This computer's identity for sync (ADR-0049): a random [`DeviceId`] and
//! a random [`InstallToken`], both kept in the install marker in the local
//! state folder and both written to the computer's `device.json` in the
//! sync folder.
//!
//! The marker, `sync-install.json`, also holds a fingerprint of where the
//! state folder is: a 64-bit FNV-1a hash of the folder's full path and this
//! computer's name. Only the hash is stored, only in the state folder, and
//! it is never written to the sync folder.
//!
//! A copied state folder is caught two ways, and either way the computer
//! takes a fresh id and token and keeps its data:
//!
//! - the fingerprint does not match the folder the marker is in
//!   ([`Identity::load_or_create`] reports [`IdentityEvent::CopiedStateFolder`]);
//! - the sync folder shows this device id with another install token
//!   ([`crate::SyncFolder::open`] reports `fresh_device_id`).
//!
//! The computer's label ("laptop", "lab") is a setting, not part of the
//! marker; [`check_label`] and [`default_label`] check and suggest one.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{DeviceId, InstallToken, SyncError};

/// The install marker's file name, in the local state folder.
pub const MARKER_FILE: &str = "sync-install.json";

/// The longest label, in characters: it must fit a 40-cell Braille line.
pub const MAX_LABEL_CHARS: usize = 40;

/// The default label's first word; the default is "Computer 1", "Computer
/// 2", and so on, never the host name.
pub const DEFAULT_LABEL_WORD: &str = "Computer";

/// What loading the identity found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityEvent {
    /// The saved identity, unchanged.
    Loaded,
    /// None was saved (or the marker could not be read): a new one was made.
    Created,
    /// The marker's fingerprint does not match the folder it is in: the
    /// state folder was copied here, so a fresh id and token were taken.
    CopiedStateFolder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct MarkerFile {
    format: u32,
    device: DeviceId,
    install_token: InstallToken,
    /// 16 hex digits: FNV-1a 64 of the state folder's path and the
    /// computer's name.
    fingerprint: String,
}

/// This computer's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    device: DeviceId,
    token: InstallToken,
    fingerprint: String,
    file: PathBuf,
}

/// FNV-1a 64, stable across Rust versions (unlike `DefaultHasher`).
fn fnv1a(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in part.as_bytes().iter().chain(std::iter::once(&0u8)) {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// The fingerprint of `state_dir` on this computer.
fn fingerprint(state_dir: &Path) -> String {
    let full = std::fs::canonicalize(state_dir).unwrap_or_else(|_| state_dir.to_path_buf());
    let host = ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .find_map(|v| std::env::var(v).ok())
        .unwrap_or_default();
    format!("{:016x}", fnv1a(&[&full.to_string_lossy(), &host]))
}

impl Identity {
    /// Loads the identity from the install marker in `state_dir`; makes and
    /// saves one when there is none, and takes a fresh id and token when the
    /// marker was copied from another folder or computer.
    pub fn load_or_create(state_dir: &Path) -> Result<(Self, IdentityEvent), SyncError> {
        std::fs::create_dir_all(state_dir).map_err(|source| SyncError::Io {
            path: state_dir.to_owned(),
            source,
        })?;
        let file = state_dir.join(MARKER_FILE);
        let here = fingerprint(state_dir);
        let saved = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice::<MarkerFile>(&b).ok());
        let event = match &saved {
            Some(m) if m.fingerprint == here => {
                return Ok((
                    Self {
                        device: m.device,
                        token: m.install_token,
                        fingerprint: here,
                        file,
                    },
                    IdentityEvent::Loaded,
                ));
            }
            Some(_) => IdentityEvent::CopiedStateFolder,
            None => {
                if file.exists() {
                    log::warn!("sync install marker could not be read; a new identity is made");
                }
                IdentityEvent::Created
            }
        };
        let id = Self {
            device: DeviceId::random(),
            token: InstallToken::random(),
            fingerprint: here,
            file,
        };
        id.save()?;
        Ok((id, event))
    }

    /// Takes a fresh id and token and saves them (another installation
    /// holds this id in the sync folder).
    pub fn renew(&mut self) -> Result<(), SyncError> {
        self.device = DeviceId::random();
        self.token = InstallToken::random();
        self.save()
    }

    /// This computer's id.
    pub fn device(&self) -> DeviceId {
        self.device
    }

    /// This installation's token.
    pub fn token(&self) -> InstallToken {
        self.token
    }

    fn save(&self) -> Result<(), SyncError> {
        let body = MarkerFile {
            format: crate::FORMAT,
            device: self.device,
            install_token: self.token,
            fingerprint: self.fingerprint.clone(),
        };
        let json =
            serde_json::to_vec_pretty(&body).map_err(|e| SyncError::Damaged(e.to_string()))?;
        textweaver_store::atomic_write(&self.file, &json)?;
        Ok(())
    }
}

/// The environment variables that name this computer or its user.
const NAME_VARIABLES: [&str; 5] = ["USERNAME", "USER", "LOGNAME", "COMPUTERNAME", "HOSTNAME"];

/// The computer and user names this process can see, for refusing them as
/// labels and for the privacy tests.
pub fn local_names() -> Vec<String> {
    NAME_VARIABLES
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// A label, trimmed, if it may be used: not empty, at most
/// [`MAX_LABEL_CHARS`] characters, no control characters, and not this
/// computer's or its user's name, so the sync folder never carries them.
pub fn check_label(label: &str) -> Result<String, SyncError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(SyncError::Label("empty"));
    }
    if label.chars().count() > MAX_LABEL_CHARS {
        return Err(SyncError::Label("longer than 40 characters"));
    }
    if label.chars().any(char::is_control) {
        return Err(SyncError::Label("holds control characters"));
    }
    if local_names().iter().any(|n| n.eq_ignore_ascii_case(label)) {
        return Err(SyncError::Label("is this computer's or account's name"));
    }
    Ok(label.to_owned())
}

/// The first "Computer N" not in `taken`.
pub fn default_label<'a>(taken: impl IntoIterator<Item = &'a str>) -> String {
    let taken: Vec<&str> = taken.into_iter().collect();
    (1..)
        .map(|n| format!("{DEFAULT_LABEL_WORD} {n}"))
        .find(|l| !taken.iter().any(|t| t.eq_ignore_ascii_case(l)))
        .unwrap_or_else(|| format!("{DEFAULT_LABEL_WORD} 1"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn made_once_then_loaded() {
        let state = tempfile::tempdir().unwrap();
        let (a, e) = Identity::load_or_create(state.path()).unwrap();
        assert_eq!(e, IdentityEvent::Created);
        let (b, e) = Identity::load_or_create(state.path()).unwrap();
        assert_eq!(e, IdentityEvent::Loaded);
        assert_eq!(a, b);
    }

    #[test]
    fn a_copied_state_folder_takes_a_fresh_id() {
        let state = tempfile::tempdir().unwrap();
        let (a, _) = Identity::load_or_create(state.path()).unwrap();
        let copy = tempfile::tempdir().unwrap();
        std::fs::copy(
            state.path().join(MARKER_FILE),
            copy.path().join(MARKER_FILE),
        )
        .unwrap();
        let (b, e) = Identity::load_or_create(copy.path()).unwrap();
        assert_eq!(e, IdentityEvent::CopiedStateFolder);
        assert_ne!(a.device(), b.device());
        assert_ne!(a.token(), b.token());
        let (c, e) = Identity::load_or_create(copy.path()).unwrap();
        assert_eq!(e, IdentityEvent::Loaded);
        assert_eq!(b.device(), c.device());
        let (a2, e) = Identity::load_or_create(state.path()).unwrap();
        assert_eq!(e, IdentityEvent::Loaded);
        assert_eq!(a2.device(), a.device());
    }

    #[test]
    fn a_damaged_marker_makes_a_new_identity() {
        let state = tempfile::tempdir().unwrap();
        std::fs::write(state.path().join(MARKER_FILE), b"{\"format\":1,\"dev").unwrap();
        let (_, e) = Identity::load_or_create(state.path()).unwrap();
        assert_eq!(e, IdentityEvent::Created);
    }

    #[test]
    fn labels_are_checked() {
        assert_eq!(check_label("  lab  ").unwrap(), "lab");
        assert!(check_label("").is_err());
        assert!(check_label(&"x".repeat(41)).is_err());
        assert!(check_label("a\tb").is_err());
        for name in local_names() {
            assert!(check_label(&name).is_err(), "a local name was accepted");
        }
        assert_eq!(default_label(["Computer 1", "lab"]), "Computer 2");
        assert_eq!(default_label([]), "Computer 1");
    }
}
