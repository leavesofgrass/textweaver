//! Plural categories (CLDR cardinal rules) for whole numbers, for the
//! languages star had catalogs for and the common right-to-left ones.

/// A CLDR plural category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluralCategory {
    /// `zero`
    Zero,
    /// `one`
    One,
    /// `two`
    Two,
    /// `few`
    Few,
    /// `many`
    Many,
    /// `other`
    Other,
}

impl PluralCategory {
    /// The name Fluent uses for the variant key.
    pub fn name(self) -> &'static str {
        match self {
            PluralCategory::Zero => "zero",
            PluralCategory::One => "one",
            PluralCategory::Two => "two",
            PluralCategory::Few => "few",
            PluralCategory::Many => "many",
            PluralCategory::Other => "other",
        }
    }
}

/// The plural category of the whole number `n` in `lang` (a language tag
/// such as `en`, `pt-BR`, `ar`). Languages without rules here use
/// English's.
pub fn category(lang: &str, n: i64) -> PluralCategory {
    use PluralCategory as P;
    let primary = lang
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let n = n.unsigned_abs();
    match primary.as_str() {
        // one: i = 0,1
        "fr" | "fa" | "hi" => {
            if n <= 1 {
                P::One
            } else {
                P::Other
            }
        }
        // Brazilian and CLDR default Portuguese: i = 0..1 is one.
        "pt" if !lang.eq_ignore_ascii_case("pt-PT") => {
            if n <= 1 {
                P::One
            } else {
                P::Other
            }
        }
        "ar" => match n {
            0 => P::Zero,
            1 => P::One,
            2 => P::Two,
            _ => match n % 100 {
                3..=10 => P::Few,
                11..=99 => P::Many,
                _ => P::Other,
            },
        },
        "he" => match n {
            1 => P::One,
            2 => P::Two,
            _ => P::Other,
        },
        "ja" | "zh" | "ko" | "th" | "vi" | "id" => P::Other,
        _ => {
            if n == 1 {
                P::One
            } else {
                P::Other
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PluralCategory as P;
    use super::category;

    #[test]
    fn rules() {
        assert_eq!(category("en", 1), P::One);
        assert_eq!(category("en", 0), P::Other);
        assert_eq!(category("en-GB", 2), P::Other);
        assert_eq!(category("fr", 0), P::One);
        assert_eq!(category("fr", 2), P::Other);
        assert_eq!(category("pt", 0), P::One);
        assert_eq!(category("pt-PT", 0), P::Other);
        assert_eq!(category("de", 1), P::One);
        assert_eq!(category("es", 5), P::Other);
        let ar: Vec<P> = [0, 1, 2, 3, 10, 11, 99, 100, 102, 111]
            .iter()
            .map(|&n| category("ar", n))
            .collect();
        assert_eq!(
            ar,
            [
                P::Zero,
                P::One,
                P::Two,
                P::Few,
                P::Few,
                P::Many,
                P::Many,
                P::Other,
                P::Other,
                P::Many
            ]
        );
        assert_eq!(category("he", 2), P::Two);
        assert_eq!(category("ja", 1), P::Other);
        assert_eq!(category("", -1), P::One);
    }
}
