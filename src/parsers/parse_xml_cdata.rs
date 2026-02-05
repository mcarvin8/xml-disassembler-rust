//! XML parser that preserves CDATA sections.
//! Uses quick-xml directly to distinguish CDATA from regular text.

use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Number, Value};

/// Parse text content - match quickxml_to_serde behavior for type inference.
fn parse_text_value(text: &str, leading_zero_as_string: bool) -> Value {
    let text = text.trim();
    if text.is_empty() {
        return Value::String(String::new());
    }
    if leading_zero_as_string && text.starts_with('0') && (text == "0" || text.len() > 1) {
        return Value::String(text.to_string());
    }
    if let Ok(v) = text.parse::<u64>() {
        if !leading_zero_as_string || !text.starts_with('0') || text == "0" {
            return Value::Number(Number::from(v));
        }
    }
    if let Ok(v) = text.parse::<f64>() {
        if !text.starts_with('0') || text.starts_with("0.") {
            if let Some(n) = Number::from_f64(v) {
                return Value::Number(n);
            }
        }
    }
    if let Ok(v) = text.parse::<bool>() {
        return Value::Bool(v);
    }
    Value::String(text.to_string())
}

/// Parse XML string to JSON Value, preserving CDATA as #cdata key.
/// Produces the same structure as quickxml_to_serde but with #cdata for CDATA content.
pub fn parse_xml_with_cdata(xml: &str) -> Result<Value, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<(String, Map<String, Value>)> = Vec::new();
    let mut root_name: Option<String> = None;
    let mut root_value: Option<Value> = None;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = Map::new();
                for attr in e.attributes() {
                    if let Ok(a) = attr {
                        let key = format!("@{}", String::from_utf8_lossy(a.key.as_ref()));
                        let val = a
                            .decode_and_unescape_value(reader.decoder())
                            .unwrap_or_default();
                        attrs.insert(key, Value::String(val.to_string()));
                    }
                }
                stack.push((name, attrs));
            }
            Ok(Event::End(_e)) => {
                if let Some((popped_name, elem)) = stack.pop() {
                    let value = if elem.is_empty() {
                        Value::Object(Map::new())
                    } else {
                        Value::Object(elem)
                    };
                    if let Some((_, parent)) = stack.last_mut() {
                        if let Some(existing) = parent.get_mut(&popped_name) {
                            if let Some(arr) = existing.as_array_mut() {
                                arr.push(value);
                            } else {
                                let prev = parent.remove(&popped_name).unwrap();
                                parent.insert(popped_name, Value::Array(vec![prev, value]));
                            }
                        } else {
                            parent.insert(popped_name, value);
                        }
                    } else {
                        root_name = Some(popped_name);
                        root_value = Some(value);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = Map::new();
                for attr in e.attributes() {
                    if let Ok(a) = attr {
                        let key = format!("@{}", String::from_utf8_lossy(a.key.as_ref()));
                        let val = a
                            .decode_and_unescape_value(reader.decoder())
                            .unwrap_or_default();
                        attrs.insert(key, Value::String(val.to_string()));
                    }
                }
                let value = if attrs.is_empty() {
                    Value::Object(Map::new())
                } else {
                    Value::Object(attrs)
                };
                if let Some((_, parent)) = stack.last_mut() {
                    if let Some(existing) = parent.get_mut(&name) {
                        if let Some(arr) = existing.as_array_mut() {
                            arr.push(value);
                        } else {
                            let prev = parent.remove(&name).unwrap();
                            parent.insert(name, Value::Array(vec![prev, value]));
                        }
                    } else {
                        parent.insert(name, value);
                    }
                } else {
                    root_name = Some(name);
                    root_value = Some(value);
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                // Preserve all text including whitespace (needed for round-trip of mixed content)
                if let Some((_, elem)) = stack.last_mut() {
                    let val = parse_text_value(&text, true);
                    if elem.contains_key("#cdata") {
                        elem.insert("#text".to_string(), val);
                    } else if elem.contains_key("#text") {
                        if let Some(prev) = elem.get_mut("#text") {
                            if let (Some(a), Some(b)) = (prev.as_str(), val.as_str()) {
                                *prev = Value::String(format!("{}{}", a, b));
                            }
                        }
                    } else {
                        elem.insert("#text".to_string(), val);
                    }
                }
            }
            Ok(Event::CData(e)) => {
                // CDATA content is already raw (unescaped) - convert bytes to string
                let content = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some((_, elem)) = stack.last_mut() {
                    if let Some(existing) = elem.get_mut("#cdata") {
                        if let Some(s) = existing.as_str() {
                            *existing = Value::String(format!("{}{}", s, content));
                        }
                    } else {
                        elem.insert("#cdata".to_string(), Value::String(content));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        buf.clear();
    }

    if let (Some(name), Some(value)) = (root_name, root_value) {
        let mut root = Map::new();
        root.insert(name, value);
        Ok(Value::Object(root))
    } else {
        Ok(Value::Object(Map::new()))
    }
}
