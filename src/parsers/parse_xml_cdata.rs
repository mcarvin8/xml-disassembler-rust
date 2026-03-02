//! XML parser that preserves CDATA sections.
//! Uses quick-xml directly to distinguish CDATA from regular text.
//!
//! In quick-xml 0.38+, entity references (&quot;, &#34;, etc.) are emitted as
//! Event::GeneralRef. We accumulate raw text + entity refs, then unescape once
//! to preserve whitespace between entities (quick-xml can drop spaces when
//! emitting Text/GeneralRef separately).

use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Number, Value};

/// Append raw entity reference to buffer (e.g. "quot" -> "&quot;").
fn append_entity_to_raw(ref_: &quick_xml::events::BytesRef<'_>, raw: &mut String) {
    let name = String::from_utf8_lossy(ref_.as_ref());
    raw.push('&');
    raw.push_str(&name);
    raw.push(';');
}

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

/// Flush accumulated raw text buffer: unescape entities and add to current element.
fn flush_text_buffer(
    raw: &mut String,
    stack: &mut [(String, Map<String, Value>)],
    is_after_comment: bool,
) {
    if raw.is_empty() {
        return;
    }
    let text = unescape(raw.as_str()).unwrap_or_default().into_owned();
    raw.clear();
    if text.is_empty() {
        return;
    }
    let val_raw = Value::String(text.clone());
    let val_parsed = parse_text_value(&text, true);
    if let Some((_, elem)) = stack.last_mut() {
        if is_after_comment {
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
            elem.insert("#text".to_string(), val_raw);
        }
    }
}

/// Parse XML string to JSON Value, preserving CDATA as #cdata key.
/// Produces the same structure as quickxml_to_serde but with #cdata for CDATA content.
pub fn parse_xml_with_cdata(xml: &str) -> Result<Value, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<(String, Map<String, Value>)> = Vec::new();
    let mut root_name: Option<String> = None;
    let mut root_value: Option<Value> = None;
    let mut text_buffer = String::new();
    let mut text_buffer_after_comment = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                flush_text_buffer(&mut text_buffer, &mut stack, text_buffer_after_comment);
                text_buffer_after_comment = false;
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
                flush_text_buffer(&mut text_buffer, &mut stack, text_buffer_after_comment);
                text_buffer_after_comment = false;
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
                flush_text_buffer(&mut text_buffer, &mut stack, text_buffer_after_comment);
                text_buffer_after_comment = false;
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
                let text = e.decode().unwrap_or_default();
                if let Some((_, elem)) = stack.last() {
                    text_buffer_after_comment = elem.contains_key("#comment");
                }
                text_buffer.push_str(&text);
            }
            Ok(Event::Comment(e)) => {
                flush_text_buffer(&mut text_buffer, &mut stack, text_buffer_after_comment);
                text_buffer_after_comment = false;
                let content = e.decode().unwrap_or_default().to_string();
                if let Some((_, elem)) = stack.last_mut() {
                    elem.insert("#comment".to_string(), Value::String(content));
                }
            }
            Ok(Event::GeneralRef(ref_)) => {
                append_entity_to_raw(&ref_, &mut text_buffer);
            }
            Ok(Event::CData(e)) => {
                flush_text_buffer(&mut text_buffer, &mut stack, text_buffer_after_comment);
                text_buffer_after_comment = false;
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
        assert_eq!(parse_text_value("09", true).as_str(), Some("09")); // u64 parses but we keep as string (fall-through)
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

    #[test]
    fn parse_xml_with_cdata_duplicate_empty_siblings_become_array() {
        // Two empty elements with same name: second triggers remove+insert Array (Event::End path)
        let xml = r#"<r><a/><a/></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        let arr = r.get("a").and_then(|a| a.as_array()).unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn parse_xml_with_cdata_text_tail_appended_after_second_comment() {
        // Comment then text (#text-tail), then comment then text -> append to #text-tail
        let xml = r#"<r><!--c1-->t1<!--c2-->t2</r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        assert_eq!(r.get("#text-tail").and_then(|t| t.as_str()), Some("t1t2"));
    }

    #[test]
    fn parse_xml_with_cdata_empty_document_returns_empty_object() {
        // Eof with no root (e.g. empty or only whitespace) -> empty object
        let xml = r#""#;
        let v = parse_xml_with_cdata(xml).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn parse_xml_with_cdata_unescapes_entities_in_text() {
        // quick-xml 0.38+ emits entities as Event::GeneralRef; we resolve and append.
        let xml = r#"<r><expr>IF(x, &quot;created&quot;, &quot;updated&quot;)</expr></r>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let r = v.get("r").and_then(|r| r.as_object()).unwrap();
        let expr = r.get("expr").and_then(|e| e.as_object()).unwrap();
        let text = expr.get("#text").and_then(|t| t.as_str()).unwrap();
        // Must have actual quote chars so round-trip produces &quot; in output
        assert!(text.contains(r#""created""#) && text.contains(r#""updated""#));
    }

    #[test]
    fn parse_xml_with_cdata_preserves_space_after_comma_in_entities() {
        // Fixture format: comma space before second entity - must preserve for round-trip
        let xml = r#"<e>IF(a, &quot;x&quot;, &quot;y&quot;)</e>"#;
        let v = parse_xml_with_cdata(xml).unwrap();
        let e = v.get("e").and_then(|e| e.as_object()).unwrap();
        let text = e.get("#text").and_then(|t| t.as_str()).unwrap();
        assert_eq!(
            text, r#"IF(a, "x", "y")"#,
            "space after comma must be preserved"
        );
    }
}
