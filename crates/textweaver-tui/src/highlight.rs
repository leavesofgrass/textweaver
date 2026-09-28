//! Code blocks drawn with syntax highlighting (Agent W4g, ADR-0032).
//!
//! A fenced code block with a language (```` ```rust ````) is split into
//! tokens with syntect (pure-Rust regular expressions, `regex-fancy`) and
//! bat's syntax definitions from two-face. Each token kind takes a color
//! from the theme's own roles, and most kinds an attribute too, so color
//! never carries meaning alone: comments are italic and dim, keywords
//! bold. The text is unchanged, and textweaver says the language when the
//! caret or Speech Cursor enters the block ("code, Rust").
//!
//! Blocks without a language, or in a language bat does not know, are
//! drawn in the theme's code colors only. Blocks over [`MAX_BYTES`] are
//! not tokenized, so a huge block never slows drawing. Tokens are cached
//! by the block's language and text.

use std::collections::HashMap;
use std::sync::OnceLock;

use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Largest block tokenized, in bytes.
pub const MAX_BYTES: usize = 256 * 1024;

/// Most blocks kept in the cache before it is cleared.
const CACHE_LIMIT: usize = 256;

/// What a token is, as far as drawing goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    /// Anything else.
    Plain,
    /// A comment.
    Comment,
    /// A keyword, or a storage word (`fn`, `let`, `class`).
    Keyword,
    /// A string or character literal.
    String,
    /// A number or other constant (`true`, `None`).
    Number,
    /// A function's name, defined or called.
    Function,
    /// A type's or class's name.
    Type,
}

/// bat's syntaxes, loaded on first use (about 50 ms).
fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// The token kind for the innermost scope that says anything.
fn classify(stack: &ScopeStack) -> Token {
    for scope in stack.as_slice().iter().rev() {
        let name = scope.build_string();
        let is = |prefix: &str| name == prefix || name.starts_with(&format!("{prefix}."));
        let token = if is("comment") {
            Token::Comment
        } else if is("string") || is("constant.character") {
            Token::String
        } else if is("constant") {
            Token::Number
        } else if is("entity.name.function") || is("support.function") || is("variable.function") {
            Token::Function
        } else if is("entity.name.type")
            || is("entity.name.class")
            || is("entity.name.struct")
            || is("entity.name.enum")
            || is("support.type")
            || is("support.class")
        {
            Token::Type
        } else if is("keyword.operator") {
            // Operators stay plain: a bold `=` is noise.
            Token::Plain
        } else if is("keyword") || is("storage") {
            Token::Keyword
        } else {
            continue;
        };
        return token;
    }
    Token::Plain
}

/// The tokens of `code` in `language` (a fence's info word: `rust`, `py`,
/// `bash`): char ranges from the block's start, in order, without
/// [`Token::Plain`]. `None` when the language is unknown or the block too
/// big.
pub fn tokens(language: &str, code: &str) -> Option<Vec<(usize, usize, Token)>> {
    if code.len() > MAX_BYTES {
        return None;
    }
    let set = syntaxes();
    let syntax = set
        .find_syntax_by_token(language)
        .or_else(|| set.find_syntax_by_name(language))?;
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut out: Vec<(usize, usize, Token)> = Vec::new();
    let mut push = |a: usize, b: usize, t: Token| {
        if a >= b || t == Token::Plain {
            return;
        }
        match out.last_mut() {
            Some(last) if last.1 == a && last.2 == t => last.1 = b,
            _ => out.push((a, b, t)),
        }
    };
    let mut line_start = 0usize;
    for line in LinesWithEndings::from(code) {
        let ops = state.parse_line(line, set).ok()?;
        // Byte offsets in the line to chars from the block's start.
        let char_at = |byte: usize| line_start + line[..byte.min(line.len())].chars().count();
        let mut last = 0usize;
        for (i, op) in ops {
            if i > last {
                push(char_at(last), char_at(i), classify(&stack));
            }
            stack.apply(&op).ok()?;
            last = i;
        }
        if last < line.len() {
            push(char_at(last), char_at(line.len()), classify(&stack));
        }
        line_start += line.chars().count();
    }
    Some(out)
}

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

    fn kinds(language: &str, code: &str) -> Vec<(String, Token)> {
        let chars: Vec<char> = code.chars().collect();
        tokens(language, code)
            .unwrap()
            .into_iter()
            .map(|(a, b, t)| (chars[a..b].iter().collect(), t))
            .collect()
    }

    #[test]
    fn rust_keywords_strings_comments_and_numbers() {
        let found = kinds(
            "rust",
            "// hello\nfn main() { let s = \"é\"; let n = 42; }\n",
        );
        let has = |text: &str, t: Token| found.iter().any(|(s, k)| s.trim() == text && *k == t);
        assert!(has("// hello", Token::Comment), "{found:?}");
        assert!(has("fn", Token::Keyword), "{found:?}");
        assert!(has("main", Token::Function), "{found:?}");
        assert!(has("\"é\"", Token::String), "{found:?}");
        assert!(has("42", Token::Number), "{found:?}");
    }

    #[test]
    fn short_names_and_unknown_languages() {
        assert!(tokens("py", "def f():\n    return 1\n").is_some());
        assert!(tokens("bash", "echo hi # there\n").is_some());
        assert!(tokens("no-such-language", "text").is_none());
        let big = "x".repeat(MAX_BYTES + 1);
        assert!(tokens("rust", &big).is_none());
    }

    #[test]
    fn the_cache_returns_the_same_tokens() {
        let mut cache = Cache::default();
        let a = cache.tokens("rust", "let x = 1;\n").unwrap();
        let b = cache.tokens("rust", "let x = 1;\n").unwrap();
        assert!(std::rc::Rc::ptr_eq(&a, &b));
        assert!(cache.tokens("nope", "x").is_none());
    }
}
