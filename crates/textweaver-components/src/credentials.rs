//! Signing in to a private components source on GitHub (B1-c2).
//!
//! A repository source (`[components] source = "owner/name"`) that is
//! private needs a token. Where it comes from, in order:
//!
//! 1. **The GitHub CLI**, when it is installed and signed in: `gh auth
//!    token` ([`gh_token`]), a process call with no window. The token stays
//!    with gh; textweaver keeps no copy of it.
//! 2. **The system credential store** ([`stored_token`]): Windows Credential
//!    Manager, macOS Keychain, or the Secret Service on Linux. The app asks
//!    for a token once and keeps it there ([`store_token`]), and nowhere
//!    else; [`forget_token`] removes it.
//!
//! A folder source needs no sign-in, and neither does a public repository.
//!
//! The token never goes into settings, the log, a message, or a file: a
//! [`Token`] prints as "hidden", and only the signed-in fetcher
//! ([`SignedInFetcher`](crate::SignedInFetcher)) reads it, to send it to
//! GitHub's API and nowhere else.

use std::sync::Mutex;

/// A GitHub token. It is never printed: `{:?}` says `Token(hidden)`, and
/// there is no `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// A token from what was typed or pasted (spaces around it are
    /// dropped), or `None` when it cannot be one: shorter than 20
    /// characters, longer than 255, or holding a space or anything other
    /// than printable ASCII. GitHub's tokens (`ghp_`, `gho_`,
    /// `github_pat_`) all fit.
    pub fn new(text: &str) -> Option<Token> {
        let t = text.trim();
        let ok = (20..=255).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_graphic());
        ok.then(|| Token(t.to_owned()))
    }

    /// The token itself, for the `Authorization` header only.
    pub(crate) fn secret(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(hidden)")
    }
}

/// Where a token came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignIn {
    /// The GitHub CLI's own sign-in (`gh auth token`).
    GitHubCli,
    /// The token kept in the system credential store.
    Stored,
}

/// Tests only ([`fake::memory_credentials`](crate::fake::memory_credentials)):
/// `Some` replaces the system credential store with this memory, and `gh`
/// is never run, so no test reads or writes a real token.
static MEMORY: Mutex<Option<Option<String>>> = Mutex::new(None);

/// Switches this process to an empty memory store and no `gh` (tests).
pub(crate) fn use_memory() {
    if let Ok(mut m) = MEMORY.lock() {
        *m = Some(None);
    }
}

/// Runs `f` on the memory store, when tests switched to it.
fn memory<T>(f: impl FnOnce(&mut Option<String>) -> T) -> Option<T> {
    let mut m = MEMORY.lock().ok()?;
    m.as_mut().map(f)
}

/// The token to use: the GitHub CLI's when it is signed in, else the one
/// in the credential store, else none.
pub fn find_token() -> Option<(Token, SignIn)> {
    if let Some(t) = gh_token() {
        return Some((t, SignIn::GitHubCli));
    }
    stored_token().map(|t| (t, SignIn::Stored))
}

/// The GitHub CLI's token (`gh auth token --hostname github.com`), when
/// `gh` is on the PATH and signed in. Nothing is said or logged about the
/// answer; a `gh` that is missing, signed out, or slow (10 seconds) gives
/// none.
pub fn gh_token() -> Option<Token> {
    if memory(|_| ()).is_some() {
        return None;
    }
    let gh = textweaver_core::process::find_program("gh")?;
    let out = textweaver_core::process::run_with_timeout(
        &gh,
        &["auth", "token", "--hostname", "github.com"],
        std::time::Duration::from_secs(10),
    )?;
    Token::new(&out)
}

/// The token kept in the system credential store, if any. A store that
/// cannot be reached is logged by its reason (never the token) and gives
/// none.
pub fn stored_token() -> Option<Token> {
    if let Some(t) = memory(|m| m.clone()) {
        return t.and_then(|t| Token::new(&t));
    }
    store::get()
}

/// Keeps `token` in the system credential store, replacing any kept
/// before. The error, in words, never holds the token.
pub fn store_token(token: &Token) -> Result<(), String> {
    if memory(|m| *m = Some(token.secret().to_owned())).is_some() {
        return Ok(());
    }
    store::set(token)
}

/// Removes the token from the system credential store: `Ok(true)` when
/// one was there. The GitHub CLI's own sign-in is not touched (`gh auth
/// logout` does that).
pub fn forget_token() -> Result<bool, String> {
    if let Some(had) = memory(|m| m.take().is_some()) {
        return Ok(had);
    }
    store::forget()
}

/// The system credential store, through `keyring` (feature `download`).
#[cfg(feature = "download")]
mod store {
    use super::Token;

    /// The credential store entry's service: the program, nothing
    /// personal.
    const SERVICE: &str = "textweaver";

    /// The credential store entry's account: the host the token is for,
    /// nothing personal.
    const ACCOUNT: &str = "github.com";

    /// Why a store call failed, in words, without the token or anything
    /// the store echoed back.
    fn reason(e: &keyring::Error) -> String {
        match e {
            keyring::Error::NoDefaultStore => "this computer has no credential store".to_owned(),
            keyring::Error::NoStorageAccess(_) => {
                "the credential store is locked or refused access".to_owned()
            }
            _ => "the credential store failed".to_owned(),
        }
    }

    pub(super) fn get() -> Option<Token> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)
            .map_err(|e| log::warn!("credential store: {}", reason(&e)))
            .ok()?;
        match entry.get_password() {
            Ok(t) => Token::new(&t),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => {
                log::warn!("credential store: {}", reason(&e));
                None
            }
        }
    }

    pub(super) fn set(token: &Token) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| reason(&e))?;
        entry.set_password(token.secret()).map_err(|e| reason(&e))
    }

    pub(super) fn forget() -> Result<bool, String> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| reason(&e))?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(reason(&e)),
        }
    }
}

/// Without downloads there is nothing to sign in to, and no store.
#[cfg(not(feature = "download"))]
mod store {
    use super::Token;

    pub(super) fn get() -> Option<Token> {
        None
    }

    pub(super) fn set(_token: &Token) -> Result<(), String> {
        Err("this build of textweaver has no credential store".to_owned())
    }

    pub(super) fn forget() -> Result<bool, String> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_checked_and_never_printed() {
        let t = Token::new("  ghp_EXAMPLEonlyNotARealToken0000  ").unwrap();
        assert_eq!(t.secret(), "ghp_EXAMPLEonlyNotARealToken0000");
        assert_eq!(format!("{t:?}"), "Token(hidden)");
        assert!(Token::new("y").is_none());
        assert!(Token::new("ghp_has a space in it 0000000").is_none());
        assert!(Token::new(&"x".repeat(256)).is_none());
    }

    #[test]
    fn the_memory_store_keeps_and_forgets() {
        use_memory();
        assert_eq!(gh_token(), None, "tests never run gh");
        let t = Token::new("gho_EXAMPLEonlyNotARealToken1111").unwrap();
        store_token(&t).unwrap();
        assert_eq!(stored_token(), Some(t.clone()));
        assert_eq!(find_token(), Some((t, SignIn::Stored)));
        assert_eq!(forget_token(), Ok(true));
        assert_eq!(forget_token(), Ok(false));
        assert_eq!(stored_token(), None);
    }
}
