//! Random 128-bit ids: [`DeviceId`] names a computer, [`SyncId`] names a
//! document, and [`LibraryId`] names a library folder. All are 32 lower-case hex digits and nothing else, so they are
//! safe as file names in the sync folder and say nothing about the computer,
//! its user, or the document's path.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::SyncError;

/// Hex digits in an id.
pub const ID_HEX_DIGITS: usize = 32;

/// 128 random bits, without a random-number crate.
///
/// The standard library seeds each thread's `RandomState` from the
/// operating system's random source; two SipHash outputs keyed that way,
/// over the time, the process id, and a counter, give ids that are unique
/// and unpredictable enough to name computers and documents. They are not
/// secrets and are never used as keys.
pub(crate) fn random_u128() -> u128 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut out: u128 = 0;
    for half in 0..2u8 {
        let mut h = RandomState::new().build_hasher();
        h.write_u8(half);
        h.write_u64(n);
        h.write_u128(nanos);
        h.write_u32(std::process::id());
        out = (out << 64) | u128::from(h.finish());
    }
    out
}

/// Whether `s` is exactly 32 lower-case hex digits.
fn is_id(s: &str) -> bool {
    s.len() == ID_HEX_DIGITS && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

macro_rules! hex_id {
    ($(#[$doc:meta])* $name:ident, $what:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u128);

        impl $name {
            /// A fresh random id.
            pub fn random() -> Self {
                Self(random_u128())
            }

            /// The id from its 128 bits (for tests and fixtures).
            pub const fn from_u128(bits: u128) -> Self {
                Self(bits)
            }

            /// Its 128 bits.
            pub const fn as_u128(self) -> u128 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:032x}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = SyncError;

            fn from_str(s: &str) -> Result<Self, SyncError> {
                if !is_id(s) {
                    return Err(SyncError::BadId { what: $what });
                }
                u128::from_str_radix(s, 16)
                    .map(Self)
                    .map_err(|_| SyncError::BadId { what: $what })
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = std::borrow::Cow::<'de, str>::deserialize(d)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

hex_id!(
    /// A computer taking part in sync: random, chosen once per
    /// installation, and replaced when a copied data folder is found
    /// ([`crate::Identity`]). Never derived from a host or user name.
    DeviceId,
    "computer id"
);

hex_id!(
    /// A document's sync id: random, the same on every computer once the
    /// document is recognized there ([`crate::docid`] finds or makes it).
    SyncId,
    "document id"
);

hex_id!(
    /// A library folder's id: random, made the first time a document in
    /// the folder is identified, and kept in the folder itself
    /// (`.textweaver/library-id.json`), so the same folder reached as
    /// `D:\Books` on one computer and `/home/student/Books` on another is
    /// recognized as one ([`crate::docid`]).
    LibraryId,
    "library folder id"
);

hex_id!(
    /// A random token for one installation, kept in the local install
    /// marker and in the computer's `device.json`, so a copied state folder
    /// is recognized.
    InstallToken,
    "install token"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_as_32_hex_digits() {
        let id = DeviceId::random();
        let s = id.to_string();
        assert_eq!(s.len(), 32);
        assert_eq!(s.parse::<DeviceId>().unwrap(), id);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<DeviceId>(&json).unwrap(), id);
        assert_eq!(DeviceId::from_u128(1).to_string(), format!("{:032x}", 1));
    }

    #[test]
    fn bad_ids_are_refused() {
        for bad in [
            "",
            "abc",
            "../../../../../../../../../../etc",
            "0123456789ABCDEF0123456789abcdef",
            "0123456789abcdef0123456789abcdef0",
            "+123456789abcdef0123456789abcdef",
        ] {
            assert!(bad.parse::<SyncId>().is_err(), "{bad}");
        }
    }

    #[test]
    fn random_ids_differ() {
        let a: std::collections::BTreeSet<_> = (0..1000).map(|_| SyncId::random()).collect();
        assert_eq!(a.len(), 1000);
    }
}
