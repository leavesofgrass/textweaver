//! CW practice: Koch-method lessons, random drills, and checking what the
//! learner copied.
//!
//! - **The Koch method** starts with two characters sent at full character
//!   speed and adds one when the learner copies 90 percent or better (the
//!   threshold is the caller's; textweaver's setting defaults to 90). The
//!   character order ([`KOCH_ORDER`]) is the one Learn CW Online (LCWO)
//!   teaches, taken from its source code: the `$kochchar` array in
//!   `inc/functions.php` of <https://github.com/dj1yfk/lcwo>, checked on
//!   Friday, October 23, 2026. Lesson 1 is K and M; each later lesson adds
//!   the next character, to lesson 40 with all 41.
//! - **Drills** ([`koch_group`], [`character_group`], [`word_group`]) draw
//!   from a small seeded generator ([`Rng`]), so a drill is the same on
//!   every platform for the same seed, and tests can name it.
//! - **Checking** ([`check`]) aligns what was typed with what was sent by
//!   edit distance, so one missed character does not count every later one
//!   wrong, and lists each miss: a character copied as another, one not
//!   copied, and one typed that was not sent. Case and spacing are not
//!   scored.

/// The Koch-method character order of Learn CW Online (LCWO), from the
/// `$kochchar` array in its `inc/functions.php`
/// (<https://github.com/dj1yfk/lcwo>). `=` is the double dash (BT).
pub const KOCH_ORDER: [char; 41] = [
    'K', 'M', 'U', 'R', 'E', 'S', 'N', 'A', 'P', 'T', 'L', 'W', 'I', '.', 'J', 'Z', '=', 'F', 'O',
    'Y', ',', 'V', 'G', '5', '/', 'Q', '9', '2', 'H', '3', '8', 'B', '?', '4', '7', 'C', '1', 'D',
    '6', '0', 'X',
];

/// The last Koch lesson: every character of [`KOCH_ORDER`].
pub const KOCH_LESSONS: u8 = 40;

/// The characters of Koch lesson `lesson` (1 is K and M; each later lesson
/// adds one), clamped to 1 to [`KOCH_LESSONS`].
pub fn koch_characters(lesson: u8) -> &'static [char] {
    let lesson = lesson.clamp(1, KOCH_LESSONS);
    &KOCH_ORDER[..usize::from(lesson) + 1]
}

/// The character lesson `lesson` adds (K and M together for lesson 1 are
/// given as M, the second).
pub fn koch_new_character(lesson: u8) -> char {
    let chars = koch_characters(lesson);
    chars[chars.len() - 1]
}

/// True when `correct` of `total` reaches `threshold_percent`: the Koch
/// method's signal to add the next character.
pub fn passes(correct: u32, total: u32, threshold_percent: u8) -> bool {
    total > 0 && u64::from(correct) * 100 >= u64::from(total) * u64::from(threshold_percent)
}

/// `correct` of `total` in whole percent, rounded down (so 89.9 is not
/// called 90).
pub fn percent(correct: u32, total: u32) -> u32 {
    if total == 0 {
        return 0;
    }
    let p = u64::from(correct) * 100 / u64::from(total);
    u32::try_from(p).unwrap_or(100)
}

/// A small seeded random generator (SplitMix64): the same numbers for the
/// same seed on every platform. Not for anything secret.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator starting from `seed`.
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 to `n - 1` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        // usize is at most 64 bits; the remainder is below n.
        (self.next_u64() % n as u64) as usize
    }
}

/// A Koch group: `size` characters of lesson `lesson`. The newest
/// character comes up about one time in three (once there are three or
/// more), as trainers weight it so it is learned; LCWO weights it too, with
/// its own weights. Two characters in a row are allowed, as on the air.
pub fn koch_group(lesson: u8, size: usize, rng: &mut Rng) -> String {
    let chars = koch_characters(lesson);
    let newest = chars[chars.len() - 1];
    (0..size.max(1))
        .map(|_| {
            if chars.len() > 2 && rng.below(3) == 0 {
                newest
            } else {
                chars[rng.below(chars.len())]
            }
        })
        .collect()
}

/// A group of `size` characters drawn evenly from every character the
/// Koch course teaches (the letters, the figures, and `. , = / ?`).
pub fn character_group(size: usize, rng: &mut Rng) -> String {
    (0..size.max(1))
        .map(|_| KOCH_ORDER[rng.below(KOCH_ORDER.len())])
        .collect()
}

/// Common English words and the abbreviations heard in every contact,
/// for word drills.
pub const WORDS: &[&str] = &[
    // Common English words.
    "THE", "OF", "AND", "TO", "IN", "IS", "IT", "YOU", "THAT", "HE", "WAS", "FOR", "ON", "ARE",
    "AS", "WITH", "HIS", "THEY", "AT", "BE", "THIS", "HAVE", "FROM", "OR", "ONE", "HAD", "BY",
    "WORD", "BUT", "NOT", "WHAT", "ALL", "WERE", "WE", "WHEN", "YOUR", "CAN", "SAID", "THERE",
    "USE", "AN", "EACH", "WHICH", "SHE", "DO", "HOW", "THEIR", "IF", "WILL", "UP", "OTHER",
    "ABOUT", "OUT", "MANY", "THEN", "THEM", "THESE", "SO", "SOME", "HER", "WOULD", "MAKE", "LIKE",
    "HIM", "INTO", "TIME", "HAS", "LOOK", "TWO", "MORE", "WRITE", "GO", "SEE", "NUMBER", "NO",
    "WAY", "COULD", "PEOPLE", "MY", "THAN", "FIRST", "WATER", "BEEN", "CALL", "WHO", "NOW", "FIND",
    "LONG", "DOWN", "DAY", "DID", "GET", "COME", "MADE", "MAY", "PART", "OVER", "NEW", "SOUND",
    "TAKE", "ONLY", "LITTLE", "WORK", "KNOW", "PLACE", "YEAR", "LIVE", "BACK", "GIVE", "MOST",
    "VERY", "AFTER", "THING", "JUST", "NAME", "GOOD", "WEATHER", "RADIO",
    // Abbreviations and Q codes of a contact.
    "CQ", "DE", "K", "KN", "R", "RST", "UR", "ES", "HR", "FB", "OM", "TNX", "TU", "73", "QTH",
    "QRZ", "QSL", "QRS", "QRM", "QSB", "WX", "ANT", "RIG", "PWR", "AGN", "PSE", "GM", "GA", "GE",
    "BK", "SK", "HW", "CPY", "NR",
];

/// Random words from [`WORDS`], separated by spaces, until they hold at
/// least `letters` characters (always at least one word).
pub fn word_group(letters: usize, rng: &mut Rng) -> String {
    let mut out = String::new();
    let mut count = 0;
    loop {
        let w = WORDS[rng.below(WORDS.len())];
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
        count += w.chars().count();
        if count >= letters.max(1) {
            return out;
        }
    }
}

/// One difference between what was sent and what was typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Miss {
    /// `sent` was copied as `typed`.
    Wrong {
        /// The character sent.
        sent: char,
        /// The character typed for it.
        typed: char,
    },
    /// `0` was sent and not copied.
    Missing(char),
    /// `0` was typed and not sent.
    Extra(char),
}

/// How a copy went: [`check`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// The characters sent (spaces not counted).
    pub total: u32,
    /// The characters copied right: `total` less every miss, extras
    /// included, and never below 0.
    pub correct: u32,
    /// The misses, in the order sent.
    pub misses: Vec<Miss>,
}

impl Check {
    /// True when everything was copied, with nothing extra.
    pub fn perfect(&self) -> bool {
        self.misses.is_empty()
    }
}

/// The characters of `s` that are scored: no white space, upper case.
fn scored(s: &str) -> Vec<char> {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Checks `typed` against `sent`: the alignment with the fewest edits
/// (Levenshtein), preferring a character copied as another over one
/// missed and one extra, and every miss in order.
pub fn check(sent: &str, typed: &str) -> Check {
    let a = scored(sent);
    let b = scored(typed);
    let (n, m) = (a.len(), b.len());
    // d[i][j]: edits to turn a[..i] into b[..j].
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let sub = d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = sub.min(d[i - 1][j] + 1).min(d[i][j - 1] + 1);
        }
    }
    let mut misses = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]) {
            if a[i - 1] != b[j - 1] {
                misses.push(Miss::Wrong {
                    sent: a[i - 1],
                    typed: b[j - 1],
                });
            }
            i -= 1;
            j -= 1;
        } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            misses.push(Miss::Missing(a[i - 1]));
            i -= 1;
        } else {
            misses.push(Miss::Extra(b[j - 1]));
            j -= 1;
        }
    }
    misses.reverse();
    let total = u32::try_from(n).unwrap_or(u32::MAX);
    let errors = u32::try_from(d[n][m]).unwrap_or(u32::MAX);
    Check {
        total,
        correct: total.saturating_sub(errors),
        misses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code;

    #[test]
    fn the_koch_order_is_lcwos_and_every_character_has_a_code() {
        let s: String = KOCH_ORDER.iter().collect();
        assert_eq!(s, "KMURESNAPTLWI.JZ=FOY,VG5/Q92H38B?47C1D60X");
        for c in KOCH_ORDER {
            assert!(code(c).is_some(), "{c} has no code");
        }
        // Every letter and figure once.
        let mut sorted = KOCH_ORDER.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 41);
    }

    #[test]
    fn lessons_start_with_two_characters_and_add_one_each() {
        assert_eq!(koch_characters(1), ['K', 'M']);
        assert_eq!(koch_characters(2), ['K', 'M', 'U']);
        assert_eq!(koch_new_character(2), 'U');
        assert_eq!(koch_characters(40).len(), 41);
        assert_eq!(koch_characters(0), ['K', 'M']);
        assert_eq!(koch_characters(99).len(), 41);
    }

    #[test]
    fn ninety_percent_passes_and_less_does_not() {
        assert!(passes(45, 50, 90));
        assert!(passes(9, 10, 90));
        assert!(!passes(44, 50, 90));
        assert!(!passes(0, 0, 90));
        assert!(passes(50, 50, 100));
        assert_eq!(percent(44, 50), 88);
        assert_eq!(percent(899, 1000), 89);
        assert_eq!(percent(1, 0), 0);
    }

    #[test]
    fn drills_are_the_same_for_the_same_seed() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(koch_group(5, 5, &mut a), koch_group(5, 5, &mut b));
        assert_eq!(character_group(5, &mut a), character_group(5, &mut b));
        assert_eq!(word_group(5, &mut a), word_group(5, &mut b));
        // Pinned, so a change to the generator is seen.
        let pinned = (
            koch_group(1, 5, &mut Rng::new(1)),
            character_group(5, &mut Rng::new(1)),
            word_group(5, &mut Rng::new(1)),
        );
        assert_eq!(pinned, ("MMKMM".into(), "2I=UC".into(), "HE KNOW".into()));
    }

    #[test]
    fn koch_groups_use_only_the_lessons_characters() {
        let mut r = Rng::new(42);
        for lesson in [1, 2, 10, 40] {
            let chars = koch_characters(lesson);
            for _ in 0..50 {
                let g = koch_group(lesson, 5, &mut r);
                assert_eq!(g.chars().count(), 5);
                assert!(g.chars().all(|c| chars.contains(&c)), "{g} in {lesson}");
            }
        }
        // The newest character comes up often.
        let all: String = (0..200).map(|_| koch_group(10, 5, &mut r)).collect();
        let newest = all.chars().filter(|&c| c == koch_new_character(10)).count();
        assert!(newest > 200, "newest {newest} of 1000");
    }

    #[test]
    fn a_word_group_has_enough_letters() {
        let mut r = Rng::new(3);
        for _ in 0..50 {
            let g = word_group(5, &mut r);
            assert!(g.chars().filter(|c| *c != ' ').count() >= 5, "{g}");
            assert!(g.split(' ').all(|w| WORDS.contains(&w)));
        }
    }

    #[test]
    fn a_copy_is_scored_by_alignment() {
        let c = check("KMRKM", "kmrkm");
        assert_eq!((c.correct, c.total), (5, 5));
        assert!(c.perfect());
        // One wrong: the rest still right.
        let c = check("KMRKMKMRKM", "RMRKMKMRKM");
        assert_eq!((c.correct, c.total), (9, 10));
        assert_eq!(
            c.misses,
            [Miss::Wrong {
                sent: 'K',
                typed: 'R'
            }]
        );
        // One missed: the later ones are not shifted into errors.
        let c = check("KMRKM", "KRKM");
        assert_eq!((c.correct, c.total), (4, 5));
        assert_eq!(c.misses, [Miss::Missing('M')]);
        // One extra.
        let c = check("KMR", "KMER");
        assert_eq!((c.correct, c.total), (2, 3));
        assert_eq!(c.misses, [Miss::Extra('E')]);
        // Spacing is not scored; nothing typed is all missed.
        assert!(check("CQ DE", "CQDE").perfect());
        let c = check("KM", "");
        assert_eq!(c.correct, 0);
        assert_eq!(c.misses, [Miss::Missing('K'), Miss::Missing('M')]);
        // Many extras never go below zero.
        assert_eq!(check("K", "MMMMM").correct, 0);
    }
}
