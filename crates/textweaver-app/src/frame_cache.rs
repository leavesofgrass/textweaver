//! Caches for what a frontend asks for on every frame (W6u): the Unicode
//! math shown, bionic reading's bold starts, the difficult words, and the
//! syllable breaks of each line. Each is kept for the document revision,
//! the range, and the settings it was made with, and made again only when
//! one of them changes; a frame of the terminal reader with the reading
//! aids on took 2 ms, most of it redoing this work.
//!
//! The caches are behind mutexes, because the helpers take `&App`; they
//! are never contended (the app lives on one thread).

use std::collections::HashMap;
use std::sync::Mutex;

use textweaver_core::{CharPos, CharRange};

/// A key and the value made for it.
type Slot<K, V> = Mutex<Option<(K, V)>>;

/// The per-frame caches of one app.
#[derive(Default)]
pub(crate) struct FrameCaches {
    pub(crate) math: Slot<MathKey, Vec<(CharRange, String)>>,
    pub(crate) bionic: Slot<
        (
            u64,
            CharRange,
            textweaver_store::reading_aids::BionicOptions,
        ),
        Vec<CharRange>,
    >,
    pub(crate) difficult: Slot<(u64, CharRange), Vec<CharRange>>,
    /// Syllable breaks by line range, for one revision and one set of
    /// options; cleared when either changes.
    pub(crate) syllables: Mutex<SyllableCache>,
}

/// What the Unicode math of a range depends on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MathKey {
    pub(crate) revision: u64,
    pub(crate) range: CharRange,
    pub(crate) asciimath: Option<char>,
}

/// Syllable breaks of the lines drawn.
#[derive(Default)]
pub(crate) struct SyllableCache {
    key: Option<(u64, textweaver_store::reading_aids::SyllableOptions)>,
    lines: HashMap<(usize, usize), Vec<CharPos>>,
}

/// Most lines' breaks kept; the cache starts again past it.
const SYLLABLE_LINES: usize = 4096;

impl SyllableCache {
    /// The breaks of `range` for `key`, made with `make` when not kept.
    pub(crate) fn get(
        &mut self,
        key: (u64, textweaver_store::reading_aids::SyllableOptions),
        range: CharRange,
        make: impl FnOnce() -> Vec<CharPos>,
    ) -> Vec<CharPos> {
        if self.key.as_ref() != Some(&key) || self.lines.len() > SYLLABLE_LINES {
            self.key = Some(key);
            self.lines.clear();
        }
        self.lines
            .entry((range.start.0, range.end.0))
            .or_insert_with(make)
            .clone()
    }
}

/// The value in `slot` for `key`, made with `make` when the key changed.
pub(crate) fn cached<K: PartialEq, V: Clone>(
    slot: &Slot<K, V>,
    key: K,
    make: impl FnOnce() -> V,
) -> V {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = guard.as_ref()
        && *k == key
    {
        return v.clone();
    }
    let v = make();
    *guard = Some((key, v.clone()));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_made_once_per_key() {
        let slot: Slot<u32, u32> = Mutex::new(None);
        let mut made = 0;
        for _ in 0..3 {
            assert_eq!(
                cached(&slot, 1, || {
                    made += 1;
                    10
                }),
                10
            );
        }
        assert_eq!(made, 1);
        assert_eq!(cached(&slot, 2, || 20), 20);
    }
}
