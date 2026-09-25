//! `cargo xtask parity`: segmentation parity report against
//! `fixtures/star-parity/`. Owner: Agent A.

pub fn run() -> anyhow::Result<()> {
    let loaders = textweaver_formats::Registry::with_builtins().ids();
    let _ = (
        textweaver_text::Document::from_plain_text(""),
        serde_json::Value::Null,
    );
    anyhow::bail!("the parity report is not implemented yet (loaders: {loaders:?})")
}
