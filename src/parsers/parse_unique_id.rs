//! Parse unique ID from XML element for file naming.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::types::XmlElement;

/// Cache for stringified elements - we use a simple approach in Rust.
/// For full equivalence we could use a type with interior mutability and weak refs.
fn create_short_hash(element: &XmlElement) -> String {
    let stringified = serde_json::to_string(element).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(stringified.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..8].to_string()
}

fn is_object(value: &Value) -> bool {
    value.is_object() && !value.is_array()
}

fn find_direct_field_match(element: &XmlElement, field_names: &[&str]) -> Option<String> {
    let obj = element.as_object()?;
    for name in field_names {
        if let Some(value) = obj.get(*name) {
            if let Some(s) = value.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Returns true if the string looks like a hash fallback (8 hex chars) rather than a real ID.
fn looks_like_hash(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn find_nested_field_match(element: &XmlElement, unique_id_elements: &str) -> Option<String> {
    let obj = element.as_object()?;
    let mut hash_fallback: Option<String> = None;
    for (_, child) in obj {
        if is_object(child) {
            let result = parse_unique_id_element(child, Some(unique_id_elements));
            if !result.is_empty() {
                if looks_like_hash(&result) {
                    if hash_fallback.is_none() {
                        hash_fallback = Some(result);
                    }
                } else {
                    return Some(result);
                }
            }
        } else if let Some(arr) = child.as_array() {
            for item in arr {
                if is_object(item) {
                    let result = parse_unique_id_element(item, Some(unique_id_elements));
                    if !result.is_empty() {
                        if looks_like_hash(&result) {
                            if hash_fallback.is_none() {
                                hash_fallback = Some(result);
                            }
                        } else {
                            return Some(result);
                        }
                    }
                }
            }
        }
    }
    hash_fallback
}

/// Get a unique ID for an element, using configured fields or a hash.
pub fn parse_unique_id_element(element: &XmlElement, unique_id_elements: Option<&str>) -> String {
    if let Some(ids) = unique_id_elements {
        let field_names: Vec<&str> = ids.split(',').map(|s| s.trim()).collect();
        find_direct_field_match(element, &field_names)
            .or_else(|| find_nested_field_match(element, ids))
            .unwrap_or_else(|| create_short_hash(element))
    } else {
        create_short_hash(element)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_direct_field() {
        let el = json!({ "name": "Get_Info", "label": "Get Info" });
        assert_eq!(parse_unique_id_element(&el, Some("name")), "Get_Info");
    }

    #[test]
    fn finds_deeply_nested_field() {
        let el = json!({
            "connector": { "targetReference": "X" },
            "value": { "elementReference": "accts.accounts" }
        });
        assert_eq!(
            parse_unique_id_element(&el, Some("elementReference")),
            "accts.accounts"
        );
    }

    #[test]
    fn prefers_real_id_over_hash_from_first_child() {
        let el = json!({
            "connector": { "targetReference": "Update_If_Existing" },
            "value": { "elementReference": "accts.accounts" }
        });
        let result = parse_unique_id_element(&el, Some("elementReference"));
        assert_eq!(result, "accts.accounts");
    }

    #[test]
    fn finds_id_in_array_element() {
        let el = json!({
            "items": [
                { "other": "x" },
                { "name": "NestedName" }
            ]
        });
        assert_eq!(parse_unique_id_element(&el, Some("name")), "NestedName");
    }
}
