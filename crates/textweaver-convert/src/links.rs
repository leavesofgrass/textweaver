//! Wikilinks in converted web pages (the Obsidian flavor): `[[Note]]`
//! points to the page of the note it resolves to, wherever that note is in
//! the vault, through the vault crate's resolver, the same one the reader
//! follows links with. The relative path between the two notes is kept, so
//! it holds between their pages when a vault is converted as a whole.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use textweaver_render::{FsResolver, Resolver};
use textweaver_vault::{LinkScope, ScopeFiles};

/// A link scope and its files, walked once and shared by every note in it.
#[derive(Debug)]
pub(crate) struct Scope {
    scope: LinkScope,
    files: ScopeFiles,
}

impl Scope {
    /// Walks `scope`.
    pub(crate) fn new(scope: LinkScope) -> Self {
        let files = scope.files();
        Scope { scope, files }
    }
}

/// The resolver for one note: embeds from the input root's resolver, and
/// wikilinks to the pages of the notes they name.
pub(crate) struct NoteLinks<'a> {
    scope: Arc<Scope>,
    folder: PathBuf,
    embeds: Option<&'a FsResolver>,
}

impl<'a> NoteLinks<'a> {
    /// For the note at `source`.
    pub(crate) fn new(scope: Arc<Scope>, source: &Path, embeds: Option<&'a FsResolver>) -> Self {
        let source = textweaver_store::library::resolve_path(source);
        let folder = source.parent().map_or_else(PathBuf::new, Path::to_owned);
        NoteLinks {
            scope,
            folder,
            embeds,
        }
    }
}

impl Resolver for NoteLinks<'_> {
    fn resolve(&self, name: &str) -> Option<String> {
        self.embeds?.resolve(name)
    }

    fn link(&self, page: &str) -> Option<String> {
        let found =
            textweaver_vault::resolve(&self.scope.scope, &self.scope.files, &self.folder, page)?;
        let is_note = found
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if !is_note {
            return None;
        }
        let rel = relative(&self.folder, &found.path.with_extension(""))?;
        Some(rel)
    }
}

/// `to` relative to the folder `from`, with `/` between parts.
fn relative(from: &Path, to: &Path) -> Option<String> {
    let a: Vec<Component<'_>> = from.components().collect();
    let b: Vec<Component<'_>> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return None;
    }
    let mut parts: Vec<String> = vec!["..".to_owned(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_between_folders() {
        let from = Path::new("/v/Notes");
        assert_eq!(
            relative(from, Path::new("/v/Projects/Plan")).as_deref(),
            Some("../Projects/Plan")
        );
        assert_eq!(
            relative(from, Path::new("/v/Notes/deep/Leaf")).as_deref(),
            Some("deep/Leaf")
        );
        assert_eq!(
            relative(from, Path::new("/v/Notes/Here")).as_deref(),
            Some("Here")
        );
    }

    #[test]
    fn wikilinks_point_to_the_page_of_the_note_anywhere_in_the_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("vault");
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        std::fs::create_dir_all(root.join("Notes")).unwrap();
        std::fs::create_dir_all(root.join("Projects")).unwrap();
        std::fs::write(root.join("Projects/Plan.md"), "plan").unwrap();
        std::fs::write(root.join("Notes/pic.png"), "png").unwrap();
        let source = root.join("Notes/Main.md");
        std::fs::write(&source, "[[Plan]]").unwrap();
        let scope = Arc::new(Scope::new(LinkScope::for_note(&source)));
        let links = NoteLinks::new(scope, &source, None);
        assert_eq!(links.link("Plan").as_deref(), Some("../Projects/Plan"));
        assert_eq!(links.link("Missing"), None);
        // Attachments keep their link as written.
        assert_eq!(links.link("pic.png"), None);
        let html = textweaver_render::render_with(
            "See [[Plan#Goals|the plan]] and [[Missing]].",
            &textweaver_render::RenderOptions {
                flavor: textweaver_render::Flavor::Obsidian,
                ..textweaver_render::RenderOptions::default()
            },
            Some(&links),
        )
        .html;
        assert!(
            html.contains("href=\"../Projects/Plan.html#goals\""),
            "{html}"
        );
        assert!(html.contains("href=\"Missing.html\""), "{html}");
    }
}
