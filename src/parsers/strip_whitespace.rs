//! Strip meaningless whitespace-only #text nodes from parsed XML structure.

use serde_json::{Map, Value};

fn is_empty_text_node(key: &str, value: &Value) -> bool {
    (key == "#text" || key == "#cdata")
        && value.as_str().map(|s| s.trim().is_empty()).unwrap_or(false)
}

fn clean_array(arr: &[Value]) -> Vec<Value> {
    arr.iter()
        .filter_map(|entry| {
            let cleaned = strip_whitespace_text_nodes(entry);
            match &cleaned {
                Value::Object(m) if m.is_empty() => None,
                _ => Some(cleaned),
            }
        })
        .collect()
}

fn clean_object(obj: &Map<String, Value>) -> Map<String, Value> {
    let mut result = Map::new();
    let has_cdata = obj.contains_key("#cdata");
    for (key, value) in obj {
        // Preserve whitespace-only #text when element has #cdata (needed for round-trip)
        if is_empty_text_node(key, value) && !(key == "#text" && has_cdata) {
            continue;
        }
        let cleaned = strip_whitespace_text_nodes(value);
        if !cleaned.is_null() || key == "#text" || key == "#cdata" {
            result.insert(key.clone(), cleaned);
        }
    }
    result
}

/// Remove meaningless whitespace-only #text nodes from the XML structure.
pub fn strip_whitespace_text_nodes(node: &Value) -> Value {
    match node {
        Value::Array(arr) => Value::Array(clean_array(arr)),
        Value::Object(obj) => Value::Object(clean_object(obj)),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_empty_text_nodes_from_array() {
        let input = json!([{ "#text": "   " }, { "#text": "keep me" }]);
        let result = strip_whitespace_text_nodes(&input);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(
            arr[0].get("#text").and_then(|v| v.as_str()),
            Some("keep me")
        );
    }

    #[test]
    fn preserves_non_empty_text() {
        let input = json!({ "#text": "  content  " });
        let result = strip_whitespace_text_nodes(&input);
        assert_eq!(
            result.get("#text").and_then(|v| v.as_str()),
            Some("  content  ")
        );
    }

    #[test]
    fn leaves_primitive_unchanged() {
        let input = json!("hello");
        let result = strip_whitespace_text_nodes(&input);
        assert_eq!(result, json!("hello"));
    }
}
