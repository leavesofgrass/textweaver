//! Property tests: parsers never panic on arbitrary input, and what the
//! writers produce reads back the same.

use proptest::prelude::*;
use textweaver_cite::{
    CiteItem, Format, Locator, LocatorLabel, Name, Reference, formats, key, pandoc,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn citation_finder_never_panics(s in "\\PC{0,200}") {
        for c in pandoc::find_citations(&s) {
            prop_assert!(c.range.start < c.range.end && c.range.end <= s.len());
            prop_assert!(s.is_char_boundary(c.range.start) && s.is_char_boundary(c.range.end));
            prop_assert_eq!(c.chars.end.0 - c.chars.start.0, s[c.range.clone()].chars().count());
        }
    }

    #[test]
    fn citation_finder_survives_citation_like_noise(s in "[\\[\\]@;,\\- a-z0-9p.{}\\\\]{0,80}") {
        let _ = pandoc::find_citations(&s);
        let _ = pandoc::citation_at(&s, s.chars().count() / 2);
    }

    #[test]
    fn reference_parsers_never_panic(s in "\\PC{0,300}") {
        let _ = formats::parse(&s, Format::BibLatex);
        let _ = formats::parse(&s, Format::Ris);
        let _ = formats::parse(&s, Format::CslJson);
        let _ = Name::parse(&s);
        let _ = textweaver_cite::Identifier::parse(&s);
        let _ = textweaver_cite::text::plain_title(&s);
    }

    #[test]
    fn written_citations_parse_back(
        keys in prop::collection::vec("[a-z][a-z0-9]{0,8}(:[a-z0-9]{1,4})?", 1..4),
        page in prop::option::of(1u32..999),
        prefix in prop::option::of("[a-z]{2,6}"),
        suppress in any::<bool>(),
    ) {
        let items: Vec<CiteItem> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| CiteItem {
                key: k.clone(),
                prefix: if i == 0 { prefix.clone().unwrap_or_default() } else { String::new() },
                locator: page.filter(|_| i == 0).map(|p| Locator { label: LocatorLabel::Page, value: p.to_string() }),
                suffix: String::new(),
                suppress_author: suppress && i == 0,
            })
            .collect();
        let text = pandoc::write_citation(&items, false);
        let found = pandoc::find_citations(&text);
        prop_assert_eq!(found.len(), 1, "{}", text);
        prop_assert_eq!(&found[0].items, &items, "{}", text);
    }

    #[test]
    fn bibtex_and_ris_keep_titles_and_names(
        title in "[A-Za-z0-9 &%$#_{}:,.'?!-]{1,60}",
        family in "[A-Z][a-z]{1,10}",
        given in "[A-Z][a-z]{0,10}",
        year in 1500i32..2100,
    ) {
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        prop_assume!(!title.is_empty());
        let mut r = Reference::new("k", "book");
        r.title = Some(title.clone());
        r.author = vec![Name::new(&family, &given)];
        r.issued = Some(textweaver_cite::CslDate::year(year));
        for format in [Format::BibTex, Format::BibLatex, Format::Ris, Format::CslJson] {
            let text = formats::write(std::slice::from_ref(&r), format).unwrap();
            let back = formats::parse(&text, format).unwrap();
            prop_assert_eq!(back.len(), 1);
            prop_assert_eq!(back[0].title.as_deref(), Some(title.as_str()), "{:?}\n{}", format, text);
            prop_assert_eq!(&back[0].author, &r.author, "{:?}", format);
            prop_assert_eq!(back[0].year(), Some(year), "{:?}", format);
        }
    }

    #[test]
    fn generated_keys_are_valid_pandoc_keys(family in "\\PL{1,12}", year in prop::option::of(1000i32..2100)) {
        let mut r = Reference::new("", "book");
        r.author = vec![Name::new(&family, "")];
        r.issued = year.map(textweaver_cite::CslDate::year);
        let k = key::base_key(&r);
        prop_assert!(key::is_valid_key(&k), "{}", k);
    }
}
