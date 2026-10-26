//! Message key order. serde_json keeps an object's keys sorted, because
//! the workspace leaves its `preserve_order` feature off: turning it on
//! would change every crate's JSON output (`tw settings export`, JSON-RPC,
//! which promises alphabetical fields). A live source's own order is
//! usually meaningful, so it is read from the message's text once, beside
//! the parsed value, and kept as a [`KeyOrder`]: for each object in the
//! message, by its JSON Pointer, its keys in the order they came.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// The keys of each object in a message, in the order the message sent
/// them, by the object's JSON Pointer (`""` for the message itself,
/// `"/checkins/0"` for the first item of its `checkins` list). Empty when
/// the order is not known: keys then come in the map's own (sorted)
/// order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyOrder(BTreeMap<String, Vec<String>>);

impl From<BTreeMap<String, Vec<String>>> for KeyOrder {
    /// The order a source reports (textweaver-mqtt's `key_order`).
    fn from(map: BTreeMap<String, Vec<String>>) -> Self {
        KeyOrder(map)
    }
}

impl KeyOrder {
    /// The order of the keys in `text`, a JSON document; `None` when it
    /// is not JSON.
    pub fn of_text(text: &str) -> Option<KeyOrder> {
        let mut out = BTreeMap::new();
        let mut de = serde_json::Deserializer::from_str(text);
        Walk {
            at: String::new(),
            out: &mut out,
        }
        .deserialize(&mut de)
        .ok()?;
        de.end().ok()?;
        Some(KeyOrder(out))
    }

    /// True when nothing is known.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The keys of the object at `at`, in message order, when known.
    pub fn keys(&self, at: &str) -> Option<&[String]> {
        self.0.get(at).map(Vec::as_slice)
    }

    /// The order of the part of the message at `prefix`, as if it were the
    /// whole message (an entry inside a list).
    pub fn sub(&self, prefix: &str) -> KeyOrder {
        if prefix.is_empty() {
            return self.clone();
        }
        KeyOrder(
            self.0
                .iter()
                .filter_map(|(k, v)| {
                    let rest = k.strip_prefix(prefix)?;
                    (rest.is_empty() || rest.starts_with('/')).then(|| (rest.to_owned(), v.clone()))
                })
                .collect(),
        )
    }

    /// Records `keys` as the order of the object at `at`.
    pub fn set(&mut self, at: &str, keys: Vec<String>) {
        self.0.insert(at.to_owned(), keys);
    }

    /// `map`'s members in message order: the keys the order knows first,
    /// as they came, then any it does not know, in the map's order.
    pub fn members<'a>(
        &self,
        at: &str,
        map: &'a Map<String, Value>,
    ) -> Vec<(&'a String, &'a Value)> {
        let mut out: Vec<(&String, &Value)> = Vec::with_capacity(map.len());
        if let Some(keys) = self.keys(at) {
            for k in keys {
                if let Some((key, v)) = map.get_key_value(k)
                    && !out.iter().any(|(o, _)| *o == key)
                {
                    out.push((key, v));
                }
            }
        }
        if out.len() < map.len() {
            for (k, v) in map {
                if !out.iter().any(|(o, _)| *o == k) {
                    out.push((k, v));
                }
            }
        }
        out
    }

    /// `value` as compact JSON text, objects in message order (`at` is
    /// its pointer).
    pub fn compact(&self, at: &str, value: &Value) -> String {
        let mut out = String::new();
        self.write_compact(at, value, &mut out);
        out
    }

    fn write_compact(&self, at: &str, value: &Value, out: &mut String) {
        match value {
            Value::Object(map) => {
                out.push('{');
                for (i, (k, v)) in self.members(at, map).into_iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&Value::String(k.clone()).to_string());
                    out.push(':');
                    self.write_compact(&child(at, k), v, out);
                }
                out.push('}');
            }
            Value::Array(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    self.write_compact(&format!("{at}/{i}"), v, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
}

/// The JSON Pointer of member `key` of the object at `at` (RFC 6901: `~`
/// is written `~0` and `/` is written `~1`).
pub fn child(at: &str, key: &str) -> String {
    format!("{at}/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// `text` parsed as JSON, with its key order; `None` when it is not JSON.
pub fn parse_ordered(text: &str) -> Option<(Value, KeyOrder)> {
    let value = serde_json::from_str(text).ok()?;
    let order = KeyOrder::of_text(text)?;
    Some((value, order))
}

/// Walks a JSON document, recording each object's keys under its pointer;
/// values are skipped, not kept.
struct Walk<'a> {
    at: String,
    out: &'a mut BTreeMap<String, Vec<String>>,
}

impl<'de> DeserializeSeed<'de> for Walk<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Walk<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let Walk { at, out } = self;
        let mut i = 0usize;
        while seq
            .next_element_seed(Walk {
                at: format!("{at}/{i}"),
                out: &mut *out,
            })?
            .is_some()
        {
            i += 1;
        }
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let Walk { at, out } = self;
        let mut keys: Vec<String> = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            map.next_value_seed(Walk {
                at: child(&at, &key),
                out: &mut *out,
            })?;
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        out.insert(at, keys);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_keep_the_message_order_at_every_depth() {
        let text = r#"{"zeta": 1, "alpha": {"y": 2, "b": [ {"q": 1, "a": 2} ]}, "a/b~": null}"#;
        let (value, order) = parse_ordered(text).unwrap();
        assert_eq!(order.keys("").unwrap(), ["zeta", "alpha", "a/b~"]);
        assert_eq!(order.keys("/alpha").unwrap(), ["y", "b"]);
        assert_eq!(order.keys("/alpha/b/0").unwrap(), ["q", "a"]);
        let map = value.as_object().unwrap();
        let keys: Vec<&str> = order
            .members("", map)
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, ["zeta", "alpha", "a/b~"]);
        assert_eq!(
            order.compact("", &value),
            r#"{"zeta":1,"alpha":{"y":2,"b":[{"q":1,"a":2}]},"a/b~":null}"#
        );
        let sub = order.sub("/alpha/b/0");
        assert_eq!(sub.keys("").unwrap(), ["q", "a"]);
        assert!(order.sub("/alp").is_empty(), "a prefix is a whole key");
    }

    #[test]
    fn unknown_keys_follow_in_map_order() {
        let value = serde_json::json!({"b": 1, "a": 2, "c": 3});
        let mut order = KeyOrder::default();
        order.set("", vec!["c".into(), "gone".into()]);
        let keys: Vec<&str> = order
            .members("", value.as_object().unwrap())
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, ["c", "a", "b"]);
        assert!(parse_ordered("not json").is_none());
        assert!(KeyOrder::of_text("{} trailing").is_none());
    }
}
