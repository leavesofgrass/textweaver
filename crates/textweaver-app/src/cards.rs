//! Study cards and the study session (B1-f1).
//!
//! Cards are made from what the reader already marked
//! ([`make_cards`]): a highlight becomes a cloze card, a note on a passage
//! a question card, and the heading of each section with notes or
//! highlights a recall card. They are kept per document by the store
//! ([`textweaver_store::cards`]) and written through the background writer.
//!
//! - **Make cards** (`make_cards`) makes them from every note and
//!   highlight, as the study sheet groups them; in the notes and
//!   highlights lists, `c` makes one from the item.
//! - **Study cards** (`study_cards`) is a session on the reveal list
//!   (crate::reveal): each card's question is said, Enter reveals and says
//!   the answer, and the reader grades it in words with 1 to 4 or the
//!   palette's Grade again, hard, good, and easy. Each grade is stored with
//!   the time, for a scheduler; nothing is scheduled here. Space answers
//!   aloud through dictation, read back and never graded by the machine.
//!   `r` asks a question card the other way round.
//! - **Cards** (`list_cards`) lists them; Enter goes to a card's source,
//!   Delete removes a card after asking.

use textweaver_core::{CharPos, CharRange, MarkerKind, Unit};
use textweaver_keymap::ActionId;
use textweaver_lexicon::args;
use textweaver_lexicon::i18n::Catalog;
use textweaver_store::cards::{Card, CardDeck, CardKind, CardSource, CardStore, Grade, card_id};
use textweaver_store::notes::{collapse, stable_id64};
use textweaver_store::{DocKey, Highlight, Note};
use textweaver_text::Document;
use textweaver_text::units::unit_at;

use crate::app::{App, ListKind};
use crate::command::Effect;
use crate::list_model::ListKey;
use crate::nav::ReadAfter;
use crate::reveal::{RevealItem, RevealList};

/// The longest question or answer kept on a card, in characters.
const CARD_TEXT_MAX: usize = 400;

/// The cards of the open document and the study session under way.
#[derive(Debug, Default)]
pub(crate) struct CardsState {
    /// The deck last read or changed, with its document.
    deck: Option<(DocKey, CardDeck)>,
    /// The study session, kept while its list is closed (the palette's
    /// grades work on it).
    session: Option<StudySession>,
}

/// A study session: the cards asked, in order, and the list as shown.
#[derive(Debug)]
struct StudySession {
    key: DocKey,
    ids: Vec<String>,
    list: RevealList,
    /// The card being asked.
    at: usize,
}

/// The headings of `doc`: range and text.
fn headings(doc: &Document) -> Vec<(CharRange, String)> {
    doc.marker_index()
        .iter(MarkerKind::Heading, None)
        .map(|m| (m.range, collapse(&doc.slice(m.range), 120)))
        .collect()
}

/// The heading `pos` falls under, as an index into `heads`.
fn section_of(heads: &[(CharRange, String)], pos: CharPos) -> Option<usize> {
    heads.iter().rposition(|h| h.0.start <= pos)
}

/// The sentence (or sentences) around `range`, with `range`'s words
/// replaced by the word "blank": `None` when nothing is left around it.
fn cloze(c: &Catalog, doc: &Document, range: CharRange) -> Option<String> {
    let last = CharPos(range.end.0.saturating_sub(1).max(range.start.0));
    let start = unit_at(doc, range.start, Unit::Sentence)?
        .start
        .min(range.start);
    let end = unit_at(doc, last, Unit::Sentence)?.end.max(range.end);
    let before = collapse(&doc.slice(CharRange::new(start, range.start)), 300);
    let after = collapse(&doc.slice(CharRange::new(range.end, end)), 300);
    if !before
        .chars()
        .chain(after.chars())
        .any(char::is_alphanumeric)
    {
        return None;
    }
    let mut out = before;
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(&c.fmt("cards-blank", &[]));
    // Punctuation right after the blank stays against it.
    if after.starts_with(char::is_alphanumeric) {
        out.push(' ');
    }
    out.push_str(&after);
    Some(out)
}

/// The first sentence of the section under heading `h`, up to the next
/// heading: `None` when the section has no text of its own.
fn first_sentence(doc: &Document, heads: &[(CharRange, String)], h: usize) -> Option<String> {
    let from = heads[h].0.end;
    let to = heads
        .get(h + 1)
        .map_or(CharPos(doc.len_chars()), |n| n.0.start);
    if from >= to {
        return None;
    }
    let body = doc.slice(CharRange::new(from, to));
    let skip = body.chars().take_while(|c| c.is_whitespace()).count();
    let at = CharPos(from.0 + skip);
    if at >= to {
        return None;
    }
    let s = unit_at(doc, at, Unit::Sentence)?;
    let text = collapse(&doc.slice(CharRange::new(at, s.end.min(to))), CARD_TEXT_MAX);
    (!text.is_empty()).then_some(text)
}

/// What to make cards from.
pub(crate) struct Sources<'a> {
    /// The document.
    pub(crate) doc: &'a Document,
    /// Its title, the section before the first heading.
    pub(crate) title: &'a str,
    /// Notes to make question cards from.
    pub(crate) notes: &'a [Note],
    /// Highlights to make cloze cards from.
    pub(crate) highlights: &'a [Highlight],
    /// Also a recall card for each section the notes and highlights are in.
    pub(crate) headings: bool,
}

fn new_card(
    kind: CardKind,
    source: CardSource,
    source_id: &str,
    range: CharRange,
    question: String,
    answer: String,
    now: i64,
) -> Card {
    Card {
        id: card_id(source, source_id),
        kind,
        source,
        source_id: source_id.to_owned(),
        range,
        question,
        answer,
        reversed: false,
        created: now,
        reviews: Vec::new(),
        extra: serde_json::Map::new(),
    }
}

/// Cards from `src`, in document order, made at `now`:
///
/// - a highlight: its sentence with the highlighted words blanked, the
///   words the answer; a highlight that is its whole sentence asks what
///   was highlighted in its section instead;
/// - a note with text on a passage: the note the question, the passage the
///   answer (notes without text or passage make none);
/// - with `headings`, a recall card for each section that holds one of
///   them and has a first sentence of its own.
pub(crate) fn make_cards(c: &Catalog, src: &Sources<'_>, now: i64) -> Vec<Card> {
    let doc = src.doc;
    let heads = headings(doc);
    let section = |pos: CharPos| -> String {
        section_of(&heads, pos).map_or_else(|| src.title.to_owned(), |h| heads[h].1.clone())
    };
    let mut out = Vec::new();
    let mut sections = std::collections::BTreeSet::new();
    for h in src.highlights {
        let range = h.range.clamp_to(doc.len_chars());
        let answer = collapse(&doc.slice(range), CARD_TEXT_MAX);
        if answer.is_empty() {
            continue;
        }
        sections.insert(section_of(&heads, range.start));
        let question = cloze(c, doc, range).unwrap_or_else(|| {
            c.fmt(
                "reveal-prompt-highlight",
                &args!["section" => section(range.start)],
            )
        });
        let kind = CardKind::Cloze;
        out.push(new_card(
            kind,
            CardSource::Highlight,
            &h.id,
            range,
            question,
            answer,
            now,
        ));
    }
    for n in src.notes {
        let range = n.range.clamp_to(doc.len_chars());
        let question = collapse(n.note.trim(), CARD_TEXT_MAX);
        let answer = collapse(&doc.slice(range), CARD_TEXT_MAX);
        if question.is_empty() || answer.is_empty() {
            continue;
        }
        sections.insert(section_of(&heads, range.start));
        let kind = CardKind::Question;
        out.push(new_card(
            kind,
            CardSource::Note,
            &n.id,
            range,
            question,
            answer,
            now,
        ));
    }
    if src.headings {
        for h in sections.into_iter().flatten() {
            let Some(answer) = first_sentence(doc, &heads, h) else {
                continue;
            };
            let (range, text) = &heads[h];
            // The same heading text twice gets two ids: its occurrence.
            let nth = heads[..h].iter().filter(|x| &x.1 == text).count();
            let source_id = stable_id64("h-", &[text, &nth.to_string()]);
            let question = c.fmt("cards-recall-question", &args!["heading" => text.as_str()]);
            let kind = CardKind::Recall;
            out.push(new_card(
                kind,
                CardSource::Heading,
                &source_id,
                *range,
                question,
                answer,
                now,
            ));
        }
    }
    out.sort_by_key(|c| (c.range.start, c.range.end));
    out
}

/// A grade's name, as said and shown.
fn grade_msg(grade: Grade) -> &'static str {
    match grade {
        Grade::Again => "cards-grade-again",
        Grade::Hard => "cards-grade-hard",
        Grade::Good => "cards-grade-good",
        Grade::Easy => "cards-grade-easy",
    }
}

fn reveal_item(card: &Card) -> RevealItem {
    RevealItem {
        prompt: card.prompt().to_owned(),
        answer: card.reveal().to_owned(),
    }
}

impl App {
    /// Runs one of the cards actions.
    pub(crate) fn cards_action(&mut self, a: ActionId) -> Vec<Effect> {
        match a {
            ActionId::MakeCards => self.make_cards(),
            ActionId::StudyCards => self.study_cards(),
            ActionId::ListCards => self.list_cards(),
            ActionId::GradeAgain => self.grade_card(Grade::Again),
            ActionId::GradeHard => self.grade_card(Grade::Hard),
            ActionId::GradeGood => self.grade_card(Grade::Good),
            ActionId::GradeEasy => self.grade_card(Grade::Easy),
            _ => vec![Effect::Redraw],
        }
    }

    /// The open document's key, or a message saying there is none.
    fn cards_doc(&mut self) -> Option<DocKey> {
        let key = self.session.as_ref().map(|s| s.key.clone());
        if key.is_none() {
            let msg = self.msg("cards-no-document");
            self.tell(&msg);
        }
        key
    }

    /// The deck of document `key`, read on first use.
    // shortcut: a deck is read on the input thread when first used; it is
    // one small file per document. Read it on the writer if decks grow.
    fn deck(&mut self, key: &DocKey) -> &mut CardDeck {
        if self.cards.deck.as_ref().is_none_or(|(k, _)| k != key) {
            let deck = self
                .paths
                .as_ref()
                .map(|p| CardStore::new(p.cards_dir()).load(key))
                .unwrap_or_default();
            self.cards.deck = Some((key.clone(), deck));
        }
        &mut self
            .cards
            .deck
            .get_or_insert_with(|| (key.clone(), CardDeck::default()))
            .1
    }

    /// Queues the deck in hand for writing.
    fn save_deck(&mut self) {
        let (Some(paths), Some((key, deck))) = (self.paths.as_ref(), self.cards.deck.as_ref())
        else {
            return;
        };
        self.writer.send(crate::writer::Job::Cards {
            store: CardStore::new(paths.cards_dir()),
            key: key.clone(),
            deck: Box::new(deck.clone()),
        });
    }

    /// Adds `made` to the open document's deck, saves it, and says how
    /// many are new.
    fn add_cards(&mut self, key: &DocKey, made: Vec<Card>) -> textweaver_store::cards::Made {
        let result = self.deck(key).add_made(made);
        self.save_deck();
        result
    }

    /// The palette's `make_cards`: cards from every note and highlight, and
    /// the sections they are in.
    fn make_cards(&mut self) -> Vec<Effect> {
        let Some(key) = self.cards_doc() else {
            return vec![Effect::Redraw];
        };
        let made = {
            let Some(s) = self.session.as_ref() else {
                return vec![Effect::Redraw];
            };
            let src = Sources {
                doc: &s.doc,
                title: &s.title,
                notes: &s.notes,
                highlights: &s.highlights,
                headings: true,
            };
            make_cards(self.cat(), &src, textweaver_store::now_ts())
        };
        if made.is_empty() {
            let msg = self.msg("cards-nothing-to-make");
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let result = self.add_cards(&key, made);
        let total = self.deck(&key).cards.len();
        let msg = self.msg_args(
            "cards-made",
            &args!["added" => result.added, "total" => total],
        );
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// `c` in the notes or highlights list: a card from item `n`. The
    /// list stays.
    fn card_from_list_item(&mut self, note: bool, n: usize) -> Vec<Effect> {
        let Some(key) = self.cards_doc() else {
            return vec![Effect::Redraw];
        };
        let made = {
            let Some(s) = self.session.as_ref() else {
                return vec![Effect::Redraw];
            };
            let (notes, highlights) = if note {
                (s.notes.get(n..=n).unwrap_or_default(), &[][..])
            } else {
                (&[][..], s.highlights.get(n..=n).unwrap_or_default())
            };
            let src = Sources {
                doc: &s.doc,
                title: &s.title,
                notes,
                highlights,
                headings: false,
            };
            make_cards(self.cat(), &src, textweaver_store::now_ts())
        };
        let Some(question) = made.first().map(|c| c.question.clone()) else {
            let msg = self.msg("cards-none-from-item");
            self.tell(&msg);
            return vec![Effect::Redraw];
        };
        let result = self.add_cards(&key, made);
        let id = if result.added > 0 {
            "cards-made-one"
        } else {
            "cards-updated-one"
        };
        let msg = self.msg_args(id, &args!["question" => question]);
        self.tell(&msg);
        vec![Effect::Redraw]
    }

    /// The palette's `study_cards`: the session, from the first card.
    fn study_cards(&mut self) -> Vec<Effect> {
        let Some(key) = self.cards_doc() else {
            return vec![Effect::Redraw];
        };
        let title = self
            .session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        let deck = self.deck(&key).clone();
        if deck.cards.is_empty() {
            let make = self.key(ActionId::MakeCards);
            let msg = self.msg_args("cards-none", &args!["key" => make]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let items = deck.cards.iter().map(reveal_item).collect();
        let list = RevealList::new(
            self.msg_args("cards-study-title", &args!["title" => title]),
            items,
        );
        let intro = self.msg_args("cards-study-intro", &args!["n" => list.len()]);
        self.cards.session = Some(StudySession {
            key,
            ids: deck.cards.iter().map(|c| c.id.clone()).collect(),
            list: list.clone(),
            at: 0,
        });
        self.show_reveal_list(&intro, list)
    }

    /// True when `list` is the study session's.
    fn is_session_list(&self, list: &RevealList) -> bool {
        self.cards
            .session
            .as_ref()
            .is_some_and(|s| s.list.title == list.title)
    }

    /// Enter on row `n` of a reveal list: the session remembers the card
    /// being asked, and the answer it shows.
    pub(crate) fn cards_row_chosen(&mut self, list: &RevealList, n: usize) {
        if self.is_session_list(list)
            && let Some(s) = self.cards.session.as_mut()
        {
            s.at = n;
            s.list = list.clone();
            s.list.reveal(n);
        }
    }

    /// Takes the study session, with the list as shown when it is open,
    /// the row being asked, and whether its list is shown now; `None`
    /// (said) when there is none for the open document.
    fn take_session(&mut self) -> Option<(StudySession, usize, bool)> {
        let open = self.session.as_ref().map(|s| s.key.clone());
        let session = self
            .cards
            .session
            .take()
            .filter(|s| Some(&s.key) == open.as_ref());
        let Some(mut session) = session else {
            let key = self.key(ActionId::StudyCards);
            let msg = self.msg_args("cards-no-session", &args!["key" => key]);
            self.tell(&msg);
            return None;
        };
        let mut at = session.at;
        let mut shown = false;
        if let Some(ListKind::Reveal(l)) = &self.list
            && l.title == session.list.title
            && let Some(m) = self.list_model.as_ref()
        {
            session.list = l.clone();
            at = m.selected;
            shown = true;
        }
        let at = at.min(session.ids.len().saturating_sub(1));
        Some((session, at, shown))
    }

    /// Grades the card being asked, stores the grade with the time, and
    /// asks the next card; after the last, the session ends.
    fn grade_card(&mut self, grade: Grade) -> Vec<Effect> {
        let Some((mut session, n, shown)) = self.take_session() else {
            return vec![Effect::Redraw];
        };
        let Some(id) = session.ids.get(n).cloned() else {
            return vec![Effect::Redraw];
        };
        let key = session.key.clone();
        if !self
            .deck(&key)
            .grade(&id, grade, textweaver_store::now_ts())
        {
            // Removed from the Cards list meanwhile.
            let msg = self.msg("cards-card-gone");
            self.tell(&msg);
            self.cards.session = Some(session);
            return vec![Effect::Redraw];
        }
        self.save_deck();
        session.list.reveal(n);
        let name = self.msg(grade_msg(grade));
        let total = session.ids.len();
        if n + 1 >= total {
            let msg = self.msg_args("cards-session-done", &args!["grade" => name, "n" => total]);
            self.tell(&msg);
            if matches!(self.list, Some(ListKind::Reveal(_))) {
                self.list = None;
                self.list_model = None;
            }
            return vec![Effect::Redraw];
        }
        let next = n + 1;
        let question = self
            .deck(&key)
            .card(&session.ids[next])
            .map(|c| c.prompt().to_owned())
            .unwrap_or_default();
        session.at = next;
        let list = session.list.clone();
        self.cards.session = Some(session);
        if !shown {
            // Graded from the palette: the list opens again on the next
            // card, introduced, and says the card with its place.
            let msg = self.msg_args(
                "cards-graded-reopen",
                &args!["grade" => name, "i" => next + 1, "n" => total],
            );
            self.tell(&msg);
            return self.reshow_reveal_list(list, next);
        }
        let msg = self.msg_args(
            "cards-graded",
            &args!["grade" => name, "i" => next + 1, "n" => total, "question" => question],
        );
        self.tell(&msg);
        self.list_reshow_quiet = true;
        self.reshow_reveal_list(list, next)
    }

    /// `r` in the session: asks a question card the other way round, or
    /// back again, from now on. The list stays on it.
    fn reverse_card(&mut self) -> Vec<Effect> {
        let Some((mut session, n, _)) = self.take_session() else {
            return vec![Effect::Redraw];
        };
        let key = session.key.clone();
        let id = session.ids.get(n).cloned().unwrap_or_default();
        let deck = self.deck(&key);
        let card = deck.card(&id).cloned();
        let msg = match card {
            Some(c) if c.kind == CardKind::Question => {
                let reversed = deck.reverse(&id).unwrap_or(false);
                let item = deck.card(&id).map(reveal_item);
                self.save_deck();
                if let Some(item) = item {
                    let question = item.prompt.clone();
                    session.list.set_item(n, item);
                    let id = if reversed {
                        "cards-reversed"
                    } else {
                        "cards-unreversed"
                    };
                    self.msg_args(id, &args!["question" => question])
                } else {
                    String::new()
                }
            }
            Some(_) => self.msg("cards-not-reversible"),
            None => self.msg("cards-card-gone"),
        };
        self.tell(&msg);
        let list = session.list.clone();
        self.cards.session = Some(session);
        self.list_reshow_quiet = true;
        self.reshow_reveal_list(list, n)
    }

    /// The palette's `list_cards`: every card, with its kind and last
    /// grade; Enter goes to its source.
    pub(crate) fn list_cards(&mut self) -> Vec<Effect> {
        self.show_cards_list(0)
    }

    fn show_cards_list(&mut self, focus: usize) -> Vec<Effect> {
        let Some(key) = self.cards_doc() else {
            return vec![Effect::Redraw];
        };
        let deck = self.deck(&key).clone();
        if deck.cards.is_empty() {
            let make = self.key(ActionId::MakeCards);
            let msg = self.msg_args("cards-none", &args!["key" => make]);
            self.tell(&msg);
            return vec![Effect::Redraw];
        }
        let c = self.cat();
        let items: Vec<String> = deck
            .cards
            .iter()
            .map(|card| {
                let kind = c.fmt(
                    match card.kind {
                        CardKind::Cloze => "cards-kind-cloze",
                        CardKind::Question => "cards-kind-question",
                        CardKind::Recall => "cards-kind-recall",
                    },
                    &[],
                );
                let grade = card.last_grade().map_or_else(
                    || c.fmt("cards-not-graded", &[]),
                    |g| c.fmt("cards-last-grade", &args!["grade" => c.fmt(grade_msg(g), &[])]),
                );
                c.fmt(
                    "cards-item",
                    &args!["kind" => kind, "question" => collapse(card.prompt(), 200), "grade" => grade],
                )
            })
            .collect();
        let msg = self.msg_args("cards-list-intro", &args!["n" => items.len()]);
        self.tell(&msg);
        self.list = Some(ListKind::Cards(
            deck.cards.iter().map(|c| c.id.clone()).collect(),
        ));
        self.pending_list_focus = Some(focus.min(items.len().saturating_sub(1)));
        vec![Effect::ShowList {
            title: self.msg("cards-list-title"),
            items,
        }]
    }

    /// Enter in the Cards list: goes to the card's source, where the note
    /// or highlight is now when it still exists.
    pub(crate) fn choose_card_row(&mut self, ids: &[String], n: usize) {
        let (Some(id), Some(key)) = (ids.get(n), self.session.as_ref().map(|s| s.key.clone()))
        else {
            return;
        };
        let Some(card) = self.deck(&key).card(id).cloned() else {
            return;
        };
        let Some(s) = self.session.as_ref() else {
            return;
        };
        let now = match card.source {
            CardSource::Note => s
                .notes
                .iter()
                .find(|x| x.id == card.source_id)
                .map(|x| x.range),
            CardSource::Highlight => s
                .highlights
                .iter()
                .find(|x| x.id == card.source_id)
                .map(|x| x.range),
            CardSource::Heading => None,
        };
        let range = now.unwrap_or(card.range).clamp_to(s.doc.len_chars());
        let text = collapse(&s.doc.slice(range), 200);
        let label = self.msg("cards-source-label");
        let msg = self.nav_message(Some(&label), range.start, &text);
        self.jump(range.start, true, ReadAfter::Follow, &msg);
    }

    /// Delete in the Cards list, after the question: removes the card and
    /// its grades, and shows the list again.
    pub(crate) fn delete_card_row(&mut self, ids: &[String], n: usize) -> Vec<Effect> {
        let (Some(id), Some(key)) = (ids.get(n), self.session.as_ref().map(|s| s.key.clone()))
        else {
            return vec![Effect::Redraw];
        };
        if self.deck(&key).remove(id).is_some() {
            self.save_deck();
            let msg = self.msg("cards-removed");
            self.tell(&msg);
        }
        if self.deck(&key).cards.is_empty() {
            self.list = None;
            return vec![Effect::Redraw];
        }
        self.show_cards_list(n)
    }

    /// The cards' own keys in a list: `c` in the notes and highlights lists
    /// makes a card from the item; in the study session, 1 to 4 grade the
    /// card and `r` reverses it.
    pub(crate) fn cards_list_key(&mut self, key: ListKey) -> Option<Vec<Effect>> {
        let ListKey::Char(c) = key else {
            return None;
        };
        let n = self.list_model.as_ref().map_or(0, |l| l.selected);
        match &self.list {
            Some(ListKind::Notes) if c.eq_ignore_ascii_case(&'c') => {
                Some(self.card_from_list_item(true, n))
            }
            Some(ListKind::Highlights) if c.eq_ignore_ascii_case(&'c') => {
                Some(self.card_from_list_item(false, n))
            }
            Some(ListKind::Reveal(l)) if self.is_session_list(l) => {
                if let Some(grade) = Grade::from_key(c) {
                    Some(self.grade_card(grade))
                } else if c.eq_ignore_ascii_case(&'r') {
                    Some(self.reverse_card())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cards.md");
        std::fs::write(&path, text).unwrap();
        textweaver_formats::load_path(&path).unwrap()
    }

    fn at(doc: &Document, needle: &str) -> CharRange {
        let text = doc.text().to_string();
        let byte = text.find(needle).unwrap();
        let start = text[..byte].chars().count();
        CharRange::new(CharPos(start), CharPos(start + needle.chars().count()))
    }

    #[test]
    fn cards_come_from_highlights_notes_and_their_headings() {
        let d = doc(
            "# Kidneys\n\n## Renal clearance\n\nThe kidneys filter the blood. Urine leaves.\n\n## Empty\n\n## Last\n\nWhole sentence.\n",
        );
        let c = Catalog::english();
        let highlights = vec![
            Highlight {
                id: "h1".into(),
                range: at(&d, "filter"),
                ..Highlight::default()
            },
            Highlight {
                id: "h2".into(),
                range: at(&d, "Whole sentence."),
                ..Highlight::default()
            },
        ];
        let notes = vec![
            Note {
                id: "n1".into(),
                range: at(&d, "Urine leaves."),
                note: "What leaves?".into(),
                ..Note::default()
            },
            Note {
                id: "n2".into(),
                range: at(&d, "Urine"),
                note: "  ".into(),
                ..Note::default()
            },
        ];
        let src = Sources {
            doc: &d,
            title: "Kidneys",
            notes: &notes,
            highlights: &highlights,
            headings: true,
        };
        let cards = make_cards(&c, &src, 7);
        let asked: Vec<(&str, &str)> = cards
            .iter()
            .map(|c| (c.question.as_str(), c.answer.as_str()))
            .collect();
        assert_eq!(
            asked,
            [
                (
                    "What does \u{201c}Renal clearance\u{201d} say?",
                    "The kidneys filter the blood."
                ),
                ("The kidneys blank the blood.", "filter"),
                ("What leaves?", "Urine leaves."),
                ("What does \u{201c}Last\u{201d} say?", "Whole sentence."),
                ("What did you highlight in Last?", "Whole sentence."),
            ]
        );
        assert_eq!(cards[1].kind, CardKind::Cloze);
        assert_eq!(cards[2].source, CardSource::Note);
        assert_eq!(cards[2].id, card_id(CardSource::Note, "n1"));
        // Made again, the ids repeat.
        let again = make_cards(&c, &src, 9);
        assert_eq!(
            again.iter().map(|c| &c.id).collect::<Vec<_>>(),
            cards.iter().map(|c| &c.id).collect::<Vec<_>>()
        );
    }
}
