//! Math navigation: a pure model for moving through an expression by term,
//! into a fraction's numerator and denominator, into scripts and roots, and
//! back out, speaking each part and reporting its source span so the app
//! can highlight it.
//!
//! The navigator never mutates the tree and never speaks by itself; each
//! move returns a [`NavStep`] (or `None` at a boundary, where the position
//! does not change and the app announces the boundary).

use serde::{Deserialize, Serialize};
use textweaver_core::CharRange;

use crate::speech::{SpeechOptions, Spoken, speak_node};
use crate::{Math, Node, NodeKind};

/// The part of its parent a navigation position is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The whole expression.
    Expression,
    /// One item of a row (a term, operator, or group).
    Term,
    /// A fraction's numerator.
    Numerator,
    /// A fraction's denominator.
    Denominator,
    /// A root's index.
    Index,
    /// The expression under a root sign.
    Radicand,
    /// The base of scripts or an accent.
    Base,
    /// A subscript.
    Subscript,
    /// A superscript.
    Superscript,
    /// The limit below a large operator or `lim`.
    LowerLimit,
    /// The limit above a large operator.
    UpperLimit,
    /// The contents of brackets or an enclosure.
    Contents,
    /// A table cell (1-based row and column).
    Cell {
        /// Row, from 1.
        row: usize,
        /// Column, from 1.
        column: usize,
    },
}

impl Role {
    /// The role as spoken before the part ("numerator"), or an empty string
    /// for plain terms.
    pub fn name(self) -> String {
        match self {
            Role::Expression => "expression".into(),
            Role::Term => String::new(),
            Role::Numerator => "numerator".into(),
            Role::Denominator => "denominator".into(),
            Role::Index => "index".into(),
            Role::Radicand => "radicand".into(),
            Role::Base => "base".into(),
            Role::Subscript => "subscript".into(),
            Role::Superscript => "superscript".into(),
            Role::LowerLimit => "lower limit".into(),
            Role::UpperLimit => "upper limit".into(),
            Role::Contents => "contents".into(),
            Role::Cell { row, column } => format!("row {row}, column {column}"),
        }
    }
}

/// What the navigator reports after a move.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavStep {
    /// The part's role in its parent.
    pub role: Role,
    /// The part spoken on its own, with its map into [`Math::source`].
    pub spoken: Spoken,
    /// The part's source chars, for highlighting.
    pub span: CharRange,
    /// Nesting depth: 0 for the whole expression.
    pub depth: usize,
    /// True when [`Navigator::enter`] can move into the part.
    pub has_children: bool,
}

impl NavStep {
    /// The role and the spoken part together ("numerator, x plus 1").
    pub fn announcement(&self) -> String {
        let role = self.role.name();
        match (role.is_empty(), self.spoken.text.is_empty()) {
            (true, _) => self.spoken.text.clone(),
            (false, true) => role,
            (false, false) => format!("{role}, {}", self.spoken.text),
        }
    }
}

/// A position in a math tree with moves between its parts.
#[derive(Clone, Debug)]
pub struct Navigator<'m> {
    math: &'m Math,
    opts: SpeechOptions,
    /// Child indices from the root, as listed by `children`.
    path: Vec<usize>,
}

impl<'m> Navigator<'m> {
    /// A navigator at the whole expression.
    pub fn new(math: &'m Math, opts: SpeechOptions) -> Self {
        Navigator {
            math,
            opts,
            path: Vec::new(),
        }
    }

    /// The child indices from the root to the current part.
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    /// The current part.
    pub fn current(&self) -> NavStep {
        let (role, node) = self.resolve(&self.path);
        self.step(role, node)
    }

    /// The next part at the same level ("next term").
    pub fn next_part(&mut self) -> Option<NavStep> {
        let (&last, parent) = self.path.split_last()?;
        let (_, p) = self.resolve(parent);
        if last + 1 >= children(p).len() {
            return None;
        }
        if let Some(l) = self.path.last_mut() {
            *l += 1;
        }
        Some(self.current())
    }

    /// The previous part at the same level.
    pub fn previous_part(&mut self) -> Option<NavStep> {
        let last = *self.path.last()?;
        if last == 0 {
            return None;
        }
        if let Some(l) = self.path.last_mut() {
            *l -= 1;
        }
        Some(self.current())
    }

    /// The first part at the same level.
    pub fn first_part(&mut self) -> Option<NavStep> {
        let l = self.path.last_mut()?;
        *l = 0;
        Some(self.current())
    }

    /// The last part at the same level.
    pub fn last_part(&mut self) -> Option<NavStep> {
        let (_, parent) = self.path.split_last()?;
        let (_, p) = self.resolve(parent);
        let n = children(p).len();
        let l = self.path.last_mut()?;
        *l = n.checked_sub(1)?;
        Some(self.current())
    }

    /// Into the current part's first child (a fraction's numerator, a
    /// group's first term, a base).
    pub fn enter(&mut self) -> Option<NavStep> {
        let (_, node) = self.resolve(&self.path);
        if children(node).is_empty() {
            return None;
        }
        self.path.push(0);
        Some(self.current())
    }

    /// Into the current part's child with `role` (for example
    /// [`Role::Denominator`] or [`Role::Superscript`]).
    pub fn enter_role(&mut self, role: Role) -> Option<NavStep> {
        let (_, node) = self.resolve(&self.path);
        let k = children(node).iter().position(|(r, _)| *r == role)?;
        self.path.push(k);
        Some(self.current())
    }

    /// Out to the enclosing part.
    pub fn exit(&mut self) -> Option<NavStep> {
        self.path.pop()?;
        Some(self.current())
    }

    /// Back to the whole expression.
    pub fn reset(&mut self) -> NavStep {
        self.path.clear();
        self.current()
    }

    fn resolve(&self, path: &[usize]) -> (Role, &'m Node) {
        let mut role = Role::Expression;
        let mut node = &self.math.root;
        for &k in path {
            match children(node).get(k) {
                Some(&(r, n)) => {
                    role = r;
                    node = n;
                }
                None => break,
            }
        }
        (role, node)
    }

    fn step(&self, role: Role, node: &Node) -> NavStep {
        NavStep {
            role,
            spoken: speak_node(self.math, node, &self.opts),
            span: node.span,
            depth: self.path.len(),
            has_children: !children(node).is_empty(),
        }
    }
}

/// The navigable parts of `n`, in reading order. Rows with one item, fonts,
/// and brackets around a row are transparent.
fn children(n: &Node) -> Vec<(Role, &Node)> {
    match &n.kind {
        NodeKind::Row(items) => {
            let items: Vec<&Node> = items
                .iter()
                .filter(|i| !matches!(i.kind, NodeKind::Space(_)))
                .collect();
            if items.len() == 1 {
                return children(items[0]);
            }
            items.into_iter().map(|i| (Role::Term, i)).collect()
        }
        NodeKind::Fraction { num, den, .. } => {
            vec![(Role::Numerator, &**num), (Role::Denominator, &**den)]
        }
        NodeKind::Root { index, radicand } => {
            let mut v = Vec::new();
            if let Some(ix) = index {
                v.push((Role::Index, &**ix));
            }
            v.push((Role::Radicand, &**radicand));
            v
        }
        NodeKind::Scripts {
            base,
            sub,
            sup,
            limits,
        } => {
            let big = *limits
                || matches!(
                    &base.kind,
                    NodeKind::Operator {
                        class: crate::OpClass::Large,
                        ..
                    }
                );
            let mut v = vec![(Role::Base, &**base)];
            if let Some(s) = sub {
                v.push((
                    if big {
                        Role::LowerLimit
                    } else {
                        Role::Subscript
                    },
                    &**s,
                ));
            }
            if let Some(p) = sup {
                v.push((
                    if big {
                        Role::UpperLimit
                    } else {
                        Role::Superscript
                    },
                    &**p,
                ));
            }
            v
        }
        NodeKind::Fenced { body, .. } => match &body.kind {
            NodeKind::Row(items) if items.len() > 1 => children(body),
            NodeKind::Table { .. } => children(body),
            _ if body.is_empty_row() => Vec::new(),
            _ => vec![(Role::Contents, &**body)],
        },
        NodeKind::Accent { base, .. } => vec![(Role::Base, &**base)],
        NodeKind::Style { body, .. } => children(body),
        NodeKind::Enclose { body, .. } => vec![(Role::Contents, &**body)],
        NodeKind::Table { rows, .. } => rows
            .iter()
            .enumerate()
            .flat_map(|(r, cells)| {
                cells.iter().enumerate().map(move |(c, cell)| {
                    (
                        Role::Cell {
                            row: r + 1,
                            column: c + 1,
                        },
                        cell,
                    )
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}
