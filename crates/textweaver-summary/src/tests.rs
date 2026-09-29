use textweaver_core::{CharPos, CharRange, MarkerKind};
use textweaver_text::{Document, DocumentMeta, Marker};

use super::*;

const ARTICLE: &str = "\
Photosynthesis lets green plants turn sunlight into chemical energy.
The process takes place in the chloroplasts of leaf cells.

Chlorophyll in the chloroplasts absorbs sunlight for photosynthesis.
Plants use that energy to make sugar from water and carbon dioxide.
Oxygen is released into the air as a by-product of photosynthesis.

My neighbor painted her fence blue last summer.
Sugar made by photosynthesis feeds the plant and the animals that eat plants.
";

fn doc(text: &str) -> Document {
    Document::from_plain_text(text)
}

#[test]
fn picks_central_sentences_in_document_order() {
    let d = doc(ARTICLE);
    let s = summarize(&d, 3);
    assert_eq!(s.len(), 3);
    for pair in s.windows(2) {
        assert!(pair[0].0.end <= pair[1].0.start, "in order: {s:?}");
    }
    let texts: Vec<&str> = s.iter().map(|(_, t)| t.as_str()).collect();
    assert!(
        !texts.iter().any(|t| t.contains("fence")),
        "the off-topic sentence is not central: {texts:?}"
    );
    assert!(texts.iter().any(|t| t.contains("photosynthesis")));
}

#[test]
fn ranges_lie_inside_the_document_and_match_the_text() {
    let d = doc(ARTICLE);
    for (range, text) in summarize(&d, 10) {
        assert!(range.end.0 <= d.len_chars());
        assert!(range.start < range.end);
        assert_eq!(one_line(&d.slice(range)), text);
    }
}

#[test]
fn is_deterministic() {
    let d = doc(ARTICLE);
    let a = summarize_with(&d, &Options::default());
    let b = summarize_with(&d, &Options::default());
    assert_eq!(a, b);
}

#[test]
fn k_larger_than_the_document_gives_every_candidate() {
    let d = doc(ARTICLE);
    let s = summarize_with(
        &d,
        &Options {
            sentences: 100,
            ..Options::default()
        },
    );
    assert_eq!(s.sentences.len(), s.candidates);
    assert_eq!(s.candidates, 7);
    assert!(!s.sampled());
}

#[test]
fn empty_and_zero_give_nothing() {
    assert!(summarize(&doc(""), 5).is_empty());
    assert!(summarize(&doc(ARTICLE), 0).is_empty());
    assert!(summarize(&doc("Too short. Also short."), 5).is_empty());
}

#[test]
fn headings_and_code_blocks_are_not_summary_sentences() {
    let text = "Photosynthesis and the energy of plants\n\n\
        Photosynthesis turns sunlight into energy in green plants.\n\n\
        let photosynthesis = plants.energy(sunlight);\n\n\
        Plants store the energy of sunlight as sugar.\n";
    let heading_end = text.find('\n').unwrap_or(0);
    let code_start = text.find("let ").unwrap_or(0);
    let code_end = code_start + text[code_start..].find('\n').unwrap_or(0);
    let chars = |b: usize| text[..b].chars().count();
    let mut heading = Marker::new(MarkerKind::Heading, CharRange::new(0, chars(heading_end)));
    heading.level = 1;
    let mut code = Marker::new(
        MarkerKind::Code,
        CharRange::new(chars(code_start), chars(code_end)),
    );
    code.level = 1;
    let d = Document::new(DocumentMeta::default(), text.into(), vec![heading, code]);
    let s = summarize(&d, 10);
    let texts: Vec<&str> = s.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Photosynthesis turns sunlight into energy in green plants.",
            "Plants store the energy of sunlight as sugar."
        ]
    );
}

#[test]
fn summarizes_a_range_only() {
    let d = doc(ARTICLE);
    let second = ARTICLE.find("Chlorophyll").unwrap_or(0);
    let third = ARTICLE.find("My neighbor").unwrap_or(0);
    let range = CharRange::new(second, third);
    let s = summarize_range(&d, range, &Options::default());
    assert_eq!(s.candidates, 3);
    for x in &s.sentences {
        assert!(range.contains_range(x.range), "{x:?} inside {range:?}");
    }
}

#[test]
fn big_documents_are_sampled_evenly() {
    let mut text = String::new();
    for i in 0..400 {
        text.push_str(&format!(
            "Paragraph {i} talks about readers, voices, and accessible documents. \
             Students with print disabilities read documents with speech.\n\n"
        ));
    }
    let d = doc(&text);
    let options = Options {
        max_candidates: 50,
        text_budget: 20_000,
        ..Options::default()
    };
    let s = summarize_with(&d, &options);
    assert!(s.sampled());
    assert!(s.chars_read <= 20_000, "{}", s.chars_read);
    assert!(s.chars_read < s.chars);
    assert!(s.ranked <= 50);
    assert_eq!(s.sentences.len(), 5);
}

#[test]
fn paragraph_sampling_spreads_through_the_document() {
    let text = "Some words in a short paragraph here.\n\n".repeat(1000);
    let d = doc(&text);
    let all = paragraphs_to_read(&d, d.full_range(), usize::MAX);
    assert_eq!(all.len(), 1000);
    let budget = 64 * 100;
    let some = paragraphs_to_read(&d, d.full_range(), budget);
    assert!(some.len() >= 64, "{}", some.len());
    assert!(some.iter().map(CharRange::len).sum::<usize>() <= budget);
    assert!(some.windows(2).all(|w| w[0].end <= w[1].start));
    let last = some.last().map_or(CharPos::ZERO, |p| p.start);
    assert!(last.0 > d.len_chars() * 9 / 10, "{last:?}");
    let whole = "Some words in a short paragraph here.";
    for p in &some {
        let piece = d.slice(*p);
        assert!(
            !piece.is_empty() && (whole.starts_with(&piece) || whole.ends_with(&piece)),
            "{piece:?}"
        );
    }
    let s = summarize_with(
        &d,
        &Options {
            text_budget: budget,
            ..Options::default()
        },
    );
    assert!(s.sampled());
    for x in &s.sentences {
        assert_eq!(x.text, whole, "only whole sentences");
    }
}

#[test]
fn one_long_paragraph_is_cut_to_the_budget() {
    let text = "A sentence with enough words to count. ".repeat(5000);
    let d = doc(&text);
    let read = paragraphs_to_read(&d, d.full_range(), 6400);
    let total: usize = read.iter().map(CharRange::len).sum();
    assert!(total <= 6400, "{total}");
    let s = summarize_with(
        &d,
        &Options {
            text_budget: 6400,
            ..Options::default()
        },
    );
    assert!(s.sampled());
    assert_eq!(s.sentences.len(), 5);
}

#[test]
fn spread_is_even_increasing_and_complete_when_small() {
    assert_eq!(spread(5, 10), vec![0, 1, 2, 3, 4]);
    let s = spread(1000, 7);
    assert_eq!(s.len(), 7);
    assert!(s.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(s[0], 0);
    assert!(s[6] >= 800);
}

#[test]
fn tokenizer_drops_stop_words_digits_and_possessives() {
    let (words, terms) = tokenize("The reader's voice doesn't read 1990 pages, a b.");
    assert_eq!(words, 9);
    assert_eq!(terms, ["reader", "voice", "read", "pages"]);
    let (_, terms) = tokenize("Ünïcode Wörter zählen");
    assert_eq!(terms, ["ünïcode", "wörter", "zählen"]);
}

#[test]
fn lexrank_scores_sum_to_one_and_favor_the_hub() {
    let s: Vec<Vec<&str>> = vec![
        vec!["plant", "energy"],
        vec!["plant", "sugar"],
        vec!["plant", "energy", "sugar"],
        vec!["fence", "blue"],
    ];
    let refs: Vec<&[&str]> = s.iter().map(Vec::as_slice).collect();
    let p = lexrank(&refs);
    let sum: f64 = p.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6, "{sum}");
    assert!(p[2] > p[0] && p[2] > p[1] && p[2] > p[3], "{p:?}");
    assert!(lexrank::<&str>(&[]).is_empty());
}

mod props {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Ranges lie inside the document, in order, never overlap, and
        /// there are at most k of them.
        #[test]
        fn ranges_inside_and_ordered(
            words in proptest::collection::vec("[a-z]{1,9}|[.!?\n ]", 0..400),
            k in 0usize..12,
        ) {
            let text = words.join(" ");
            let d = doc(&text);
            let s = summarize(&d, k);
            prop_assert!(s.len() <= k);
            for (r, _) in &s {
                prop_assert!(r.start < r.end);
                prop_assert!(r.end.0 <= d.len_chars());
            }
            for w in s.windows(2) {
                prop_assert!(w[0].0.end <= w[1].0.start);
            }
        }
    }
}
