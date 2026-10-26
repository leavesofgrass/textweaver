//! The values view's model: one line per field, meaning first
//! ("Temperature: 21.4 C"), each updated in place as messages arrive, for
//! sensor-style sources.
//!
//! - **Order.** Mapped fields (`[[live.sources.<id>.fields]]`) come first,
//!   by their `order`, else as listed; the other fields follow in the
//!   order the messages sent them (the key order is read from the
//!   message's text, [`super::KeyOrder`]). A field keeps its place once
//!   shown: a field new to a later message goes after the ones before it.
//! - **Nothing vanishes.** A field missing from a later message on its
//!   topic reads "no value"; a field not updated for the stale time reads
//!   "21.4 C, no recent value".
//! - **Shape.** A nested object is a group under a heading ("Battery",
//!   then "Voltage: 12.4"); a list of plain values is one line per item
//!   ("Readings 1: 3"), up to [`LIST_LIMIT`], then a line saying how many
//!   more; a list of objects is a group per item, titled by its name or
//!   id. A message that is not an object (`21.4`) is one field named after
//!   the topic's last level.
//! - **Topics.** With more than one topic, each topic's fields are a group
//!   under a heading naming the topic.
//!
//! Every change of a line's value is kept as a [`FieldChange`] for the
//! announcements to read ([`LiveValues::take_changes`]); the model itself
//! announces nothing.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use textweaver_store::{LiveField, LiveNotice, field_path_segments};

use super::log::{clean, escape};
use super::order::{KeyOrder, child};

/// The most items of one list shown; the rest are counted on one line.
pub const LIST_LIMIT: usize = 20;

/// The most fields a values view keeps, over every topic.
pub const MAX_VALUES: usize = 2_000;

/// The most changes kept until they are taken.
const MAX_CHANGES: usize = 500;

/// Keys that title a list item's group, first found first.
const TITLE_NAMES: &[&str] = &["name", "label", "title", "id"];

/// What a values view is opened with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValuesOptions {
    /// The title (the source's name).
    pub title: String,
    /// The source's field mapping.
    pub fields: Vec<LiveField>,
    /// A value not updated for this long reads "no recent value"; `None`
    /// never.
    pub stale_after: Option<Duration>,
    /// The fewest time between two announcements of one field.
    pub min_interval: Duration,
}

/// The words a values view is rendered with, in the interface's language
/// ([`App::live_value_labels`](crate::App::live_value_labels)).
pub struct ValueLabels {
    /// Shown before the first message.
    pub empty: String,
    /// A field with no value.
    pub no_value: String,
    /// Added after a value not updated for the stale time.
    pub no_recent: String,
    /// The line counting a list's items past [`LIST_LIMIT`]: "12 more".
    pub more: Box<dyn Fn(usize) -> String>,
}

impl Default for ValueLabels {
    /// The English words (tests).
    fn default() -> Self {
        ValueLabels {
            empty: "No values yet.".into(),
            no_value: "no value".into(),
            no_recent: "no recent value".into(),
            more: Box::new(|n| format!("{n} more")),
        }
    }
}

/// One field's value changed (or appeared, or went), for the
/// announcements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldChange {
    /// The topic it came on.
    pub topic: String,
    /// Where it is in the message, as a JSON Pointer (`/battery/voltage`);
    /// the same path [`App::live_set_field_notice`](crate::App::live_set_field_notice)
    /// takes.
    pub path: String,
    /// Its label ("Temperature").
    pub label: String,
    /// The value before, as shown ("21.4 C"); `None` for no value.
    pub old: Option<String>,
    /// The value now; `None` for no value.
    pub new: Option<String>,
    /// The whole line, meaning first ("Temperature: 21.6 C").
    pub line: String,
    /// The reader wants this field's changes shown (`show = "on_change"`).
    pub show: bool,
    /// The reader wants this field's changes spoken in textweaver's own
    /// voice (`speak = "on_change"`).
    pub speak: bool,
    /// The fewest time between two announcements of this field.
    pub min_interval: Duration,
}

/// A field as the cursor finds it, for commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueField {
    /// The topic it came on.
    pub topic: String,
    /// Its JSON Pointer in the message.
    pub path: String,
    /// Its label.
    pub label: String,
    /// Whether its changes are shown.
    pub show: LiveNotice,
    /// Whether its changes are spoken.
    pub speak: LiveNotice,
}

/// One line.
#[derive(Clone, Debug)]
struct Slot {
    /// Its JSON Pointer in the topic's messages.
    path: String,
    label: String,
    /// The value as shown; `None` for no value.
    text: Option<String>,
    /// When a message last carried it.
    at: Instant,
    stale: bool,
}

/// A heading and what is under it.
#[derive(Clone, Debug, Default)]
struct Node {
    lines: Vec<Slot>,
    groups: Vec<Group>,
}

#[derive(Clone, Debug)]
struct Group {
    path: String,
    title: String,
    node: Node,
}

/// One topic's fields.
#[derive(Clone, Debug)]
struct Topic {
    name: String,
    /// The mapped fields, in their order.
    mapped: Vec<(usize, Slot)>,
    /// The others, as the messages sent them.
    root: Node,
}

/// The values view's fields.
pub struct LiveValues {
    options: ValuesOptions,
    topics: Vec<Topic>,
    changes: Vec<FieldChange>,
    /// The same changes, for the announcement throttle
    /// (crate::live::announce), which takes them on its own, so a
    /// frontend reading [`take_changes`](Self::take_changes) misses none.
    announce: Vec<FieldChange>,
    /// The topic and path of the field changed last.
    newest: Option<(String, String)>,
    count: usize,
    /// Fields left out at [`MAX_VALUES`].
    pub dropped: usize,
}

impl std::fmt::Debug for LiveValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveValues")
            .field("title", &self.options.title)
            .field("topics", &self.topics.len())
            .field("fields", &self.count)
            .finish_non_exhaustive()
    }
}

/// A message being applied: what it carried and changed.
struct Ctx<'a> {
    now: Instant,
    labels: &'a ValueLabels,
    seen: HashSet<String>,
    changed: Vec<(String, Option<String>, Option<String>)>,
    room: usize,
    dropped: usize,
    /// A line changed without its value changing (it was stale).
    redraw: bool,
}

impl Ctx<'_> {
    /// Updates `s` to `label` and `text`, noting a change.
    fn touch(&mut self, s: &mut Slot, label: String, text: Option<String>) {
        self.seen.insert(s.path.clone());
        s.label = label;
        s.at = self.now;
        self.redraw |= s.stale;
        s.stale = false;
        if s.text != text {
            self.changed
                .push((s.path.clone(), s.text.clone(), text.clone()));
            s.text = text;
        }
    }

    /// A new line, or `None` past [`MAX_VALUES`].
    fn fresh(&mut self, path: String, label: String, text: Option<String>) -> Option<Slot> {
        if self.room == 0 {
            self.dropped += 1;
            return None;
        }
        self.room -= 1;
        self.seen.insert(path.clone());
        self.changed.push((path.clone(), None, text.clone()));
        Some(Slot {
            path,
            label,
            text,
            at: self.now,
            stale: false,
        })
    }

    /// Sets the line at `path` in `lines` to `text`, adding it at the end
    /// when new.
    fn put(&mut self, lines: &mut Vec<Slot>, path: String, label: String, text: Option<String>) {
        if let Some(s) = lines.iter_mut().find(|s| s.path == path) {
            self.touch(s, label, text);
        } else if let Some(s) = self.fresh(path, label, text) {
            lines.push(s);
        }
    }
}

/// A key as a label: underscores, hyphens and dots as spaces, the first
/// letter capitalized ("temp_c" reads "Temp c").
pub fn auto_label(key: &str) -> String {
    let words: Vec<&str> = key
        .split(['_', '-', '.', ' '])
        .filter(|w| !w.is_empty())
        .collect();
    let joined = clean(&words.join(" "));
    let mut chars = joined.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => clean(key),
    }
}

/// A mapping path as a JSON Pointer; `None` when it is neither dotted nor
/// a pointer.
pub fn path_pointer(path: &str) -> Option<String> {
    let segs = field_path_segments(path)?;
    Some(segs.iter().fold(String::new(), |at, k| child(&at, k)))
}

/// A JSON Pointer as the path a mapping stores: dotted when every key is
/// plain (`battery.voltage`), else the pointer itself.
pub fn pointer_path(pointer: &str) -> String {
    let segs: Vec<String> = pointer
        .split('/')
        .skip(1)
        .map(|k| k.replace("~1", "/").replace("~0", "~"))
        .collect();
    let plain = !segs.is_empty()
        && segs
            .iter()
            .all(|k| !k.is_empty() && !k.contains(['.', '/', '~']) && k.trim() == k);
    if plain {
        segs.join(".")
    } else {
        pointer.to_owned()
    }
}

/// The value at `pointer` in `value`.
fn at_pointer<'a>(value: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(value);
    }
    value.pointer(pointer)
}

/// A plain value as shown with no mapping: strings cleaned, numbers as
/// sent, true and false as words; `None` for null.
fn plain_text(v: &Value, order: &KeyOrder, at: &str) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(clean(s)),
        Value::Object(_) | Value::Array(_) => Some(clean(&order.compact(at, v))),
        other => Some(other.to_string()),
    }
}

/// A mapped value as shown: the value map's words, else the number with
/// the field's decimals, then its unit.
fn mapped_text(v: &Value, f: &LiveField, order: &KeyOrder, at: &str) -> Option<String> {
    let raw = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => plain_text(v, order, at)?,
    };
    if let Some(words) = f.values.get(&raw) {
        return Some(clean(words));
    }
    let text = match (v, f.decimals) {
        (Value::Number(n), Some(d)) => match n.as_f64() {
            Some(x) => format!("{x:.prec$}", prec = usize::from(d)),
            None => n.to_string(),
        },
        _ => plain_text(v, order, at)?,
    };
    let unit = clean(&f.unit);
    Some(if unit.is_empty() {
        text
    } else {
        format!("{text} {unit}")
    })
}

/// True for a mapping that only sets `show` and `speak`: it chooses the
/// field's announcements without moving it into the mapped fields.
fn notice_only(f: &LiveField) -> bool {
    f.label.trim().is_empty()
        && f.unit.trim().is_empty()
        && f.decimals.is_none()
        && f.values.is_empty()
        && f.order.is_none()
}

impl LiveValues {
    /// An empty values view.
    pub fn new(options: ValuesOptions) -> Self {
        LiveValues {
            options,
            topics: Vec::new(),
            changes: Vec::new(),
            announce: Vec::new(),
            newest: None,
            count: 0,
            dropped: 0,
        }
    }

    /// The options it was opened with, its mapping kept current.
    pub fn options(&self) -> &ValuesOptions {
        &self.options
    }

    /// How many fields it shows.
    pub fn len(&self) -> usize {
        self.count
    }

    /// True before the first field.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The mapped fields that place a field (not only its announcements),
    /// as (index, pointer), in their display order.
    fn placing(&self) -> Vec<(usize, String)> {
        let mut m: Vec<(usize, String)> = self
            .options
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| !notice_only(f))
            .filter_map(|(i, f)| Some((i, path_pointer(&f.path)?)))
            .collect();
        m.sort_by_key(|(i, _)| (self.options.fields[*i].order.unwrap_or(u32::MAX), *i));
        m
    }

    /// The mapping entry for the field at `pointer`, if any.
    fn mapping(&self, pointer: &str) -> Option<&LiveField> {
        self.options
            .fields
            .iter()
            .find(|f| path_pointer(&f.path).as_deref() == Some(pointer))
    }

    /// Applies a message on `topic`: its fields are updated in place, new
    /// ones added at their place, and missing ones read "no value".
    /// Returns true when anything shown changed. The changes are kept for
    /// [`take_changes`](Self::take_changes).
    pub fn apply(
        &mut self,
        topic: &str,
        value: &Value,
        order: &KeyOrder,
        now: Instant,
        labels: &ValueLabels,
    ) -> bool {
        let placing = self.placing();
        let ti = match self.topics.iter().position(|t| t.name == topic) {
            Some(i) => i,
            None => {
                self.topics.push(Topic {
                    name: topic.to_owned(),
                    mapped: Vec::new(),
                    root: Node::default(),
                });
                self.topics.len() - 1
            }
        };
        let mut ctx = Ctx {
            now,
            labels,
            seen: HashSet::new(),
            changed: Vec::new(),
            room: MAX_VALUES.saturating_sub(self.count),
            dropped: 0,
            redraw: false,
        };
        let skip: HashSet<String> = placing.iter().map(|(_, p)| p.clone()).collect();
        // Mapped fields, at their place.
        let mut mapped = std::mem::take(&mut self.topics[ti].mapped);
        for (rank, (i, pointer)) in placing.iter().enumerate() {
            let Some(v) = at_pointer(value, pointer) else {
                continue;
            };
            let f = &self.options.fields[*i];
            let text = mapped_text(v, f, order, pointer);
            let label = if f.label.trim().is_empty() {
                auto_label(pointer.rsplit('/').next().unwrap_or_default())
            } else {
                clean(&f.label)
            };
            if let Some((_, s)) = mapped.iter_mut().find(|(_, s)| s.path == *pointer) {
                ctx.touch(s, label, text);
            } else if let Some(s) = ctx.fresh(pointer.clone(), label, text) {
                // At its rank: before the first with a later one.
                let at = mapped
                    .iter()
                    .position(|(r, _)| *r > rank)
                    .unwrap_or(mapped.len());
                mapped.insert(at, (rank, s));
            }
        }
        self.topics[ti].mapped = mapped;
        // The rest, as sent.
        let leaf = topic.rsplit('/').next().unwrap_or(topic);
        let mut root = std::mem::take(&mut self.topics[ti].root);
        match value {
            Value::Object(_) => walk(&mut root, value, "", order, &skip, &mut ctx),
            Value::Array(items) => list(
                &mut root,
                items,
                "",
                &auto_label(leaf),
                order,
                &skip,
                &mut ctx,
            ),
            scalar if !skip.contains("") => {
                let text = plain_text(scalar, order, "");
                ctx.put(&mut root.lines, String::new(), auto_label(leaf), text);
            }
            _ => {}
        }
        // What this message did not carry reads "no value".
        let seen = std::mem::take(&mut ctx.seen);
        let mut gone = Vec::new();
        for (_, s) in &mut self.topics[ti].mapped {
            if !seen.contains(&s.path) && s.text.is_some() {
                gone.push((s.path.clone(), s.text.take(), None));
                s.stale = false;
            }
        }
        clear_unseen(&mut root, &seen, &mut gone);
        self.topics[ti].root = root;
        ctx.changed.extend(gone);
        let Ctx {
            changed,
            dropped,
            redraw,
            ..
        } = ctx;
        self.dropped += dropped;
        self.count = self.topics.iter().map(Topic::len).sum();
        let any = redraw || !changed.is_empty();
        for (path, old, new) in changed {
            self.note_change(ti, &path, old, new, labels);
        }
        any
    }

    fn note_change(
        &mut self,
        ti: usize,
        path: &str,
        old: Option<String>,
        new: Option<String>,
        labels: &ValueLabels,
    ) {
        let topic = self.topics[ti].name.clone();
        let Some(slot) = self.topics[ti].find(path) else {
            return;
        };
        let label = slot.label.clone();
        let line = line_text(slot, labels);
        let (show, speak) = self.notices(path);
        self.newest = Some((topic.clone(), path.to_owned()));
        let change = FieldChange {
            topic,
            path: path.to_owned(),
            label,
            old,
            new,
            line,
            show: show == LiveNotice::OnChange,
            speak: speak == LiveNotice::OnChange,
            min_interval: self.options.min_interval,
        };
        for queue in [&mut self.changes, &mut self.announce] {
            if queue.len() >= MAX_CHANGES {
                queue.remove(0);
            }
            queue.push(change.clone());
        }
    }

    /// The field at `path`'s choices: its mapping's, else the defaults
    /// (shown, not spoken).
    fn notices(&self, path: &str) -> (LiveNotice, LiveNotice) {
        match self.mapping(path) {
            Some(f) => (f.show, f.speak),
            None => {
                let d = LiveField::default();
                (d.show, d.speak)
            }
        }
    }

    /// The changes since they were last taken, oldest first.
    pub fn take_changes(&mut self) -> Vec<FieldChange> {
        std::mem::take(&mut self.changes)
    }

    /// The changes since the announcements last took them, oldest first.
    pub(crate) fn take_announcements(&mut self) -> Vec<FieldChange> {
        std::mem::take(&mut self.announce)
    }

    /// Marks values not updated for the stale time; true when a line
    /// changed.
    pub fn refresh_stale(&mut self, now: Instant) -> bool {
        let Some(after) = self.options.stale_after else {
            return false;
        };
        let mut changed = false;
        for t in &mut self.topics {
            t.each_mut(&mut |s| {
                let stale = s.text.is_some() && now.saturating_duration_since(s.at) >= after;
                if s.stale != stale {
                    s.stale = stale;
                    changed = true;
                }
            });
        }
        changed
    }

    /// Sets the show and speak choices of the field at `path` (a JSON
    /// Pointer) in the mapping, adding an entry that only chooses them
    /// when the field has none. The field stays where it is. Returns the
    /// mapping, to be saved.
    pub fn set_notice(
        &mut self,
        path: &str,
        show: Option<LiveNotice>,
        speak: Option<LiveNotice>,
    ) -> &[LiveField] {
        let i = match self
            .options
            .fields
            .iter()
            .position(|f| path_pointer(&f.path).as_deref() == Some(path))
        {
            Some(i) => i,
            None => {
                self.options.fields.push(LiveField {
                    path: pointer_path(path),
                    ..LiveField::default()
                });
                self.options.fields.len() - 1
            }
        };
        let f = &mut self.options.fields[i];
        if let Some(s) = show {
            f.show = s;
        }
        if let Some(s) = speak {
            f.speak = s;
        }
        &self.options.fields
    }

    /// The fields in the order shown, each with its topic.
    fn shown(&self) -> Vec<(&Topic, &Slot)> {
        let mut out = Vec::with_capacity(self.count);
        for t in &self.topics {
            for (_, s) in &t.mapped {
                out.push((t, s));
            }
            t.root.each(&mut |s| out.push((t, s)));
        }
        out
    }

    /// The field on the `n`th line (counting fields only, from 0).
    pub fn field(&self, n: usize) -> Option<ValueField> {
        let (t, s) = *self.shown().get(n)?;
        let (show, speak) = self.notices(&s.path);
        Some(ValueField {
            topic: t.name.clone(),
            path: s.path.clone(),
            label: s.label.clone(),
            show,
            speak,
        })
    }

    /// The first field at `path` (a JSON Pointer), on any topic.
    pub fn field_by_path(&self, path: &str) -> Option<ValueField> {
        let n = self.shown().iter().position(|(_, s)| s.path == path)?;
        self.field(n)
    }

    /// The line number (counting fields only) of the field changed last,
    /// and its line.
    pub fn newest(&self, labels: &ValueLabels) -> Option<(usize, String)> {
        let (topic, path) = self.newest.as_ref()?;
        self.shown()
            .iter()
            .enumerate()
            .find(|(_, (t, s))| t.name == *topic && s.path == *path)
            .map(|(i, (_, s))| (i, line_text(s, labels)))
    }

    /// The view as Markdown: the title, then each field as a list item,
    /// under its groups' headings, and under its topic's heading when
    /// there is more than one topic.
    pub fn render(&self, labels: &ValueLabels) -> String {
        let mut md = format!("# {}\n\n", escape(&clean(&self.options.title)));
        if self.count == 0 {
            md.push_str(&escape(&labels.empty));
            md.push('\n');
            return md;
        }
        let many = self.topics.len() > 1;
        let base = if many { 3 } else { 2 };
        for t in &self.topics {
            if t.len() == 0 {
                continue;
            }
            if many {
                md.push_str(&format!("## {}\n\n", escape(&clean(&t.name))));
            }
            let mut lines: Vec<&Slot> = t.mapped.iter().map(|(_, s)| s).collect();
            lines.extend(&t.root.lines);
            items(&mut md, &lines, labels);
            for g in &t.root.groups {
                group(&mut md, g, base, labels);
            }
        }
        md
    }
}

/// The lines as list items, then a blank line.
fn items(md: &mut String, lines: &[&Slot], labels: &ValueLabels) {
    if lines.is_empty() {
        return;
    }
    for s in lines {
        md.push_str("- ");
        md.push_str(&escape(&line_text(s, labels)));
        md.push('\n');
    }
    md.push('\n');
}

fn group(md: &mut String, g: &Group, level: usize, labels: &ValueLabels) {
    if g.node.len() == 0 {
        return;
    }
    md.push_str(&"#".repeat(level.min(6)));
    md.push(' ');
    md.push_str(&escape(&g.title));
    md.push_str("\n\n");
    let lines: Vec<&Slot> = g.node.lines.iter().collect();
    items(md, &lines, labels);
    for sub in &g.node.groups {
        group(md, sub, level + 1, labels);
    }
}

/// A field's line, meaning first: "Temperature: 21.4 C", "Door: no
/// value", "Temperature: 21.4 C, no recent value".
fn line_text(s: &Slot, labels: &ValueLabels) -> String {
    match &s.text {
        Some(t) if s.stale => format!("{}: {t}, {}", s.label, labels.no_recent),
        Some(t) => format!("{}: {t}", s.label),
        None => format!("{}: {}", s.label, labels.no_value),
    }
}

impl Node {
    fn len(&self) -> usize {
        self.lines.len() + self.groups.iter().map(|g| g.node.len()).sum::<usize>()
    }

    fn each<'a>(&'a self, f: &mut dyn FnMut(&'a Slot)) {
        for s in &self.lines {
            f(s);
        }
        for g in &self.groups {
            g.node.each(f);
        }
    }

    fn each_mut(&mut self, f: &mut dyn FnMut(&mut Slot)) {
        for s in &mut self.lines {
            f(s);
        }
        for g in &mut self.groups {
            g.node.each_mut(f);
        }
    }

    fn find(&self, path: &str) -> Option<&Slot> {
        self.lines
            .iter()
            .find(|s| s.path == path)
            .or_else(|| self.groups.iter().find_map(|g| g.node.find(path)))
    }

    /// The group at `path`, added at the end when new, titled `title`.
    fn group(&mut self, path: &str, title: String) -> &mut Node {
        let i = match self.groups.iter().position(|g| g.path == path) {
            Some(i) => {
                self.groups[i].title = title;
                i
            }
            None => {
                self.groups.push(Group {
                    path: path.to_owned(),
                    title,
                    node: Node::default(),
                });
                self.groups.len() - 1
            }
        };
        &mut self.groups[i].node
    }
}

impl Topic {
    fn len(&self) -> usize {
        self.mapped.len() + self.root.len()
    }

    fn find(&self, path: &str) -> Option<&Slot> {
        self.mapped
            .iter()
            .map(|(_, s)| s)
            .find(|s| s.path == path)
            .or_else(|| self.root.find(path))
    }

    fn each_mut(&mut self, f: &mut dyn FnMut(&mut Slot)) {
        for (_, s) in &mut self.mapped {
            f(s);
        }
        self.root.each_mut(f);
    }
}

/// Every line not in `seen` reads "no value"; its change is noted.
fn clear_unseen(
    node: &mut Node,
    seen: &HashSet<String>,
    gone: &mut Vec<(String, Option<String>, Option<String>)>,
) {
    for s in &mut node.lines {
        if !seen.contains(&s.path) && s.text.is_some() {
            gone.push((s.path.clone(), s.text.take(), None));
            s.stale = false;
        }
    }
    for g in &mut node.groups {
        clear_unseen(&mut g.node, seen, gone);
    }
}

/// An object's members into `node`, in message order: plain values as
/// lines, objects as groups, lists as [`list`] shows them.
fn walk(
    node: &mut Node,
    value: &Value,
    at: &str,
    order: &KeyOrder,
    skip: &HashSet<String>,
    ctx: &mut Ctx<'_>,
) {
    let Value::Object(map) = value else {
        return;
    };
    for (k, v) in order.members(at, map) {
        let path = child(at, k);
        if skip.contains(&path) {
            continue;
        }
        let label = auto_label(k);
        match v {
            Value::Object(_) => {
                let sub = node.group(&path, label);
                walk(sub, v, &path, order, skip, ctx);
            }
            Value::Array(items) => list(node, items, &path, &label, order, skip, ctx),
            _ => {
                let text = plain_text(v, order, &path);
                ctx.put(&mut node.lines, path, label, text);
            }
        }
    }
}

/// A list's items: plain values as numbered lines ("Readings 1: 3"),
/// objects as a group each, titled by a name or id, else numbered; at
/// most [`LIST_LIMIT`], then a line counting the rest.
fn list(
    node: &mut Node,
    items: &[Value],
    at: &str,
    label: &str,
    order: &KeyOrder,
    skip: &HashSet<String>,
    ctx: &mut Ctx<'_>,
) {
    for (i, v) in items.iter().enumerate().take(LIST_LIMIT) {
        let path = format!("{at}/{i}");
        let numbered = format!("{label} {}", i + 1);
        match v {
            Value::Object(map) => {
                let title = title_of(map).unwrap_or(numbered);
                let sub = node.group(&path, title);
                walk(sub, v, &path, order, skip, ctx);
            }
            Value::Array(inner) => {
                let sub = node.group(&path, numbered.clone());
                list(sub, inner, &path, &numbered, order, skip, ctx);
            }
            _ => {
                let text = plain_text(v, order, &path);
                ctx.put(&mut node.lines, path, numbered, text);
            }
        }
    }
    if items.len() > LIST_LIMIT {
        let more = (ctx.labels.more)(items.len() - LIST_LIMIT);
        ctx.put(
            &mut node.lines,
            format!("{at}/more"),
            label.to_owned(),
            Some(more),
        );
    }
}

/// A list item's title: its name, label, title or id, when one is plain.
fn title_of(map: &Map<String, Value>) -> Option<String> {
    TITLE_NAMES.iter().find_map(|n| {
        let (_, v) = map.iter().find(|(k, _)| k.eq_ignore_ascii_case(n))?;
        match v {
            Value::String(s) if !s.trim().is_empty() => Some(clean(s)),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::order::parse_ordered;

    fn values(fields: Vec<LiveField>) -> LiveValues {
        LiveValues::new(ValuesOptions {
            title: "Station".into(),
            fields,
            stale_after: Some(Duration::from_secs(60)),
            min_interval: Duration::from_secs(10),
        })
    }

    fn send(v: &mut LiveValues, topic: &str, json: &str, now: Instant) -> bool {
        let (value, order) = parse_ordered(json).unwrap();
        v.apply(topic, &value, &order, now, &ValueLabels::default())
    }

    /// The Markdown without its escapes, to read.
    fn body(v: &LiveValues) -> String {
        v.render(&ValueLabels::default()).replace('\\', "")
    }

    #[test]
    fn fields_keep_the_message_order_and_their_place() {
        let mut v = values(Vec::new());
        let t = Instant::now();
        send(
            &mut v,
            "wx",
            r#"{"zeta": 1, "temp_c": 21.4, "alpha": "a"}"#,
            t,
        );
        assert_eq!(
            body(&v),
            "# Station\n\n- Zeta: 1\n- Temp c: 21.4\n- Alpha: a\n\n"
        );
        // A later message in another order moves nothing; a new field goes
        // last; a missing one reads no value.
        send(&mut v, "wx", r#"{"new": true, "alpha": "b", "zeta": 2}"#, t);
        assert_eq!(
            body(&v),
            "# Station\n\n- Zeta: 2\n- Temp c: no value\n- Alpha: b\n- New: true\n\n"
        );
    }

    #[test]
    fn mapping_labels_units_decimals_values_and_order() {
        let fields = vec![
            LiveField {
                path: "door".into(),
                values: [("1".to_owned(), "open".to_owned())].into(),
                order: Some(2),
                ..LiveField::default()
            },
            LiveField {
                path: "/sensor/temp".into(),
                label: "Temperature".into(),
                unit: "C".into(),
                decimals: Some(1),
                order: Some(1),
                ..LiveField::default()
            },
        ];
        let mut v = values(fields);
        send(
            &mut v,
            "wx",
            r#"{"rain": 0, "door": 1, "sensor": {"temp": 21.44, "rh": 55}}"#,
            Instant::now(),
        );
        assert_eq!(
            body(&v),
            "# Station\n\n- Temperature: 21.4 C\n- Door: open\n- Rain: 0\n\n## Sensor\n\n- Rh: 55\n\n"
        );
    }

    #[test]
    fn stale_values_say_so_on_a_virtual_clock() {
        let mut v = values(Vec::new());
        let t = Instant::now();
        send(&mut v, "wx", r#"{"temp": 21.4}"#, t);
        assert!(!v.refresh_stale(t + Duration::from_secs(59)));
        assert!(v.refresh_stale(t + Duration::from_secs(60)));
        assert!(body(&v).contains("- Temp: 21.4, no recent value\n"));
        send(
            &mut v,
            "wx",
            r#"{"temp": 21.4}"#,
            t + Duration::from_secs(61),
        );
        assert!(body(&v).contains("- Temp: 21.4\n"));
    }

    #[test]
    fn lists_groups_and_topics() {
        let mut v = values(Vec::new());
        let t = Instant::now();
        let many: Vec<String> = (0..23).map(|i| i.to_string()).collect();
        let json = format!(
            r#"{{"readings": [{}], "pumps": [{{"name": "North", "on": true}}, {{"on": false}}]}}"#,
            many.join(",")
        );
        send(&mut v, "a/wx", &json, t);
        let md = body(&v);
        assert!(
            md.contains("- Readings 20: 19\n- Readings: 3 more\n"),
            "{md}"
        );
        assert!(
            md.contains("## North\n\n- Name: North\n- On: true\n"),
            "{md}"
        );
        assert!(md.contains("## Pumps 2\n\n- On: false\n"), "{md}");
        send(&mut v, "b/door", "1", t);
        let md = body(&v);
        assert!(md.contains("## a/wx\n\n- Readings 1: 0\n"), "{md}");
        assert!(md.contains("### North\n"), "{md}");
        assert!(md.ends_with("## b/door\n\n- Door: 1\n\n"), "{md}");
    }

    #[test]
    fn changes_carry_old_new_and_the_choices() {
        let mut v = values(vec![LiveField {
            path: "temp".into(),
            speak: LiveNotice::OnChange,
            ..LiveField::default()
        }]);
        let t = Instant::now();
        send(&mut v, "wx", r#"{"temp": 1, "rh": 5}"#, t);
        let first = v.take_changes();
        assert_eq!(first.len(), 2);
        send(&mut v, "wx", r#"{"temp": 2, "rh": 5}"#, t);
        let c = v.take_changes();
        assert_eq!(c.len(), 1, "an unchanged field is not a change");
        assert_eq!(c[0].path, "/temp");
        assert_eq!(
            (c[0].old.as_deref(), c[0].new.as_deref()),
            (Some("1"), Some("2"))
        );
        assert_eq!(c[0].line, "Temp: 2");
        assert!(c[0].speak && c[0].show);
        assert_eq!(c[0].min_interval, Duration::from_secs(10));
        // Choosing for an unmapped field keeps it where it is.
        let before = body(&v);
        v.set_notice("/rh", None, Some(LiveNotice::OnChange));
        assert_eq!(v.options().fields[1].path, "rh");
        send(&mut v, "wx", r#"{"temp": 2, "rh": 6}"#, t);
        let c = v.take_changes();
        assert!(c[0].speak, "{c:?}");
        assert_eq!(body(&v), before.replace("Rh: 5", "Rh: 6"));
    }

    #[test]
    fn paths_round_trip() {
        assert_eq!(path_pointer("a.b").as_deref(), Some("/a/b"));
        assert_eq!(pointer_path("/a/b"), "a.b");
        assert_eq!(pointer_path("/a.b/c"), "/a.b/c");
        assert_eq!(path_pointer("/a.b/c").as_deref(), Some("/a.b/c"));
        assert_eq!(pointer_path("/x/0"), "x.0");
        assert_eq!(auto_label("wind_speed-kmh"), "Wind speed kmh");
    }
}
