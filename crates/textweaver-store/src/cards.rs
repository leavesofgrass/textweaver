//! Study cards (B1-f1): cards made from what a reader already marked, kept
//! per document in `cards/<doc-key>.json` beside the state files.
//!
//! - A highlight becomes a **cloze** card: its sentence with the
//!   highlighted words blanked; the answer is the highlighted words.
//! - A note on a passage becomes a **question** card: the note is the
//!   question and the passage the answer, or the reverse
//!   ([`Card::reversed`]).
//! - A heading with its first sentence becomes a **recall** card.
//!
//! Each card keeps where it came from ([`Card::source`],
//! [`Card::source_id`], [`Card::range`]) so a list can go to the place, and
//! every grade the reader gave it with the time ([`Card::reviews`]), from
//! which [`crate::schedule`] works out when it is due (B1-f2).
//!
//! A deck remembers the ids of cards removed on this computer
//! ([`CardDeck::removed`]), so sync can tell a card removed here from one
//! that arrived from another computer and is not written here yet. Grades
//! from two computers are merged as a union ([`Card::merge_reviews`]), so
//! both survive.
//!
//! A card's id is derived from its source, so making cards again from the
//! same notes and highlights updates the same cards (their grades kept)
//! rather than adding copies, on this computer or another.
//!
//! This is a new file format, a proposal for the owner's review; it is not
//! frozen ([`CARDS_FORMAT`]).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use textweaver_core::CharRange;

use crate::doc_state::DocKey;
use crate::{StoreError, atomic_write};

/// The format of `cards/<doc-key>.json` this version writes.
pub const CARDS_FORMAT: u32 = 1;

/// The folder of card files, in the data folder.
pub const CARDS_DIR: &str = "cards";

/// What a card asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    /// A sentence with words blanked; the answer is the blanked words.
    Cloze,
    /// A question and its answer.
    Question,
    /// A heading to recall; the answer is what the section says first.
    Recall,
}

/// What a card was made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSource {
    /// A highlight, by its id.
    Highlight,
    /// A note, by its id.
    Note,
    /// A heading, by an id made from its text.
    Heading,
}

impl CardSource {
    /// The name stored and used in card ids.
    pub fn as_str(self) -> &'static str {
        match self {
            CardSource::Highlight => "highlight",
            CardSource::Note => "note",
            CardSource::Heading => "heading",
        }
    }
}

/// How well the reader recalled a card, in the reader's own judgment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    /// Not recalled.
    Again,
    /// Recalled with difficulty.
    Hard,
    /// Recalled.
    Good,
    /// Recalled easily.
    Easy,
}

impl Grade {
    /// The four grades, in key order (1 to 4).
    pub const ALL: [Grade; 4] = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy];

    /// The stored name.
    pub fn as_str(self) -> &'static str {
        match self {
            Grade::Again => "again",
            Grade::Hard => "hard",
            Grade::Good => "good",
            Grade::Easy => "easy",
        }
    }

    /// The grade for key `1` to `4`.
    pub fn from_key(c: char) -> Option<Grade> {
        let n = c.to_digit(10)?;
        Grade::ALL
            .get(usize::try_from(n).ok()?.checked_sub(1)?)
            .copied()
    }
}

/// One grade given to a card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    /// The grade.
    pub grade: Grade,
    /// When it was given (Unix seconds, UTC).
    pub ts: i64,
}

/// A study card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    /// Stable id, derived from the source ([`card_id`]).
    pub id: String,
    /// What it asks.
    pub kind: CardKind,
    /// What it was made from.
    pub source: CardSource,
    /// The note's or highlight's id, or the heading's derived id.
    pub source_id: String,
    /// The source's range in the canonical text when the card was made.
    pub range: CharRange,
    /// The question, as made.
    pub question: String,
    /// The answer, as made.
    pub answer: String,
    /// Asked the other way round: the answer is asked and the question
    /// revealed (question cards).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reversed: bool,
    /// When it was made (Unix seconds, UTC).
    #[serde(default)]
    pub created: i64,
    /// Every grade given, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviews: Vec<Review>,
    /// Unknown fields (a scheduler's, or a newer version's), preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Card {
    /// What is asked: the question, or the answer when reversed.
    pub fn prompt(&self) -> &str {
        if self.reversed {
            &self.answer
        } else {
            &self.question
        }
    }

    /// What is revealed: the answer, or the question when reversed.
    pub fn reveal(&self) -> &str {
        if self.reversed {
            &self.question
        } else {
            &self.answer
        }
    }

    /// The last grade given, if any.
    pub fn last_grade(&self) -> Option<Grade> {
        self.reviews.iter().max_by_key(|r| r.ts).map(|r| r.grade)
    }

    /// Adds the grades in `other` this card lacks: the union of both, in
    /// time order, so grades given on two computers both survive. Returns
    /// whether any was added.
    pub fn merge_reviews(&mut self, other: &[Review]) -> bool {
        let before = self.reviews.len();
        for r in other {
            if !self.reviews.contains(r) {
                self.reviews.push(r.clone());
            }
        }
        if self.reviews.len() == before {
            return false;
        }
        self.reviews
            .sort_by(|a, b| a.ts.cmp(&b.ts).then(a.grade.as_str().cmp(b.grade.as_str())));
        true
    }
}

/// The id of the card made from `source` with id `source_id`.
pub fn card_id(source: CardSource, source_id: &str) -> String {
    crate::notes::stable_id64("card-", &[source.as_str(), source_id])
}

/// One document's cards.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDeck {
    /// The file's format ([`CARDS_FORMAT`]).
    #[serde(default)]
    pub format: u32,
    /// The cards, in document order.
    #[serde(default)]
    pub cards: Vec<Card>,
    /// The ids of cards removed on this computer, so sync publishes the
    /// removal (B1-f2). Added without raising [`CARDS_FORMAT`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// Unknown fields, preserved.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for CardDeck {
    fn default() -> Self {
        CardDeck {
            format: CARDS_FORMAT,
            cards: Vec::new(),
            removed: Vec::new(),
            extra: serde_json::Map::new(),
        }
    }
}

/// What [`CardDeck::add_made`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Made {
    /// Cards new to the deck.
    pub added: usize,
    /// Cards already there whose question or answer changed.
    pub updated: usize,
}

impl CardDeck {
    /// The card with `id`.
    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards.iter().find(|c| c.id == id)
    }

    /// Adds freshly made cards: a card already in the deck (the same id)
    /// takes the new question, answer, and range and keeps its grades,
    /// direction, and creation time; the others are added. The deck stays
    /// in document order.
    pub fn add_made(&mut self, made: Vec<Card>) -> Made {
        let mut out = Made::default();
        for card in made {
            match self.cards.iter_mut().find(|c| c.id == card.id) {
                Some(old) => {
                    if old.question != card.question || old.answer != card.answer {
                        out.updated += 1;
                    }
                    old.question = card.question;
                    old.answer = card.answer;
                    old.range = card.range;
                    old.kind = card.kind;
                }
                None => {
                    out.added += 1;
                    self.removed.retain(|id| *id != card.id);
                    self.cards.push(card);
                }
            }
        }
        self.cards.sort_by_key(|c| (c.range.start, c.range.end));
        out
    }

    /// Records `grade` for card `id` at `ts`; false when there is no such
    /// card.
    pub fn grade(&mut self, id: &str, grade: Grade, ts: i64) -> bool {
        let Some(card) = self.cards.iter_mut().find(|c| c.id == id) else {
            return false;
        };
        card.reviews.push(Review { grade, ts });
        true
    }

    /// Asks card `id` the other way round, or back again: its new
    /// direction, or `None` when there is no such card.
    pub fn reverse(&mut self, id: &str) -> Option<bool> {
        let card = self.cards.iter_mut().find(|c| c.id == id)?;
        card.reversed = !card.reversed;
        Some(card.reversed)
    }

    /// Removes card `id`, and remembers it was removed here: the card, if
    /// there was one.
    pub fn remove(&mut self, id: &str) -> Option<Card> {
        let i = self.cards.iter().position(|c| c.id == id)?;
        if !self.removed.iter().any(|r| r == id) {
            self.removed.push(id.to_owned());
        }
        Some(self.cards.remove(i))
    }
}

/// Reads and writes `cards/<doc-key>.json`.
#[derive(Clone, Debug)]
pub struct CardStore {
    dir: PathBuf,
}

impl CardStore {
    /// A store under `dir`, normally [`Paths::cards_dir`](crate::Paths::cards_dir).
    pub fn new(dir: PathBuf) -> Self {
        CardStore { dir }
    }

    /// The folder holding the card files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn file(&self, key: &DocKey) -> PathBuf {
        self.dir.join(format!("{}.json", key.0))
    }

    /// The document's cards; an empty deck when it has no file. A file
    /// that does not parse is renamed to `<key>.corrupt-<unix time>.bak`
    /// first, so the next save cannot overwrite the grades it holds.
    pub fn load(&self, key: &DocKey) -> CardDeck {
        let path = self.file(key);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return CardDeck::default();
        };
        match serde_json::from_str(&text) {
            Ok(deck) => deck,
            Err(e) => {
                crate::atomic::set_aside(&path, &e);
                CardDeck::default()
            }
        }
    }

    /// Saves the document's cards atomically.
    pub fn save(&self, key: &DocKey, deck: &CardDeck) -> Result<(), StoreError> {
        let path = self.file(key);
        let text = serde_json::to_string_pretty(deck).map_err(|e| StoreError::Parse {
            path: path.clone(),
            message: e.to_string(),
        })?;
        atomic_write(&path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textweaver_core::CharPos;

    fn card(source_id: &str, start: usize, q: &str) -> Card {
        Card {
            id: card_id(CardSource::Note, source_id),
            kind: CardKind::Question,
            source: CardSource::Note,
            source_id: source_id.into(),
            range: CharRange::new(CharPos(start), CharPos(start + 4)),
            question: q.into(),
            answer: "Answer.".into(),
            reversed: false,
            created: 10,
            reviews: Vec::new(),
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn making_again_keeps_grades_and_adds_only_new_cards() {
        let mut deck = CardDeck::default();
        let made = deck.add_made(vec![card("b", 20, "Second?"), card("a", 5, "First?")]);
        assert_eq!(
            made,
            Made {
                added: 2,
                updated: 0
            }
        );
        assert_eq!(deck.cards[0].question, "First?");
        let id = deck.cards[0].id.clone();
        assert!(deck.grade(&id, Grade::Good, 100));
        assert_eq!(deck.reverse(&id), Some(true));
        let made = deck.add_made(vec![
            card("a", 6, "First, changed?"),
            card("c", 30, "Third?"),
        ]);
        assert_eq!(
            made,
            Made {
                added: 1,
                updated: 1
            }
        );
        let a = deck.card(&id).unwrap();
        assert_eq!(a.question, "First, changed?");
        assert_eq!(a.last_grade(), Some(Grade::Good));
        assert!(a.reversed);
        assert_eq!(a.prompt(), "Answer.");
        assert_eq!(a.reveal(), "First, changed?");
        assert_eq!(deck.cards.len(), 3);
        assert!(deck.remove(&id).is_some());
        assert_eq!(deck.removed, vec![id.clone()]);
        assert!(!deck.grade(&id, Grade::Again, 1));
        // Made again: no longer removed.
        deck.add_made(vec![card("a", 6, "First?")]);
        assert!(deck.removed.is_empty());
    }

    #[test]
    fn grades_merge_as_a_union_in_time_order() {
        let mut a = card("a", 1, "Q?");
        a.reviews = vec![
            Review {
                grade: Grade::Good,
                ts: 10,
            },
            Review {
                grade: Grade::Hard,
                ts: 30,
            },
        ];
        let theirs = vec![
            Review {
                grade: Grade::Again,
                ts: 20,
            },
            Review {
                grade: Grade::Good,
                ts: 10,
            },
        ];
        assert!(a.merge_reviews(&theirs));
        let ts: Vec<i64> = a.reviews.iter().map(|r| r.ts).collect();
        assert_eq!(ts, vec![10, 20, 30]);
        assert_eq!(a.last_grade(), Some(Grade::Hard));
        assert!(!a.merge_reviews(&theirs));
    }

    #[test]
    fn grades_from_keys() {
        assert_eq!(Grade::from_key('1'), Some(Grade::Again));
        assert_eq!(Grade::from_key('4'), Some(Grade::Easy));
        assert_eq!(Grade::from_key('0'), None);
        assert_eq!(Grade::from_key('5'), None);
        assert_eq!(Grade::from_key('x'), None);
    }

    #[test]
    fn decks_round_trip_and_damaged_files_are_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        let store = CardStore::new(dir.path().join(CARDS_DIR));
        let key = DocKey::untitled(1);
        assert!(store.load(&key).cards.is_empty());
        let mut deck = CardDeck::default();
        deck.add_made(vec![card("a", 1, "Q?")]);
        let id = deck.cards[0].id.clone();
        deck.grade(&id, Grade::Hard, 42);
        store.save(&key, &deck).unwrap();
        let text = std::fs::read_to_string(store.file(&key)).unwrap();
        assert!(text.contains("\"grade\": \"hard\""), "{text}");
        assert_eq!(store.load(&key), deck);
        std::fs::write(store.file(&key), "{\"cards\": [").unwrap();
        assert!(store.load(&key).cards.is_empty());
        let kept = std::fs::read_dir(store.dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .count();
        assert_eq!(kept, 1);
    }
}
