//! Subcommands. Each file is owned by one agent (docs/tasks.md).

pub mod backends;
pub mod info;
pub mod marks;
pub mod migrate;
pub mod open;
pub mod search;
pub mod serve;
pub mod speak;
pub mod text;
pub mod voices;

/// The error every Phase 0 subcommand returns.
pub fn not_implemented(name: &str) -> anyhow::Result<()> {
    anyhow::bail!("`tw {name}` is not implemented yet")
}
