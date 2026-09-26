//! Heading ids: GitHub-style slugs, made unique within a document.

use std::collections::HashMap;

/// A GitHub-style slug (`textweaver_text::slug::slugify`, the one rule
/// the reader also follows links by).
pub use textweaver_text::slug::slugify;

/// Hands out unique ids: a repeated slug gets `-1`, `-2`, and so on.
#[derive(Debug, Default)]
pub struct Slugger {
    seen: HashMap<String, usize>,
}

impl Slugger {
    /// Reserves `id` exactly (an explicit id from the source); later slugs
    /// that collide with it get a suffix.
    pub fn reserve(&mut self, id: &str) -> String {
        self.seen.entry(id.to_owned()).or_insert(0);
        id.to_owned()
    }

    /// A unique id for heading text.
    pub fn unique(&mut self, text: &str) -> String {
        let base = slugify(text);
        let mut candidate = base.clone();
        loop {
            match self.seen.get_mut(&base) {
                None => {
                    self.seen.insert(base.clone(), 0);
                    return candidate;
                }
                Some(n) => {
                    *n += 1;
                    candidate = format!("{base}-{n}");
                    if !self.seen.contains_key(&candidate) {
                        self.seen.insert(candidate.clone(), 0);
                        return candidate;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_get_suffixes() {
        let mut s = Slugger::default();
        assert_eq!(s.unique("Intro"), "intro");
        assert_eq!(s.unique("Intro"), "intro-1");
        assert_eq!(s.unique("Intro"), "intro-2");
        s.reserve("notes");
        assert_eq!(s.unique("Notes"), "notes-1");
    }
}
