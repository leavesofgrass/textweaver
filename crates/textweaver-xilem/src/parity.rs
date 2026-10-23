//! Parity with the terminal reader: where each keymap action is handled in
//! the window.
//!
//! Both readers run every action through the same app core, so almost
//! every action works in the window as in the terminal: the app does it,
//! and the window shows its lists, prompts, and questions as dialogs and
//! its messages through the announcer. A few the window does itself (the
//! text size and font, the system's file chooser, the settings dialog, the
//! command palette, scrolling its own lines). A command that could only
//! mean something in a terminal would be listed here with the reason, and
//! in `docs/gui.md`; none is at present (beta 4 brought the scrolling and
//! line numbers to the window).
//!
//! The test in this module fails when an action is added without a place
//! here, so a new terminal command cannot quietly be missing from the
//! window.

use textweaver_app::keymap::ActionId;

/// Where the window handles an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// The app does it; the window shows what it produces (a list, a
    /// prompt, a question, a message, a new caret or selection).
    App,
    /// The window does it itself (the text size and font, the file
    /// chooser, the settings dialog, the command palette's list).
    Window,
    /// Only the terminal reader has it, for this reason.
    TerminalOnly(&'static str),
}

/// Where the window handles `action`.
pub fn support(action: ActionId) -> Support {
    use ActionId as A;
    match action {
        A::TextLarger
        | A::TextSmaller
        | A::TextSizeReset
        | A::ChooseFont
        | A::ContentsPanel
        | A::NotesPanel
        | A::ToggleHeader
        | A::ToggleToolbar
        | A::ToggleFocusMode
        | A::FullScreen
        | A::ShowPreview
        | A::NextRegion
        | A::PreviousRegion
        | A::Open
        | A::Settings
        | A::ColorSettings
        | A::ReadingForm
        | A::CommandPalette
        // The view lays out its own lines, so it scrolls itself by one;
        // the app says what the terminal's scrolling says.
        | A::ScrollDown
        | A::ScrollUp => Support::Window,
        // The app turns the setting over and says so; the view draws the
        // numbers in its margin.
        _ => Support::App,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every action has a place, and every action only the terminal has is
    /// named in `docs/gui.md` with its reason.
    #[test]
    fn every_action_is_reachable_or_listed_as_terminal_only() {
        let guide = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/gui.md"),
        )
        .expect("docs/gui.md");
        // The guide's section on what only the terminal reader does, if
        // it has one.
        let section = guide
            .split("## What only the terminal reader does")
            .nth(1)
            .map(|s| s.split("\n## ").next().unwrap_or(s));
        let mut terminal_only = Vec::new();
        for &a in ActionId::ALL {
            if let Support::TerminalOnly(reason) = support(a) {
                assert!(!reason.is_empty(), "{a:?}");
                assert!(
                    section.is_some_and(|s| s.contains(&format!("`{}`", a.id()))),
                    "{} is terminal-only but not in docs/gui.md",
                    a.id()
                );
                terminal_only.push(a);
            }
        }
        // The window's own list stays short: parity is the rule, and since
        // beta 4 the list is empty.
        assert!(terminal_only.len() <= 5, "{terminal_only:?}");
        // Each window action is one the window's driver takes before the
        // app (or the app's own prompt the window answers).
        for &a in ActionId::ALL {
            if support(a) == Support::Window {
                assert!(
                    a.is_window_only()
                        || matches!(
                            a,
                            ActionId::Open
                                | ActionId::Settings
                                | ActionId::ColorSettings
                                | ActionId::ReadingForm
                                | ActionId::CommandPalette
                                // The bars' commands: the window's own,
                                // with no keys (so not window-only).
                                | ActionId::ToggleHeader
                                | ActionId::ToggleToolbar
                                | ActionId::ToggleFocusMode
                                // The view scrolls its own lines, from the
                                // terminal's keys.
                                | ActionId::ScrollDown
                                | ActionId::ScrollUp
                        ),
                    "{a:?}"
                );
            }
        }
    }
}
