//! Code blocks drawn with syntax highlighting (Agent W4g, ADR-0032).
//!
//! The tokens come from `textweaver_render::highlight`, the same syntect
//! syntaxes (bat's, from two-face) and token kinds the HTML pages use.
//! Each token kind takes a color from the theme's own roles, and most
//! kinds an attribute too, so color never carries meaning alone: comments
//! are italic and dim, keywords bold. The text is unchanged, and
//! textweaver says the language when the caret or Speech Cursor enters
//! the block ("code, Rust").
//!
//! Blocks without a language, or in a language bat does not know, are
//! drawn in the theme's code colors only. Blocks over
//! `textweaver_render::highlight::MAX_BYTES` are not tokenized, so a huge
//! block never slows drawing. Tokens are cached by the block's language
//! and text.

use std::collections::HashMap;

pub use textweaver_render::highlight::{Token, tokens};

/// Most blocks kept in the cache before it is cleared.
const CACHE_LIMIT: usize = 256;

/// A block's tokens, shared between draws.
pub type Tokens = std::rc::Rc<Vec<(usize, usize, Token)>>;

/// Tokens by language and block text, so a block is tokenized once.
#[derive(Debug, Default)]
pub struct Cache {
    blocks: HashMap<(String, String), Option<Tokens>>,
}

impl Cache {
    /// The tokens of a block, from the cache when it was seen before.
    pub fn tokens(&mut self, language: &str, code: &str) -> Option<Tokens> {
        let key = (language.to_owned(), code.to_owned());
        if let Some(found) = self.blocks.get(&key) {
            return found.clone();
        }
        if self.blocks.len() >= CACHE_LIMIT {
            self.blocks.clear();
        }
        let found = tokens(language, code).map(std::rc::Rc::new);
        self.blocks.insert(key, found.clone());
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_returns_the_same_tokens() {
        let mut cache = Cache::default();
        let a = cache.tokens("rust", "let x = 1;\n").unwrap();
        let b = cache.tokens("rust", "let x = 1;\n").unwrap();
        assert!(std::rc::Rc::ptr_eq(&a, &b));
        assert!(cache.tokens("nope", "x").is_none());
    }
}
