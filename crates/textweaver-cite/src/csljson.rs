//! CSL-JSON import and export.

use serde_json::Value;

use crate::error::{CiteError, Result};
use crate::reference::Reference;

/// Parses CSL-JSON: an array of items, a single item, or an object with an
/// `items` array (as some exporters write). A leading byte-order mark is
/// ignored. Entries that are not objects are skipped.
pub fn parse(text: &str) -> Result<Vec<Reference>> {
    let text = text.trim_start_matches('\u{feff}');
    let value: Value = serde_json::from_str(text)
        .map_err(|e| CiteError::parse("CSL-JSON", Some(e.line()), json_message(&e)))?;
    let items = match value {
        Value::Array(items) => items,
        Value::Object(mut map) => match map.remove("items") {
            Some(Value::Array(items)) => items,
            Some(other) => {
                map.insert("items".into(), other);
                vec![Value::Object(map)]
            }
            None => vec![Value::Object(map)],
        },
        _ => {
            return Err(CiteError::parse(
                "CSL-JSON",
                None,
                "expected a list of references or one reference",
            ));
        }
    };
    items
        .into_iter()
        .filter(Value::is_object)
        .enumerate()
        .map(|(i, item)| {
            serde_json::from_value::<Reference>(item).map_err(|e| {
                CiteError::parse(
                    "CSL-JSON",
                    None,
                    format!("reference {} is malformed: {e}", i + 1),
                )
            })
        })
        .collect()
}

/// Writes references as a pretty-printed CSL-JSON array (two-space indent,
/// non-ASCII kept as is), ending with a newline.
pub fn write(refs: &[Reference]) -> Result<String> {
    let mut out = serde_json::to_string_pretty(refs).map_err(|e| CiteError::Serialize {
        format: "CSL-JSON",
        message: e.to_string(),
    })?;
    out.push('\n');
    Ok(out)
}

fn json_message(e: &serde_json::Error) -> String {
    let msg = e.to_string();
    // serde_json appends " at line L column C"; the line is reported separately.
    match msg.rfind(" at line ") {
        Some(i) => format!("{} (column {})", &msg[..i], e.column()),
        None => msg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_array_single_object_and_items_wrapper() {
        assert_eq!(parse(r#"[{"id":"a"},{"id":"b"}]"#).unwrap().len(), 2);
        assert_eq!(parse(r#"{"id":"solo","title":"T"}"#).unwrap()[0].id, "solo");
        assert_eq!(parse(r#"{"items":[{"id":"x"}]}"#).unwrap()[0].id, "x");
        assert_eq!(parse("\u{feff}[]").unwrap().len(), 0);
    }

    #[test]
    fn errors_name_the_line() {
        let err = parse("[\n{\"id\": }\n]").unwrap_err().to_string();
        assert!(err.contains("on line 2"), "{err}");
    }
}
