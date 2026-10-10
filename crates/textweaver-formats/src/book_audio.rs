//! The recorded audio of a DAISY book, phrase by phrase.
//!
//! DAISY 2.02 and DAISY 3 books with audio pair each phrase of the text
//! with clips of the recording in their SMIL files: a `par` holds a `text`
//! element pointing at the phrase and one or more `audio` elements, each a
//! clip of an MP3 or WAV file from `clipBegin` (DAISY 3) or `clip-begin`
//! (DAISY 2.02) to its end. [`book_audio`] walks the book again, as the
//! loader does, noting where each element with an id lands in the text,
//! and returns each phrase's range with its clips, so a reader can play the
//! recording with the highlight following it.
//!
//! - **Phrases** run from where their text starts to where the next phrase
//!   with text starts. Audio with no text of its own (a page number said
//!   aloud, an `audio` outside any `par`) joins the phrase before it, so
//!   nothing in the recording is skipped and nothing plays out of order.
//! - **Text without audio** has no phrase: a reader speaks it instead.
//! - **Books with no text** (the loader reads their headings) get one
//!   phrase per heading, holding every clip from the heading's place in
//!   the recording to the next heading's.
//! - **Clip times** accept SMIL clock values (`0:01:02.5`, `1:02.5`),
//!   time counts (`62.5s`, `62500ms`, `1.5min`, `1h`) and DAISY 2.02's
//!   `npt=` prefix. A missing end plays to the end of the file.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use textweaver_core::{CharPos, CharRange};
use textweaver_text::Document;

use crate::daisy::Collect;
use crate::package::{dir_of, parse_xml, resolve};
use crate::{LoadError, LoadOptions, Source};

/// One clip of the recording.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioClip {
    /// The audio file: on disk, or an archive member (`book.zip!a.mp3`),
    /// which [`crate::archive::read_path`] reads.
    pub file: PathBuf,
    /// Where the clip starts in the file.
    pub begin: Duration,
    /// Where it ends; `None` plays to the end of the file.
    pub end: Option<Duration>,
}

/// One phrase of the text and the clips that say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioPar {
    /// The phrase in the document's text.
    pub range: CharRange,
    /// Its clips, in order.
    pub clips: Vec<AudioClip>,
}

/// A DAISY book's recorded audio.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookAudio {
    /// The phrases with audio, in order, not overlapping.
    pub pars: Vec<AudioPar>,
    /// False for a book with no text, only headings and audio.
    pub has_text: bool,
}

/// The recorded audio of the DAISY book at `path` (its package file, its
/// `ncc.html`, or the archive holding it), loaded with `options` as the
/// document was, so the ranges match its text. `None` when it is not a
/// DAISY book or has no audio.
///
/// shortcut: the book is walked a second time (as long as opening it
/// took); keep the phrases from the first load if that ever shows.
pub fn book_audio(path: &Path, options: &LoadOptions) -> Result<Option<BookAudio>, LoadError> {
    let mut collect = Collect::default();
    let (doc, root) = if is_archive(path) {
        match crate::archive::load_daisy_collecting(path, options, &mut collect)? {
            Some(doc) => (doc, Root::Archive(path.to_owned())),
            None => return Ok(None),
        }
    } else {
        let bytes =
            crate::archive::read_path(path).map_err(|e| LoadError::Io(path.to_owned(), e))?;
        let folder = path.parent().map(Path::to_owned).unwrap_or_default();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut read = |rel: &str| crate::daisy::read_beside(Some(&folder), rel);
        let doc = if crate::daisy::is_daisy_package(&bytes) {
            crate::daisy::load_package(&name, &bytes, &mut read, options, Some(&mut collect))?
        } else if crate::daisy2::is_ncc(&Source::Path(path.to_owned()), &bytes) {
            crate::daisy2::load_ncc(&name, &bytes, &mut read, options, Some(&mut collect))?
        } else {
            return Ok(None);
        };
        (doc, Root::Folder(folder))
    };
    let audio = phrases(&collect, &root, &doc);
    Ok((!audio.pars.is_empty()).then_some(audio))
}

/// True when `path` is an archive on disk (by its first bytes).
fn is_archive(path: &Path) -> bool {
    let mut head = [0u8; 512];
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let n = f.read(&mut head).unwrap_or(0);
    crate::archive::ArchiveKind::sniff(&head[..n]).is_some()
}

/// Where a book's files are.
enum Root {
    /// A folder on disk (or inside an archive, written `a.zip!folder`).
    Folder(PathBuf),
    /// The root of an archive.
    Archive(PathBuf),
}

impl Root {
    fn path(&self, rel: &str) -> PathBuf {
        match self {
            Root::Folder(f) => f.join(rel),
            Root::Archive(a) => crate::archive::member_path(a, rel),
        }
    }
}

/// One `par` (or a lone `audio` or `text`) of a SMIL file.
#[derive(Debug, Default)]
struct Unit {
    /// Which SMIL file, by index.
    smil: usize,
    /// The ids of the element and everything in it.
    ids: Vec<String>,
    /// The text it shows.
    text: Option<(String, Option<String>)>,
    clips: Vec<AudioClip>,
}

/// The units of every SMIL file, in reading order.
fn units(collect: &Collect, root: &Root) -> Vec<Unit> {
    let mut out = Vec::new();
    for (i, (path, text)) in collect.smils.iter().enumerate() {
        let Ok(xml) = parse_xml(text) else { continue };
        let dir = dir_of(path);
        visit(xml.root_element(), i, dir, root, 0, &mut out);
    }
    out
}

fn is(n: roxmltree::Node<'_, '_>, name: &str) -> bool {
    n.is_element() && n.tag_name().name().eq_ignore_ascii_case(name)
}

fn visit(
    node: roxmltree::Node<'_, '_>,
    smil: usize,
    dir: &str,
    root: &Root,
    depth: usize,
    out: &mut Vec<Unit>,
) {
    if depth > crate::MAX_NESTING {
        return;
    }
    if is(node, "par") || is(node, "audio") || is(node, "text") {
        let all = || std::iter::once(node).chain(node.descendants());
        out.push(Unit {
            smil,
            ids: all()
                .filter_map(|n| n.attribute("id"))
                .map(str::to_owned)
                .collect(),
            text: all()
                .find(|n| is(*n, "text"))
                .and_then(|t| t.attribute("src"))
                .map(|src| resolve(dir, src)),
            clips: all()
                .filter(|n| is(*n, "audio"))
                .filter_map(|a| clip(a, dir, root))
                .collect(),
        });
        return;
    }
    for c in node.children().filter(roxmltree::Node::is_element) {
        visit(c, smil, dir, root, depth + 1, out);
    }
}

/// An `audio` element as a clip.
fn clip(a: roxmltree::Node<'_, '_>, dir: &str, root: &Root) -> Option<AudioClip> {
    let src = a.attribute("src")?;
    let attr = |names: [&str; 2]| names.iter().find_map(|n| a.attribute(*n));
    let begin = attr(["clipBegin", "clip-begin"])
        .and_then(clock)
        .unwrap_or_default();
    let end = attr(["clipEnd", "clip-end"]).and_then(clock);
    Some(AudioClip {
        file: root.path(&resolve(dir, src).0),
        begin,
        end,
    })
}

/// A SMIL clip time: a clock value (`h:mm:ss.f`, `mm:ss.f`) or a time
/// count (`12.5s`, `250ms`, `1.5min`, `2h`, or bare seconds), with or
/// without DAISY 2.02's `npt=`. Exact to the microsecond.
pub(crate) fn clock(s: &str) -> Option<Duration> {
    let s = s.trim();
    let s = s.strip_prefix("npt=").unwrap_or(s).trim();
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        let (h, m, sec) = match parts.as_slice() {
            [h, m, sec] => (h.trim().parse::<u64>().ok()?, *m, *sec),
            [m, sec] => (0, *m, *sec),
            _ => return None,
        };
        let m = m.trim().parse::<u64>().ok()?;
        let micros = h
            .checked_mul(3600)?
            .checked_add(m.checked_mul(60)?)?
            .checked_mul(1_000_000)?
            .checked_add(decimal_micros(sec)?)?;
        return Some(Duration::from_micros(micros));
    }
    // The unit in microseconds.
    let (num, scale) = if let Some(n) = s.strip_suffix("ms") {
        (n, 1_000)
    } else if let Some(n) = s.strip_suffix("min") {
        (n, 60_000_000)
    } else if let Some(n) = s.strip_suffix('h') {
        (n, 3_600_000_000)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1_000_000)
    } else {
        (s, 1_000_000)
    };
    let micros = decimal_micros(num)?;
    // `decimal_micros` reads seconds; rescale to the unit.
    let micros = u128::from(micros) * scale / 1_000_000;
    u64::try_from(micros).ok().map(Duration::from_micros)
}

/// `"12.345"` as 12,345,000 (millionths), without floating point.
fn decimal_micros(s: &str) -> Option<u64> {
    let s = s.trim();
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    let whole: u64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut f = 0u64;
    for (i, c) in frac.chars().take(6).enumerate() {
        f += u64::from(c.to_digit(10)?) * 10u64.pow(5 - u32::try_from(i).ok()?);
    }
    whole.checked_mul(1_000_000)?.checked_add(f)
}

/// The phrases: each unit whose text was found starts one, and every unit
/// after it up to the next such unit adds its clips. A book with no text
/// is anchored by its headings instead.
fn phrases(collect: &Collect, root: &Root, doc: &Document) -> BookAudio {
    let units = units(collect, root);
    let mut anchors: Vec<(usize, usize)> = units
        .iter()
        .enumerate()
        .filter_map(|(i, u)| {
            let (file, Some(frag)) = u.text.as_ref()? else {
                return None;
            };
            let at = collect.positions.get(&(file.clone(), frag.clone()))?;
            Some((i, *at))
        })
        .collect();
    let has_text = !anchors.is_empty();
    if !has_text {
        let smil_index: HashMap<&str, usize> = collect
            .smils
            .iter()
            .enumerate()
            .map(|(i, (p, _))| (p.as_str(), i))
            .collect();
        for (key, (file, frag)) in &collect.nav {
            let (Some(&at), Some(&smil)) =
                (collect.positions.get(key), smil_index.get(file.as_str()))
            else {
                continue;
            };
            let in_file = || units.iter().enumerate().filter(|(_, u)| u.smil == smil);
            let found = frag
                .as_ref()
                .and_then(|f| in_file().find(|(_, u)| u.ids.contains(f)))
                .or_else(|| in_file().next());
            if let Some((i, _)) = found {
                anchors.push((i, at));
            }
        }
        anchors.sort_by_key(|&(i, _)| i);
    }
    // Text that goes backward (a SMIL out of step with the text) cannot
    // be followed: its audio joins the phrase before.
    // Where two start at one place (a page number, which is not read,
    // before the next paragraph), the later one starts the phrase and the
    // earlier one's audio joins the phrase before.
    let mut kept: Vec<(usize, usize)> = Vec::with_capacity(anchors.len());
    for a in anchors {
        match kept.last().copied() {
            Some(l) if a.0 <= l.0 || a.1 < l.1 => {}
            Some(l) if a.1 == l.1 => {
                if let Some(last) = kept.last_mut() {
                    *last = a;
                }
            }
            _ => kept.push(a),
        }
    }
    let len = doc.len_chars();
    let mut pars = Vec::new();
    for (k, &(unit, at)) in kept.iter().enumerate() {
        let from = if k == 0 { 0 } else { unit };
        let (to, end) = kept.get(k + 1).map_or((units.len(), len), |&(u, a)| (u, a));
        let clips: Vec<AudioClip> = units[from..to]
            .iter()
            .flat_map(|u| u.clips.iter().cloned())
            .collect();
        if !clips.is_empty() && end > at {
            pars.push(AudioPar {
                range: CharRange::new(CharPos(at), CharPos(end.min(len))),
                clips,
            });
        }
    }
    BookAudio { pars, has_text }
}

#[cfg(test)]
mod tests;
