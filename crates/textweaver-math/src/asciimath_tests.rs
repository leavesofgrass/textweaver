//! Tests of the ASCIIMath symbol table and grammar.

use super::*;

/// The single top-level node of `m`.
fn only(m: &Math) -> &Node {
    match &m.root.kind {
        NodeKind::Row(items) if items.len() == 1 => &items[0],
        other => panic!("{} parsed to {other:?}", m.source),
    }
}

fn text_of(n: &Node) -> Option<&str> {
    n.token_text()
}

#[test]
fn every_symbol_is_matched_by_its_own_input() {
    for (input, am) in SYMBOLS {
        let chars: Vec<char> = input.chars().collect();
        let (got, n) = longest_symbol(&chars, 0).unwrap_or_else(|| panic!("{input}"));
        assert_eq!(n, chars.len(), "{input}");
        assert_eq!(got, *am, "{input}");
    }
}

#[test]
fn inputs_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for (input, _) in SYMBOLS {
        assert!(seen.insert(*input), "duplicate input {input}");
    }
}

/// Every table entry parses to the node its kind promises, without
/// diagnostics.
#[test]
fn every_symbol_parses_to_its_node() {
    for (input, am) in SYMBOLS {
        let src = match am {
            Am::FenceFn(..)
            | Am::Sqrt
            | Am::Accent(_)
            | Am::Cancel
            | Am::Font(_)
            | Am::Func(_)
            | Am::FuncIdent(_) => format!("{input}(x)"),
            Am::Root | Am::Frac | Am::Over | Am::Under => format!("{input}(a)(b)"),
            Am::Skip2 => format!("{input}(red)(x)"),
            Am::Text => format!("{input}(ab)"),
            Am::Quote => "\"ab\"".to_owned(),
            Am::Left(_) => format!("{input}x"),
            Am::Sub | Am::Sup | Am::Slash => format!("a{input}b"),
            _ => (*input).to_owned(),
        };
        let m = parse_asciimath(&src);
        let what = format!("{input} as {src}");
        match am {
            Am::Ident(s) => {
                let n = only(&m);
                assert!(matches!(n.kind, NodeKind::Ident(_)), "{what}");
                assert_eq!(text_of(n), Some(*s), "{what}");
            }
            Am::Op(s, class) => {
                let n = only(&m);
                assert_eq!(text_of(n), Some(*s), "{what}");
                assert_eq!(n.op_class(), Some(*class), "{what}");
            }
            Am::Large(s) => {
                let n = only(&m);
                assert_eq!(text_of(n), Some(*s), "{what}");
                assert_eq!(n.op_class(), Some(OpClass::Large), "{what}");
            }
            Am::LimFunc(s) => {
                assert!(
                    matches!(&only(&m).kind, NodeKind::Function(f) if f == s),
                    "{what}"
                );
            }
            Am::Func(name) | Am::FuncIdent(name) => {
                let NodeKind::Row(items) = &only(&m).kind else {
                    panic!("{what}");
                };
                assert_eq!(text_of(&items[0]), Some(*name), "{what}");
                assert!(matches!(items[1].kind, NodeKind::Fenced { .. }), "{what}");
            }
            Am::FenceFn(o, c) => {
                let NodeKind::Fenced { open, close, .. } = &only(&m).kind else {
                    panic!("{what}");
                };
                assert_eq!((open.as_str(), close.as_str()), (*o, *c), "{what}");
            }
            Am::Sqrt => assert!(
                matches!(only(&m).kind, NodeKind::Root { index: None, .. }),
                "{what}"
            ),
            Am::Root => assert!(
                matches!(only(&m).kind, NodeKind::Root { index: Some(_), .. }),
                "{what}"
            ),
            Am::Frac | Am::Slash => assert!(
                matches!(only(&m).kind, NodeKind::Fraction { bar: true, .. }),
                "{what}"
            ),
            Am::Over => assert!(
                matches!(
                    only(&m).kind,
                    NodeKind::Scripts {
                        sup: Some(_),
                        limits: true,
                        ..
                    }
                ),
                "{what}"
            ),
            Am::Under => assert!(
                matches!(
                    only(&m).kind,
                    NodeKind::Scripts {
                        sub: Some(_),
                        limits: true,
                        ..
                    }
                ),
                "{what}"
            ),
            Am::Skip2 => assert_eq!(text_of(only(&m)), Some("x"), "{what}"),
            Am::Accent(k) => assert!(
                matches!(&only(&m).kind, NodeKind::Accent { accent, .. } if accent == k),
                "{what}"
            ),
            Am::Cancel => assert!(matches!(only(&m).kind, NodeKind::Enclose { .. }), "{what}"),
            Am::Font(v) => assert!(
                matches!(&only(&m).kind, NodeKind::Style { variant, .. } if variant == v),
                "{what}"
            ),
            Am::Text | Am::Quote => assert!(
                matches!(&only(&m).kind, NodeKind::Text(t) if t == "ab"),
                "{what}"
            ),
            Am::Word(w) => assert!(
                matches!(&only(&m).kind, NodeKind::Text(t) if t == w),
                "{what}"
            ),
            Am::Space(_) => assert!(matches!(only(&m).kind, NodeKind::Space(_)), "{what}"),
            Am::Left(o) => assert!(
                matches!(&only(&m).kind, NodeKind::Fenced { open, .. } if open == o),
                "{what}"
            ),
            Am::Right(c) => {
                assert!(!m.diagnostics.is_empty(), "{what}");
                if !c.is_empty() {
                    assert_eq!(text_of(only(&m)), Some(*c), "{what}");
                }
            }
            Am::LeftRight(s) => {
                let n = only(&m);
                assert_eq!(text_of(n), Some(*s), "{what}");
                assert_eq!(n.op_class(), Some(OpClass::Fence), "{what}");
            }
            Am::Sub => assert!(
                matches!(only(&m).kind, NodeKind::Scripts { sub: Some(_), .. }),
                "{what}"
            ),
            Am::Sup => assert!(
                matches!(only(&m).kind, NodeKind::Scripts { sup: Some(_), .. }),
                "{what}"
            ),
            Am::Def(target) => {
                if target.starts_with("{:") {
                    assert!(
                        matches!(&only(&m).kind, NodeKind::Row(items) if items.len() == 2),
                        "{what}"
                    );
                } else {
                    let t = parse_asciimath(target);
                    assert_eq!(text_of(only(&m)), text_of(only(&t)), "{what}");
                }
            }
        }
        // Left brackets without a close are reported; everything else is clean.
        if !matches!(am, Am::Right(_) | Am::Left(_)) {
            assert!(m.is_clean(), "{what}: {:?}", m.diagnostics);
        }
    }
}

#[test]
fn fractions_drop_brackets() {
    let m = parse_asciimath("(a+b)/(c+d)");
    let NodeKind::Fraction { num, den, .. } = &only(&m).kind else {
        panic!()
    };
    assert!(matches!(&num.kind, NodeKind::Row(i) if i.len() == 3));
    assert!(matches!(&den.kind, NodeKind::Row(i) if i.len() == 3));
}

#[test]
fn sums_take_limits() {
    let m = parse_asciimath("sum_(i=1)^n i");
    let NodeKind::Row(items) = &m.root.kind else {
        panic!()
    };
    assert!(matches!(
        items[0].kind,
        NodeKind::Scripts { limits: true, .. }
    ));
}

#[test]
fn matrices_and_cases() {
    let m = parse_asciimath("[[1,2],[3,4]]");
    let NodeKind::Fenced { body, .. } = &only(&m).kind else {
        panic!()
    };
    assert!(matches!(
        &body.kind,
        NodeKind::Table { kind: TableKind::Matrix, rows } if rows.len() == 2 && rows[0].len() == 2
    ));
    let m = parse_asciimath("{(1, x > 0),(0, x <= 0):}");
    let NodeKind::Fenced { body, .. } = &only(&m).kind else {
        panic!()
    };
    assert!(matches!(
        &body.kind,
        NodeKind::Table {
            kind: TableKind::Cases,
            ..
        }
    ));
    // Rows of different lengths are not a matrix.
    let m = parse_asciimath("[(1,2),(3)]");
    let NodeKind::Fenced { body, .. } = &only(&m).kind else {
        panic!()
    };
    assert!(matches!(body.kind, NodeKind::Row(_)));
}

#[test]
fn bars_signs_and_numbers() {
    let m = parse_asciimath("|x|");
    assert!(matches!(&only(&m).kind, NodeKind::Fenced { open, .. } if open == "|"));
    let m = parse_asciimath("x^-1");
    let NodeKind::Scripts { sup: Some(sup), .. } = &only(&m).kind else {
        panic!()
    };
    assert!(matches!(&sup.kind, NodeKind::Row(i) if i.len() == 2));
    let m = parse_asciimath("3.14");
    assert!(matches!(&only(&m).kind, NodeKind::Number(n) if n == "3.14"));
}
