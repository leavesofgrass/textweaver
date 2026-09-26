//! Bionic reading: embolden the first part of each word so the eye has a
//! fixation point.
//!
//! Star's rule (`gui/mixin_fontspacing.py`, `_bionic_word`): every run of two
//! or more letters gets its leading `round(len × 0.4)` letters in bold, at
//! least one. textweaver keeps that as [`BionicOptions::star`] and makes the
//! ratio and the minimum length configurable.
//!
//! Differences from Star, all deliberate:
//!
//! - Lengths count grapheme clusters, so a combining accent is never split
//!   from its letter.
//! - Words are UAX #29 word segments (the same rule as navigation), so
//!   `don't` is one word, and a word with digits (`v2`, `COVID19`, `1990s`)
//!   is skipped whole. Star's letters-only regex bolded the `COVID` of
//!   `COVID19`.
//! - URLs, email addresses, code-like tokens (`snake_case`, `a::b`, `f()`),
//!   inline `` `code` `` spans, and, in a document, `Code` markers are
//!   skipped. Star skipped only `<code>` elements.
//!
//! The output is a list of canonical char ranges to embolden. It never
//! changes the text, so speech and highlighting are unaffected.

use serde::{Deserialize, Serialize};
use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;
use unicode_segmentation::UnicodeSegmentation;

use crate::util::{ByteToPos, SkipSet, code_marker_ranges, text_skip_ranges, word_segments};

/// Options for [`bionic_text`] and [`bionic_range`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BionicOptions {
    /// Share of each word to embolden, from 0.1 to 0.9. Star used 0.4.
    pub ratio: f32,
    /// Words shorter than this (in graphemes) are left alone. Star used 2.
    pub min_word_len: usize,
    /// Skip words that contain a digit.
    pub skip_numbers: bool,
    /// Skip URLs and email addresses.
    pub skip_urls: bool,
    /// Skip code: `Code` markers, inline `` `code` ``, and code-like tokens.
    pub skip_code: bool,
}

impl Default for BionicOptions {
    fn default() -> Self {
        Self::star()
    }
}

impl BionicOptions {
    /// Star's settings: 40 percent, words of two letters or more.
    pub fn star() -> Self {
        BionicOptions {
            ratio: 0.4,
            min_word_len: 2,
            skip_numbers: true,
            skip_urls: true,
            skip_code: true,
        }
    }

    /// Star's settings with a different fixation ratio.
    pub fn with_ratio(ratio: f32) -> Self {
        BionicOptions {
            ratio,
            ..Self::star()
        }
    }

    fn clamped_ratio(&self) -> f32 {
        if self.ratio.is_finite() {
            self.ratio.clamp(0.1, 0.9)
        } else {
            0.4
        }
    }
}

/// How many leading graphemes of a word of `len` graphemes to embolden:
/// `round(len × ratio)`, at least 1 and at most `len`. The ratio is clamped
/// to `0.1..=0.9`.
pub fn fixation_len(len: usize, ratio: f32) -> usize {
    if len == 0 {
        return 0;
    }
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.1, 0.9)
    } else {
        0.4
    };
    // Lengths are small; f64 is exact for them.
    let n = (len as f64 * f64::from(ratio)).round() as usize;
    n.clamp(1, len)
}

/// The ranges to embolden in `text`, whose first char is at `base`.
pub fn bionic_text(text: &str, base: CharPos, opts: &BionicOptions) -> Vec<CharRange> {
    let skip = SkipSet::new(text_skip_ranges(text, base, opts.skip_urls, opts.skip_code));
    bionic_with_skip(text, base, opts, &skip)
}

/// The ranges to embolden in `range` of `doc`. `Code` markers are skipped
/// when [`BionicOptions::skip_code`] is set.
pub fn bionic_range(doc: &Document, range: CharRange, opts: &BionicOptions) -> Vec<CharRange> {
    let range = range.clamp_to(doc.len_chars());
    let text = doc.slice(range);
    let mut skips = text_skip_ranges(&text, range.start, opts.skip_urls, opts.skip_code);
    if opts.skip_code {
        skips.extend(code_marker_ranges(doc, range));
    }
    bionic_with_skip(&text, range.start, opts, &SkipSet::new(skips))
}

fn bionic_with_skip(
    text: &str,
    base: CharPos,
    opts: &BionicOptions,
    skip: &SkipSet,
) -> Vec<CharRange> {
    let ratio = opts.clamped_ratio();
    let min = opts.min_word_len.max(1);
    let mut conv = ByteToPos::new(text, base);
    let mut out = Vec::new();
    for (b, word) in word_segments(text) {
        if opts.skip_numbers && word.chars().any(|c| c.is_numeric()) {
            continue;
        }
        let graphemes: Vec<&str> = word.graphemes(true).collect();
        if graphemes.len() < min {
            continue;
        }
        let start = conv.pos(b);
        let word_range = CharRange::new(start, start.saturating_add(word.chars().count()));
        if skip.overlaps(word_range) {
            continue;
        }
        let n = fixation_len(graphemes.len(), ratio);
        let chars: usize = graphemes[..n].iter().map(|g| g.chars().count()).sum();
        out.push(CharRange::new(start, start.saturating_add(chars)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_text::{Marker, core::MarkerKind};

    fn bold(text: &str, opts: &BionicOptions) -> Vec<String> {
        bionic_text(text, CharPos(0), opts)
            .into_iter()
            .map(|r| text.chars().skip(r.start.0).take(r.len()).collect())
            .collect()
    }

    #[test]
    fn star_rule() {
        // Star: n = max(1, round(len * 0.4)).
        assert_eq!(fixation_len(2, 0.4), 1);
        assert_eq!(fixation_len(3, 0.4), 1);
        assert_eq!(fixation_len(4, 0.4), 2);
        assert_eq!(fixation_len(5, 0.4), 2);
        assert_eq!(fixation_len(7, 0.4), 3);
        assert_eq!(fixation_len(10, 0.4), 4);
        assert_eq!(fixation_len(1, 0.9), 1);
        assert_eq!(fixation_len(0, 0.4), 0);
        assert_eq!(fixation_len(10, f32::NAN), 4);
        assert_eq!(
            bold("Reading is fun, a joy.", &BionicOptions::star()),
            ["Rea", "i", "f", "j"]
        );
    }

    #[test]
    fn ratio_is_configurable() {
        assert_eq!(
            bold("Reading helps", &BionicOptions::with_ratio(0.6)),
            ["Read", "hel"]
        );
    }

    #[test]
    fn skips_numbers_urls_and_code() {
        let t =
            "See https://example.org/page and COVID19 in 1990s via `cargo test` or snake_case now";
        assert_eq!(
            bold(t, &BionicOptions::star()),
            ["S", "a", "i", "v", "o", "n"]
        );
        let mut keep = BionicOptions::star();
        keep.skip_numbers = false;
        keep.skip_urls = false;
        keep.skip_code = false;
        assert!(bold(t, &keep).len() > 6);
    }

    #[test]
    fn graphemes_are_not_split() {
        // "e" + combining acute: two chars, one grapheme.
        let t = "cafe\u{301}s";
        let r = bionic_text(t, CharPos(0), &BionicOptions::with_ratio(0.8));
        // 5 graphemes * 0.8 = 4 graphemes: "cafe" plus the accent, 5 chars.
        assert_eq!(r, vec![CharRange::new(0, 5)]);
    }

    #[test]
    fn document_code_markers_are_skipped() {
        let mut doc = Document::from_plain_text("Words then code here\n");
        doc = Document::new(
            doc.meta.clone(),
            doc.text().clone(),
            vec![Marker {
                kind: MarkerKind::Code,
                range: CharRange::new(11, 20),
                level: 0,
                label: None,
                reference: None,
            }],
        );
        let r = bionic_range(&doc, doc.full_range(), &BionicOptions::star());
        assert_eq!(r, vec![CharRange::new(0, 2), CharRange::new(6, 8)]);
    }

    #[test]
    fn positions_are_offset_by_base() {
        let r = bionic_text("über alles", CharPos(100), &BionicOptions::star());
        assert_eq!(r, vec![CharRange::new(100, 102), CharRange::new(105, 107)]);
    }
}
