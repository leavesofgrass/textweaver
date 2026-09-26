//! Conversions between the saved `[reading_aids]` settings
//! (`textweaver_store::reading_aids`, plain data) and this crate's working
//! types, which carry the checks, CSS, and layout.
//!
//! The store keeps its own copies so it depends on nothing but
//! `textweaver-core` (ADR-0001). Each type here converts from a reference
//! to its saved twin with `From`, and back. The fonts convert with
//! [`crate::fonts::from_store`] and [`crate::fonts::to_store`], because both
//! sides of that pair belong to other crates.

use textweaver_store::reading_aids as saved;

use crate::bionic::BionicOptions;
use crate::rsvp::{Pacing, RsvpPosition, RsvpSettings};
use crate::ruler::{RulerMode, RulerScope, RulerSettings};
use crate::spacing::TextSpacing;
use crate::syllables::SyllableOptions;

/// `From` both ways between a fieldless enum and its saved twin.
macro_rules! enum_twins {
    ($ty:ident { $($v:ident),* $(,)? }) => {
        impl From<saved::$ty> for $ty {
            fn from(v: saved::$ty) -> Self {
                match v {
                    $(saved::$ty::$v => $ty::$v,)*
                }
            }
        }

        impl From<$ty> for saved::$ty {
            fn from(v: $ty) -> Self {
                match v {
                    $($ty::$v => saved::$ty::$v,)*
                }
            }
        }
    };
}

/// `From` both ways between a struct and its saved twin, field by field
/// (each field converted with `From`, so enum fields work too).
macro_rules! struct_twins {
    ($ty:ident { $($f:ident),* $(,)? }) => {
        impl From<&saved::$ty> for $ty {
            #[allow(clippy::clone_on_copy, clippy::useless_conversion)]
            fn from(v: &saved::$ty) -> Self {
                $ty { $($f: v.$f.clone().into(),)* }
            }
        }

        impl From<&$ty> for saved::$ty {
            #[allow(clippy::clone_on_copy, clippy::useless_conversion)]
            fn from(v: &$ty) -> Self {
                saved::$ty { $($f: v.$f.clone().into(),)* }
            }
        }
    };
}

enum_twins!(Pacing { Timer, External });
enum_twins!(RsvpPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
});
enum_twins!(RulerMode {
    Off,
    CurrentLine,
    Ruler
});
enum_twins!(RulerScope { Row, Line });

struct_twins!(RsvpSettings {
    wpm,
    pacing,
    clause_pause,
    sentence_pause,
    paragraph_pause,
    long_word_len,
    long_word_step,
    long_word_max,
    show_previous,
    show_next,
    position,
    font_size_pt,
    lead_words,
});
struct_twins!(BionicOptions {
    ratio,
    min_word_len,
    skip_numbers,
    skip_urls,
    skip_code,
});
struct_twins!(TextSpacing {
    line_height,
    paragraph_spacing,
    letter_spacing,
    word_spacing,
});
struct_twins!(RulerSettings {
    mode,
    scope,
    rows_above,
    rows_below,
    mask_outside,
});
struct_twins!(SyllableOptions {
    separator,
    left_min,
    right_min,
    min_word_len,
    skip_urls,
    skip_code,
});

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::*;

    /// The working type and its saved twin agree on the defaults, convert
    /// both ways without loss, and serialize to the same TOML.
    fn twins<W, S>(working: W)
    where
        W: Default + PartialEq + std::fmt::Debug + Serialize + for<'a> From<&'a S>,
        S: Default + PartialEq + std::fmt::Debug + Serialize + for<'a> From<&'a W>,
    {
        assert_eq!(W::from(&S::default()), W::default());
        let saved = S::from(&working);
        assert_eq!(W::from(&saved), working);
        assert_eq!(
            toml::to_string(&working).unwrap(),
            toml::to_string(&saved).unwrap()
        );
    }

    #[test]
    fn every_setting_matches_its_saved_twin() {
        twins::<_, saved::RsvpSettings>(RsvpSettings {
            wpm: 450,
            pacing: Pacing::External,
            position: RsvpPosition::BottomRight,
            lead_words: -2,
            ..RsvpSettings::default()
        });
        twins::<_, saved::BionicOptions>(BionicOptions::with_ratio(0.6));
        twins::<_, saved::TextSpacing>(TextSpacing::generous());
        twins::<_, saved::RulerSettings>(RulerSettings {
            mode: RulerMode::CurrentLine,
            scope: RulerScope::Row,
            ..RulerSettings::default()
        });
        twins::<_, saved::SyllableOptions>(SyllableOptions {
            separator: "-".into(),
            ..SyllableOptions::default()
        });
        for p in RsvpPosition::ALL {
            assert_eq!(RsvpPosition::from(saved::RsvpPosition::from(p)), p);
        }
    }
}
