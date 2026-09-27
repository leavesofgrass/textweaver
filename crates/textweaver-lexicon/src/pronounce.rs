//! Respelling CMUdict's ARPAbet for reading aloud: `R AH1 N IH0 NG` becomes
//! `RUN-ing`, with the stressed syllable in capitals, in the spirit of
//! Wikipedia's pronunciation respelling key.
//!
//! The rules are deliberately simple, so a respelling is a guide, not a
//! phonetic transcription:
//!
//! - syllables split before the longest consonant cluster that can start
//!   an English syllable (`st`, `pr`, `str`, ...), except that a single
//!   consonant after a stressed short vowel closes that syllable (`RUN-ing`,
//!   not `RUH-ning`);
//! - long vowels before one consonant take a silent `e` (`TIME`, `HOME`,
//!   `MAKE`), and are spelled `igh`, `oh`, `ay` at the end of a syllable;
//! - primary stress is written in capitals; secondary stress is not marked.

/// The respelling of an ARPAbet pronunciation, or an empty string when the
/// text is not ARPAbet.
pub fn respell(arpabet: &str) -> String {
    let phones: Vec<(&str, Option<u8>)> = arpabet
        .split_whitespace()
        .map(|p| {
            let digit = p
                .chars()
                .last()
                .and_then(|c| c.to_digit(10))
                .map(|d| d as u8);
            let base = if digit.is_some() {
                &p[..p.len() - 1]
            } else {
                p
            };
            (base, digit)
        })
        .collect();
    if phones.is_empty() || phones.iter().any(|(b, _)| sound(b).is_none()) {
        return String::new();
    }
    let vowels: Vec<usize> = phones
        .iter()
        .enumerate()
        .filter(|(_, (b, _))| is_vowel(b))
        .map(|(i, _)| i)
        .collect();
    if vowels.is_empty() {
        return phones
            .iter()
            .filter_map(|(b, _)| sound(b))
            .collect::<String>();
    }
    // Syllable boundaries: the index where each syllable starts.
    let mut starts = vec![0usize];
    for pair in vowels.windows(2) {
        let (v1, v2) = (pair[0], pair[1]);
        let cluster: Vec<&str> = phones[v1 + 1..v2].iter().map(|(b, _)| *b).collect();
        let onset = if cluster.len() == 1 {
            let (vowel, stress) = phones[v1];
            if matches!(stress, Some(1 | 2)) && is_short(vowel) {
                0
            } else {
                1
            }
        } else {
            (0..=cluster.len())
                .rev()
                .find(|&k| is_onset(&cluster[cluster.len() - k..]))
                .unwrap_or(0)
        };
        starts.push(v2 - onset);
    }
    starts.push(phones.len());
    let mut out: Vec<String> = Vec::new();
    for w in starts.windows(2) {
        let syl = &phones[w[0]..w[1]];
        out.push(spell_syllable(syl));
    }
    out.join("-")
}

fn spell_syllable(syl: &[(&str, Option<u8>)]) -> String {
    let Some(v) = syl.iter().position(|(b, _)| is_vowel(b)) else {
        return syl.iter().filter_map(|(b, _)| sound(b)).collect();
    };
    let (vowel, stress) = syl[v];
    let onset: String = syl[..v].iter().filter_map(|(b, _)| sound(b)).collect();
    let coda_phones = &syl[v + 1..];
    let coda: String = coda_phones.iter().filter_map(|(b, _)| sound(b)).collect();
    let single = coda_phones.len() == 1;
    let (nucleus, silent_e) = match (vowel, coda.is_empty()) {
        ("AY", true) if onset.is_empty() => ("eye", false),
        ("AY", true) => ("igh", false),
        ("AY", false) => ("i", single),
        ("EY", true) => ("ay", false),
        ("EY", false) => ("a", single),
        ("OW", true) => ("oh", false),
        ("OW", false) => ("o", single),
        ("AH", true) => ("uh", false),
        ("AH", false) => ("u", false),
        ("EH", true) => ("eh", false),
        ("IH", true) => ("ih", false),
        (other, _) => (sound(other).unwrap_or(""), false),
    };
    let mut s = format!("{onset}{nucleus}{coda}");
    if silent_e {
        s.push('e');
    }
    if stress == Some(1) {
        s.to_uppercase()
    } else {
        s
    }
}

fn is_vowel(p: &str) -> bool {
    matches!(
        p,
        "AA" | "AE"
            | "AH"
            | "AO"
            | "AW"
            | "AY"
            | "EH"
            | "ER"
            | "EY"
            | "IH"
            | "IY"
            | "OW"
            | "OY"
            | "UH"
            | "UW"
    )
}

fn is_short(p: &str) -> bool {
    matches!(p, "AE" | "EH" | "IH" | "AH" | "UH")
}

/// Consonant clusters that can start an English syllable.
fn is_onset(c: &[&str]) -> bool {
    const STOPS: [&str; 9] = ["P", "B", "T", "D", "K", "G", "F", "TH", "SH"];
    match c {
        [] => true,
        [one] => *one != "NG",
        ["S", second] => matches!(*second, "P" | "T" | "K" | "M" | "N" | "L" | "W" | "F"),
        [first, "R" | "L" | "W" | "Y"] => STOPS.contains(first),
        ["S", "P" | "T" | "K", "R" | "L" | "W" | "Y"] => true,
        _ => false,
    }
}

/// The spelling of one ARPAbet phone (without its stress digit).
fn sound(p: &str) -> Option<&'static str> {
    Some(match p {
        "AA" => "ah",
        "AE" => "a",
        "AH" => "u",
        "AO" => "aw",
        "AW" => "ow",
        "AY" => "eye",
        "EH" => "e",
        "ER" => "ur",
        "EY" => "ay",
        "IH" => "i",
        "IY" => "ee",
        "OW" => "oh",
        "OY" => "oy",
        "UH" => "oo",
        "UW" => "oo",
        "B" => "b",
        "CH" => "ch",
        "D" => "d",
        "DH" => "th",
        "F" => "f",
        "G" => "g",
        "HH" => "h",
        "JH" => "j",
        "K" => "k",
        "L" => "l",
        "M" => "m",
        "N" => "n",
        "NG" => "ng",
        "P" => "p",
        "R" => "r",
        "S" => "s",
        "SH" => "sh",
        "T" => "t",
        "TH" => "th",
        "V" => "v",
        "W" => "w",
        "Y" => "y",
        "Z" => "z",
        "ZH" => "zh",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::respell;

    #[test]
    fn respellings() {
        for (arpabet, expect) in [
            ("R AH1 N IH0 NG", "RUN-ing"),
            ("D AO1 G", "DAWG"),
            ("AH0 B AW1 T", "uh-BOWT"),
            ("B AH0 N AE1 N AH0", "buh-NAN-uh"),
            ("T AY1 M", "TIME"),
            ("HH AY1", "HIGH"),
            ("AY1", "EYE"),
            ("HH OW1 M", "HOME"),
            ("M EY1 K", "MAKE"),
            ("D EY1", "DAY"),
            ("D IH1 K SH AH0 N EH2 R IY0", "DIK-shuh-ner-ee"),
            ("M IH0 S T R EH1 S", "mih-STRES"),
            ("W ER1 D", "WURD"),
        ] {
            assert_eq!(respell(arpabet), expect, "{arpabet}");
        }
    }

    #[test]
    fn not_arpabet() {
        assert_eq!(respell(""), "");
        assert_eq!(respell("kuh-NIGH-nee"), "");
        assert_eq!(respell("HH M"), "hm");
    }
}
