//! Transform XmlElement to various formats.

use crate::types::XmlElement;

pub async fn transform_to_yaml(parsed_xml: &XmlElement) -> String {
    serde_yaml::to_string(parsed_xml).unwrap_or_default()
}

pub async fn transform_to_json5(parsed_xml: &XmlElement) -> String {
    serde_json::to_string_pretty(parsed_xml).unwrap_or_default()
}

pub async fn transform_to_json(parsed_xml: &XmlElement) -> String {
    serde_json::to_string_pretty(parsed_xml).unwrap_or_default()
}
