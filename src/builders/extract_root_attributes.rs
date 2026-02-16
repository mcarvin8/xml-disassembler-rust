//! Extract XML attributes from root element.

use serde_json::{Map, Value};

use crate::types::XmlElement;

fn attr_value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    }
}

/// Extracts XML attributes from a root element and returns them as a flat map.
/// Handles @-prefixed keys (e.g. @xmlns, @version) and xmlns (default namespace).
/// Converts values to strings for consistent XML output.
pub fn extract_root_attributes(element: &XmlElement) -> XmlElement {
    let mut attributes = Map::new();
    if let Some(obj) = element.as_object() {
        for (key, value) in obj {
            let is_attr = key.starts_with('@') || key == "xmlns";
            if is_attr {
                let attr_key = if key == "xmlns" {
                    "@xmlns".to_string()
                } else {
                    key.clone()
                };
                attributes.insert(attr_key, Value::String(attr_value_to_string(value)));
            }
        }
    }
    Value::Object(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_at_prefixed_attributes() {
        let element = json!({
            "@xmlns": "http://example.com",
            "@version": "1.0",
            "child": "ignored"
        });
        let attrs = extract_root_attributes(&element);
        let obj = attrs.as_object().unwrap();
        assert_eq!(
            obj.get("@xmlns").and_then(|v| v.as_str()),
            Some("http://example.com")
        );
        assert_eq!(obj.get("@version").and_then(|v| v.as_str()), Some("1.0"));
        assert!(obj.get("child").is_none());
    }

    #[test]
    fn returns_empty_for_non_object() {
        let element = json!("string");
        let attrs = extract_root_attributes(&element);
        assert!(attrs.as_object().unwrap().is_empty());
    }

    #[test]
    fn converts_xmlns_to_at_xmlns() {
        let element = json!({ "xmlns": "http://ns.example.com", "child": {} });
        let attrs = extract_root_attributes(&element);
        assert_eq!(
            attrs
                .as_object()
                .unwrap()
                .get("@xmlns")
                .and_then(|v| v.as_str()),
            Some("http://ns.example.com")
        );
    }
}
