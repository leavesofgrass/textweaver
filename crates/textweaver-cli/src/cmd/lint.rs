//! `tw lint`: the Markdown lint of edit mode (Ctrl+F8) for files, from
//! the command line (Agent W4g, ADR-0032). Rules: heading levels, list
//! markers, trailing spaces, link references without a definition, and
//! bare web addresses; see `textweaver_app::lint`.
//!
//! Each problem is one line, meaning first: `Line 3: heading level 3 after
//! level 1; use level 2.` The exit status is 1 when any file has a problem,
//! so scripts can check.

use std::path::PathBuf;

use anyhow::Context as _;
use serde::Serialize;
use textweaver_app::lint::{LintProblem, lint_markdown};

/// Arguments for `tw lint`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Markdown files to check.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

/// One problem, for JSON.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Problem {
    file: PathBuf,
    /// Line number from 1.
    line: usize,
    /// Column from 1, in characters.
    column: usize,
    rule: &'static str,
    message: String,
}

/// The problems of `text` with their lines and columns.
fn located(file: &std::path::Path, text: &str, problems: Vec<LintProblem>) -> Vec<Problem> {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(
            text.chars()
                .enumerate()
                .filter(|(_, c)| *c == '\n')
                .map(|(i, _)| i + 1),
        )
        .collect();
    problems
        .into_iter()
        .map(|p| {
            let at = p.range.start.0;
            let line = starts.partition_point(|&s| s <= at);
            let column = at - starts[line.saturating_sub(1)] + 1;
            Problem {
                file: file.to_owned(),
                line,
                column,
                rule: p.rule,
                message: p.message,
            }
        })
        .collect()
}

/// The report for people: per file, a count, then one line per problem.
fn render(files: &[(PathBuf, Vec<Problem>)]) -> String {
    let mut out = String::new();
    for (file, problems) in files {
        let name = file.display();
        match problems.len() {
            0 => out.push_str(&format!("{name}: no lint problems.\n")),
            1 => out.push_str(&format!("{name}: 1 lint problem.\n")),
            n => out.push_str(&format!("{name}: {n} lint problems.\n")),
        }
        for p in problems {
            out.push_str(&format!("  Line {}: {}\n", p.line, p.message));
        }
    }
    out
}

/// Runs `tw lint`.
pub fn run(args: Args) -> anyhow::Result<()> {
    let mut files = Vec::new();
    for file in &args.files {
        let text = std::fs::read_to_string(file)
            .with_context(|| format!("Could not read {}", file.display()))?;
        let problems = located(file, &text, lint_markdown(&text));
        files.push((file.clone(), problems));
    }
    if args.json {
        let all: Vec<&Problem> = files.iter().flat_map(|(_, p)| p).collect();
        super::print_all(&format!("{}\n", serde_json::to_string_pretty(&all)?))?;
    } else {
        super::print_all(&render(&files))?;
    }
    if files.iter().any(|(_, p)| !p.is_empty()) {
        return Err(super::NothingFound.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_have_lines_and_read_meaning_first() {
        let file = PathBuf::from("notes.md");
        let text = "# Title\n\n### Deep\n\nSee https://example.org today.\n";
        let problems = located(&file, text, lint_markdown(text));
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert_eq!((problems[0].line, problems[0].column), (3, 1));
        assert_eq!((problems[1].line, problems[1].column), (5, 5));
        assert_eq!(problems[1].rule, "bare-url");
        let text = render(&[(file, problems)]);
        assert!(text.starts_with("notes.md: 2 lint problems.\n"), "{text}");
        assert!(
            text.contains("  Line 3: heading level 3 after level 1; use level 2.\n"),
            "{text}"
        );
        assert_eq!(
            render(&[(PathBuf::from("a.md"), Vec::new())]),
            "a.md: no lint problems.\n"
        );
    }
}
