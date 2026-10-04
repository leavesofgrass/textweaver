//! `cargo xtask parity`: segmentation parity report against
//! `fixtures/star-parity/`. Owner: Agent A.
//!
//! For each fixture, loads the document with textweaver's loaders and
//! compares its word tokens, sentence starts, and paragraph starts with what
//! star produced (the JSON export in `fixtures/star-parity/`). Canonical
//! texts differ by design (ADR-0002), so offsets are not compared directly:
//! the two word sequences are aligned (longest common subsequence of exact
//! token text), and sentence and paragraph starts are compared as word
//! positions through that alignment.
//!
//! Every difference is classified by a rule that names its cause. The report
//! is written to `parity-report.md` in the Cargo target folder
//! (`CARGO_TARGET_DIR`, or `target/` at the workspace root), or to the path
//! given with `--out <path>`. It is build output, not a tracked document. The
//! task fails if any difference is left unexplained, so a change in
//! segmentation cannot slip by.
//!
//! It also fails when the explained deltas change: the count of each rule
//! per fixture is committed in [`BASELINE`], so a change that moves
//! textweaver toward one of star's bugs (dropping `Dr.` from the
//! abbreviations makes an `abbreviation` delta vanish) fails as surely as
//! one that adds a delta. Unexplained deltas alone left that gate blind,
//! found by breaking it on purpose (docs/dev/testing.md, "Gates and what
//! breaks them"). When a change is meant, `cargo xtask parity
//! --update-baseline` rewrites the file; commit it with the reason.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use textweaver_formats::{LoadOptions, Registry, Source};
use textweaver_text::core::{CharRange, MarkerKind, Unit};
use textweaver_text::units::{ABBREVIATIONS, AMBIGUOUS_ABBREVIATIONS};
use textweaver_text::{Document, segments};

const FIXTURES: &[&str] = &["sample.txt", "sample.md", "sample.html"];
/// The committed delta counts, relative to the workspace root.
const BASELINE: &str = "xtask/parity-baseline.txt";
const UNEXPLAINED: &str = "UNEXPLAINED";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

pub fn run() -> Result<()> {
    let root = root();
    let mut report = String::new();
    let mut summary = String::new();
    let mut unexplained = 0usize;
    let mut used: BTreeSet<&'static str> = BTreeSet::new();
    let mut per_fixture: Vec<(&str, Counts)> = Vec::new();
    for name in FIXTURES {
        let (section, row, counts) = fixture_section(&root, name)?;
        report.push_str(&section);
        summary.push_str(&row);
        unexplained += counts.unexplained;
        used.extend(counts.by_rule.keys().copied());
        per_fixture.push((*name, counts));
    }
    let mut out = String::new();
    out.push_str(HEADER);
    out.push_str("## Summary\n\n");
    out.push_str(
        "| Fixture | star words | textweaver words | Aligned | Word deltas | Sentence starts (star / tw / shared) | Sentence deltas | Paragraph starts (star / tw / shared) | Paragraph deltas | Unexplained |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    out.push_str(&summary);
    out.push('\n');
    out.push_str("## Causes\n\n| Rule | Cause |\n|---|---|\n");
    for (rule, cause) in RULES {
        if used.contains(rule) {
            let _ = writeln!(out, "| `{rule}` | {cause} |");
        }
    }
    out.push('\n');
    out.push_str(&report);
    out.push_str(FOOTER);
    let args: Vec<String> = std::env::args().skip(2).collect();
    let update = args.iter().any(|a| a == "--update-baseline");
    let path = report_path(&root, args.into_iter().filter(|a| a != "--update-baseline"))?;
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    println!("wrote {}", path.display());
    if unexplained > 0 {
        bail!(
            "{unexplained} unexplained parity deltas; see {}",
            path.display()
        );
    }
    let current = baseline_text(&per_fixture);
    let baseline_path = root.join(BASELINE);
    if update {
        std::fs::write(&baseline_path, &current)
            .with_context(|| format!("writing {}", baseline_path.display()))?;
        println!("wrote {BASELINE}; commit it with the reason for the change");
        return Ok(());
    }
    let committed =
        std::fs::read_to_string(&baseline_path).with_context(|| format!("reading {BASELINE}"))?;
    let changes = baseline_changes(&committed, &current);
    if !changes.is_empty() {
        for c in &changes {
            println!("{c}");
        }
        bail!(
            "the parity deltas changed ({} {}); see {}. If the change is meant, run \
             cargo xtask parity --update-baseline and commit {BASELINE} with the reason",
            changes.len(),
            if changes.len() == 1 {
                "count"
            } else {
                "counts"
            },
            path.display()
        );
    }
    println!("Pass: the parity deltas match {BASELINE}");
    Ok(())
}

/// The committed form: one line per fixture and rule, `fixture rule count`.
fn baseline_text(per_fixture: &[(&str, Counts)]) -> String {
    let mut out = String::from(
        "# star parity: explained deltas per fixture and rule (cargo xtask parity).\n\
         # Written by `cargo xtask parity --update-baseline`; a change fails the task.\n",
    );
    for (name, counts) in per_fixture {
        for (rule, n) in &counts.by_rule {
            let _ = writeln!(out, "{name} {rule} {n}");
        }
    }
    out
}

/// Each count that differs, one line, meaning first.
fn baseline_changes(committed: &str, current: &str) -> Vec<String> {
    let parse = |t: &str| -> BTreeMap<(String, String), String> {
        t.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .filter_map(|l| {
                let mut p = l.split_whitespace();
                Some((
                    (p.next()?.to_owned(), p.next()?.to_owned()),
                    p.next()?.to_owned(),
                ))
            })
            .collect()
    };
    let (before, after) = (parse(committed), parse(current));
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
    keys.into_iter()
        .filter_map(|k| {
            let (b, a) = (before.get(k), after.get(k));
            (b != a).then(|| {
                format!(
                    "Changed: {} {}: {} deltas, was {}",
                    k.0,
                    k.1,
                    a.map_or("0", String::as_str),
                    b.map_or("0", String::as_str)
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod baseline_tests {
    use super::*;

    #[test]
    fn a_vanished_rule_is_a_change() {
        let committed = "# c\nsample.md abbreviation 1\nsample.md list-item 4\n";
        assert!(baseline_changes(committed, committed).is_empty());
        let now = "sample.md list-item 4\n";
        assert_eq!(
            baseline_changes(committed, now),
            ["Changed: sample.md abbreviation: 0 deltas, was 1"]
        );
        let more = "sample.md abbreviation 1\nsample.md list-item 5\n";
        assert_eq!(
            baseline_changes(committed, more),
            ["Changed: sample.md list-item: 5 deltas, was 4"]
        );
    }

    #[test]
    fn the_committed_baseline_is_well_formed() {
        let text = std::fs::read_to_string(root().join(BASELINE)).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
        assert!(!lines.is_empty());
        for l in lines {
            let p: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(p.len(), 3, "{l}");
            assert!(FIXTURES.contains(&p[0]), "{l}");
            assert!(p[2].parse::<usize>().is_ok(), "{l}");
        }
    }
}

/// Where the report goes: `--out <path>` (or `--out=<path>`) when given,
/// otherwise `parity-report.md` in `CARGO_TARGET_DIR`, or in `target/` at the
/// workspace root.
fn report_path(root: &Path, mut args: impl Iterator<Item = String>) -> Result<PathBuf> {
    let mut out = None;
    while let Some(arg) = args.next() {
        if arg == "--out" {
            out = Some(PathBuf::from(args.next().context("--out needs a path")?));
        } else if let Some(value) = arg.strip_prefix("--out=") {
            out = Some(PathBuf::from(value));
        } else {
            bail!("unknown option for `cargo xtask parity`: {arg} (expected --out <path>)");
        }
    }
    if let Some(out) = out {
        return Ok(out);
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .filter(|v| !v.is_empty())
        .map_or_else(|| root.join("target"), PathBuf::from);
    let target = if target.is_relative() {
        root.join(target)
    } else {
        target
    };
    Ok(target.join("parity-report.md"))
}

const HEADER: &str = "# star parity report

Generated by `cargo xtask parity` (`xtask/src/parity.rs`); do not edit by hand.

It compares textweaver's segmentation of the three fixtures with star 0.1.31's
(`fixtures/star-parity/*.json`, exported by `tools/star_parity_export.py`):

- **Words.** star's tokens (`\\b\\w[\\w'-]*` over its `plain_text`) against
  textweaver's `Unit::Word` segments of its canonical text.
- **Sentence starts.** star's sentence map (`sentence_start_words`, the word
  indices star navigates by) against the first word of each textweaver
  `Unit::Sentence`.
- **Paragraph starts.** star's `paragraph_starts` (after each blank-line run
  in `plain_text`) against the first word of each textweaver
  `Unit::Paragraph`.

The canonical texts differ by design (ADR-0002, Phase 0 amendment 6), so
nothing is compared by offset. The word sequences are aligned by their
longest common subsequence of exact token text; a sentence or paragraph start
is shared when both start at aligned words. Every delta below carries the rule
that explains it; the task fails on any delta no rule explains.

Documents are loaded with default options (`LoadOptions::default()`): code
kept in the text, footnotes deferred to a Footnotes section. For HTML, star's
default route is Pandoc (`prefer_pandoc = true` with Pandoc 3.9 on PATH);
star's native HTML route produces an empty document (bug 1), so there is
nothing else to compare against.

";

const FOOTER: &str = "## Differences the fixtures do not exercise

Deliberate rule differences from star that these fixtures happen not to
contain (each is covered by a unit test in `crates/textweaver-text`):

- **Trailing `-` and `'`.** star's regex makes them part of a word (`rock-`,
  `students'`); UAX #29 ends the word before them.
- **Sentence openers.** star starts a sentence only before an ASCII capital,
  a quote, or `(`; UAX #29 also before non-ASCII capitals (`Élan`) and after
  `?` or `!` followed by a digit.
- **Ambiguous abbreviations.** `a.m.`, `p.m.`, `etc.`, `Inc.` and similar end
  a sentence when the next word is capitalized (\"until 5 p.m. Then we
  left\"), and continue it otherwise.
- **Ellipsis.** As in star, `…` followed by a capital ends a sentence; `...`
  (three periods) follows UAX #29, which also ends a sentence before a
  capital.
- **Plain-text lines.** star joins every single line break into a space in
  its canonical text; textweaver keeps the lines (for line navigation) but a
  sentence still flows across them unless the next line starts a list item,
  table row, or other block.
- **Code lines.** In a code block every line is one sentence; prose rules do
  not split `println!(\"hi\")` after the `!`.
";

/// Every rule a delta can be explained by, with its cause.
const RULES: &[(&str, &str)] = &[
    (
        "uax29-join",
        r#"UAX #29 keeps letters or digits joined by `.`, `,`, `'` or `_` in one word (`e.g`, `2.0.1`, `12.50`, `1,250`, `example.org`); star's `\w` regex splits them. Word navigation and highlighting move over the whole token."#,
    ),
    (
        "front-matter",
        r#"star speaks Markdown front matter (bug 6); textweaver parses it into the document metadata (title, author)."#,
    ),
    (
        "table-narration",
        r#"star writes its structured table narration ("Table with 3 columns: …", "Row 1: Name is Ada, …") into the canonical text; textweaver keeps one row of cells per line and speaks the same words at narration time as inserted spans."#,
    ),
    (
        "table-layout",
        r#"star (via Pandoc) reads an HTML table as a Pandoc simple table, dashes and all; textweaver keeps one row of cells per line."#,
    ),
    (
        "caption",
        r#"Pandoc moves an HTML table's caption after the table (`: Scores`); textweaver reads it before the table and labels the table with it."#,
    ),
    (
        "code-kept",
        r#"star drops code blocks from its `plain_text` (`tts_skip_code`); textweaver keeps code in the text (it is displayed and navigable) and skips it when reading with `skip_code`."#,
    ),
    (
        "footnote-heading",
        r#"textweaver puts deferred footnote definitions under a "Footnotes" heading."#,
    ),
    (
        "abbreviation",
        r#"star splits a sentence after every abbreviation such as `Dr.` or `Mr.` (bug 11); textweaver's abbreviation list keeps it with the following words."#,
    ),
    (
        "list-item",
        r#"A list item is a sentence of its own in textweaver; star joins list items into one run (bug 4)."#,
    ),
    (
        "table-row",
        r#"A table row is a sentence of its own in textweaver."#,
    ),
    (
        "heading",
        r#"A heading is a sentence and paragraph of its own; star merges a heading into a following list (bug 3)."#,
    ),
    (
        "list-lines",
        r#"textweaver's list is one paragraph of item lines after a blank line; star's list-marker regex eats the blank line before a list, merging it with the heading above (bug 3)."#,
    ),
    (
        "footnote-reference",
        r#"A sentence ending in a footnote reference (`footnote.[1] It …`) ends after the reference in textweaver; star needs the period right before the space."#,
    ),
    (
        "no-counterpart",
        r#"The start falls on a word that exists in only one of the two texts (see the word deltas)."#,
    ),
];

/// One aligned or unaligned step through the two word sequences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Equal(usize, usize),
    Star(usize),
    Tw(usize),
}

/// Longest-common-subsequence alignment of two token sequences.
fn align(a: &[String], b: &[String]) -> Vec<Op> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut ops = Vec::new();
    while i < n && j < m {
        // Among equally long alignments, prefer skipping a star word when
        // that loses nothing, so text star has in front (front matter)
        // does not steal matches from the text both share.
        if a[i] == b[j] && dp[i + 1][j] < dp[i][j] {
            ops.push(Op::Equal(i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(Op::Star(i));
            i += 1;
        } else {
            ops.push(Op::Tw(j));
            j += 1;
        }
    }
    ops.extend((i..n).map(Op::Star));
    ops.extend((j..m).map(Op::Tw));
    ops
}

struct Star {
    text: Vec<char>,
    words: Vec<(usize, usize, String)>,
    sentence_words: Vec<usize>,
    paragraph_words: Vec<usize>,
}

fn star_data(v: &Value) -> Result<Star> {
    let text: Vec<char> = v["plain_text"]
        .as_str()
        .context("plain_text")?
        .chars()
        .collect();
    let words = v["word_tokens"]
        .as_array()
        .context("word_tokens")?
        .iter()
        .map(|t| {
            let s = t[0].as_u64().unwrap_or(0) as usize;
            let e = t[1].as_u64().unwrap_or(0) as usize;
            (s, e, t[2].as_str().unwrap_or("").to_owned())
        })
        .collect::<Vec<_>>();
    let nums = |key: &str| -> Result<Vec<usize>> {
        Ok(v[key]
            .as_array()
            .with_context(|| key.to_owned())?
            .iter()
            .filter_map(Value::as_u64)
            .map(|n| n as usize)
            .collect())
    };
    let sentence_words = nums("sentence_start_words")?;
    let paragraph_words = first_words_at(
        &nums("paragraph_starts")?,
        &words.iter().map(|w| w.0).collect::<Vec<_>>(),
    );
    Ok(Star {
        text,
        words,
        sentence_words,
        paragraph_words,
    })
}

/// For each char offset, the index of the first word starting at or after it.
fn first_words_at(offsets: &[usize], word_starts: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = offsets
        .iter()
        .map(|&o| word_starts.partition_point(|&w| w < o))
        .filter(|&i| i < word_starts.len())
        .collect();
    out.dedup();
    out
}

struct Tw {
    doc: Document,
    words: Vec<CharRange>,
    texts: Vec<String>,
    sentence_words: Vec<usize>,
    paragraph_words: Vec<usize>,
}

fn tw_data(path: &Path) -> Result<Tw> {
    let doc = Registry::with_builtins()
        .load(&Source::Path(path.to_owned()), &LoadOptions::default())
        .with_context(|| format!("loading {}", path.display()))?;
    let words = segments(&doc, Unit::Word);
    let texts = words.iter().map(|r| doc.slice(*r)).collect();
    let starts: Vec<usize> = words.iter().map(|r| r.start.0).collect();
    let unit_words = |unit: Unit| {
        let offs: Vec<usize> = segments(&doc, unit).iter().map(|r| r.start.0).collect();
        first_words_at(&offs, &starts)
    };
    let sentence_words = unit_words(Unit::Sentence);
    let paragraph_words = unit_words(Unit::Paragraph);
    Ok(Tw {
        doc,
        words,
        texts,
        sentence_words,
        paragraph_words,
    })
}

/// A run of unaligned words between two aligned ones.
struct Hunk {
    star: Vec<usize>,
    tw: Vec<usize>,
}

fn hunks(ops: &[Op]) -> Vec<Hunk> {
    let mut out = Vec::new();
    let mut cur = Hunk {
        star: Vec::new(),
        tw: Vec::new(),
    };
    for op in ops {
        match *op {
            Op::Equal(..) => {
                if !cur.star.is_empty() || !cur.tw.is_empty() {
                    out.push(std::mem::replace(
                        &mut cur,
                        Hunk {
                            star: Vec::new(),
                            tw: Vec::new(),
                        },
                    ));
                }
            }
            Op::Star(i) => cur.star.push(i),
            Op::Tw(j) => cur.tw.push(j),
        }
    }
    if !cur.star.is_empty() || !cur.tw.is_empty() {
        out.push(cur);
    }
    out
}

struct Ctx<'a> {
    star: &'a Star,
    tw: &'a Tw,
    star_to_tw: Vec<Option<usize>>,
    tw_to_star: Vec<Option<usize>>,
}

impl Ctx<'_> {
    /// star's text of the sentence (star sentence map) containing word `i`.
    fn star_sentence_of(&self, i: usize) -> String {
        let s = &self.star.sentence_words;
        let k = s.partition_point(|&w| w <= i).saturating_sub(1);
        let a = s.get(k).copied().unwrap_or(0);
        let b = s.get(k + 1).copied().unwrap_or(self.star.words.len());
        let start = self.star.words.get(a).map_or(0, |w| w.0);
        let end = if b < self.star.words.len() {
            self.star.words[b].0
        } else {
            self.star.text.len()
        };
        self.star.text[start..end].iter().collect()
    }

    fn tw_in(&self, j: usize, kind: MarkerKind, level: Option<u8>) -> bool {
        let pos = self.tw.words[j].start;
        self.tw
            .doc
            .marker_index()
            .containing(pos)
            .any(|m| m.kind == kind && level.is_none_or(|l| m.level == l))
    }

    fn tw_starts_marker(&self, j: usize, kind: MarkerKind) -> bool {
        let pos = self.tw.words[j].start;
        self.tw
            .doc
            .marker_index()
            .starting_at(pos)
            .iter()
            .any(|m| m.kind == kind)
    }

    fn star_words(&self, idx: &[usize]) -> Vec<&str> {
        idx.iter().map(|&i| self.star.words[i].2.as_str()).collect()
    }

    fn tw_words(&self, idx: &[usize]) -> Vec<&str> {
        idx.iter().map(|&j| self.tw.texts[j].as_str()).collect()
    }

    fn explain_words(&self, h: &Hunk) -> &'static str {
        let s = self.star_words(&h.star);
        let t = self.tw_words(&h.tw);
        let alnum = |w: &[&str]| -> String {
            w.iter()
                .flat_map(|x| x.chars())
                .filter(|c| c.is_alphanumeric())
                .collect()
        };
        if !s.is_empty() && !t.is_empty() && alnum(&s) == alnum(&t) && s.len() > t.len() {
            return "uax29-join";
        }
        // Words only in star.
        if t.is_empty() {
            let first = h.star[0];
            let last = h.star[h.star.len() - 1];
            let sentence = self.star_sentence_of(first);
            if first < self.star.paragraph_words.get(1).copied().unwrap_or(0)
                && self
                    .star
                    .text
                    .iter()
                    .collect::<String>()
                    .starts_with("title:")
            {
                return "front-matter";
            }
            // The narration, or its first word matched against a heading
            // word just before it ("A Table" / "Table with ...").
            let next = self.star_sentence_of(last + 1);
            if is_narration(&sentence) || (s == ["Table"] && next.starts_with("Table with")) {
                return "table-narration";
            }
            if s == ["Scores"] {
                return "caption";
            }
        }
        // Words only in textweaver.
        if s.is_empty() {
            if h.tw
                .iter()
                .all(|&j| self.tw_in(j, MarkerKind::Code, Some(1)))
            {
                return "code-kept";
            }
            if t == ["Footnotes"] && self.tw_in(h.tw[0], MarkerKind::Heading, None) {
                return "footnote-heading";
            }
            if h.tw.iter().all(|&j| self.tw_in(j, MarkerKind::Table, None)) {
                let sentence =
                    h.tw.first()
                        .and_then(|&j| self.tw_to_star_neighbour(j))
                        .map(|i| self.star_sentence_of(i))
                        .unwrap_or_default();
                if is_narration(&sentence) {
                    return "table-narration";
                }
                if t == ["Scores"] {
                    return "caption";
                }
            }
            if t == ["Scores"] && self.tw_in_caption(h.tw[0]) {
                return "caption";
            }
        }
        // Both sides differ inside a table.
        if !h.tw.is_empty() && h.tw.iter().all(|&j| self.tw_in(j, MarkerKind::Table, None)) {
            return self.table_rule();
        }
        UNEXPLAINED
    }

    /// How star's canonical text holds tables in this fixture.
    fn table_rule(&self) -> &'static str {
        let text: String = self.star.text.iter().collect();
        if text.contains("Table with ") {
            "table-narration"
        } else {
            "table-layout"
        }
    }

    /// The nearest aligned star word before textweaver word `j`.
    fn tw_to_star_neighbour(&self, j: usize) -> Option<usize> {
        (0..=j).rev().find_map(|k| self.tw_to_star[k])
    }

    fn tw_in_caption(&self, j: usize) -> bool {
        let text = &self.tw.texts[j];
        self.tw
            .doc
            .marker_index()
            .iter(MarkerKind::Table, None)
            .any(|m| m.label.as_deref() == Some(text.as_str()))
    }

    /// Why textweaver starts a sentence at word `j` and star does not.
    fn explain_tw_sentence(&self, j: usize) -> &'static str {
        if self.tw_to_star[j].is_none() {
            return "no-counterpart";
        }
        if self.tw_starts_marker(j, MarkerKind::ListItem) {
            return "list-item";
        }
        if self.tw_starts_marker(j, MarkerKind::TableRow) {
            return "table-row";
        }
        if self.tw_starts_marker(j, MarkerKind::Heading) {
            return "heading";
        }
        if self.tw_in(j, MarkerKind::Code, Some(1)) {
            return "code-kept";
        }
        if self.tw_in(j, MarkerKind::Table, None) {
            return self.table_rule();
        }
        // The previous sentence ends in a footnote reference.
        if j > 0 {
            let prev = self.tw.words[j - 1];
            if self
                .tw
                .doc
                .marker_index()
                .containing(prev.start)
                .any(|m| m.kind == MarkerKind::Footnote && m.level == 0)
            {
                return "footnote-reference";
            }
        }
        UNEXPLAINED
    }

    /// Why star starts a sentence at its word `i` and textweaver does not.
    fn explain_star_sentence(&self, i: usize) -> &'static str {
        if self.star_to_tw[i].is_none() {
            return "no-counterpart";
        }
        if i > 0 {
            let (_, end, prev) = &self.star.words[i - 1];
            let dotted = format!("{prev}.");
            let followed_by_dot = self.star.text.get(*end) == Some(&'.');
            if followed_by_dot
                && (ABBREVIATIONS.contains(&dotted.as_str())
                    || AMBIGUOUS_ABBREVIATIONS
                        .iter()
                        .any(|a| a.ends_with(&dotted) || *a == dotted))
            {
                return "abbreviation";
            }
        }
        if is_narration(&self.star_sentence_of(i)) {
            return "table-narration";
        }
        UNEXPLAINED
    }

    fn explain_tw_paragraph(&self, j: usize) -> &'static str {
        if self.tw_to_star[j].is_none() {
            return "no-counterpart";
        }
        if self.tw_starts_marker(j, MarkerKind::List) {
            return "list-lines";
        }
        if self.tw_in(j, MarkerKind::Table, None) || self.tw_in_caption(j) {
            return self.table_rule();
        }
        if self.tw_in(j, MarkerKind::Code, Some(1)) {
            return "code-kept";
        }
        UNEXPLAINED
    }

    fn explain_star_paragraph(&self, i: usize) -> &'static str {
        if self.star_to_tw[i].is_none() {
            return "no-counterpart";
        }
        if is_narration(&self.star_sentence_of(i)) {
            return "table-narration";
        }
        if let Some(j) = self.star_to_tw[i]
            && (self.tw_in(j, MarkerKind::Table, None) || self.tw_in_caption(j))
        {
            return self.table_rule();
        }
        UNEXPLAINED
    }

    /// A few words of textweaver text starting at word `j`.
    fn tw_context(&self, j: usize) -> String {
        let end = (j + 6).min(self.tw.texts.len());
        let a = self.tw.words[j].start;
        let b = self.tw.words[end - 1].end;
        one_line(&self.tw.doc.slice(CharRange::new(a, b)))
    }

    /// A few words of star text starting at word `i`.
    fn star_context(&self, i: usize) -> String {
        let end = (i + 6).min(self.star.words.len());
        let a = self.star.words[i].0;
        let b = self.star.words[end - 1].1;
        one_line(&self.star.text[a..b].iter().collect::<String>())
    }

    /// star's words just before word `i`, for sentence-boundary context.
    fn star_before(&self, i: usize) -> String {
        let a = self.star.words[i.saturating_sub(3)].0;
        let b = self.star.words[i].0;
        one_line(&self.star.text[a..b].iter().collect::<String>())
    }
}

/// True for a sentence of star's structured table narration.
fn is_narration(sentence: &str) -> bool {
    sentence.starts_with("Table with") || sentence.starts_with("Row ")
}

fn one_line(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

fn code(s: &str) -> String {
    if s.contains('`') {
        format!("`` {s} ``")
    } else {
        format!("`{s}`")
    }
}

#[derive(Default)]
struct Counts {
    by_rule: BTreeMap<&'static str, usize>,
    unexplained: usize,
}

impl Counts {
    fn add(&mut self, rule: &'static str) {
        *self.by_rule.entry(rule).or_default() += 1;
        if rule == UNEXPLAINED {
            self.unexplained += 1;
        }
    }
}

fn fixture_section(root: &Path, name: &str) -> Result<(String, String, Counts)> {
    let json_path = root
        .join("fixtures/star-parity")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&json_path)
        .with_context(|| format!("reading {}", json_path.display()))?;
    let v: Value = serde_json::from_str(&raw)?;
    let star = star_data(&v)?;
    let tw = tw_data(&root.join("fixtures").join(name))?;
    let star_texts: Vec<String> = star.words.iter().map(|w| w.2.clone()).collect();
    let ops = align(&star_texts, &tw.texts);
    let mut star_to_tw = vec![None; star.words.len()];
    let mut tw_to_star = vec![None; tw.texts.len()];
    for op in &ops {
        if let Op::Equal(i, j) = *op {
            star_to_tw[i] = Some(j);
            tw_to_star[j] = Some(i);
        }
    }
    let ctx = Ctx {
        star: &star,
        tw: &tw,
        star_to_tw,
        tw_to_star,
    };
    let mut counts = Counts::default();
    let mut s = String::new();
    let _ = writeln!(s, "## `fixtures/{name}`\n");

    // Words.
    let word_hunks = hunks(&ops);
    let aligned = ops.iter().filter(|o| matches!(o, Op::Equal(..))).count();
    let _ = writeln!(
        s,
        "### Words\n\nStar {} words, textweaver {}, {} aligned, {} deltas.\n",
        star.words.len(),
        tw.texts.len(),
        aligned,
        word_hunks.len()
    );
    if !word_hunks.is_empty() {
        s.push_str("| star | textweaver | Rule |\n|---|---|---|\n");
        for h in &word_hunks {
            let rule = ctx.explain_words(h);
            counts.add(rule);
            let show = |w: Vec<&str>| {
                if w.is_empty() {
                    "—".to_owned()
                } else {
                    w.iter().map(|x| code(x)).collect::<Vec<_>>().join(" ")
                }
            };
            let _ = writeln!(
                s,
                "| {} | {} | `{rule}` |",
                show(ctx.star_words(&h.star)),
                show(ctx.tw_words(&h.tw))
            );
        }
        s.push('\n');
    }

    // Sentences and paragraphs.
    let (sent_rows, sent_stats) = compare_starts(
        &ctx,
        &star.sentence_words,
        &tw.sentence_words,
        &mut counts,
        |c, j| c.explain_tw_sentence(j),
        |c, i| c.explain_star_sentence(i),
    );
    let _ = writeln!(
        s,
        "### Sentence starts\n\nStar {}, textweaver {}, {} shared, {} deltas.\n",
        sent_stats.0,
        sent_stats.1,
        sent_stats.2,
        sent_rows.len()
    );
    if !sent_rows.is_empty() {
        s.push_str("| Only in | Where | Rule |\n|---|---|---|\n");
        s.push_str(&sent_rows.concat());
        s.push('\n');
    }
    let (para_rows, para_stats) = compare_starts(
        &ctx,
        &star.paragraph_words,
        &tw.paragraph_words,
        &mut counts,
        |c, j| c.explain_tw_paragraph(j),
        |c, i| c.explain_star_paragraph(i),
    );
    let _ = writeln!(
        s,
        "### Paragraph starts\n\nStar {}, textweaver {}, {} shared, {} deltas.\n",
        para_stats.0,
        para_stats.1,
        para_stats.2,
        para_rows.len()
    );
    if !para_rows.is_empty() {
        s.push_str("| Only in | Where | Rule |\n|---|---|---|\n");
        s.push_str(&para_rows.concat());
        s.push('\n');
    }
    let rules: Vec<String> = counts
        .by_rule
        .iter()
        .map(|(r, n)| format!("`{r}` {n}"))
        .collect();
    let _ = writeln!(s, "Deltas by rule: {}.\n", rules.join(", "));

    let row = format!(
        "| `{name}` | {} | {} | {} | {} | {} / {} / {} | {} | {} / {} / {} | {} | {} |\n",
        star.words.len(),
        tw.texts.len(),
        aligned,
        word_hunks.len(),
        sent_stats.0,
        sent_stats.1,
        sent_stats.2,
        sent_rows.len(),
        para_stats.0,
        para_stats.1,
        para_stats.2,
        para_rows.len(),
        counts.unexplained
    );
    Ok((s, row, counts))
}

/// Compares start word positions; returns table rows and (star, tw, shared).
fn compare_starts(
    ctx: &Ctx<'_>,
    star_starts: &[usize],
    tw_starts: &[usize],
    counts: &mut Counts,
    explain_tw: impl Fn(&Ctx<'_>, usize) -> &'static str,
    explain_star: impl Fn(&Ctx<'_>, usize) -> &'static str,
) -> (Vec<String>, (usize, usize, usize)) {
    let tw_set: BTreeSet<usize> = tw_starts.iter().copied().collect();
    let star_mapped: BTreeSet<usize> = star_starts
        .iter()
        .filter_map(|&i| ctx.star_to_tw.get(i).copied().flatten())
        .collect();
    let shared = tw_set.intersection(&star_mapped).count();
    // (tw word position for ordering, row)
    let mut rows: Vec<(usize, String)> = Vec::new();
    for &i in star_starts {
        let mapped = ctx.star_to_tw.get(i).copied().flatten();
        if mapped.is_some_and(|j| tw_set.contains(&j)) {
            continue;
        }
        let rule = explain_star(ctx, i);
        counts.add(rule);
        let order = mapped.unwrap_or_else(|| {
            (0..i)
                .rev()
                .find_map(|k| ctx.star_to_tw[k])
                .map_or(0, |j| j + 1)
        });
        rows.push((
            order,
            format!(
                "| star | {} ‖ {} | `{rule}` |\n",
                code(&ctx.star_before(i)),
                code(&ctx.star_context(i))
            ),
        ));
    }
    for &j in tw_starts {
        if star_mapped.contains(&j) {
            continue;
        }
        let rule = explain_tw(ctx, j);
        counts.add(rule);
        let before = if j == 0 {
            String::new()
        } else {
            let a = ctx.tw.words[j.saturating_sub(3)].start;
            let b = ctx.tw.words[j].start;
            one_line(&ctx.tw.doc.slice(CharRange::new(a, b)))
        };
        rows.push((
            j,
            format!(
                "| textweaver | {} ‖ {} | `{rule}` |\n",
                code(&before),
                code(&ctx.tw_context(j))
            ),
        ));
    }
    rows.sort_by_key(|r| r.0);
    (
        rows.into_iter().map(|r| r.1).collect(),
        (star_starts.len(), tw_starts.len(), shared),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn alignment_finds_the_common_subsequence() {
        let ops = align(&v(&["a", "e", "g", "b"]), &v(&["a", "e.g", "b"]));
        assert_eq!(
            ops,
            [
                Op::Equal(0, 0),
                Op::Star(1),
                Op::Star(2),
                Op::Tw(1),
                Op::Equal(3, 2)
            ]
        );
        let h = hunks(&ops);
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].star.clone(), h[0].tw.clone()), (vec![1, 2], vec![1]));
    }

    #[test]
    fn first_words_at_offsets() {
        assert_eq!(first_words_at(&[0, 5, 6, 99], &[0, 4, 8]), [0, 2]);
    }

    #[test]
    fn report_path_options() {
        let root = Path::new("workspace");
        let args = |v: &[&str]| {
            v.iter()
                .map(|s| (*s).to_owned())
                .collect::<Vec<_>>()
                .into_iter()
        };
        assert_eq!(
            report_path(root, args(&["--out", "out/report.md"])).unwrap(),
            PathBuf::from("out/report.md")
        );
        assert_eq!(
            report_path(root, args(&["--out=r.md"])).unwrap(),
            PathBuf::from("r.md")
        );
        assert!(report_path(root, args(&["--out"])).is_err());
        assert!(report_path(root, args(&["--check"])).is_err());
        let default = report_path(root, args(&[])).unwrap();
        assert!(
            default.ends_with("parity-report.md"),
            "{}",
            default.display()
        );
        assert!(
            !default.starts_with("workspace/docs"),
            "{}",
            default.display()
        );
    }
}
