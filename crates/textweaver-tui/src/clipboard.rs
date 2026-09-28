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

/// The route for this process's terminal.
pub fn detect_here() -> Route {
    detect(Os::current(), |name| std::env::var(name).ok())
}

/// The system clipboard, opened on first use and kept open: on X11 the
/// copied text lives as long as the clipboard object does.
#[derive(Default)]
pub struct SystemClipboard {
    #[cfg(feature = "clipboard")]
    inner: Option<arboard::Clipboard>,
}

impl std::fmt::Debug for SystemClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SystemClipboard")
    }
}

impl SystemClipboard {
    /// Puts `text` on the system clipboard.
    ///
    /// # Errors
    ///
    /// A message for the listener when the clipboard cannot be opened or
    /// written, or when this build has no system clipboard.
    pub fn set_text(&mut self, text: &str) -> Result<(), String> {
        #[cfg(feature = "clipboard")]
        {
            if self.inner.is_none() {
                self.inner = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
            }
            match self.inner.as_mut() {
                Some(c) => c.set_text(text.to_owned()).map_err(|e| e.to_string()),
                None => Err("the system clipboard is not available".to_owned()),
            }
        }
        #[cfg(not(feature = "clipboard"))]
        {
            let _ = text;
            Err("this build has no system clipboard".to_owned())
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
