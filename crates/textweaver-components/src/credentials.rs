//! Secrets kept in the system credential store: the GitHub token for a
//! private components source (B1-c2), and the password of each live
//! source (beta 5).
//!
//! Every entry has the service `textweaver` and an account naming what it
//! is for, never a person: `github.com` for the token, `live:<id>` for the
//! live source `id` ([`live_account`]). Only a build with the
//! `credentials` feature (which `download` brings) has the store; without
//! it, [`has_store`] is false and storing says so in words ([`NO_STORE`]).
//!
//! # The GitHub token
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
//! [`Token`] prints as "hidden", and only the signed-in fetcher (with
//! downloads) reads it, to send it to GitHub's API and nowhere else.
//!
//! # Live source passwords
//!
//! A live source that signs in to its broker has its user name in
//! `settings.toml` and its password here only: [`store_password`],
//! [`stored_password`], [`forget_password`]. A [`Password`] is any
//! printable text up to [`PASSWORD_MAX_CHARS`] characters; like a token it
//! prints as "hidden" and has no `Display`.

use std::collections::BTreeMap;
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

/// The longest password a [`Password`] holds, in characters.
pub const PASSWORD_MAX_CHARS: usize = 1024;

/// A password, such as a live source's. It is never printed: `{:?}` says
/// `Password(hidden)`, and there is no `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    /// A password from what was typed or pasted, kept exactly (spaces
    /// included), or `None` when it cannot be one: empty, longer than
    /// [`PASSWORD_MAX_CHARS`] characters, or holding a control character
    /// (a line break, a tab, an escape).
    pub fn new(text: &str) -> Option<Password> {
        let ok = !text.is_empty()
            && text.chars().count() <= PASSWORD_MAX_CHARS
            && !text.chars().any(char::is_control);
        ok.then(|| Password(text.to_owned()))
    }

    /// The password itself, to sign in with and for nothing else: never
    /// to show, log, or write to a file.
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(hidden)")
    }
}

/// Why a secret cannot be kept in this build, in words.
pub const NO_STORE: &str =
    "this build of textweaver has no credential store, so it cannot keep a password";

/// The credential store account for the live source `id`: `live:<id>`.
pub fn live_account(id: &str) -> String {
    format!("live:{id}")
}

/// True when this build has a credential store to keep secrets in. It may
/// still be locked or missing on this computer; storing then says why.
pub fn has_store() -> bool {
    memory(|_| ()).is_some() || cfg!(feature = "credentials")
}

/// The password kept for the live source `id`, if any. A store that
/// cannot be reached is logged by its reason (never the password) and
/// gives none.
pub fn stored_password(id: &str) -> Option<Password> {
    get(&live_account(id)).and_then(|p| Password::new(&p))
}

/// Keeps `password` for the live source `id` in the system credential
/// store, replacing any kept before. The error, in words, never holds the
/// password.
pub fn store_password(id: &str, password: &Password) -> Result<(), String> {
    if !has_store() {
        return Err(NO_STORE.to_owned());
    }
    set(&live_account(id), password.reveal())
}

/// Removes the live source `id`'s password from the system credential
/// store: `Ok(true)` when one was there.
pub fn forget_password(id: &str) -> Result<bool, String> {
    forget(&live_account(id))
}

/// Where a token came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignIn {
    /// The GitHub CLI's own sign-in (`gh auth token`).
    GitHubCli,
    /// The token kept in the system credential store.
    Stored,
}

/// Tests only (`fake::memory_credentials`): `Some` replaces the system
/// credential store with this memory, by account, and `gh` is never run,
/// so no test reads or writes a real secret.
static MEMORY: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

/// Switches this process to a memory store and no `gh` (tests). The first
/// call makes it empty; later calls keep what other tests in the same
/// process stored, so tests running at once do not wipe each other.
pub(crate) fn use_memory() {
    if let Ok(mut m) = MEMORY.lock()
        && m.is_none()
    {
        *m = Some(BTreeMap::new());
    }
}

/// Runs `f` on the memory store, when tests switched to it.
fn memory<T>(f: impl FnOnce(&mut BTreeMap<String, String>) -> T) -> Option<T> {
    let mut m = MEMORY.lock().ok()?;
    m.as_mut().map(f)
}

/// The secret kept for `account`: the memory's in tests, else the system
/// store's.
fn get(account: &str) -> Option<String> {
    if let Some(v) = memory(|m| m.get(account).cloned()) {
        return v;
    }
    store::get(account)
}

/// Keeps `secret` for `account`, replacing any kept before.
fn set(account: &str, secret: &str) -> Result<(), String> {
    if memory(|m| m.insert(account.to_owned(), secret.to_owned())).is_some() {
        return Ok(());
    }
    store::set(account, secret)
}

/// Removes `account`'s secret: `Ok(true)` when one was there.
fn forget(account: &str) -> Result<bool, String> {
    if let Some(had) = memory(|m| m.remove(account).is_some()) {
        return Ok(had);
    }
    store::forget(account)
}

/// The GitHub token's account: the host it is for, nothing personal.
const GITHUB_ACCOUNT: &str = "github.com";

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
    get(GITHUB_ACCOUNT).and_then(|t| Token::new(&t))
}

/// Keeps `token` in the system credential store, replacing any kept
/// before. The error, in words, never holds the token.
pub fn store_token(token: &Token) -> Result<(), String> {
    set(GITHUB_ACCOUNT, token.secret())
}

/// Removes the token from the system credential store: `Ok(true)` when
/// one was there. The GitHub CLI's own sign-in is not touched (`gh auth
/// logout` does that).
pub fn forget_token() -> Result<bool, String> {
    forget(GITHUB_ACCOUNT)
}

/// The system credential store, through `keyring` (feature
/// `credentials`).
#[cfg(feature = "credentials")]
mod store {
    /// The credential store entry's service: the program, nothing
    /// personal.
    const SERVICE: &str = "textweaver";

    /// Why a store call failed, in words, without the secret or anything
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

    pub(super) fn get(account: &str) -> Option<String> {
        let entry = keyring::Entry::new(SERVICE, account)
            .map_err(|e| log::warn!("credential store: {}", reason(&e)))
            .ok()?;
        match entry.get_password() {
            Ok(t) => Some(t),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => {
                log::warn!("credential store: {}", reason(&e));
                None
            }
        }
    }

    pub(super) fn set(account: &str, secret: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(|e| reason(&e))?;
        entry.set_password(secret).map_err(|e| reason(&e))
    }

    pub(super) fn forget(account: &str) -> Result<bool, String> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(|e| reason(&e))?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(reason(&e)),
        }
    }
}

/// A build without the credential store keeps nothing.
#[cfg(not(feature = "credentials"))]
mod store {
    pub(super) fn get(_account: &str) -> Option<String> {
        None
    }

    pub(super) fn set(_account: &str, _secret: &str) -> Result<(), String> {
        Err(super::NO_STORE.to_owned())
    }

    pub(super) fn forget(_account: &str) -> Result<bool, String> {
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

    #[test]
    fn a_password_is_any_printable_text_and_never_printed() {
        let p = Password::new(" pass phrase, with spaces ").unwrap();
        assert_eq!(p.reveal(), " pass phrase, with spaces ", "kept exactly");
        assert_eq!(format!("{p:?}"), "Password(hidden)");
        assert_eq!(Password::new("x").unwrap().reveal(), "x");
        assert!(Password::new("caf\u{e9} \u{1f426}").is_some());
        assert!(Password::new("").is_none());
        assert!(Password::new("line\nbreak").is_none());
        assert!(Password::new("esc\u{1b}[0m").is_none());
        assert!(Password::new(&"p".repeat(PASSWORD_MAX_CHARS)).is_some());
        assert!(Password::new(&"p".repeat(PASSWORD_MAX_CHARS + 1)).is_none());
    }

    /// Live passwords sit beside the GitHub token, each under its own
    /// account. The token test may run at the same time, so this one
    /// checks only its own sources.
    #[test]
    fn live_passwords_are_kept_by_source() {
        use_memory();
        assert!(has_store());
        assert_eq!(live_account("practice"), "live:practice");
        let a = Password::new("example only one").unwrap();
        let b = Password::new("example only two").unwrap();
        store_password("pw-test-a", &a).unwrap();
        store_password("pw-test-b", &b).unwrap();
        assert_eq!(stored_password("pw-test-a"), Some(a.clone()));
        assert_eq!(stored_password("pw-test-b"), Some(b));
        assert_eq!(
            memory(|m| m.contains_key("live:pw-test-a")),
            Some(true),
            "the account is live:<id>"
        );
        assert_eq!(forget_password("pw-test-b"), Ok(true));
        assert_eq!(forget_password("pw-test-b"), Ok(false));
        assert_eq!(stored_password("pw-test-b"), None);
        assert_eq!(stored_password("pw-test-a"), Some(a));
        assert_eq!(forget_password("pw-test-a"), Ok(true));
    }
}
