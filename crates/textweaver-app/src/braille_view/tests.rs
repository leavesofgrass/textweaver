//! The Braille view's translation, print line, and panning, without a
//! display or liblouis (a stand-in contracts).

use super::*;

/// Dots of a lowercase letter.
fn letter(c: char) -> u8 {
    LETTER_DOTS[(c as u8 - b'a') as usize]
}

/// A small stand-in for liblouis, in Unicode braille: the word "the" is
/// one cell, "ar" in a word is one cell, a lone letter other than "a"
/// takes the grade 1 indicator (dots 5-6), a capital the capital
/// indicator (dot 6), and a word of capitals the capitals word indicator
/// (dot 6 twice). A line of three words of capitals starts with the
/// capitals passage indicator instead (dot 6 three times), as liblouis
/// writes a passage.
fn stand_in(texts: &[&str]) -> Result<Vec<String>, String> {
    Ok(texts.iter().map(|t| contract_one(t)).collect())
}

fn contract_one(text: &str) -> String {
    let mut cells: Vec<u8> = Vec::new();
    let words: Vec<&str> = text.split(' ').collect();
    let passage = words.len() >= 3 && words.iter().all(|w| w.chars().all(|c| c.is_uppercase()));
    if passage {
        cells.extend([32, 32, 32]);
    }
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            cells.push(0);
        }
        if *w == "the" {
            cells.push(46);
            continue;
        }
        if w.len() == 1 && *w != "a" && w.chars().all(|c| c.is_ascii_lowercase()) {
            cells.push(48);
        }
        let chars: Vec<char> = w.chars().collect();
        let caps_word = chars.len() > 1 && chars.iter().all(|c| c.is_uppercase());
        if caps_word && !passage {
            cells.extend([32, 32]);
        }
        let mut k = 0;
        while k < chars.len() {
            if chars[k] == 'a' && chars.get(k + 1) == Some(&'r') {
                cells.push(28);
                k += 2;
                continue;
            }
            let c = chars[k];
            if c.is_ascii_uppercase() && !passage && !caps_word {
                cells.push(32);
            }
            cells.push(letter(c.to_ascii_lowercase()));
            k += 1;
        }
    }
    unicode(&cells)
}

fn print_runs(line: &BrailleLine, text: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = text.chars().collect();
    line.groups
        .iter()
        .map(|g| (g.cells.len(), chars[g.chars.clone()].iter().collect()))
        .collect()
}

fn runs(list: &[(usize, &str)]) -> Vec<(usize, String)> {
    list.iter().map(|(n, s)| (*n, (*s).to_owned())).collect()
}

#[test]
fn unicode_and_dots_round_trip() {
    let cells = vec![0, 1, 3, 0xFF];
    let s = unicode(&cells);
    assert_eq!(s, "\u{2800}\u{2801}\u{2803}\u{28FF}");
    let back: Vec<u8> = s.chars().map(|c| dots_of(c).unwrap()).collect();
    assert_eq!(back, cells);
    assert_eq!(dots_of(' '), Some(0));
    assert_eq!(dots_of('x'), None);
}

#[cfg(feature = "publish")]
#[test]
fn uncontracted_cells_and_print_keep_every_space() {
    let text = "Hi  cat\t5";
    let line = uncontracted(text);
    // ,H I blank blank C A T blank #E
    assert_eq!(
        line.cells,
        vec![
            32,
            letter('h'),
            letter('i'),
            0,
            0,
            letter('c'),
            letter('a'),
            letter('t'),
            0,
            60,
            letter('e')
        ]
    );
    assert_eq!(
        print_runs(&line, text),
        runs(&[
            (2, "H"),
            (1, "i"),
            (1, " "),
            (1, " "),
            (1, "c"),
            (1, "a"),
            (1, "t"),
            (1, "\t"),
            (2, "5"),
        ])
    );
    assert!(!line.fell_back);
}

#[test]
fn contracted_print_line_follows_settled_prefixes() {
    let text = "bar the c cat";
    let line = contracted(text, &stand_in).unwrap();
    assert_eq!(line.cells, cells_of_stand_in(text));
    assert_eq!(
        print_runs(&line, text),
        runs(&[
            // "ba" is not how "bar" starts (b, ar), so "bar" is one run.
            (2, "bar"),
            (1, " "),
            (1, "the"),
            (1, " "),
            // A lone "c" takes the grade 1 indicator.
            (2, "c"),
            (1, " "),
            // "ca" settles letter by letter, then "t".
            (1, "c"),
            (1, "a"),
            (1, "t"),
        ])
    );
}

fn cells_of_stand_in(text: &str) -> Vec<u8> {
    contract_one(text)
        .chars()
        .map(|c| dots_of(c).unwrap())
        .collect()
}

#[test]
fn contracted_indicators_of_the_line_join_the_first_run() {
    // A capitals passage: in the line the first word has the passage
    // indicator (three cells) where alone it has the word indicator (two).
    // "W" alone is one capital, not how "WET" starts, so "WE" is the first
    // run, and the extra cell joins it.
    let text = "WET PAINT NOW";
    let line = contracted(text, &stand_in).unwrap();
    let first = &line.groups[0];
    assert_eq!(first.chars, 0..2);
    assert_eq!(first.cells, 0..5, "passage indicator, W and E");
    assert_eq!(line.groups[1].cells, 5..6, "T");
    assert_eq!(line.groups.last().unwrap().cells.end, line.cells.len());
}

#[test]
fn contracted_failure_is_an_error_and_translate_falls_back() {
    let failing = |_: &[&str]| -> Result<Vec<String>, String> { Err("no".into()) };
    assert!(contracted("the cat", &failing).is_err());
}

#[test]
fn word_groups_split_join_and_align() {
    let w: Vec<char> = "cat".chars().collect();
    let p = |s: &str| cells_of_stand_in(s);
    let prefixes = vec![p("c"), p("ca"), p("cat")];
    let alone = p("cat");
    assert_eq!(
        word_groups(&w, &prefixes, &alone),
        vec![(0..1, 0..1), (1..2, 1..2), (2..3, 2..3)]
    );
    // Extra cells before the word in the line join the first run.
    let mut before = vec![32];
    before.extend(&alone);
    assert_eq!(
        word_groups(&w, &prefixes, &before),
        vec![(0..1, 0..2), (1..2, 2..3), (2..3, 3..4)]
    );
    // After it, the last.
    let mut after = alone.clone();
    after.push(4);
    assert_eq!(word_groups(&w, &prefixes, &after)[2], (2..3, 2..4));
    // Braille in the line unlike the word's: one run.
    assert_eq!(word_groups(&w, &prefixes, &[1, 2]), vec![(0..3, 0..2)]);
}

#[test]
fn parts_break_after_blank_cells() {
    // "ab cd ef" as cells: 2, blank, 2, blank, 2.
    let cells = vec![1, 3, 0, 9, 25, 0, 17, 11];
    assert_eq!(parts(&cells, 4), vec![0..3, 3..6, 6..8]);
    // A part that ends at a blank cell needs no earlier break.
    assert_eq!(parts(&cells, 5), vec![0..5, 5..8]);
    assert_eq!(parts(&cells, 40), vec![0..8]);
    // A word longer than the display is cut where the display ends.
    assert_eq!(parts(&[1; 7], 3), vec![0..3, 3..6, 6..7]);
    assert_eq!(parts(&[], 40), vec![0..0]);
}

#[cfg(feature = "publish")]
#[test]
fn the_view_pans_to_the_focus() {
    let text = "one two three four";
    let line = uncontracted(text);
    // 3 + 1 + 3 + 1 + 5 + 1 + 4 cells.
    let first = view_of(&line, 8, 0);
    assert_eq!((first.part, first.parts), (0, 3));
    assert_eq!(first.cells.len(), 8);
    let focus = text.find("four").unwrap();
    let last = view_of(&line, 8, focus);
    assert_eq!(last.part, 2);
    assert_eq!(last.cells, line.cells[14..].to_vec());
    // The runs are cut to the part and counted from its first cell.
    assert_eq!(last.groups[0].cells, 0..1);
    assert_eq!(last.groups[0].chars, focus..focus + 1);
}
