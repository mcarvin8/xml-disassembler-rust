//! Merge multiple XML elements into one.

use serde_json::{Map, Value};

use crate::types::XmlElement;

fn is_mergeable_object(value: &Value) -> bool {
    value.is_object() && !value.is_array()
}

fn merge_element_content(target: &mut Map<String, Value>, source: &Map<String, Value>) {
    for (key, value) in source {
        if value.is_array() {
            merge_array_value(target, key, value.as_array().unwrap());
        } else if is_mergeable_object(value) {
            merge_object_value(target, key, value.as_object().unwrap());
        } else {
            merge_primitive_value(target, key, value);
        }
    }
}

fn merge_array_value(target: &mut Map<String, Value>, key: &str, value: &[Value]) {
    if !target.contains_key(key) {
        target.insert(key.to_string(), Value::Array(value.to_vec()));
    } else if let Some(Value::Array(arr)) = target.get_mut(key) {
        arr.extend(value.iter().cloned());
    } else {
        let existing = target.remove(key).unwrap();
        target.insert(
            key.to_string(),
            Value::Array(
                [vec![existing], value.to_vec()]
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
        );
    }
}

fn merge_object_value(target: &mut Map<String, Value>, key: &str, value: &Map<String, Value>) {
    if let Some(Value::Array(arr)) = target.get_mut(key) {
        arr.push(Value::Object(value.clone()));
    } else if let Some(existing) = target.get(key) {
        let existing = existing.clone();
        target.insert(
            key.to_string(),
            Value::Array(vec![existing, Value::Object(value.clone())]),
        );
    } else {
        target.insert(key.to_string(), Value::Object(value.clone()));
    }
}

fn merge_primitive_value(target: &mut Map<String, Value>, key: &str, value: &Value) {
    if !target.contains_key(key) {
        target.insert(key.to_string(), value.clone());
    }
}

fn default_xml_declaration() -> Value {
    let mut decl = Map::new();
    decl.insert("@version".to_string(), Value::String("1.0".to_string()));
    decl.insert("@encoding".to_string(), Value::String("UTF-8".to_string()));
    Value::Object(decl)
}

fn build_final_xml_element(
    declaration: Option<&Value>,
    root_key: &str,
    content: Map<String, Value>,
) -> XmlElement {
    let mut result = Map::new();
    let decl = declaration.cloned().unwrap_or_else(default_xml_declaration);
    result.insert("?xml".to_string(), decl);
    result.insert(root_key.to_string(), Value::Object(content));
    Value::Object(result)
}

/// Reorder the root element's child keys to match the given order.
/// Keys not in `key_order` are appended at the end.
pub fn reorder_root_keys(element: &XmlElement, key_order: &[String]) -> Option<XmlElement> {
    let obj = element.as_object()?;
    let root_key = obj.keys().find(|k| *k != "?xml")?.clone();
    let root_content = obj.get(&root_key)?.as_object()?;
    let mut reordered = Map::new();
    for key in key_order {
        if let Some(v) = root_content.get(key) {
            reordered.insert(key.clone(), v.clone());
        }
    }
    for (key, value) in root_content {
        if !reordered.contains_key(key) {
            reordered.insert(key.clone(), value.clone());
        }
    }
    let mut result = Map::new();
    if let Some(decl) = obj.get("?xml") {
        result.insert("?xml".to_string(), decl.clone());
    }
    result.insert(root_key, Value::Object(reordered));
    Some(Value::Object(result))
}

/// Merge multiple XML elements into one.
pub fn merge_xml_elements(elements: &[XmlElement]) -> Option<XmlElement> {
    if elements.is_empty() {
        log::error!("No elements to merge.");
        return None;
    }

    let first = &elements[0];
    let root_key = first.as_object()?.keys().find(|k| *k != "?xml")?.clone();
    let mut merged_content = Map::new();

    for element in elements {
        if let Some(obj) = element.as_object() {
            if let Some(root_content) = obj.get(&root_key) {
                if let Some(content_obj) = root_content.as_object() {
                    merge_element_content(&mut merged_content, content_obj);
                }
            }
        }
    }

    let declaration = first.as_object()?.get("?xml");
    Some(build_final_xml_element(
        declaration,
        &root_key,
        merged_content,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_empty_returns_none() {
        assert!(merge_xml_elements(&[]).is_none());
    }

    #[test]
    fn merge_single_element_preserves_structure() {
        let el = json!({
            "?xml": { "@version": "1.0", "@encoding": "UTF-8" },
            "Root": { "@xmlns": "http://example.com", "child": "a" }
        });
        let merged = merge_xml_elements(std::slice::from_ref(&el)).unwrap();
        assert!(merged.get("?xml").is_some());
        let root = merged.get("Root").and_then(|v| v.as_object()).unwrap();
        assert_eq!(root.get("child").and_then(|v| v.as_str()), Some("a"));
        assert_eq!(
            root.get("@xmlns").and_then(|v| v.as_str()),
            Some("http://example.com")
        );
    }

    #[test]
    fn merge_two_elements_combines_nested_objects_into_array() {
        let a = json!({ "Root": { "section": { "name": "first" } } });
        let b = json!({ "Root": { "section": { "name": "second" } } });
        let merged = merge_xml_elements(&[a, b]).unwrap();
        let root = merged.get("Root").and_then(|v| v.as_object()).unwrap();
        let sections = root.get("section").and_then(|v| v.as_array()).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(
            sections[0].get("name").and_then(|v| v.as_str()),
            Some("first")
        );
        assert_eq!(
            sections[1].get("name").and_then(|v| v.as_str()),
            Some("second")
        );
    }

    #[test]
    fn reorder_root_keys_reorders_and_appends_extra() {
        let el = json!({
            "?xml": { "@version": "1.0" },
            "Root": { "z": "last", "a": "first", "m": "mid" }
        });
        let reordered = reorder_root_keys(&el, &["a".into(), "m".into()]).unwrap();
        let root = reordered.get("Root").and_then(|v| v.as_object()).unwrap();
        let keys: Vec<_> = root.keys().filter(|k| !k.starts_with('@')).cloned().collect();
        assert_eq!(keys, ["a", "m", "z"]);
    }
}
