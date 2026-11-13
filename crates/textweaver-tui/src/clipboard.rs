//! Where copied text goes (Agent W4g, ADR-0032): to the terminal as an
//! OSC 52 sequence, which most terminals pass to the system clipboard (over
//! SSH too), or, where the terminal cannot take it, straight to the system
//! clipboard with `arboard` (the `clipboard` feature).
//!
//! [`detect`] decides from the environment, once at startup:
//!
//! - over SSH or inside tmux: OSC 52, since the system clipboard here is
//!   not the listener's;
//! - Windows: OSC 52 in Windows Terminal (`WT_SESSION`), VS Code, WezTerm,
//!   and terminals that set `TERM` (mintty, Alacritty); the system
//!   clipboard in the old console window, which ignores OSC 52;
//! - macOS: the system clipboard in Terminal.app, which ignores OSC 52;
//!   OSC 52 elsewhere (iTerm2, kitty, WezTerm);
//! - Linux and the BSDs: the system clipboard in VTE terminals (GNOME
//!   Terminal, Tilix) and on the virtual console, which ignore OSC 52; both
//!   in Konsole, whose support depends on its version; OSC 52 elsewhere
//!   (xterm, kitty, foot, Alacritty, WezTerm).
//!
//! The first time text goes to the system clipboard, textweaver says so
//! once: "Copied with the system clipboard, because this terminal cannot
//! take copied text."
//!
//! A secret (the join code, the join link) goes to the system clipboard
//! marked private (security QW4): on Windows it is kept out of clipboard
//! history (Windows+V) and the cloud clipboard, on macOS it is marked
//! concealed for clipboard managers, and on Linux it is marked for
//! clipboard managers to skip. OSC 52 cannot mark anything, so a secret
//! sent that way is plain copied text to the terminal.

use textweaver_app::lexicon::i18n::Catalog;

/// Where copied text is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// An OSC 52 sequence to the terminal.
    Osc52,
    /// The system clipboard.
    System,
    /// Both: the terminal may or may not take OSC 52.
    Both,
}

/// The operating system, for [`detect`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and the other Unix systems.
    Unix,
}

impl Os {
    /// The system this runs on.
    pub fn current() -> Self {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Unix
        }
    }
}

/// Where copied text should go on `os`, given the environment (`env`
/// returns a variable's value).
pub fn detect(os: Os, env: impl Fn(&str) -> Option<String>) -> Route {
    let set = |name: &str| env(name).is_some_and(|v| !v.is_empty());
    let is = |name: &str, value: &str| env(name).is_some_and(|v| v == value);
    if set("SSH_CONNECTION") || set("SSH_TTY") || set("TMUX") {
        return Route::Osc52;
    }
    let known_osc52 = is("TERM_PROGRAM", "vscode") || is("TERM_PROGRAM", "WezTerm");
    match os {
        Os::Windows => {
            if set("WT_SESSION") || known_osc52 || set("TERM") {
                Route::Osc52
            } else {
                Route::System
            }
        }
        Os::MacOs => {
            if is("TERM_PROGRAM", "Apple_Terminal") {
                Route::System
            } else {
                Route::Osc52
            }
        }
        Os::Unix => {
            if known_osc52 {
                Route::Osc52
            } else if set("VTE_VERSION") || is("TERM", "linux") {
                Route::System
            } else if set("KONSOLE_VERSION") {
                Route::Both
            } else {
                Route::Osc52
            }
        }
    }
}

/// True when this computer's clipboard is the listener's: not over SSH
/// or inside tmux (`env` returns a variable's value).
pub fn clipboard_is_local(env: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| env(name).is_some_and(|v| !v.is_empty());
    !(set("SSH_CONNECTION") || set("SSH_TTY") || set("TMUX"))
}

/// The system clipboard's image, for Join from a QR image
/// (`textweaver_app::clipboard_image`): on Windows, macOS and Linux, from
/// the clipboard of the computer the reader runs on. It reads no text.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemImages;

impl textweaver_app::Clipboard for SystemImages {
    fn read(&mut self) -> textweaver_app::ClipboardContents {
        textweaver_app::ClipboardContents::default()
    }

    fn read_image(&mut self) -> Option<textweaver_app::ClipboardImage> {
        textweaver_app::clipboard_image::read_system_image()
    }
}

/// The route for this process's terminal.
pub fn detect_here() -> Route {
    detect(Os::current(), |name| std::env::var(name).ok())
}

/// What a recording clipboard was asked to hold: each text, and whether
/// it was a secret (tests; see [`SystemClipboard::recording`]).
pub type ClipboardLog = std::sync::Arc<std::sync::Mutex<Vec<(String, bool)>>>;

/// The system clipboard, opened on first use and kept open: on X11 the
/// copied text lives as long as the clipboard object does.
#[derive(Default)]
pub struct SystemClipboard {
    #[cfg(feature = "clipboard")]
    inner: Option<arboard::Clipboard>,
    /// A recording clipboard writes nothing; it keeps each request.
    log: Option<ClipboardLog>,
}

impl std::fmt::Debug for SystemClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SystemClipboard")
    }
}

impl SystemClipboard {
    /// A clipboard that never touches the system's: it records what it is
    /// asked to hold in the returned log (tests).
    pub fn recording() -> (Self, ClipboardLog) {
        let log = ClipboardLog::default();
        let clip = SystemClipboard {
            log: Some(log.clone()),
            ..SystemClipboard::default()
        };
        (clip, log)
    }

    /// Puts `text` on the system clipboard; a `secret` is kept out of
    /// clipboard history and the cloud clipboard where the system allows
    /// it.
    ///
    /// # Errors
    ///
    /// A message for the listener when the clipboard cannot be opened or
    /// written, or when this build has no system clipboard.
    pub fn set_text(&mut self, c: &Catalog, text: &str, secret: bool) -> Result<(), String> {
        if let Some(log) = &self.log {
            if let Ok(mut l) = log.lock() {
                l.push((text.to_owned(), secret));
            }
            return Ok(());
        }
        #[cfg(feature = "clipboard")]
        {
            if self.inner.is_none() {
                self.inner = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
            }
            match self.inner.as_mut() {
                Some(clip) => {
                    let set = clip.set();
                    let set = if secret { private(set) } else { set };
                    set.text(text.to_owned()).map_err(|e| e.to_string())
                }
                None => Err(c.tr("tui-clip-not-available")),
            }
        }
        #[cfg(not(feature = "clipboard"))]
        {
            let _ = (text, secret);
            Err(c.tr("tui-clip-not-built"))
        }
    }
}

/// `set` marked private: out of Windows clipboard history and the cloud
/// clipboard, concealed on macOS, and skipped by Linux clipboard managers.
#[cfg(feature = "clipboard")]
pub(crate) fn private(set: arboard::Set<'_>) -> arboard::Set<'_> {
    #[cfg(windows)]
    {
        use arboard::SetExtWindows;
        set.exclude_from_history().exclude_from_cloud()
    }
    #[cfg(target_os = "macos")]
    {
        use arboard::SetExtApple;
        set.exclude_from_history()
    }
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
    ))]
    {
        use arboard::SetExtLinux;
        set.exclude_from_history()
    }
    #[cfg(not(any(
        windows,
        all(unix, not(any(target_os = "android", target_os = "emscripten")))
    )))]
    {
        set
    }
}

/// Reads the system clipboard for Paste (the `clipboard` feature): its
/// plain text and its HTML, which browsers and word processors put beside
/// the text. `arboard` cannot read RTF; the app converts the HTML to
/// Markdown.
#[cfg(feature = "clipboard")]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemReader;

#[cfg(feature = "clipboard")]
impl textweaver_app::Clipboard for SystemReader {
    fn read(&mut self) -> textweaver_app::ClipboardContents {
        let Ok(mut c) = arboard::Clipboard::new() else {
            return textweaver_app::ClipboardContents::default();
        };
        textweaver_app::ClipboardContents {
            text: c.get_text().ok(),
            html: c.get().html().ok(),
            rtf: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    #[test]
    fn windows_terminal_takes_osc52_and_the_old_console_does_not() {
        assert_eq!(
            detect(Os::Windows, env(&[("WT_SESSION", "abc")])),
            Route::Osc52
        );
        assert_eq!(detect(Os::Windows, env(&[])), Route::System);
        assert_eq!(
            detect(Os::Windows, env(&[("TERM", "xterm-256color")])),
            Route::Osc52
        );
    }

    #[test]
    fn terminal_app_and_vte_use_the_system_clipboard() {
        assert_eq!(
            detect(Os::MacOs, env(&[("TERM_PROGRAM", "Apple_Terminal")])),
            Route::System
        );
        assert_eq!(
            detect(Os::MacOs, env(&[("TERM_PROGRAM", "iTerm.app")])),
            Route::Osc52
        );
        assert_eq!(
            detect(Os::Unix, env(&[("VTE_VERSION", "7600")])),
            Route::System
        );
        assert_eq!(detect(Os::Unix, env(&[("TERM", "linux")])), Route::System);
        assert_eq!(
            detect(Os::Unix, env(&[("KONSOLE_VERSION", "230800")])),
            Route::Both
        );
        assert_eq!(
            detect(Os::Unix, env(&[("TERM", "xterm-kitty")])),
            Route::Osc52
        );
    }

    #[test]
    fn ssh_and_tmux_always_use_osc52() {
        for os in [Os::Windows, Os::MacOs, Os::Unix] {
            assert_eq!(
                detect(
                    os,
                    env(&[
                        ("SSH_CONNECTION", "1 2 3 4"),
                        ("TERM_PROGRAM", "Apple_Terminal"),
                        ("VTE_VERSION", "1")
                    ])
                ),
                Route::Osc52
            );
            assert_eq!(detect(os, env(&[("TMUX", "/tmp/x")])), Route::Osc52);
        }
    }
}
