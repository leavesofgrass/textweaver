//! What the terminal reader found out about the terminal it runs in, probed
//! once at startup: the color level, whether the terminal reorders
//! right-to-left text itself, and where copied text goes. One line in the
//! log says all three, so support staff can read what was detected.

use textweaver_theme::ColorSupport;

use crate::clipboard::Route;

/// The terminal's color level, right-to-left handling and clipboard route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalInfo {
    /// The color level drawn at (`TEXTWEAVER_COLOR` and `NO_COLOR` honored).
    pub color: ColorSupport,
    /// True when the terminal reorders right-to-left text itself
    /// ([`crate::bidi::terminal_reorders`]).
    pub reorders_rtl: bool,
    /// Where copied text goes ([`crate::clipboard::detect_here`]).
    pub clipboard: Route,
}

impl TerminalInfo {
    /// Probes the terminal this process runs in.
    pub fn detect() -> Self {
        TerminalInfo {
            color: ColorSupport::detect(),
            reorders_rtl: crate::bidi::terminal_reorders(),
            clipboard: crate::clipboard::detect_here(),
        }
    }

    /// The startup log line, in words: "Terminal: truecolor colors,
    /// clipboard by OSC 52, right-to-left text reordered by textweaver."
    pub fn log_line(&self) -> String {
        let color = match self.color {
            ColorSupport::TrueColor => "truecolor",
            ColorSupport::Ansi256 => "256",
            ColorSupport::Ansi16 => "16",
            ColorSupport::NoColor => "no",
        };
        let clipboard = match self.clipboard {
            Route::Osc52 => "by OSC 52",
            Route::System => "by the system clipboard",
            Route::Both => "by OSC 52 and the system clipboard",
        };
        let rtl = if self.reorders_rtl {
            "the terminal"
        } else {
            "textweaver"
        };
        format!(
            "Terminal: {color} colors, clipboard {clipboard}, right-to-left text reordered by {rtl}."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_line_says_each_probe_in_words() {
        let info = TerminalInfo {
            color: ColorSupport::Ansi256,
            reorders_rtl: true,
            clipboard: Route::System,
        };
        assert_eq!(
            info.log_line(),
            "Terminal: 256 colors, clipboard by the system clipboard, right-to-left text reordered by the terminal."
        );
    }
}
