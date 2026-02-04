//! Get transformer for format - returns output string from XmlElement.

use crate::transformers::{
    transform_to_ini, transform_to_json, transform_to_json5, transform_to_toml, transform_to_yaml,
};
use crate::types::XmlElement;

/// Transform XmlElement to string in the given format.
/// Returns None if format is not supported (e.g. "xml" uses build_xml_string instead).
pub async fn transform_format(format: &str, xml_content: &XmlElement) -> Option<String> {
    let result = match format.to_lowercase().as_str() {
        "yaml" | "yml" => transform_to_yaml(xml_content).await,
        "json5" => transform_to_json5(xml_content).await,
        "json" => transform_to_json(xml_content).await,
        "toml" => transform_to_toml(xml_content).await,
        "ini" => transform_to_ini(xml_content).await,
        _ => return None,
    };
    Some(result)
}
