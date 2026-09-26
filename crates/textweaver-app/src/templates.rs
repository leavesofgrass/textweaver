//! New documents from templates (the `new_from_template` command).
//!
//! A template is Markdown with placeholders: `{{title}}`, `{{author}}`,
//! and `{{date}}`. textweaver has three built in (an essay, a report, and
//! notes), each with front matter holding the title, author, and date, and
//! a References heading for the bibliography. Your own templates are the
//! `.md` files in the `templates` folder of the configuration folder; they
//! are listed after the built-in ones, by file name.
//!
//! The date is today's date on this computer, in its local time zone
//! (`2026-09-26`); it is read from the system, never assumed. The author
//! is `author` in the `[editing]` section of `settings.toml`, when set.

use std::path::Path;

use textweaver_store::DocKey;
use textweaver_text::Document;

use crate::app::App;
use crate::authoring_state::AuthoringList;
use crate::command::{Effect, PromptPurpose};
use crate::edit::AfterLeave;

/// A document template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Template {
    /// Its name ("Essay", or the file name without `.md`).
    pub(crate) name: String,
    /// Its Markdown, with placeholders.
    pub(crate) text: String,
    /// True for one from the templates folder.
    pub(crate) user: bool,
}

impl Template {
    /// The name as listed.
    pub(crate) fn label(&self) -> String {
        if self.user {
            format!("{}, your template", self.name)
        } else {
            self.name.clone()
        }
    }

    /// The template with its placeholders filled.
    pub(crate) fn fill(&self, title: &str, author: &str, date: &str) -> String {
        let mut out = self.text.clone();
        for (key, value) in [("title", title), ("author", author), ("date", date)] {
            for pattern in [format!("{{{{{key}}}}}"), format!("{{{{ {key} }}}}")] {
                out = out.replace(&pattern, value);
            }
        }
        out
    }
}

const FRONT: &str = "---\ntitle: \"{{title}}\"\nauthor: \"{{author}}\"\ndate: {{date}}\n---\n\n";

/// The built-in templates.
pub(crate) fn builtin() -> Vec<Template> {
    let t = |name: &str, body: &str| Template {
        name: name.to_owned(),
        text: format!("{FRONT}{body}"),
        user: false,
    };
    vec![
        t(
            "Essay",
            "# {{title}}\n\n## Introduction\n\n\n\n## Discussion\n\n\n\n## Conclusion\n\n\n\n## References\n",
        ),
        t(
            "Report",
            "# {{title}}\n\n## Summary\n\n\n\n## Background\n\n\n\n## Method\n\n\n\n## Results\n\n\n\n## Recommendations\n\n\n\n## References\n",
        ),
        t(
            "Notes",
            "# {{title}}\n\n## Key points\n\n- \n\n## Questions\n\n- \n\n## References\n",
        ),
    ]
}

/// The user's templates in `dir`, by file name.
pub(crate) fn user_templates(dir: &Path) -> Vec<Template> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Template> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown")
                })
        })
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            let name = p.file_stem()?.to_string_lossy().into_owned();
            Some(Template {
                name,
                text: text.replace("\r\n", "\n"),
                user: true,
            })
        })
        .collect();
    out.sort_by_key(|t| t.name.to_lowercase());
    out
}

/// Today's date on this computer, in its local time zone, as
/// `YYYY-MM-DD`: from the system clock and time zone (Windows'
/// `GetLocalTime`, else the `date` command), falling back to the UTC date
/// of the system clock when the local date cannot be read.
pub fn local_date() -> String {
    local_date_from_system().unwrap_or_else(utc_date)
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn local_date_from_system() -> Option<String> {
    // SAFETY: GetLocalTime takes no arguments, cannot fail, and only fills
    // the SYSTEMTIME it returns by value; no memory is shared.
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (t.wYear > 0).then(|| format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay))
}

#[cfg(not(windows))]
fn local_date_from_system() -> Option<String> {
    let out = std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    let ok = s.len() == 10
        && s.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    ok.then(|| s.to_owned())
}

/// The UTC date of the system clock.
fn utc_date() -> String {
    let ts = textweaver_store::now_ts();
    textweaver_store::time::rfc3339(ts)
        .chars()
        .take(10)
        .collect()
}

impl App {
    /// The templates: built-in, then the user's.
    fn templates(&self) -> Vec<Template> {
        let mut all = builtin();
        if let Some(p) = &self.paths {
            all.extend(user_templates(&p.config_dir.join("templates")));
        }
        all
    }

    /// The `new_from_template` command: the list of templates (after
    /// resolving unsaved edits).
    pub(crate) fn new_from_template(&mut self) -> Vec<Effect> {
        if self.edit.is_some() {
            return self.leave_edit(None, None, AfterLeave::Templates);
        }
        let all = self.templates();
        let n = all.len();
        let folder = self
            .paths
            .as_ref()
            .map(|p| p.config_dir.join("templates").display().to_string())
            .unwrap_or_default();
        self.tell(&format!(
            "New document from a template, {n} templates. Enter chooses. Your own templates go in {folder}."
        ));
        self.show_authoring_list(AuthoringList::Templates(all))
    }

    /// A template was chosen: ask for the title.
    pub(crate) fn template_chosen(&mut self, t: Template) -> Vec<Effect> {
        self.authoring.template = Some(t);
        self.prompt(PromptPurpose::TemplateTitle)
    }

    /// The title prompt's answer: the new document, in edit mode, marked
    /// unsaved, with the caret on the first empty line after a heading.
    pub(crate) fn answer_template_title(&mut self, text: &str) -> Vec<Effect> {
        let Some(t) = self.authoring.template.take() else {
            return vec![Effect::Redraw];
        };
        let title = match text.trim() {
            "" => "Untitled".to_owned(),
            t => t.to_owned(),
        };
        let author = self
            .settings
            .editing
            .extra
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let date = local_date();
        let body = t.fill(&title, &author, &date);
        self.untitled += 1;
        let n = std::process::id()
            .wrapping_mul(100)
            .wrapping_add(self.untitled);
        let mut doc = Document::from_plain_text("");
        doc.meta.format = "markdown".into();
        self.open_document(doc, DocKey::untitled(n), title.clone());
        self.enter_edit(Some(body.clone()));
        // The caret on the first blank line after the first heading below
        // the title: where the writing starts.
        let start = first_writing_line(&body);
        if let Some(ed) = self.edit.as_mut().and_then(|e| e.session.editor_mut()) {
            ed.set_selection(textweaver_editor::Selection::caret(start));
        }
        self.after_edit(&ropey::Rope::new(), &[]);
        self.tell(&format!(
            "New document from the {} template: {title}. Dated {date}. The caret is where the writing starts. Remember to save.",
            t.name
        ));
        vec![Effect::Redraw]
    }
}

/// The char position of the first blank line after the second heading (the
/// first section), else the end.
fn first_writing_line(text: &str) -> textweaver_core::CharPos {
    let mut headings = 0;
    let mut pos = 0usize;
    let mut after_heading = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('#') {
            headings += 1;
            after_heading = headings >= 2;
        } else if after_heading && t.is_empty() && pos > 0 {
            // Skip the blank line right after the heading when another
            // follows it: land on the empty paragraph line.
            return textweaver_core::CharPos(pos + line.chars().count());
        }
        pos += line.chars().count();
    }
    textweaver_core::CharPos(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_fill_their_placeholders() {
        let essay = &builtin()[0];
        let text = essay.fill("On Bees", "A. Writer", "2026-09-26");
        assert!(text.starts_with(
            "---\ntitle: \"On Bees\"\nauthor: \"A. Writer\"\ndate: 2026-09-26\n---\n"
        ));
        assert!(text.contains("# On Bees\n"));
        assert!(text.ends_with("## References\n"));
        let custom = Template {
            name: "x".into(),
            text: "{{ title }} by {{author}}".into(),
            user: true,
        };
        assert_eq!(custom.fill("T", "A", "D"), "T by A");
        assert_eq!(custom.label(), "x, your template");
    }

    #[test]
    fn dates_look_like_dates() {
        let d = local_date();
        assert_eq!(d.len(), 10, "{d}");
        assert_eq!(&d[4..5], "-");
        assert_eq!(utc_date().len(), 10);
    }

    #[test]
    fn writing_starts_under_the_first_section() {
        let text = builtin()[0].fill("T", "", "2026-09-26");
        let at = first_writing_line(&text).0;
        let before: String = text.chars().take(at).collect();
        assert!(before.ends_with("## Introduction\n\n"), "{before:?}");
    }

    #[test]
    fn user_templates_come_from_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Lab report.md"), "# {{title}}\r\n").unwrap();
        std::fs::write(dir.path().join("skip.txt"), "no").unwrap();
        let t = user_templates(dir.path());
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].name, "Lab report");
        assert_eq!(t[0].text, "# {{title}}\n");
    }
}
