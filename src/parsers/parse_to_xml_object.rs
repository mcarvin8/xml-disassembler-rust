//! Parse file to XmlElement - supports XML, YAML, JSON.

use crate::parsers::{
    extract_xml_declaration_from_raw, extract_xmlns_from_raw, parse_xml_from_str,
};
use crate::types::XmlElement;
use serde_json::Value;
use tokio::fs;

pub async fn parse_to_xml_object(file_path: &str) -> Option<XmlElement> {
    if file_path.to_lowercase().ends_with(".xml") {
        let content = fs::read_to_string(file_path).await.ok()?;
        let mut parsed = parse_xml_from_str(&content, file_path)?;
        if let Some(obj) = parsed.as_object_mut() {
            // quickxml_to_serde drops the declaration - extract from raw and add at top level
            if let Some(decl) = extract_xml_declaration_from_raw(&content) {
                obj.insert("?xml".to_string(), decl);
            }
            // quickxml_to_serde drops xmlns - extract from raw and add to root element
            if let Some(xmlns) = extract_xmlns_from_raw(&content) {
                let root_key = obj.keys().find(|k| *k != "?xml")?.clone();
                if let Some(root_val) = obj.get_mut(&root_key) {
                    if let Some(root_obj) = root_val.as_object_mut() {
                        if !root_obj.contains_key("@xmlns") {
                            root_obj.insert("@xmlns".to_string(), Value::String(xmlns));
                        }
                    }
                }
            }
        }
        return Some(parsed);
    }

    let content = fs::read_to_string(file_path).await.ok()?;

    if file_path.to_lowercase().ends_with(".yaml") || file_path.to_lowercase().ends_with(".yml") {
        return serde_yaml::from_str(&content).ok();
    }

    if file_path.to_lowercase().ends_with(".json5") {
        return json5::from_str(&content).ok();
    }

    if file_path.to_lowercase().ends_with(".json") {
        return serde_json::from_str(&content).ok();
    }

    None
}
