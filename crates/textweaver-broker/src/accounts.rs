//! Named accounts, kept in `broker.toml` in textweaver's data folder.
//!
//! Each account has a name, a role, the topic filters it may read and, for
//! a logger, the filters it may publish to. The password is never stored:
//! only a salted PBKDF2-HMAC-SHA256 hash from ring, written as
//! `pbkdf2-sha256$ITERATIONS$SALT$HASH` with the salt and hash in hex.
//!
//! ```toml
//! [[account]]
//! name = "netcontrol"
//! role = "logger"
//! read = ["netlog/#"]
//! publish = ["netlog/#"]
//! password = "pbkdf2-sha256$600000$...$..."
//! ```
//!
//! A reader can never publish, whatever the file says: a reader with
//! publish topics is refused when the file is read.

use std::num::NonZeroU32;
use std::path::Path;

use ring::rand::SecureRandom;
use ring::{pbkdf2, rand::SystemRandom};
use serde::{Deserialize, Serialize};
use textweaver_mqtt::topic::filter_problem;

use crate::Perms;

/// The file's name in textweaver's data folder.
pub const ACCOUNTS_FILE: &str = "broker.toml";

/// PBKDF2 rounds for new passwords (OWASP's figure for HMAC-SHA256).
pub const DEFAULT_ITERATIONS: u32 = 600_000;

/// The fewest rounds a stored hash may have.
const MIN_ITERATIONS: u32 = 1_000;

/// The most accounts the file may hold.
const MAX_ACCOUNTS: usize = 1_000;

/// The most topic filters one account may list, read and publish each.
const MAX_FILTERS: usize = 64;

/// The largest file read.
const MAX_FILE: u64 = 1 << 20;

const SALT_LEN: usize = 16;
const HASH_LEN: usize = 32;

/// What an account may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Subscribes to its read topics; may never publish.
    Reader,
    /// Subscribes to its read topics and publishes to its publish topics.
    Logger,
}

impl Role {
    /// "reader" or "logger".
    pub fn name(self) -> &'static str {
        match self {
            Role::Reader => "reader",
            Role::Logger => "logger",
        }
    }

    /// Reads "reader" or "logger", in any case.
    pub fn parse(text: &str) -> Option<Role> {
        match text.trim().to_ascii_lowercase().as_str() {
            "reader" => Some(Role::Reader),
            "logger" => Some(Role::Logger),
            _ => None,
        }
    }
}

/// One account. The password is only its hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// The user name: letters, digits, dot, dash and underscore.
    pub name: String,
    /// Reader or logger.
    pub role: Role,
    /// Topic filters it may subscribe to (`+` and `#` allowed).
    #[serde(default)]
    pub read: Vec<String>,
    /// Topic filters it may publish to; always empty for a reader.
    #[serde(default)]
    pub publish: Vec<String>,
    /// `pbkdf2-sha256$ITERATIONS$SALT$HASH`.
    password: String,
}

impl Account {
    /// A new account with `password` hashed at [`DEFAULT_ITERATIONS`].
    pub fn new(
        name: &str,
        role: Role,
        read: Vec<String>,
        publish: Vec<String>,
        password: &str,
    ) -> Result<Account, String> {
        Account::with_iterations(name, role, read, publish, password, DEFAULT_ITERATIONS)
    }

    /// As [`Account::new`], with a chosen number of PBKDF2 rounds (at least
    /// 1,000). Fewer rounds are quicker to check and to guess; tests use
    /// them.
    pub fn with_iterations(
        name: &str,
        role: Role,
        read: Vec<String>,
        publish: Vec<String>,
        password: &str,
        iterations: u32,
    ) -> Result<Account, String> {
        if password.is_empty() {
            return Err("Account error: the password is empty.".into());
        }
        if password.chars().count() > 1024 {
            return Err("Account error: the password is longer than 1024 characters.".into());
        }
        let account = Account {
            name: name.to_string(),
            role,
            read,
            publish,
            password: hash_password(password, iterations.max(MIN_ITERATIONS))?,
        };
        account.check()?;
        Ok(account)
    }

    /// Whether `password` is this account's, in constant time.
    pub fn verify(&self, password: &str) -> bool {
        match parse_hash(&self.password) {
            Some((iterations, salt, hash)) => pbkdf2::verify(
                pbkdf2::PBKDF2_HMAC_SHA256,
                iterations,
                &salt,
                password.as_bytes(),
                &hash,
            )
            .is_ok(),
            None => false,
        }
    }

    /// What the account may do.
    pub(crate) fn perms(&self) -> Perms {
        Perms {
            read: self.read.clone(),
            publish: match self.role {
                Role::Reader => Vec::new(),
                Role::Logger => self.publish.clone(),
            },
        }
    }

    /// The account's problems, as one sentence naming it.
    fn check(&self) -> Result<(), String> {
        let name = &self.name;
        if let Some(why) = name_problem(name) {
            return Err(format!("Account error: {why}."));
        }
        let bad = |why: String| Err(format!("Account error: {name}: {why}."));
        if self.read.len() > MAX_FILTERS || self.publish.len() > MAX_FILTERS {
            return bad(format!("more than {MAX_FILTERS} topics"));
        }
        for t in self.read.iter().chain(&self.publish) {
            if let Some(why) = filter_problem(t) {
                return bad(format!("{why}: {}", textweaver_mqtt::clean_text(t, 200)));
            }
        }
        if self.role == Role::Reader && !self.publish.is_empty() {
            return bad("a reader may not have publish topics; make it a logger".into());
        }
        if parse_hash(&self.password).is_none() {
            return bad("the stored password is not a pbkdf2-sha256 hash".into());
        }
        Ok(())
    }
}

/// Why a user name cannot be used, or `None`.
pub fn name_problem(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("the user name is empty".into());
    }
    if name.len() > 64 {
        return Some("the user name is longer than 64 characters".into());
    }
    if let Some(c) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
    {
        let shown = if c.is_ascii_graphic() {
            c.to_string()
        } else {
            format!("a character with code {}", u32::from(c))
        };
        return Some(format!(
            "the user name holds {shown}; use letters, digits, dot, dash and underscore"
        ));
    }
    if name.eq_ignore_ascii_case(textweaver_mqtt::JOIN_USER) {
        return Some(format!(
            "the user name {} is kept for join codes",
            textweaver_mqtt::JOIN_USER
        ));
    }
    None
}

/// Every account, as `broker.toml` holds them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accounts {
    #[serde(default, rename = "account")]
    accounts: Vec<Account>,
}

impl Accounts {
    /// Reads `path`. A missing file is no accounts.
    pub fn load(path: &Path) -> Result<Accounts, String> {
        let shown = path.display();
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Accounts::default()),
            Err(e) => return Err(format!("Account error: cannot read {shown}: {e}.")),
        };
        if meta.len() > MAX_FILE {
            return Err(format!(
                "Account error: {shown} is larger than {MAX_FILE} bytes."
            ));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Account error: cannot read {shown}: {e}."))?;
        Accounts::parse(&text).map_err(|e| format!("{e} In {shown}."))
    }

    /// Reads the file's text and checks every account.
    pub fn parse(text: &str) -> Result<Accounts, String> {
        let accounts: Accounts = toml::from_str(text).map_err(|e| {
            let why = e.message().trim_end_matches('.').to_string();
            format!(
                "Account error: the file is not valid: {}.",
                textweaver_mqtt::clean_text(&why, 300)
            )
        })?;
        if accounts.accounts.len() > MAX_ACCOUNTS {
            return Err(format!("Account error: more than {MAX_ACCOUNTS} accounts."));
        }
        for (i, a) in accounts.accounts.iter().enumerate() {
            a.check()?;
            if accounts.accounts[..i]
                .iter()
                .any(|b| b.name.eq_ignore_ascii_case(&a.name))
            {
                return Err(format!("Account error: the name {} is used twice.", a.name));
            }
        }
        Ok(accounts)
    }

    /// Writes the accounts to `path`, through a new file renamed over the
    /// old one, so a failed write never leaves half a file. On Unix the
    /// file is readable by its owner only.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let shown = path.display();
        let body = toml::to_string(self)
            .map_err(|e| format!("Account error: cannot write {shown}: {e}."))?;
        let text = format!(
            "# textweaver's broker accounts. Passwords are kept only as salted\n# hashes. Change this file with tw broker account add, remove and list.\n\n{body}"
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Account error: cannot make {}: {e}.", dir.display()))?;
        }
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".new");
        let tmp = std::path::PathBuf::from(tmp);
        crate::identity::write_private(&tmp, text.as_bytes())
            .map_err(|e| format!("Account error: cannot write {}: {e}.", tmp.display()))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| format!("Account error: cannot write {shown}: {e}."))
    }

    /// Adds `account`, or replaces the one with its name when `replace`
    /// is true. Returns whether one was replaced.
    pub fn add(&mut self, account: Account, replace: bool) -> Result<bool, String> {
        account.check()?;
        if let Some(old) = self
            .accounts
            .iter_mut()
            .find(|a| a.name.eq_ignore_ascii_case(&account.name))
        {
            if !replace {
                return Err(format!(
                    "Account error: {} already exists. Give --replace to change it.",
                    old.name
                ));
            }
            *old = account;
            return Ok(true);
        }
        if self.accounts.len() >= MAX_ACCOUNTS {
            return Err(format!("Account error: more than {MAX_ACCOUNTS} accounts."));
        }
        self.accounts.push(account);
        Ok(false)
    }

    /// Removes the account with this name. Returns whether there was one.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|a| !a.name.eq_ignore_ascii_case(name));
        self.accounts.len() != before
    }

    /// Every account, in the file's order.
    pub fn list(&self) -> &[Account] {
        &self.accounts
    }

    /// The account with this name.
    pub fn get(&self, name: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.name == name)
    }

    /// True when there is no account.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// The account's permissions when `password` is right. An unknown name
    /// costs as much time as a wrong password, so a guesser cannot tell
    /// which names exist.
    pub(crate) fn login(&self, name: &str, password: &str) -> Option<Perms> {
        match self.get(name) {
            Some(a) if a.verify(password) => Some(a.perms()),
            Some(_) => None,
            None => {
                let rounds = self
                    .accounts
                    .first()
                    .and_then(|a| parse_hash(&a.password))
                    .map_or(DEFAULT_ITERATIONS, |(n, _, _)| n.get());
                let rounds = NonZeroU32::new(rounds).unwrap_or(NonZeroU32::MIN);
                let mut out = [0u8; HASH_LEN];
                pbkdf2::derive(
                    pbkdf2::PBKDF2_HMAC_SHA256,
                    rounds,
                    &[0u8; SALT_LEN],
                    password.as_bytes(),
                    &mut out,
                );
                None
            }
        }
    }
}

fn hash_password(password: &str, iterations: u32) -> Result<String, String> {
    let mut salt = [0u8; SALT_LEN];
    SystemRandom::new()
        .fill(&mut salt)
        .map_err(|_| "Account error: no secure random numbers on this computer.".to_string())?;
    let rounds = NonZeroU32::new(iterations).unwrap_or(NonZeroU32::MIN);
    let mut hash = [0u8; HASH_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        rounds,
        &salt,
        password.as_bytes(),
        &mut hash,
    );
    Ok(format!(
        "pbkdf2-sha256${iterations}${}${}",
        hex(&salt),
        hex(&hash)
    ))
}

fn parse_hash(text: &str) -> Option<(NonZeroU32, Vec<u8>, Vec<u8>)> {
    let mut parts = text.split('$');
    if parts.next()? != "pbkdf2-sha256" {
        return None;
    }
    let iterations: u32 = parts.next()?.parse().ok()?;
    if !(MIN_ITERATIONS..=100_000_000).contains(&iterations) {
        return None;
    }
    let salt = unhex(parts.next()?)?;
    let hash = unhex(parts.next()?)?;
    if parts.next().is_some() || salt.len() != SALT_LEN || hash.len() != HASH_LEN {
        return None;
    }
    Some((NonZeroU32::new(iterations)?, salt, hash))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick(name: &str, role: Role, publish: &[&str]) -> Account {
        Account::with_iterations(
            name,
            role,
            vec!["netlog/#".into()],
            publish.iter().map(|s| s.to_string()).collect(),
            "correct horse",
            1_000,
        )
        .unwrap()
    }

    #[test]
    fn passwords_are_hashed_and_checked() {
        let a = quick("ada", Role::Reader, &[]);
        assert!(a.password.starts_with("pbkdf2-sha256$1000$"));
        assert!(!a.password.contains("correct horse"));
        assert!(a.verify("correct horse"));
        assert!(!a.verify("correct horsE"));
        let b = quick("ada", Role::Reader, &[]);
        assert_ne!(a.password, b.password, "salts differ");
    }

    #[test]
    fn readers_never_publish() {
        let a = quick("ada", Role::Reader, &[]);
        assert!(a.perms().publish.is_empty());
        let err = Account::with_iterations(
            "ada",
            Role::Reader,
            vec![],
            vec!["netlog/#".into()],
            "pw",
            1_000,
        )
        .unwrap_err();
        assert!(
            err.contains("a reader may not have publish topics"),
            "{err}"
        );
    }

    #[test]
    fn names_are_plain_and_join_is_kept() {
        assert!(name_problem("net-control_1.a").is_none());
        assert!(name_problem("").is_some());
        assert!(name_problem("ada example").unwrap().contains("code 32"));
        assert!(
            name_problem("JOIN")
                .unwrap()
                .contains("kept for join codes")
        );
        assert!(name_problem("a$b").unwrap().contains("holds $"));
    }

    #[test]
    fn file_round_trips_and_refuses_mistakes() {
        let mut all = Accounts::default();
        assert!(
            !all.add(quick("netcontrol", Role::Logger, &["netlog/#"]), false)
                .unwrap()
        );
        assert!(!all.add(quick("ada", Role::Reader, &[]), false).unwrap());
        let err = all.add(quick("ADA", Role::Reader, &[]), false).unwrap_err();
        assert!(err.contains("already exists"), "{err}");
        assert!(all.add(quick("ada", Role::Reader, &[]), true).unwrap());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(ACCOUNTS_FILE);
        all.save(&path).unwrap();
        let back = Accounts::load(&path).unwrap();
        assert_eq!(back, all);
        assert!(back.login("ada", "correct horse").is_some());
        assert!(back.login("ada", "wrong").is_none());
        assert!(back.login("nobody", "correct horse").is_none());
        assert!(!dir.path().join("broker.toml.new").exists());

        assert!(all.remove("Ada"));
        assert!(!all.remove("ada"));

        let text = std::fs::read_to_string(&path).unwrap();
        let tampered = text.replacen("role = \"logger\"", "role = \"reader\"", 1);
        let err = Accounts::parse(&tampered).unwrap_err();
        assert!(
            err.contains("a reader may not have publish topics"),
            "{err}"
        );
        let err = Accounts::parse("[[account]]\nname = \"x\"\nrole = \"admin\"\npassword = \"\"\n")
            .unwrap_err();
        assert!(
            err.starts_with("Account error: the file is not valid:"),
            "{err}"
        );
        let err =
            Accounts::parse("[[account]]\nname = \"x\"\nrole = \"reader\"\npassword = \"plain\"\n")
                .unwrap_err();
        assert!(err.contains("not a pbkdf2-sha256 hash"), "{err}");
        assert_eq!(
            Accounts::load(&dir.path().join("missing.toml")).unwrap(),
            Accounts::default()
        );
    }
}
