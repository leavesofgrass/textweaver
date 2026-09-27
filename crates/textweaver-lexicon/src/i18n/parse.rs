//! The Fluent subset's parser (see the module docs of `i18n`).

use std::collections::{BTreeSet, HashMap};

/// A piece of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// Plain text.
    Text(String),
    /// `{ $name }`
    Var(String),
    /// `{ -name }`
    Term(String),
    /// `{ other-message }`
    Ref(String),
    /// `{ $var -> [key] ... *[key] ... }`
    Select {
        var: String,
        variants: Vec<(String, Vec<Part>)>,
        default: usize,
    },
}

/// A problem in a catalog file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct CatalogError {
    /// The line, from 1.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

type Messages = HashMap<String, Vec<Part>>;

/// Parses a whole catalog into messages and terms.
pub(crate) fn resource(text: &str) -> Result<(Messages, Messages), Vec<CatalogError>> {
    let mut messages = HashMap::new();
    let mut terms = HashMap::new();
    let mut errors = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_end_matches('\r');
        let start = i;
        i += 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |message: String| CatalogError {
            line: start + 1,
            message,
        };
        if line.starts_with(char::is_whitespace) {
            errors.push(err("indented text outside a message".into()));
            continue;
        }
        let Some((id, first)) = line.split_once('=') else {
            errors.push(err("expected `id = text`".into()));
            continue;
        };
        let id = id.trim();
        let (is_term, name) = match id.strip_prefix('-') {
            Some(n) => (true, n),
            None => (false, id),
        };
        if !valid_id(name) {
            errors.push(err(format!("`{id}` is not a valid message id")));
            continue;
        }
        // The value: the rest of the line and the indented lines after it.
        let mut value_lines: Vec<&str> = vec![first.trim()];
        while i < lines.len() {
            let l = lines[i].trim_end_matches('\r');
            if l.starts_with(char::is_whitespace) && !l.trim().is_empty() {
                if l.trim_start().starts_with('.') {
                    errors.push(CatalogError {
                        line: i + 1,
                        message: "attributes are not supported".into(),
                    });
                }
                value_lines.push(l.trim());
                i += 1;
            } else if l.trim().is_empty()
                && lines
                    .get(i + 1)
                    .is_some_and(|n| n.starts_with(char::is_whitespace) && !n.trim().is_empty())
            {
                value_lines.push("");
                i += 1;
            } else {
                break;
            }
        }
        while value_lines.first().is_some_and(|l| l.is_empty()) {
            value_lines.remove(0);
        }
        let value = value_lines.join("\n");
        if value.is_empty() {
            errors.push(err(format!("`{id}` has no text")));
            continue;
        }
        match pattern(&value) {
            Ok(parts) => {
                let map = if is_term { &mut terms } else { &mut messages };
                if map.insert(name.to_owned(), parts).is_some() {
                    errors.push(err(format!("`{id}` is defined twice")));
                }
            }
            Err(e) => errors.push(err(format!("`{id}`: {e}"))),
        }
    }
    if errors.is_empty() {
        Ok((messages, terms))
    } else {
        Err(errors)
    }
}

fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Parses a message's text into parts.
fn pattern(text: &str) -> Result<Vec<Part>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let parts = pattern_until(&chars, &mut at, false)?;
    if at < chars.len() {
        return Err("unexpected `}`".into());
    }
    Ok(parts)
}

/// Text and placeables up to the end, or (in a variant) the end of the
/// line or a `}` that closes the select.
fn pattern_until(chars: &[char], at: &mut usize, variant: bool) -> Result<Vec<Part>, String> {
    let mut parts = Vec::new();
    let mut text = String::new();
    while *at < chars.len() {
        let c = chars[*at];
        if c == '}' || (variant && c == '\n') {
            break;
        }
        if c == '{' {
            *at += 1;
            if !text.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut text)));
            }
            match placeable(chars, at)? {
                Part::Text(t) => text.push_str(&t),
                p => parts.push(p),
            }
            continue;
        }
        text.push(c);
        *at += 1;
    }
    if variant {
        let t = text.trim_end().to_owned();
        text = t;
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(parts)
}

fn skip_space(chars: &[char], at: &mut usize) {
    while *at < chars.len() && chars[*at].is_whitespace() {
        *at += 1;
    }
}

fn name(chars: &[char], at: &mut usize) -> String {
    let mut s = String::new();
    while *at < chars.len()
        && (chars[*at].is_ascii_alphanumeric() || matches!(chars[*at], '-' | '_'))
    {
        s.push(chars[*at]);
        *at += 1;
    }
    s
}

/// A placeable, after its `{`, through its `}`.
fn placeable(chars: &[char], at: &mut usize) -> Result<Part, String> {
    skip_space(chars, at);
    let part = match chars.get(*at) {
        Some('$') => {
            *at += 1;
            let var = name(chars, at);
            if var.is_empty() {
                return Err("a `$` without a variable name".into());
            }
            skip_space(chars, at);
            if chars.get(*at) == Some(&'-') && chars.get(*at + 1) == Some(&'>') {
                *at += 2;
                return select(chars, at, var);
            }
            Part::Var(var)
        }
        Some('-') => {
            *at += 1;
            let n = name(chars, at);
            if n.is_empty() {
                return Err("a `-` without a term name".into());
            }
            Part::Term(n)
        }
        Some('"') => {
            *at += 1;
            let mut s = String::new();
            loop {
                match chars.get(*at) {
                    Some('"') => {
                        *at += 1;
                        break;
                    }
                    Some('\\') => {
                        if let Some(&n) = chars.get(*at + 1) {
                            s.push(n);
                        }
                        *at += 2;
                    }
                    Some(&c) => {
                        s.push(c);
                        *at += 1;
                    }
                    None => return Err("a string literal is not closed".into()),
                }
            }
            Part::Text(s)
        }
        Some(c) if c.is_ascii_alphabetic() => Part::Ref(name(chars, at)),
        _ => return Err("expected a variable, term, message, or string in `{ }`".into()),
    };
    skip_space(chars, at);
    if chars.get(*at) != Some(&'}') {
        return Err("a `{` is not closed".into());
    }
    *at += 1;
    Ok(part)
}

/// The variants of a select, after `->`, through the closing `}`.
fn select(chars: &[char], at: &mut usize, var: String) -> Result<Part, String> {
    let mut variants = Vec::new();
    let mut default = None;
    loop {
        skip_space(chars, at);
        match chars.get(*at) {
            Some('}') => {
                *at += 1;
                break;
            }
            Some('*') => {
                if default.is_some() {
                    return Err("two default variants".into());
                }
                default = Some(variants.len());
                *at += 1;
            }
            Some('[') => {}
            _ => return Err("expected `[key]` in a select".into()),
        }
        if chars.get(*at) != Some(&'[') {
            return Err("expected `[` after `*`".into());
        }
        *at += 1;
        skip_space(chars, at);
        let key = name(chars, at);
        skip_space(chars, at);
        if key.is_empty() || chars.get(*at) != Some(&']') {
            return Err("a variant key is not closed with `]`".into());
        }
        *at += 1;
        while chars.get(*at) == Some(&' ') {
            *at += 1;
        }
        let parts = pattern_until(chars, at, true)?;
        variants.push((key, parts));
    }
    let Some(default) = default else {
        return Err("a select needs a default variant, marked with `*`".into());
    };
    Ok(Part::Select {
        var,
        variants,
        default,
    })
}

/// The variables used in `parts`.
pub(crate) fn collect_vars(parts: &[Part], out: &mut BTreeSet<String>) {
    for p in parts {
        match p {
            Part::Var(v) => {
                out.insert(v.clone());
            }
            Part::Select { var, variants, .. } => {
                out.insert(var.clone());
                for (_, v) in variants {
                    collect_vars(v, out);
                }
            }
            _ => {}
        }
    }
}
