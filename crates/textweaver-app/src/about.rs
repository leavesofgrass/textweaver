//! Help's ways out to the docs, About's facts, and asking first-run
//! questions again.
//!
//! - **Quick start** opens the `QUICKSTART.md` packaged beside the program
//!   as a document; when there is none, it offers the online page.
//! - **Documentation** opens the packaged guides in textweaver
//!   ([`crate::help_docs`]); **Online documentation** and **Report a
//!   problem** show their web address and ask before opening a browser
//!   ([`App::offer_open`]); nothing is sent.
//! - **About** lists the facts a problem report needs, one per line, each
//!   starting with what it is: version, build, license, copyright,
//!   components, speech engines, and folders.
//! - **Ask again about first-run choices** clears the markers of the
//!   hybrid-mode question and the optional components list, so both are
//!   offered at the next start.

use std::path::{Path, PathBuf};

use textweaver_lexicon::args;

use crate::app::{App, ListKind};
use crate::command::Effect;
use textweaver_keymap::Frontend;

/// The copyright line, as the `NOTICE` file at the repository root has it.
/// A macro so [`COPYRIGHT`] and [`VERSION_TEXT`] share one literal.
macro_rules! copyright {
    () => {
        "Copyright (C) 2026 Jon Pielaet"
    };
}

/// textweaver's copyright line. It is a legal notice, so it is shown as is
/// in every language: in About and after the version in `--version`.
pub const COPYRIGHT: &str = copyright!();

/// What `tw --version` and `textweaver --version` print after the program's
/// name: the version, then the copyright line.
pub const VERSION_TEXT: &str = concat!(env!("CARGO_PKG_VERSION"), "\n", copyright!());

/// The documentation site.
pub const DOCS_ADDRESS: &str = "https://leavesofgrass.github.io/textweaver/";
/// The quick start on the documentation site.
pub const QUICK_START_ADDRESS: &str = "https://leavesofgrass.github.io/textweaver/quickstart/";
/// Where problems are reported: the project's issues.
pub const REPORT_ADDRESS: &str = "https://github.com/leavesofgrass/textweaver/issues";
/// The quick start's name in the packages (`cargo xtask dist`, `gui-dist`).
const QUICK_START_FILE: &str = "QUICKSTART.md";

/// The packaged quick start: beside the program, or one folder up (a
/// program installed in a `bin` folder).
fn packaged_quick_start(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|d| d.join(QUICK_START_FILE))
        .find(|p| p.is_file())
}

impl App {
    /// Help, Quick start: the packaged guide opens as a document; with
    /// none beside the program, the online page is offered.
    pub(crate) fn quick_start(&mut self) -> Vec<Effect> {
        let found = std::env::current_exe()
            .ok()
            .and_then(|exe| packaged_quick_start(&exe));
        match found {
            Some(path) => self.open_command(path),
            None => {
                let q = self.msg_args(
                    "about-quick-start-online",
                    &args!["address" => QUICK_START_ADDRESS],
                );
                self.offer_open(QUICK_START_ADDRESS.to_owned(), &q);
                vec![Effect::Redraw]
            }
        }
    }

    /// Help, Report a problem: the address, and a question before a
    /// browser opens. Nothing is sent from textweaver.
    pub(crate) fn report_problem(&mut self) -> Vec<Effect> {
        let q = self.msg_args("about-report-question", &args!["address" => REPORT_ADDRESS]);
        self.offer_open(REPORT_ADDRESS.to_owned(), &q);
        vec![Effect::Redraw]
    }

    /// Ask again about first-run choices: the hybrid-mode question and the
    /// optional components list are offered at the next start.
    pub(crate) fn ask_first_run_again(&mut self) {
        let _ = self.update_settings(|s| {
            s.accessibility.hybrid_offered = false;
            s.components.chooser_shown = false;
            s.updates.asked = false;
        });
        let msg = self.msg("about-first-run-again");
        self.tell(&msg);
    }

    /// The facts About lists, in order, each starting with its name.
    pub(crate) fn about_facts(&mut self) -> Vec<String> {
        let c = self.catalog();
        let frontend = match self.keymap.frontend() {
            Frontend::Gui => c.tr("about-frontend-window"),
            _ => c.tr("about-frontend-terminal"),
        };
        let profile = if cfg!(debug_assertions) {
            c.tr("about-profile-debug")
        } else {
            c.tr("about-profile-release")
        };
        let mut facts = vec![
            c.fmt(
                "about-version",
                &args!["version" => env!("CARGO_PKG_VERSION")],
            ),
            c.fmt(
                "about-build",
                &args![
                    "frontend" => frontend,
                    "profile" => profile,
                    "os" => std::env::consts::OS,
                    "arch" => std::env::consts::ARCH,
                ],
            ),
            c.fmt(
                "about-license",
                &args!["license" => env!("CARGO_PKG_LICENSE")],
            ),
            COPYRIGHT.to_owned(),
            c.fmt(
                "about-engine-in-use",
                &args!["engine" => self.backend_name.as_str()],
            ),
        ];
        let Some(paths) = self.paths.clone() else {
            facts.push(c.tr("about-no-folders"));
            return facts;
        };
        // Probing the engines starts nothing that speaks; it runs only in a
        // session that keeps files, so tests never probe this machine.
        let found: Vec<&str> = textweaver_engines::speech_registry_for(&self.settings)
            .list()
            .into_iter()
            .filter(|b| b.available && !b.opt_in && b.id != "null")
            .map(|b| b.name)
            .collect();
        facts.push(if found.is_empty() {
            c.tr("about-engines-none")
        } else {
            c.fmt("about-engines", &args!["engines" => found.join(", ")])
        });
        facts.push(self.component_registry().status_line(&c, &paths.data_dir));
        for (id, dir) in [
            ("about-folder-settings", &paths.config_dir),
            ("about-folder-data", &paths.data_dir),
            ("about-folder-cache", &paths.cache_dir),
        ] {
            facts.push(c.fmt(id, &args!["path" => dir.display().to_string()]));
        }
        facts
    }

    /// Help, About: the facts, as a list to read and copy from.
    pub(crate) fn about(&mut self) -> Vec<Effect> {
        let items = self.about_facts();
        let title = self.msg("about-title");
        let intro = self.msg_args("about-intro", &args!["n" => items.len()]);
        self.list = Some(ListKind::Info);
        self.tell(&intro);
        vec![Effect::ShowList { title, items }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copyright_line_matches_the_notice_file() {
        let notice = include_str!("../../../NOTICE");
        assert_eq!(notice.lines().next(), Some(COPYRIGHT));
        assert!(VERSION_TEXT.ends_with(COPYRIGHT));
        assert!(VERSION_TEXT.starts_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn the_packaged_quick_start_is_found_beside_the_program_or_one_up() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("textweaver-gui.exe");
        assert_eq!(packaged_quick_start(&exe), None);
        std::fs::write(tmp.path().join(QUICK_START_FILE), "# Quick start\n").unwrap();
        assert_eq!(
            packaged_quick_start(&exe),
            Some(tmp.path().join(QUICK_START_FILE))
        );
        std::fs::write(bin.join(QUICK_START_FILE), "# Quick start\n").unwrap();
        assert_eq!(packaged_quick_start(&exe), Some(bin.join(QUICK_START_FILE)));
    }
}
