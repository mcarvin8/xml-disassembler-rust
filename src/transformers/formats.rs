//! Transform XmlElement to various formats.

use serde_json::Value;

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

pub async fn transform_to_toml(parsed_xml: &XmlElement) -> String {
    toml::to_string_pretty(&convert_json_to_toml_value(parsed_xml)).unwrap_or_default()
}

pub async fn transform_to_ini(parsed_xml: &XmlElement) -> String {
    // Simple INI-like output - Node's ini package produces nested structure
    // We output a basic section/key=value format
    convert_to_ini_string(parsed_xml)
}

fn convert_json_to_toml_value(v: &Value) -> toml::Value {
    match v {
        Value::Null => toml::Value::String(String::new()),
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(arr) => {
            toml::Value::Array(arr.iter().map(convert_json_to_toml_value).collect())
        }
        Value::Object(obj) => {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                table.insert(k.clone(), convert_json_to_toml_value(v));
            }
            toml::Value::Table(table)
        }
    }
}

fn convert_to_ini_string(v: &Value) -> String {
    let mut out = String::new();
    if let Some(obj) = v.as_object() {
        for (section, value) in obj {
            if section.starts_with('@')
                || section == "?xml"
                || section == "#text"
                || section == "#cdata"
            {
                continue;
            }
            out.push_str(&format!("[{}]\n", section));
            if let Some(inner) = value.as_object() {
                for (k, val) in inner {
                    if !k.starts_with('@') && k != "#text" && k != "#cdata" {
                        if let Some(s) = val.as_str() {
                            out.push_str(&format!("{} = {}\n", k, s));
                        }
                    }
                }
            }
            out.push('\n');
        }
    }
    out
}
