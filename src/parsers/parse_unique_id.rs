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

fn find_nested_field_match(element: &XmlElement, unique_id_elements: &str) -> Option<String> {
    let obj = element.as_object()?;
    for (_, child) in obj {
        if is_object(child) {
            let result = parse_unique_id_element(child, Some(unique_id_elements));
            if !result.is_empty() {
                return Some(result);
            }
        }
    }
    None
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
