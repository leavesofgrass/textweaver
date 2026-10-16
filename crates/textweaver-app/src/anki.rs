//! Anki packages (B1-f3): study cards written to and read from `.apkg`
//! files, so a reader can take their cards to Anki and study an Anki deck
//! in textweaver.
//!
//! An `.apkg` is a zip holding an SQLite collection and a media map. The
//! format is Anki's own; it is written here with SQLite (rusqlite, with
//! SQLite built in), not with an Anki library.
//!
//! **Export** ([`export_apkg`]) writes one note per card, of a Basic note
//! type with the fields Front and Back, in one deck per document named
//! after it. Each note is tagged with the document (spaces made
//! underscores) and the card's kind (`textweaver::cloze`,
//! `textweaver::question`, `textweaver::recall`, and
//! `textweaver::reversed` for a card asked the other way round). The note's
//! GUID is the card's id. Scheduling is carried: a graded card goes as a
//! review card with its interval, ease, and due day, and every grade as a
//! review in Anki's log; a card never graded goes as a new card.
//!
//! **Import** ([`read_apkg`]) reads the collection of Anki 2.1 and later
//! (`collection.anki21b`, `collection.anki21`, or `collection.anki2`).
//! Basic notes become one card, and the second card of a Basic and
//! reversed note a card asked the other way round. Fields are HTML in
//! Anki; they become plain text. Cloze notes are skipped, and so is a
//! note that is nothing but pictures or sound; each is counted. Grades come
//! from Anki's review log, and textweaver works out the schedule again
//! from them with its own SM-2, so intervals can differ from Anki's. For a
//! note Anki made, only the last grade of each day is kept, as Anki's
//! learning steps grade a card several times in minutes.
//!
//! A card's id on import is the note's GUID when the note came from
//! textweaver, and an id made from the GUID otherwise, so importing the
//! same package twice adds no card twice
//! ([`textweaver_store::CardDeck::add_imported`]).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::Connection;
use serde_json::json;
use textweaver_store::cards::card_id;
use textweaver_store::schedule::{DAY_SECS, Schedule};
use textweaver_store::{Card, CardKind, CardSource, Grade, Review};

/// What can go wrong with a package.
#[derive(Debug, thiserror::Error)]
pub enum AnkiError {
    /// The file is not a zip.
    #[error("not an Anki package: {0}")]
    Zip(String),
    /// The zip holds no collection.
    #[error("not an Anki package: it holds no collection")]
    NoCollection,
    /// The collection could not be read or written.
    #[error("the Anki collection could not be read: {0}")]
    Sqlite(String),
}

impl From<rusqlite::Error> for AnkiError {
    fn from(e: rusqlite::Error) -> Self {
        AnkiError::Sqlite(e.to_string())
    }
}

impl From<zip::result::ZipError> for AnkiError {
    fn from(e: zip::result::ZipError) -> Self {
        AnkiError::Zip(e.to_string())
    }
}

/// One deck to export: its name and its cards.
#[derive(Clone, Debug)]
pub struct ExportDeck<'a> {
    /// The deck's name in Anki, normally the document's title.
    pub name: String,
    /// The document's title, for its tag.
    pub document: String,
    /// The cards.
    pub cards: &'a [Card],
}

/// One card read from a package, with the deck it was in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedCard {
    /// The Anki deck's name, with `::` between levels.
    pub deck: String,
    /// The card.
    pub card: Card,
}

/// What a package held.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Package {
    /// The cards, in the order Anki keeps the notes.
    pub cards: Vec<ImportedCard>,
    /// Cloze notes, skipped.
    pub skipped_cloze: usize,
    /// Notes with nothing but pictures or sound, skipped.
    pub skipped_media: usize,
    /// Notes skipped for another reason: no text, or a card of a note type
    /// with more than two cards.
    pub skipped_other: usize,
}

impl Package {
    /// The names of the decks the cards are in, sorted.
    pub fn decks(&self) -> Vec<String> {
        let mut out: Vec<String> = self.cards.iter().map(|c| c.deck.clone()).collect();
        out.sort();
        out.dedup();
        out
    }
}

/// The model (note type) id textweaver writes, the same in every package
/// so Anki recognizes its own copy on a second import.
const MODEL_ID: i64 = 1_760_000_000_101;

/// The tag prefix of textweaver's own tags.
const TAG_PREFIX: &str = "textweaver::";

/// The tag of a reversed card.
const TAG_REVERSED: &str = "textweaver::reversed";

/// Anki's schema 11 collection, as Anki 2.1 reads and upgrades it.
const SCHEMA: &str = "
CREATE TABLE col (id integer primary key, crt integer not null, mod integer not null,
  scm integer not null, ver integer not null, dty integer not null, usn integer not null,
  ls integer not null, conf text not null, models text not null, decks text not null,
  dconf text not null, tags text not null);
CREATE TABLE notes (id integer primary key, guid text not null, mid integer not null,
  mod integer not null, usn integer not null, tags text not null, flds text not null,
  sfld integer not null, csum integer not null, flags integer not null, data text not null);
CREATE TABLE cards (id integer primary key, nid integer not null, did integer not null,
  ord integer not null, mod integer not null, usn integer not null, type integer not null,
  queue integer not null, due integer not null, ivl integer not null, factor integer not null,
  reps integer not null, lapses integer not null, left integer not null, odue integer not null,
  odid integer not null, flags integer not null, data text not null);
CREATE TABLE revlog (id integer primary key, cid integer not null, usn integer not null,
  ease integer not null, ivl integer not null, lastIvl integer not null,
  factor integer not null, time integer not null, type integer not null);
CREATE TABLE graves (usn integer not null, oid integer not null, type integer not null);
CREATE INDEX ix_notes_usn on notes (usn);
CREATE INDEX ix_cards_usn on cards (usn);
CREATE INDEX ix_revlog_usn on revlog (usn);
CREATE INDEX ix_cards_nid on cards (nid);
CREATE INDEX ix_cards_sched on cards (did, queue, due);
CREATE INDEX ix_revlog_cid on revlog (cid);
CREATE INDEX ix_notes_csum on notes (csum);
";

/// A deck id from its name: stable, so exporting again updates the same
/// deck, and never 1, Anki's Default deck.
fn deck_id(name: &str) -> i64 {
    // FNV-1a.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    // Below 2^52, so it stays exact in JSON numbers, and above 2^40.
    i64::try_from(h % (1 << 51)).unwrap_or(0) + (1 << 40)
}

/// The tag for a document: its title with spaces made underscores, as an
/// Anki tag holds no space.
fn document_tag(title: &str) -> String {
    let t: String = title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    if t.is_empty() { "textweaver".into() } else { t }
}

fn kind_name(kind: CardKind) -> &'static str {
    match kind {
        CardKind::Cloze => "cloze",
        CardKind::Question => "question",
        CardKind::Recall => "recall",
    }
}

/// Plain text as an Anki field: HTML special characters escaped, line
/// breaks as `<br>`.
fn to_field(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.trim().chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\n' => out.push_str("<br>"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// Anki's note checksum: the first eight hex digits of the SHA-1 of the
/// first field's text.
fn checksum(text: &str) -> i64 {
    use sha1::{Digest, Sha1};
    let d = Sha1::digest(text.as_bytes());
    i64::from(u32::from_be_bytes([d[0], d[1], d[2], d[3]]))
}

/// Anki's button for a grade: 1 to 4.
fn ease_of(grade: Grade) -> i64 {
    match grade {
        Grade::Again => 1,
        Grade::Hard => 2,
        Grade::Good => 3,
        Grade::Easy => 4,
    }
}

fn factor(ease: f64) -> i64 {
    // An ease is between 1.3 and a few units.
    #[allow(clippy::cast_possible_truncation)]
    let f = (ease * 1000.0).round() as i64;
    f
}

/// Writes `decks` as an Anki package, at `now` (Unix seconds).
pub fn export_apkg(decks: &[ExportDeck<'_>], now: i64) -> Result<Vec<u8>, AnkiError> {
    // The collection's creation day: today, or the earliest due day, so no
    // due day is negative.
    let earliest = decks
        .iter()
        .flat_map(|d| d.cards.iter())
        .filter_map(|c| c.schedule().due)
        .min()
        .unwrap_or(now)
        .min(now);
    let crt = earliest - earliest.rem_euclid(DAY_SECS);

    let conn = Connection::open_in_memory()?;
    conn.execute_batch(SCHEMA)?;

    let mut deck_json = serde_json::Map::new();
    deck_json.insert("1".into(), deck_value(1, "Default", now));
    let mut first_deck = 1;
    for d in decks {
        let id = deck_id(&d.name);
        if first_deck == 1 {
            first_deck = id;
        }
        deck_json.insert(id.to_string(), deck_value(id, &d.name, now));
    }
    let models = json!({ MODEL_ID.to_string(): model_value(first_deck, now) });
    let conf = json!({
        "activeDecks": [1], "curDeck": 1, "newSpread": 0, "collapseTime": 1200,
        "timeLim": 0, "estTimes": true, "dueCounts": true, "curModel": MODEL_ID.to_string(),
        "nextPos": 1, "sortType": "noteFld", "sortBackwards": false, "addToCur": true
    });
    let dconf = json!({ "1": {
        "id": 1, "name": "Default", "mod": 0, "usn": 0, "maxTaken": 60, "autoplay": true,
        "timer": 0, "replayq": true, "dyn": false,
        "new": { "bury": true, "delays": [1.0, 10.0], "initialFactor": 2500,
                 "ints": [1, 4, 7], "order": 1, "perDay": 20, "separate": true },
        "rev": { "bury": true, "ease4": 1.3, "fuzz": 0.05, "ivlFct": 1.0,
                 "maxIvl": 36500, "minSpace": 1, "perDay": 200 },
        "lapse": { "delays": [10.0], "leechAction": 0, "leechFails": 8,
                   "minInt": 1, "mult": 0.0 }
    }});
    conn.execute(
        "INSERT INTO col VALUES (1, ?1, ?2, ?3, 11, 0, 0, 0, ?4, ?5, ?6, ?7, '{}')",
        rusqlite::params![
            crt,
            now * 1000,
            now * 1000,
            conf.to_string(),
            models.to_string(),
            serde_json::Value::Object(deck_json).to_string(),
            dconf.to_string(),
        ],
    )?;

    let mut next_id = now * 1000;
    let mut revlog_ids = HashSet::new();
    let mut position = 0i64;
    for d in decks {
        let did = deck_id(&d.name);
        let doc_tag = document_tag(&d.document);
        for card in d.cards {
            next_id += 1;
            let id = next_id;
            let mut tags = vec![
                doc_tag.clone(),
                format!("{TAG_PREFIX}{}", kind_name(card.kind)),
            ];
            if card.reversed {
                tags.push(TAG_REVERSED.into());
            }
            let front = card.prompt().trim();
            let flds = format!("{}\u{1f}{}", to_field(front), to_field(card.reveal()));
            conn.execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, -1, ?5, ?6, ?7, ?8, 0, '')",
                rusqlite::params![
                    id,
                    card.id,
                    MODEL_ID,
                    now,
                    format!(" {} ", tags.join(" ")),
                    flds,
                    front,
                    checksum(front),
                ],
            )?;
            // The grades, as Anki's review log, and the schedule they give.
            let mut reviews: Vec<&Review> = card.reviews.iter().collect();
            reviews.sort_by_key(|r| r.ts);
            let mut s = Schedule::default();
            let mut lapses = 0;
            for r in &reviews {
                let last = s;
                s = s.after(r.grade, r.ts);
                if r.grade == Grade::Again && last.repetitions > 0 {
                    lapses += 1;
                }
                let mut rid = r.ts.saturating_mul(1000);
                while !revlog_ids.insert(rid) {
                    rid += 1;
                }
                let kind = if last.is_new() { 0 } else { 1 };
                conn.execute(
                    "INSERT INTO revlog VALUES (?1, ?2, -1, ?3, ?4, ?5, ?6, 0, ?7)",
                    rusqlite::params![
                        rid,
                        id,
                        ease_of(r.grade),
                        s.interval_days,
                        last.interval_days,
                        factor(s.ease),
                        kind,
                    ],
                )?;
            }
            let (kind, due, ivl, fct) = match s.due {
                None => {
                    position += 1;
                    (0, position, 0, 0)
                }
                Some(due) => (
                    2,
                    (due - crt).div_euclid(DAY_SECS),
                    i64::from(s.interval_days),
                    factor(s.ease),
                ),
            };
            conn.execute(
                "INSERT INTO cards VALUES (?1, ?2, ?3, 0, ?4, -1, ?5, ?5, ?6, ?7, ?8, ?9, ?10,
                 0, 0, 0, 0, '')",
                rusqlite::params![
                    id,
                    id,
                    did,
                    now,
                    kind,
                    due,
                    ivl,
                    fct,
                    i64::try_from(reviews.len()).unwrap_or(i64::MAX),
                    lapses,
                ],
            )?;
        }
    }

    let data = conn.serialize("main")?;
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("collection.anki2", opts)?;
        zip.write_all(&data)
            .map_err(|e| AnkiError::Zip(e.to_string()))?;
        zip.start_file("media", opts)?;
        zip.write_all(b"{}")
            .map_err(|e| AnkiError::Zip(e.to_string()))?;
        zip.finish()?;
    }
    Ok(out.into_inner())
}

fn deck_value(id: i64, name: &str, now: i64) -> serde_json::Value {
    json!({
        "id": id, "name": name, "mod": now, "usn": -1, "desc": "", "dyn": 0, "conf": 1,
        "collapsed": false, "browserCollapsed": false, "extendNew": 10, "extendRev": 50,
        "newToday": [0, 0], "revToday": [0, 0], "lrnToday": [0, 0], "timeToday": [0, 0]
    })
}

fn model_value(did: i64, now: i64) -> serde_json::Value {
    let field = |name: &str, ord: u32| {
        json!({ "name": name, "ord": ord, "sticky": false, "rtl": false,
                "font": "Arial", "size": 20, "media": [] })
    };
    json!({
        "id": MODEL_ID, "name": "Basic (textweaver)", "type": 0, "mod": now, "usn": -1,
        "sortf": 0, "did": did, "tags": [], "vers": [],
        "flds": [field("Front", 0), field("Back", 1)],
        "tmpls": [{
            "name": "Card 1", "ord": 0, "qfmt": "{{Front}}",
            "afmt": "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
            "bqfmt": "", "bafmt": "", "did": null
        }],
        "css": ".card { font-family: arial; font-size: 20px; text-align: left; }",
        "latexPre": "\\documentclass[12pt]{article}\n\\special{papersize=3in,5in}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amssymb,amsmath}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0in}\n\\begin{document}\n",
        "latexPost": "\\end{document}",
        "req": [[0, "any", [0]]]
    })
}

static CLOZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{c\d+::").expect("a valid regex"));
static SOUND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[sound:[^\]]*\]").expect("a valid regex"));
static MEDIA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<(img|audio|video|object|embed)\b").expect("a valid regex"));
static BREAK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<br\s*/?>|</(div|p|li|h[1-6]|tr|blockquote)\s*>").expect("a valid regex")
});
static DROP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<(style|script)\b.*?</(style|script)\s*>").expect("a valid regex")
});
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]*>").expect("a valid regex"));

/// True when an Anki field holds a picture or sound.
fn has_media(field: &str) -> bool {
    SOUND.is_match(field) || MEDIA.is_match(field)
}

/// An Anki field's HTML as plain text: line breaks kept, tags and sounds
/// dropped, character references decoded, blank lines collapsed.
pub fn html_to_text(html: &str) -> String {
    let s = SOUND.replace_all(html, "");
    let s = DROP.replace_all(&s, "");
    let s = BREAK.replace_all(&s, "\n");
    let s = TAG.replace_all(&s, "");
    let s = decode_entities(&s);
    let mut lines: Vec<String> = Vec::new();
    for line in s.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() && lines.last().is_none_or(String::is_empty) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}

/// The common character references, decoded (`&amp;`, `&lt;`, `&#233;`,
/// `&#xe9;`, `&nbsp;`, ...); anything else is kept as written.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest[1..].find(';').map(|e| e + 1).filter(|&e| e <= 10);
        let decoded = end.and_then(|e| {
            let name = &rest[1..e];
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => ' ',
                _ => {
                    let num = name.strip_prefix('#')?;
                    let n = match num.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => num.parse().ok()?,
                    };
                    char::from_u32(n)?
                }
            };
            Some((c, e))
        });
        match decoded {
            Some((c, e)) => {
                out.push(c);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The collection's bytes from the package: the newest form first.
fn collection(bytes: &[u8]) -> Result<Vec<u8>, AnkiError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    let read = |zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str| -> Option<Vec<u8>> {
        let mut f = zip.by_name(name).ok()?;
        let mut out = Vec::new();
        f.read_to_end(&mut out).ok()?;
        Some(out)
    };
    if let Some(packed) = read(&mut zip, "collection.anki21b") {
        let mut out = Vec::new();
        ruzstd::decoding::StreamingDecoder::new(packed.as_slice())
            .map_err(|e| AnkiError::Sqlite(e.to_string()))?
            .read_to_end(&mut out)
            .map_err(|e| AnkiError::Sqlite(e.to_string()))?;
        return Ok(out);
    }
    read(&mut zip, "collection.anki21")
        .or_else(|| read(&mut zip, "collection.anki2"))
        .ok_or(AnkiError::NoCollection)
}

/// The deck names by id: from the `decks` table (Anki 2.1.50 and later,
/// levels joined by the unit separator) or the collection's JSON.
fn deck_names(conn: &Connection) -> Result<HashMap<i64, String>, AnkiError> {
    if let Ok(mut st) = conn.prepare("SELECT id, name FROM decks") {
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = HashMap::new();
        for row in rows {
            let (id, name) = row?;
            out.insert(id, name.replace('\u{1f}', "::"));
        }
        return Ok(out);
    }
    let text: String = conn.query_row("SELECT decks FROM col", [], |r| r.get(0))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| AnkiError::Sqlite(e.to_string()))?;
    let mut out = HashMap::new();
    if let Some(map) = json.as_object() {
        for (id, deck) in map {
            if let (Ok(id), Some(name)) = (id.parse(), deck["name"].as_str()) {
                out.insert(id, name.to_owned());
            }
        }
    }
    Ok(out)
}

/// The grades in Anki's review log, by card id. Manual reschedules (types
/// 4 and 5, or ease 0) are not grades and are left out.
fn grades(conn: &Connection) -> Result<HashMap<i64, Vec<Review>>, AnkiError> {
    let mut st = conn.prepare("SELECT id, cid, ease, type FROM revlog ORDER BY id")?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    let mut out: HashMap<i64, Vec<Review>> = HashMap::new();
    for row in rows {
        let (id, cid, ease, kind) = row?;
        let grade = match ease {
            1 => Grade::Again,
            2 => Grade::Hard,
            3 => Grade::Good,
            4 => Grade::Easy,
            _ => continue,
        };
        if kind > 3 {
            continue;
        }
        out.entry(cid).or_default().push(Review {
            grade,
            ts: id.div_euclid(1000),
        });
    }
    Ok(out)
}

/// Keeps the last grade of each day (UTC), oldest first.
fn last_of_each_day(reviews: Vec<Review>) -> Vec<Review> {
    let mut by_day: BTreeMap<i64, Review> = BTreeMap::new();
    for r in reviews {
        let day = r.ts.div_euclid(DAY_SECS);
        if by_day.get(&day).is_none_or(|old| old.ts <= r.ts) {
            by_day.insert(day, r);
        }
    }
    by_day.into_values().collect()
}

/// Reads an Anki package's cards, at `now` (Unix seconds, the cards'
/// creation time).
pub fn read_apkg(bytes: &[u8], now: i64) -> Result<Package, AnkiError> {
    let mut db = collection(bytes)?;
    // A collection in write-ahead mode cannot be read from memory; its
    // header says so in bytes 18 and 19, and the rollback journal is the
    // same file format otherwise.
    if db.len() > 19 && db[18] == 2 && db[19] == 2 {
        db[18] = 1;
        db[19] = 1;
    }
    let mut conn = Connection::open_in_memory()?;
    let len = db.len();
    conn.deserialize_read_exact("main", Cursor::new(db), len, true)?;

    let decks = deck_names(&conn)?;
    let mut grades = grades(&conn)?;
    // The cards of each note: (card id, deck id, ord).
    let mut by_note: HashMap<i64, Vec<(i64, i64, i64)>> = HashMap::new();
    {
        let mut st = conn.prepare("SELECT id, nid, did, ord FROM cards ORDER BY ord")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?;
        for row in rows {
            let (id, nid, did, ord) = row?;
            by_note.entry(nid).or_default().push((id, did, ord));
        }
    }

    let mut out = Package::default();
    let mut st = conn.prepare("SELECT id, guid, tags, flds FROM notes ORDER BY id")?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (nid, guid, tags, flds) = row?;
        let fields: Vec<&str> = flds.split('\u{1f}').collect();
        if fields.iter().any(|f| CLOZE.is_match(f)) {
            out.skipped_cloze += 1;
            continue;
        }
        let media = fields.iter().any(|f| has_media(f));
        let front = fields.first().map(|f| html_to_text(f)).unwrap_or_default();
        let back = fields
            .iter()
            .skip(1)
            .map(|f| html_to_text(f))
            .filter(|f| !f.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if front.is_empty() || back.is_empty() {
            if media {
                out.skipped_media += 1;
            } else {
                out.skipped_other += 1;
            }
            continue;
        }
        let tags: Vec<&str> = tags.split_whitespace().collect();
        let ours = tags.iter().any(|t| t.starts_with(TAG_PREFIX));
        let kind = if tags.contains(&"textweaver::cloze") {
            CardKind::Cloze
        } else if tags.contains(&"textweaver::recall") {
            CardKind::Recall
        } else {
            CardKind::Question
        };
        let flipped = tags.contains(&TAG_REVERSED);
        let cards = by_note.remove(&nid).unwrap_or_default();
        let mut skipped = false;
        for (cid, did, ord) in cards {
            if ord > 1 {
                skipped = true;
                continue;
            }
            let reversed = (ord == 1) != flipped;
            // A textweaver note's front is what was asked: the answer
            // when it was reversed.
            let (question, answer) = if flipped {
                (back.clone(), front.clone())
            } else {
                (front.clone(), back.clone())
            };
            let id = if ours && ord == 0 && guid.starts_with("card-") {
                guid.clone()
            } else if ord == 0 {
                card_id(CardSource::Anki, &guid)
            } else {
                card_id(CardSource::Anki, &format!("{guid}/{ord}"))
            };
            let reviews = grades.remove(&cid).unwrap_or_default();
            let reviews = if ours {
                reviews
            } else {
                last_of_each_day(reviews)
            };
            out.cards.push(ImportedCard {
                deck: decks.get(&did).cloned().unwrap_or_default(),
                card: Card {
                    id,
                    kind,
                    source: CardSource::Anki,
                    source_id: guid.clone(),
                    range: textweaver_core::CharRange::default(),
                    question,
                    answer,
                    reversed,
                    created: now,
                    reviews,
                    extra: serde_json::Map::new(),
                },
            });
        }
        if skipped {
            out.skipped_other += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_790_000_000;

    fn card(id: &str, kind: CardKind, q: &str, a: &str, grades: &[(Grade, i64)]) -> Card {
        Card {
            id: id.into(),
            kind,
            source: CardSource::Note,
            source_id: "n".into(),
            range: textweaver_core::CharRange::default(),
            question: q.into(),
            answer: a.into(),
            reversed: false,
            created: 1,
            reviews: grades
                .iter()
                .map(|&(grade, ts)| Review { grade, ts })
                .collect(),
            extra: serde_json::Map::new(),
        }
    }

    fn sample() -> Vec<Card> {
        let mut reversed = card(
            "card-b",
            CardKind::Question,
            "Which organ filters blood?",
            "The kidney & its <nephrons>.",
            &[],
        );
        reversed.reversed = true;
        vec![
            card(
                "card-a",
                CardKind::Cloze,
                "The kidney filters blank.",
                "blood",
                &[
                    (Grade::Good, NOW - 10 * DAY_SECS),
                    (Grade::Again, NOW - 9 * DAY_SECS),
                    (Grade::Easy, NOW - 8 * DAY_SECS),
                ],
            ),
            reversed,
            card(
                "card-c",
                CardKind::Recall,
                "What does \u{201c}Renal clearance\u{201d} say?",
                "Clearance is a rate.\nIt is measured in mL/min.",
                &[(Grade::Hard, NOW - DAY_SECS)],
            ),
        ]
    }

    #[test]
    fn export_then_import_gives_the_same_cards() {
        let cards = sample();
        let bytes = export_apkg(
            &[ExportDeck {
                name: "Pharmacology 2".into(),
                document: "Pharmacology 2".into(),
                cards: &cards,
            }],
            NOW,
        )
        .unwrap();
        let pkg = read_apkg(&bytes, NOW).unwrap();
        assert_eq!(
            (pkg.skipped_cloze, pkg.skipped_media, pkg.skipped_other),
            (0, 0, 0)
        );
        assert_eq!(pkg.decks(), vec!["Pharmacology 2".to_owned()]);
        assert_eq!(pkg.cards.len(), 3);
        for (want, got) in cards.iter().zip(&pkg.cards) {
            let got = &got.card;
            assert_eq!(got.id, want.id);
            assert_eq!(got.kind, want.kind);
            assert_eq!(got.question, want.question);
            assert_eq!(got.answer, want.answer);
            assert_eq!(got.reversed, want.reversed);
            assert_eq!(got.reviews, want.reviews);
            assert_eq!(got.schedule(), want.schedule());
        }
    }

    #[test]
    fn export_carries_the_schedule_and_tags() {
        let cards = sample();
        let bytes = export_apkg(
            &[ExportDeck {
                name: "Week 4".into(),
                document: "Week 4".into(),
                cards: &cards,
            }],
            NOW,
        )
        .unwrap();
        let db = collection(&bytes).unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        let len = db.len();
        conn.deserialize_read_exact("main", Cursor::new(db), len, true)
            .unwrap();
        let crt: i64 = conn
            .query_row("SELECT crt FROM col", [], |r| r.get(0))
            .unwrap();
        let (kind, due, ivl, factor, reps, lapses): (i64, i64, i64, i64, i64, i64) = conn
            .query_row(
                "SELECT c.type, c.due, c.ivl, c.factor, c.reps, c.lapses FROM cards c
                 JOIN notes n ON n.id = c.nid WHERE n.guid = 'card-a'",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        let s = cards[0].schedule();
        assert_eq!((kind, reps, lapses), (2, 3, 1));
        assert_eq!(ivl, i64::from(s.interval_days));
        assert_eq!(factor, (s.ease * 1000.0).round() as i64);
        assert_eq!(due, (s.due.unwrap() - crt).div_euclid(DAY_SECS));
        let tags: String = conn
            .query_row("SELECT tags FROM notes WHERE guid = 'card-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            tags.split_whitespace().collect::<Vec<_>>(),
            ["Week_4", "textweaver::question", "textweaver::reversed"]
        );
        let flds: String = conn
            .query_row("SELECT flds FROM notes WHERE guid = 'card-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            flds,
            "The kidney &amp; its &lt;nephrons&gt;.\u{1f}Which organ filters blood?"
        );
        let logged: i64 = conn
            .query_row("SELECT count(*) FROM revlog", [], |r| r.get(0))
            .unwrap();
        assert_eq!(logged, 4);
    }

    /// A package as Anki writes one: a Basic note, a Basic and reversed
    /// note, a cloze note, a picture-only note, and a note with a sound
    /// beside its text, with a learning day in the review log.
    fn anki_fixture() -> Vec<u8> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        let decks = json!({
            "1": { "id": 1, "name": "Default" },
            "1500": { "id": 1500, "name": "Biology::Cells" }
        });
        conn.execute(
            "INSERT INTO col VALUES (1, 0, 0, 0, 11, 0, 0, 0, '{}', '{}', ?1, '{}', '{}')",
            [decks.to_string()],
        )
        .unwrap();
        let notes: [(i64, &str, &str); 5] = [
            (
                10,
                "abc",
                "What is the <b>powerhouse</b>?\u{1f}The mitochondrion.<br>Mostly.",
            ),
            (11, "def", "Nucleus&nbsp;role\u{1f}<div>Holds the DNA</div>"),
            (12, "ghi", "The {{c1::ribosome}} makes proteins.\u{1f}"),
            (13, "jkl", "<img src=\"cell.png\">\u{1f}[sound:cell.mp3]"),
            (14, "mno", "Membrane [sound:m.mp3]\u{1f}Lipid bilayer"),
        ];
        for (id, guid, flds) in notes {
            conn.execute(
                "INSERT INTO notes VALUES (?1, ?2, 1, 0, 0, ' biology ', ?3, '', 0, 0, '')",
                rusqlite::params![id, guid, flds],
            )
            .unwrap();
        }
        let cards: [(i64, i64, i64); 6] = [
            (100, 10, 0),
            (101, 11, 0),
            (102, 11, 1),
            (103, 12, 0),
            (104, 13, 0),
            (105, 14, 0),
        ];
        for (id, nid, ord) in cards {
            conn.execute(
                "INSERT INTO cards VALUES (?1, ?2, 1500, ?3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                 0, 0, 0, 0, '')",
                rusqlite::params![id, nid, ord],
            )
            .unwrap();
        }
        // Card 100: two learning steps on one day, then a review; and a
        // manual reschedule, which is not a grade.
        let day = NOW - 5 * DAY_SECS;
        let log: [(i64, i64, i64); 4] = [
            ((day) * 1000, 1, 0),
            ((day + 600) * 1000, 3, 0),
            ((NOW - DAY_SECS) * 1000, 4, 1),
            ((NOW - 100) * 1000, 0, 4),
        ];
        for (id, ease, kind) in log {
            conn.execute(
                "INSERT INTO revlog VALUES (?1, 100, 0, ?2, 0, 0, 0, 0, ?3)",
                rusqlite::params![id, ease, kind],
            )
            .unwrap();
        }
        let data = conn.serialize("main").unwrap();
        let mut out = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("collection.anki21", opts).unwrap();
        zip.write_all(&data).unwrap();
        zip.start_file("media", opts).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();
        out.into_inner()
    }

    #[test]
    fn an_anki_deck_imports_as_plain_text_cards() {
        let pkg = read_apkg(&anki_fixture(), NOW).unwrap();
        assert_eq!(pkg.skipped_cloze, 1);
        assert_eq!(pkg.skipped_media, 1);
        assert_eq!(pkg.skipped_other, 0);
        assert_eq!(pkg.decks(), vec!["Biology::Cells".to_owned()]);
        let cards: Vec<&Card> = pkg.cards.iter().map(|c| &c.card).collect();
        assert_eq!(cards.len(), 4);
        assert_eq!(cards[0].question, "What is the powerhouse?");
        assert_eq!(cards[0].answer, "The mitochondrion.\nMostly.");
        assert_eq!(cards[0].source, CardSource::Anki);
        assert_eq!(cards[0].id, card_id(CardSource::Anki, "abc"));
        // The learning day is one grade, the last; the reschedule is none.
        assert_eq!(
            cards[0].reviews,
            vec![
                Review {
                    grade: Grade::Good,
                    ts: NOW - 5 * DAY_SECS + 600,
                },
                Review {
                    grade: Grade::Easy,
                    ts: NOW - DAY_SECS,
                },
            ]
        );
        assert_eq!(cards[1].question, "Nucleus role");
        assert_eq!(cards[1].answer, "Holds the DNA");
        assert!(!cards[1].reversed);
        assert!(cards[2].reversed);
        assert_eq!(cards[2].prompt(), "Holds the DNA");
        assert_ne!(cards[1].id, cards[2].id);
        assert_eq!(cards[3].question, "Membrane");
        assert!(cards.iter().all(|c| c.kind == CardKind::Question));
    }

    #[test]
    fn importing_the_same_package_twice_adds_nothing() {
        let bytes = anki_fixture();
        let mut deck = textweaver_store::CardDeck::default();
        let first = read_apkg(&bytes, NOW).unwrap();
        let r = deck.add_imported(first.cards.into_iter().map(|c| c.card).collect());
        assert_eq!((r.added, r.already), (4, 0));
        let again = read_apkg(&bytes, NOW + 50).unwrap();
        let r = deck.add_imported(again.cards.into_iter().map(|c| c.card).collect());
        assert_eq!((r.added, r.already), (0, 4));
        assert_eq!(deck.cards.len(), 4);
        assert_eq!(deck.cards[0].reviews.len(), 2);
    }

    #[test]
    fn not_a_package_is_an_error() {
        assert!(matches!(
            read_apkg(b"plain text", NOW),
            Err(AnkiError::Zip(_))
        ));
        let mut out = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut out);
        zip.start_file("media", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.finish().unwrap();
        assert!(matches!(
            read_apkg(&out.into_inner(), NOW),
            Err(AnkiError::NoCollection)
        ));
    }

    #[test]
    fn html_becomes_plain_text() {
        assert_eq!(
            html_to_text("<div>One&amp;two</div><div><br></div><div>Three &#233;</div>"),
            "One&two\n\nThree \u{e9}"
        );
        assert_eq!(html_to_text("<style>.x{}</style>A&nbsp; b"), "A b");
        assert_eq!(document_tag(" Week  4: Renal "), "Week_4:_Renal");
    }
}
