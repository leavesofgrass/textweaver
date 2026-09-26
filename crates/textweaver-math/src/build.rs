//! Helpers shared by the two parsers: row construction and bracket pairing.

use textweaver_core::CharRange;

use crate::{Node, NodeKind, OpClass};

/// Deepest nesting the parsers build; deeper input becomes error nodes, so
/// arbitrary input can never overflow the stack.
pub(crate) const MAX_DEPTH: usize = 48;

/// A node for a list of items: the item itself when there is one, otherwise
/// a row spanning the items (or an empty row at `at`).
pub(crate) fn row_or_single(mut items: Vec<Node>, at: usize) -> Node {
    if items.len() == 1 {
        return items.remove(0);
    }
    let span = items_span(&items).unwrap_or(CharRange::empty(at));
    Node::new(NodeKind::Row(items), span)
}

/// A row node with the given span.
pub(crate) fn row(items: Vec<Node>, span: CharRange) -> Node {
    Node::new(NodeKind::Row(items), span)
}

/// The range from the first item's start to the last item's end.
pub(crate) fn items_span(items: &[Node]) -> Option<CharRange> {
    let first = items.first()?;
    let last = items.last()?;
    Some(CharRange::new(
        first.span.start,
        last.span.end.max(first.span.end),
    ))
}

/// Appends `extra` to a script that already exists (`x^a^b` is an error in
/// LaTeX; the second script joins the first instead of nesting, so the tree
/// never grows deeper than the input's bracket nesting).
pub(crate) fn append(existing: Node, extra: Node) -> Node {
    let span = existing.span.cover(extra.span);
    match existing.kind {
        NodeKind::Row(mut items) => {
            items.push(extra);
            Node::new(NodeKind::Row(items), span)
        }
        kind => Node::new(
            NodeKind::Row(vec![Node::new(kind, existing.span), extra]),
            span,
        ),
    }
}

fn opener(n: &Node) -> Option<&str> {
    match &n.kind {
        NodeKind::Operator {
            text,
            class: OpClass::Open | OpClass::Fence,
        } => Some(text),
        _ => None,
    }
}

/// The closing bracket of `n`: a closing or fence operator, possibly
/// carrying scripts (`(a+b)^2` has its `^2` on the `)`).
fn closer(n: &Node) -> Option<(&str, OpClass)> {
    match &n.kind {
        NodeKind::Operator { text, class } if matches!(class, OpClass::Close | OpClass::Fence) => {
            Some((text, *class))
        }
        NodeKind::Scripts { base, .. } => match &base.kind {
            NodeKind::Operator { text, class }
                if matches!(class, OpClass::Close | OpClass::Fence) =>
            {
                Some((text, *class))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Pairs brackets typed as plain characters in a row (`(a+b)`, `|x|`,
/// `\lfloor x \rfloor`) into [`NodeKind::Fenced`] nodes, so speech can say
/// "the absolute value of x" and navigation can enter the group. Scripts on
/// a closing bracket move to the whole group. Unmatched brackets stay as
/// operators.
pub(crate) fn group_fences(items: Vec<Node>) -> Vec<Node> {
    let any = items
        .iter()
        .any(|n| opener(n).is_some() || closer(n).is_some());
    if !any {
        return items;
    }
    let mut out: Vec<Node> = Vec::with_capacity(items.len());
    // Indices into `out` of unmatched openers, with whether they are fences.
    let mut stack: Vec<(usize, bool)> = Vec::new();
    for item in items {
        let close = closer(&item).map(|(t, c)| (t.to_owned(), c));
        let fence_open = matches!(
            &item.kind,
            NodeKind::Operator {
                class: OpClass::Fence,
                ..
            }
        );
        let mut matched: Option<usize> = None;
        if let Some((text, class)) = &close {
            if *class == OpClass::Fence {
                // A fence closes the innermost open fence of the same kind,
                // if it is the innermost opener.
                if let Some(&(idx, true)) = stack.last()
                    && opener(&out[idx]) == Some(text.as_str())
                {
                    matched = Some(idx);
                }
            } else {
                // A closing bracket closes the innermost non-fence opener;
                // fences left open inside stay plain operators.
                if let Some(pos) = stack.iter().rposition(|&(_, f)| !f) {
                    matched = Some(stack[pos].0);
                }
            }
        }
        match matched {
            Some(idx) => {
                stack.retain(|&(i, _)| i < idx);
                let body: Vec<Node> = out.drain(idx + 1..).collect();
                let Some(open) = out.pop() else {
                    out.push(item);
                    continue;
                };
                out.push(close_group(open, body, item));
            }
            None => {
                let is_open = matches!(
                    &item.kind,
                    NodeKind::Operator {
                        class: OpClass::Open,
                        ..
                    }
                ) || fence_open;
                if is_open {
                    stack.push((out.len(), fence_open));
                }
                out.push(item);
            }
        }
    }
    out
}

fn close_group(open: Node, body: Vec<Node>, close: Node) -> Node {
    let open_text = open.token_text().unwrap_or_default().to_owned();
    let body_span = items_span(&body).unwrap_or(CharRange::empty(open.span.end));
    let body = Node::new(NodeKind::Row(body), body_span);
    match close.kind {
        NodeKind::Scripts {
            base,
            sub,
            sup,
            limits,
        } => {
            let close_text = base.token_text().unwrap_or_default().to_owned();
            let fenced = Node::new(
                NodeKind::Fenced {
                    open: open_text,
                    close: close_text,
                    body: Box::new(body),
                },
                CharRange::new(open.span.start, base.span.end),
            );
            Node::new(
                NodeKind::Scripts {
                    base: Box::new(fenced),
                    sub,
                    sup,
                    limits,
                },
                CharRange::new(open.span.start, close.span.end),
            )
        }
        kind => {
            let close_text = match &kind {
                NodeKind::Operator { text, .. } => text.clone(),
                _ => String::new(),
            };
            Node::new(
                NodeKind::Fenced {
                    open: open_text,
                    close: close_text,
                    body: Box::new(body),
                },
                CharRange::new(open.span.start, close.span.end),
            )
        }
    }
}
