//! Writes the built-in theme files (`themes/*.toml`) from star's palettes
//! and prints the table of contrast adjustments as Markdown.
//!
//! Run: `cargo run -p textweaver-theme --example generate_builtin_themes`

use std::path::PathBuf;

use textweaver_theme::star;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("themes");
    std::fs::create_dir_all(&dir)?;
    for p in &star::PALETTES {
        std::fs::write(
            dir.join(format!("{}.toml", p.name)),
            star::generated_file(p)?,
        )?;
    }
    println!("{}", star::adjustments_table()?);
    Ok(())
}
