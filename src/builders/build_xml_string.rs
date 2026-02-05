//! Build XML string from XmlElement structure.

use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use serde_json::{Map, Value};

use crate::types::XmlElement;

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    }
}

fn write_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    content: &Value,
    indent_level: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let indent = "    ".repeat(indent_level);
    let child_indent = "    ".repeat(indent_level + 1);

    match content {
        Value::Object(obj) => {
            let (attrs, children): (Vec<_>, Vec<_>) =
                obj.iter().partition(|(k, _)| k.starts_with('@'));

            let attr_name = |k: &str| k.trim_start_matches('@').to_string();

            let mut text_content = String::new();
            let mut cdata_content = String::new();
            let child_elements: Vec<(&String, &Value)> = children
                .iter()
                .filter_map(|(k, v)| {
                    if *k == "#text" {
                        text_content = value_to_string(v);
                        None
                    } else if *k == "#cdata" {
                        cdata_content = value_to_string(v);
                        None
                    } else {
                        Some((*k, *v))
                    }
                })
                .collect();

            let has_children = child_elements.iter().any(|(_, v)| {
                v.is_object()
                    || (v.is_array() && v.as_array().map(|a| !a.is_empty()).unwrap_or(false))
            });

            let attrs: Vec<(String, String)> = attrs
                .iter()
                .map(|(k, v)| (attr_name(k), value_to_string(v)))
                .collect();

            let mut start = BytesStart::new(name);
            for (k, v) in &attrs {
                start.push_attribute((k.as_str(), v.as_str()));
            }
            writer.write_event(Event::Start(start))?;

            if has_children || !child_elements.is_empty() {
                writer.write_event(Event::Text(BytesText::new(
                    format!("\n{}", child_indent).as_str(),
                )))?;

                let child_count = child_elements.len();
                for (idx, (child_name, child_value)) in child_elements.iter().enumerate() {
                    let is_last = idx == child_count - 1;
                    match child_value {
                        Value::Array(arr) => {
                            let arr_len = arr.len();
                            for (i, item) in arr.iter().enumerate() {
                                let arr_last = i == arr_len - 1;
                                write_element(writer, child_name, item, indent_level + 1)?;
                                if !arr_last {
                                    writer.write_event(Event::Text(BytesText::new(
                                        format!("\n{}", child_indent).as_str(),
                                    )))?;
                                }
                            }
                            if !is_last {
                                writer.write_event(Event::Text(BytesText::new(
                                    format!("\n{}", child_indent).as_str(),
                                )))?;
                            }
                        }
                        Value::Object(_) => {
                            write_element(writer, child_name, child_value, indent_level + 1)?;
                            if !is_last {
                                writer.write_event(Event::Text(BytesText::new(
                                    format!("\n{}", child_indent).as_str(),
                                )))?;
                            }
                        }
                        _ => {
                            writer
                                .write_event(Event::Start(BytesStart::new(child_name.as_str())))?;
                            // BytesText::new() expects unescaped content; the writer escapes when writing
                            writer.write_event(Event::Text(BytesText::new(
                                value_to_string(child_value).as_str(),
                            )))?;
                            writer.write_event(Event::End(BytesEnd::new(child_name.as_str())))?;
                            if !is_last {
                                writer.write_event(Event::Text(BytesText::new(
                                    format!("\n{}", child_indent).as_str(),
                                )))?;
                            }
                        }
                    }
                }

                writer.write_event(Event::Text(BytesText::new(
                    format!("\n{}", indent).as_str(),
                )))?;
            } else if !cdata_content.is_empty() || !text_content.is_empty() {
                // Output #text first (e.g. whitespace before CDATA), then #cdata
                if !text_content.is_empty() {
                    writer.write_event(Event::Text(BytesText::new(text_content.as_str())))?;
                }
                if !cdata_content.is_empty() {
                    writer.write_event(Event::CData(BytesCData::new(cdata_content.as_str())))?;
                }
            }

            writer.write_event(Event::End(BytesEnd::new(name)))?;
        }
        Value::Array(arr) => {
            for item in arr {
                write_element(writer, name, item, indent_level)?;
            }
        }
        _ => {
            writer.write_event(Event::Start(BytesStart::new(name)))?;
            // BytesText::new() expects unescaped content; the writer escapes when writing
            writer.write_event(Event::Text(BytesText::new(
                value_to_string(content).as_str(),
            )))?;
            writer.write_event(Event::End(BytesEnd::new(name)))?;
        }
    }

    Ok(())
}

fn build_xml_from_object(
    element: &Map<String, Value>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 4);

    let (declaration, root_key, root_value) = if let Some(decl) = element.get("?xml") {
        let root_key = element
            .keys()
            .find(|k| *k != "?xml")
            .cloned()
            .unwrap_or_else(|| "root".to_string());
        let root_value = element
            .get(&root_key)
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        (Some(decl), root_key, root_value)
    } else {
        let root_key = element
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "root".to_string());
        let root_value = element
            .get(&root_key)
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        (None, root_key, root_value)
    };

    if declaration.is_some() {
        if let Some(decl) = declaration {
            if let Some(obj) = decl.as_object() {
                let version = obj
                    .get("@version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1.0");
                let encoding = obj.get("@encoding").and_then(|v| v.as_str());
                let standalone = obj.get("@standalone").and_then(|v| v.as_str());
                writer.write_event(Event::Decl(BytesDecl::new(version, encoding, standalone)))?;
            }
        }
    }

    write_element(&mut writer, &root_key, &root_value, 0)?;

    let result = String::from_utf8(writer.into_inner())?;
    Ok(result.trim_end().to_string())
}

/// Build XML string from XmlElement.
pub fn build_xml_string(element: &XmlElement) -> String {
    match element {
        Value::Object(obj) => build_xml_from_object(obj).unwrap_or_default(),
        _ => String::new(),
    }
}
