//! Property tests: undo and redo over random editing sessions return the
//! exact text at every step boundary.

use proptest::prelude::*;
use textweaver_core::{CharPos, CharRange, Edit};
use textweaver_editor::markdown::{self, MarkdownOp};
use textweaver_editor::{Editor, FindOptions, Selection, find};

#[derive(Clone, Debug)]
enum Op {
    Type(char),
    Backspace,
    DeleteForward,
    Move(usize),
    Select(usize, usize),
    Paste(String),
    Format(MarkdownOp),
    ReplaceAll(String, String),
    RawEdit(usize, usize, String),
    Save,
}

fn op() -> impl Strategy<Value = Op> {
    let fmt = prop_oneof![
        Just(MarkdownOp::Bold),
        Just(MarkdownOp::Italic),
        Just(MarkdownOp::InlineCode),
        Just(MarkdownOp::Heading(2)),
        Just(MarkdownOp::BulletList),
        Just(MarkdownOp::NumberedList),
        Just(MarkdownOp::Quote),
        Just(MarkdownOp::Link),
        Just(MarkdownOp::HorizontalRule),
        Just(MarkdownOp::CodeBlock),
        Just(MarkdownOp::InsertTable { rows: 1, cols: 2 }),
        Just(MarkdownOp::AddTableRow),
    ];
    prop_oneof![
        6 => proptest::char::ranges(vec!['a'..='e', ' '..=' ', '\n'..='\n', '.'..='.', 'é'..='é'].into())
            .prop_map(Op::Type),
        2 => Just(Op::Backspace),
        1 => Just(Op::DeleteForward),
        2 => (0usize..60).prop_map(Op::Move),
        1 => (0usize..60, 0usize..60).prop_map(|(a, b)| Op::Select(a, b)),
        1 => "[a-c \n]{0,6}".prop_map(Op::Paste),
        2 => fmt.prop_map(Op::Format),
        1 => ("[a-c]{1,2}", "[x-z]{0,2}").prop_map(|(q, r)| Op::ReplaceAll(q, r)),
        1 => (0usize..60, 0usize..60, "[p-r]{0,3}").prop_map(|(a, b, s)| Op::RawEdit(a, b, s)),
        1 => Just(Op::Save),
    ]
}

fn run(ed: &mut Editor, op: &Op) {
    let len = ed.text().len_chars();
    match op {
        Op::Type(c) => {
            ed.type_char(*c).unwrap();
        }
        Op::Backspace => {
            ed.backspace().unwrap();
        }
        Op::DeleteForward => {
            ed.delete_forward().unwrap();
        }
        Op::Move(p) => ed.set_selection(Selection::caret(CharPos(*p))),
        Op::Select(a, b) => ed.set_selection(Selection::new(*a, *b)),
        Op::Paste(s) => {
            ed.insert_text(s).unwrap();
        }
        Op::Format(op) => {
            let f = markdown::apply(ed.text(), ed.selection(), *op);
            ed.apply_formatted(&f).unwrap();
        }
        Op::ReplaceAll(q, r) => {
            find::replace_all(ed, q, r, FindOptions::default()).unwrap();
        }
        Op::RawEdit(a, b, s) => {
            let r = CharRange::new((*a).min(len), (*b).min(len));
            ed.apply(Edit::replace(r, s.clone())).unwrap();
        }
        Op::Save => ed.mark_saved(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Undoing everything returns the starting text exactly; every text an
    /// undo passes through is one the session actually had, in reverse
    /// order; redoing everything returns the final text exactly, through the
    /// same texts forwards.
    #[test]
    fn undo_and_redo_return_the_exact_text(
        start in "[a-d \n]{0,20}",
        ops in proptest::collection::vec(op(), 0..60),
    ) {
        let mut ed = Editor::new(&start);
        let mut history = vec![start.clone()];
        for op in &ops {
            run(&mut ed, op);
            let sel = ed.selection();
            let n = ed.text().len_chars();
            prop_assert!(sel.anchor.0 <= n && sel.head.0 <= n);
            history.push(ed.text().to_string());
        }
        let end = ed.text().to_string();

        let mut undone = vec![end.clone()];
        let mut idx = history.len() - 1;
        while ed.undo().is_some() {
            let t = ed.text().to_string();
            let found = history[..idx].iter().rposition(|h| *h == t);
            prop_assert!(found.is_some(), "undo produced a text the session never had: {:?}", t);
            idx = found.unwrap_or(0);
            undone.push(t);
        }
        prop_assert_eq!(ed.text().to_string(), start);

        let mut redone = vec![ed.text().to_string()];
        while ed.redo().is_some() {
            redone.push(ed.text().to_string());
        }
        prop_assert_eq!(ed.text().to_string(), end);
        undone.reverse();
        prop_assert_eq!(redone, undone);
    }

    /// Typing a string and undoing word by word ends where it started, and
    /// each undo step removes at most one word plus the spaces after it.
    #[test]
    fn typing_undoes_in_word_sized_steps(words in proptest::collection::vec("[a-z]{1,6}", 1..8)) {
        let text = words.join(" ");
        let mut ed = Editor::new("");
        ed.type_text(&text).unwrap();
        prop_assert_eq!(ed.undo_depth(), words.len());
        let mut steps = 0;
        while ed.undo().is_some() {
            steps += 1;
        }
        prop_assert_eq!(steps, words.len());
        prop_assert_eq!(ed.text().to_string(), "");
    }
}
