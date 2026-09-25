//! `cargo xtask keyboard`: writes `docs/keyboard.md`. Owner: Agent C.

pub fn run() -> anyhow::Result<()> {
    let actions = textweaver_keymap::ActionId::ALL.len();
    anyhow::bail!("keyboard.md generation is not implemented yet ({actions} actions defined)")
}
