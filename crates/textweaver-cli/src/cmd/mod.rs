//! Subcommands. Each file is owned by one agent (docs/history/tasks.md).

pub mod backends;
pub mod cite;
pub mod convert;
pub mod convert_layout;
pub mod define;
pub mod dictate;
pub mod eloquence;
pub mod export_audio;
pub mod info;
pub mod library;
pub mod lint;
pub mod marks;
pub mod migrate;
pub mod ocr;
pub mod open;
pub mod profiles;
pub mod search;
pub mod serve;
pub mod settings;
pub mod speak;
pub mod stats;
pub mod summarize;
pub mod text;
pub mod vault;
pub mod voices;

/// Writes `text` to standard output. A closed pipe (`tw text big.pdf |
/// head`) ends the output quietly instead of a panic about "failed printing
/// to stdout"; any other failure is reported.
pub(crate) fn print_all(text: &str) -> anyhow::Result<()> {
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    let result = out.write_all(text.as_bytes()).and_then(|()| out.flush());
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn print_all_writes_and_reports_nothing_on_success() {
        assert!(super::print_all("").is_ok());
    }
}
