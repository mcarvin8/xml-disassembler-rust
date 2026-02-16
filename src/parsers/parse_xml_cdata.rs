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
                for a in e.attributes().flatten() {
                    let key = format!("@{}", String::from_utf8_lossy(a.key.as_ref()));
                    let val = a
                        .decode_and_unescape_value(reader.decoder())
                        .unwrap_or_default();
                    attrs.insert(key, Value::String(val.to_string()));
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
                for a in e.attributes().flatten() {
                    let key = format!("@{}", String::from_utf8_lossy(a.key.as_ref()));
                    let val = a
                        .decode_and_unescape_value(reader.decoder())
                        .unwrap_or_default();
                    attrs.insert(key, Value::String(val.to_string()));
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
                    let val_raw = Value::String(text.clone());
                    let val_parsed = parse_text_value(&text, true);
                    if elem.contains_key("#comment") {
                        // Text after comment goes to #text-tail - preserve raw for round-trip
                        if let Some(prev) = elem.get_mut("#text-tail") {
                            if let (Some(a), Some(b)) = (prev.as_str(), val_raw.as_str()) {
                                *prev = Value::String(format!("{}{}", a, b));
                            }
                        } else {
                            elem.insert("#text-tail".to_string(), val_raw);
                        }
                    } else if elem.contains_key("#cdata") {
                        elem.insert("#text".to_string(), val_raw);
                    } else if elem.contains_key("#text") {
                        if let Some(prev) = elem.get_mut("#text") {
                            if let (Some(a), Some(b)) = (prev.as_str(), val_parsed.as_str()) {
                                *prev = Value::String(format!("{}{}", a, b));
                            }
                        }
                    } else {
                        // First #text: use raw to preserve whitespace before comment/CDATA
                        elem.insert("#text".to_string(), val_raw);
                    }
                }
            }
            Ok(Event::Comment(e)) => {
                let content = e.unescape().unwrap_or_default().to_string();
                if let Some((_, elem)) = stack.last_mut() {
                    elem.insert("#comment".to_string(), Value::String(content));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_xml_with_cdata_simple_element() {
        let xml = r#"<root><a>hello</a></root>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let root = v.get("root").and_then(|r| r.as_object()).unwrap();
        let a = root.get("a").and_then(|a| a.as_object()).unwrap();
        assert_eq!(a.get("#text").and_then(|t| t.as_str()), Some("hello"));
    }

    #[test]
    fn parse_xml_with_cdata_preserves_cdata() {
        let xml = r#"<root><x><![CDATA[<escaped>]]></x></root>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let root = v.get("root").and_then(|r| r.as_object()).unwrap();
        let x = root.get("x").and_then(|x| x.as_object()).unwrap();
        assert_eq!(x.get("#cdata").and_then(|c| c.as_str()), Some("<escaped>"));
    }

    #[test]
    fn parse_xml_with_cdata_empty_element() {
        let xml = r#"<root><empty/></root>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let root = v.get("root").and_then(|r| r.as_object()).unwrap();
        assert!(root.get("empty").is_some());
    }

    #[test]
    fn parse_xml_with_cdata_comment() {
        let xml = r#"<root><!-- comment --><a>1</a></root>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let root = v.get("root").and_then(|r| r.as_object()).unwrap();
        assert!(root.get("#comment").or(root.get("a")).is_some());
    }

    #[test]
    fn parse_xml_with_cdata_attributes() {
        let xml = r#"<root id="x"><a>1</a></root>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let root = v.get("root").and_then(|r| r.as_object()).unwrap();
        assert_eq!(root.get("@id").and_then(|v| v.as_str()), Some("x"));
    }

    #[test]
    fn parse_xml_with_cdata_multiple_children() {
        let xml = r#"<r><n>42</n><b>true</b></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        assert!(r.get("n").is_some());
        assert!(r.get("b").is_some());
    }

    #[test]
    fn parse_xml_with_cdata_text_tail_after_comment() {
        let xml = r#"<r><!-- comment -->tail</r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        assert_eq!(
            r.get("#comment").and_then(|c| c.as_str()),
            Some(" comment ")
        );
        assert_eq!(r.get("#text-tail").and_then(|t| t.as_str()), Some("tail"));
    }

    #[test]
    fn parse_xml_with_cdata_empty_root_returns_empty_object() {
        let xml = r#"<root></root>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let root = v.get("root").and_then(|r| r.as_object()).unwrap();
        assert!(root.is_empty());
    }

    #[test]
    fn parse_xml_with_cdata_mixed_content_appends_text() {
        // Two text nodes in same element (e.g. <a>hello</a><b/> then text "world" in same parent)
        let xml = r#"<r><a>hello<x/>world</a></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        let a = r.get("a").and_then(|a| a.as_object()).unwrap();
        // First text "hello", then after <x/>, text "world" appends to #text
        assert!(a.get("#text").is_some());
        let t = a.get("#text").and_then(|t| t.as_str()).unwrap();
        assert!(t.contains("hello") && t.contains("world"));
    }

    #[test]
    fn parse_xml_with_cdata_appends_multiple_cdata_sections() {
        let xml = r#"<r><x><![CDATA[a]]><![CDATA[b]]></x></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let x = v
            .get("r")
            .and_then(|r| r.get("x"))
            .and_then(|x| x.as_object())
            .unwrap();
        assert_eq!(x.get("#cdata").and_then(|c| c.as_str()), Some("ab"));
    }

    #[test]
    fn parse_xml_with_cdata_invalid_returns_err() {
        let result = parse_xml_with_cdata("<<");
        assert!(result.is_err());
    }

    #[test]
    fn parse_xml_with_cdata_duplicate_sibling_elements_become_array() {
        let xml = r#"<r><item>a</item><item>b</item></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        let items = r.get("item").and_then(|i| i.as_array()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].get("#text").and_then(|t| t.as_str()), Some("a"));
        assert_eq!(items[1].get("#text").and_then(|t| t.as_str()), Some("b"));
    }

    #[test]
    fn parse_xml_with_cdata_empty_element_with_attributes() {
        let xml = r#"<r><empty id="x"/></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let empty = v
            .get("r")
            .and_then(|r| r.get("empty"))
            .and_then(|e| e.as_object())
            .unwrap();
        assert_eq!(empty.get("@id").and_then(|v| v.as_str()), Some("x"));
    }

    #[test]
    fn parse_text_value_number_bool_and_leading_zero() {
        assert!(parse_text_value("", true).as_str().map(|s| s.is_empty()) == Some(true));
        assert!(parse_text_value("42", false).as_i64() == Some(42));
        assert!(parse_text_value("42", true).as_i64() == Some(42));
        assert_eq!(parse_text_value("0", true).as_str(), Some("0")); // leading_zero_as_string keeps "0" as string
        assert!(parse_text_value("0", false).as_i64() == Some(0));
        assert_eq!(parse_text_value("01", true).as_str(), Some("01"));
        assert!(
            parse_text_value("2.5", true)
                .as_f64()
                .map(|f| (f - 2.5).abs() < 1e-9)
                == Some(true)
        );
        assert!(
            parse_text_value("0.5", false)
                .as_f64()
                .map(|f| (f - 0.5).abs() < 1e-9)
                == Some(true)
        );
        assert_eq!(parse_text_value("0.5", true).as_str(), Some("0.5")); // leading zero kept as string
        assert!(parse_text_value("true", true).as_bool() == Some(true));
        assert!(parse_text_value("false", true).as_bool() == Some(false));
        assert_eq!(parse_text_value("hello", true).as_str(), Some("hello"));
    }
}
