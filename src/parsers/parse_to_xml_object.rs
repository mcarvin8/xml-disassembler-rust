//! Parse file to XmlElement - supports XML, YAML, JSON, TOML, INI.

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

    if file_path.to_lowercase().ends_with(".yaml")
        || file_path.to_lowercase().ends_with(".yml")
    {
        return serde_yaml::from_str(&content).ok();
    }

    if file_path.to_lowercase().ends_with(".json5") {
        return json5::from_str(&content).ok();
    }

    if file_path.to_lowercase().ends_with(".json") {
        return serde_json::from_str(&content).ok();
    }

    if file_path.to_lowercase().ends_with(".toml") {
        let toml_val: toml::Value = toml::from_str(&content).ok()?;
        return serde_json::from_str(&serde_json::to_string(&toml_val).ok()?).ok();
    }

    if file_path.to_lowercase().ends_with(".ini") {
        return parse_ini_to_value(&content);
    }

    None
}

fn parse_ini_to_value(content: &str) -> Option<XmlElement> {
    let mut result = serde_json::Map::new();
    let mut current_section = String::new();
    let mut section_map = serde_json::Map::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if !current_section.is_empty() {
                result.insert(current_section.clone(), Value::Object(section_map.clone()));
            }
            current_section = line[1..line.len() - 1].to_string();
            section_map = serde_json::Map::new();
        } else if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim().to_string();
            section_map.insert(key, Value::String(val));
        }
    }
    if !current_section.is_empty() {
        result.insert(current_section, Value::Object(section_map));
    }
    Some(Value::Object(result))
}
