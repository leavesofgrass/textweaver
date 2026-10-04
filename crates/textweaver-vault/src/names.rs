//! Note file names: sanitizing and de-duplicating.

use std::collections::HashSet;

/// Longest note name, in chars (star's limit).
pub const MAX_NAME_CHARS: usize = 120;

/// A file-name-safe note name from `name` (star's `_sanitize_filename`):
/// `<>:"/\|?*` and control characters become spaces, runs of spaces
/// collapse, trailing dots and spaces go, at most 120 chars. Also avoids
/// Windows' reserved device names (`CON`, `NUL`, `COM1`, ...) and
/// Obsidian's link-breaking `[`, `]`, `#`, `^`, and `|`, which star let
/// through. An empty result is `note`.
pub fn sanitize(name: &str) -> String {
    let mapped: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*[]#^".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let collapsed = mapped.split_whitespace().collect::<Vec<_>>().join(" ");
    // A name ending in `.md` would make `Name.md.md`.
    let collapsed = match collapsed.len().checked_sub(3) {
        Some(at)
            if collapsed.is_char_boundary(at) && collapsed[at..].eq_ignore_ascii_case(".md") =>
        {
            collapsed[..at].to_owned()
        }
        _ => collapsed,
    };
    let mut out: String = collapsed.chars().take(MAX_NAME_CHARS).collect();
    while out.ends_with(['.', ' ']) {
        out.pop();
    }
    let out = out.trim_start().to_owned();
    if out.is_empty() {
        return "note".to_owned();
    }
    if is_reserved(&out) {
        return format!("{out} note");
    }
    out
}

/// Windows reserves these names (with any extension) for devices.
fn is_reserved(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit())
}

/// Hands out note names unique without regard to case (Windows and macOS
/// file systems, and Obsidian's link resolution, ignore case): `Name`,
/// then `Name 2`, `Name 3`, ... (star's rule).
#[derive(Debug, Default)]
pub struct NameAllocator {
    used: HashSet<String>,
}

impl NameAllocator {
    /// An allocator with nothing taken.
    pub fn new() -> Self {
        NameAllocator::default()
    }

    /// Marks `name` as taken.
    pub fn reserve(&mut self, name: &str) {
        self.used.insert(name.to_lowercase());
    }

    /// True when `name` is taken.
    pub fn is_taken(&self, name: &str) -> bool {
        self.used.contains(&name.to_lowercase())
    }

    /// A free name based on `base` (already sanitized), now taken.
    pub fn allocate(&mut self, base: &str) -> String {
        let mut candidate = base.to_owned();
        let mut n = 2;
        while self.is_taken(&candidate) {
            candidate = format!("{base} {n}");
            n += 1;
        }
        self.reserve(&candidate);
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_like_star_and_more() {
        assert_eq!(sanitize("a/b:c?"), "a b c");
        assert_eq!(sanitize("  Title.  "), "Title");
        assert_eq!(sanitize("Line\nbreak\tand|pipe"), "Line break and pipe");
        assert_eq!(sanitize("[[Link]] #tag ^block"), "Link tag block");
        assert_eq!(sanitize(""), "note");
        assert_eq!(sanitize("..."), "note");
        assert_eq!(sanitize("CON"), "CON note");
        assert_eq!(sanitize("com1"), "com1 note");
        assert_eq!(sanitize("Console"), "Console");
        assert_eq!(sanitize("Chapter 1.MD"), "Chapter 1");
        assert_eq!(sanitize(".md"), "note");
        assert_eq!(sanitize(&"é".repeat(200)).chars().count(), MAX_NAME_CHARS);
    }

    #[test]
    fn allocates_case_insensitively() {
        let mut a = NameAllocator::new();
        assert_eq!(a.allocate("Note"), "Note");
        assert_eq!(a.allocate("note"), "note 2");
        assert_eq!(a.allocate("NOTE"), "NOTE 3");
        a.reserve("Other");
        assert_eq!(a.allocate("other"), "other 2");
    }
}
