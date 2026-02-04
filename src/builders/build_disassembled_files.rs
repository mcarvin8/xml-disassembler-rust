//! Build disassembled files from source XML file.

use crate::builders::{build_disassembled_file, extract_root_attributes};
use crate::parsers::{
    extract_xml_declaration_from_raw, extract_xmlns_from_raw, parse_element_unified,
};
use crate::types::{BuildDisassembledFilesOptions, XmlElementArrayMap, XmlElementParams};
use serde_json::{Map, Value};
use tokio::fs;

const BATCH_SIZE: usize = 20;

fn get_root_info(parsed_xml: &Value) -> Option<(String, Value, Option<Value>)> {
    let obj = parsed_xml.as_object()?;
    let xml_declaration = obj.get("?xml").cloned();
    let root_element_name = obj.keys().find(|k| *k != "?xml")?.clone();
    let root_element = obj.get(&root_element_name)?.clone();
    Some((root_element_name, root_element, xml_declaration))
}

fn order_xml_element_keys(content: &Map<String, Value>, key_order: &[String]) -> Value {
    let mut ordered = Map::new();
    for key in key_order {
        if let Some(v) = content.get(key) {
            ordered.insert(key.clone(), v.clone());
        }
    }
    Value::Object(ordered)
}

#[allow(clippy::too_many_arguments)]
async fn disassemble_element_keys(
    root_element: &Value,
    key_order: &[String],
    disassembled_path: &str,
    root_element_name: &str,
    root_attributes: &Value,
    xml_declaration: Option<&Value>,
    unique_id_elements: Option<&str>,
    strategy: &str,
    format: &str,
) -> (Map<String, Value>, XmlElementArrayMap, usize, bool) {
    let mut leaf_content = Map::new();
    let mut nested_groups = XmlElementArrayMap::new();
    let mut leaf_count = 0usize;
    let mut has_nested_elements = false;

    let empty_map = Map::new();
    let root_obj = root_element.as_object().unwrap_or(&empty_map);

    for key in key_order {
        let elements = if let Some(val) = root_obj.get(key) {
            if val.is_array() {
                val.as_array().unwrap().clone()
            } else {
                vec![val.clone()]
            }
        } else {
            continue;
        };

        for chunk in elements.chunks(BATCH_SIZE) {
            for element in chunk {
                let result = parse_element_unified(XmlElementParams {
                    element: element.clone(),
                    disassembled_path,
                    unique_id_elements,
                    root_element_name,
                    root_attributes: root_attributes.clone(),
                    key,
                    leaf_content: Value::Object(Map::new()),
                    leaf_count,
                    has_nested_elements,
                    format,
                    xml_declaration: xml_declaration.cloned(),
                    strategy,
                })
                .await;

                if let Some(obj) = result.leaf_content.as_object() {
                    if let Some(arr) = obj.get(key) {
                        if let Some(existing) = leaf_content.get_mut(key) {
                            if let Some(existing_arr) = existing.as_array_mut() {
                                if let Some(new_arr) = arr.as_array() {
                                    existing_arr.extend(new_arr.iter().cloned());
                                }
                            }
                        } else {
                            leaf_content.insert(key.clone(), arr.clone());
                        }
                    }
                }

                if strategy == "grouped-by-tag" {
                    if let Some(groups) = result.nested_groups {
                        for (tag, arr) in groups {
                            nested_groups.entry(tag).or_default().extend(arr);
                        }
                    }
                }

                leaf_count = result.leaf_count;
                has_nested_elements = result.has_nested_elements;
            }
        }
    }

    (leaf_content, nested_groups, leaf_count, has_nested_elements)
}

async fn write_nested_groups(
    nested_groups: &XmlElementArrayMap,
    strategy: &str,
    options: &WriteNestedOptions<'_>,
) {
    if strategy != "grouped-by-tag" {
        return;
    }
    for (tag, arr) in nested_groups {
        let _ = build_disassembled_file(crate::types::BuildDisassembledFileOptions {
            content: Value::Array(arr.clone()),
            disassembled_path: options.disassembled_path,
            output_file_name: Some(&format!("{}.{}", tag, options.format)),
            subdirectory: None,
            wrap_key: Some(tag),
            is_grouped_array: true,
            root_element_name: options.root_element_name,
            root_attributes: options.root_attributes.clone(),
            format: options.format,
            xml_declaration: options.xml_declaration.clone(),
            unique_id_elements: None,
        })
        .await;
    }
}

struct WriteNestedOptions<'a> {
    disassembled_path: &'a str,
    root_element_name: &'a str,
    root_attributes: Value,
    xml_declaration: Option<Value>,
    format: &'a str,
}

pub async fn build_disassembled_files_unified(
    options: BuildDisassembledFilesOptions<'_>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let BuildDisassembledFilesOptions {
        file_path,
        disassembled_path,
        base_name,
        post_purge,
        format,
        unique_id_elements,
        strategy,
    } = options;

    let xml_content = match fs::read_to_string(file_path).await {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let parsed_xml = match crate::parsers::parse_xml_from_str(&xml_content, file_path) {
        Some(p) => p,
        None => return Ok(()),
    };

    let (root_element_name, root_element, xml_declaration_from_parse) =
        match get_root_info(&parsed_xml) {
            Some(info) => info,
            None => return Ok(()),
        };
    // quickxml_to_serde drops the declaration - extract from raw XML if missing
    let xml_declaration =
        xml_declaration_from_parse.or_else(|| extract_xml_declaration_from_raw(&xml_content));

    let mut root_attributes = extract_root_attributes(&root_element);
    // quickxml_to_serde drops xmlns - extract from raw XML and add if missing
    if root_attributes.get("@xmlns").is_none() {
        if let Some(xmlns) = extract_xmlns_from_raw(&xml_content) {
            if let Some(obj) = root_attributes.as_object_mut() {
                obj.insert("@xmlns".to_string(), Value::String(xmlns));
            }
        }
    }
    let key_order: Vec<String> = root_element
        .as_object()
        .map(|o| o.keys().filter(|k| !k.starts_with('@')).cloned().collect())
        .unwrap_or_default();

    let (leaf_content, nested_groups, leaf_count, has_nested_elements) = disassemble_element_keys(
        &root_element,
        &key_order,
        disassembled_path,
        &root_element_name,
        &root_attributes,
        xml_declaration.as_ref(),
        unique_id_elements,
        strategy,
        format,
    )
    .await;

    if !has_nested_elements && leaf_count > 0 {
        log::error!(
            "The XML file {} only has leaf elements. This file will not be disassembled.",
            file_path
        );
        return Ok(());
    }

    let write_opts = WriteNestedOptions {
        disassembled_path,
        root_element_name: &root_element_name,
        root_attributes: root_attributes.clone(),
        xml_declaration: xml_declaration.clone(),
        format,
    };
    write_nested_groups(&nested_groups, strategy, &write_opts).await;

    // Persist root key order so reassembly can match original document order.
    let key_order_path = std::path::Path::new(disassembled_path).join(".key_order.json");
    if let Ok(json) = serde_json::to_string(&key_order) {
        let _ = fs::write(key_order_path, json).await;
    }

    if leaf_count > 0 {
        let final_leaf_content = if strategy == "grouped-by-tag" {
            order_xml_element_keys(&leaf_content, &key_order)
        } else {
            Value::Object(leaf_content.clone())
        };

        let _ = build_disassembled_file(crate::types::BuildDisassembledFileOptions {
            content: final_leaf_content,
            disassembled_path,
            output_file_name: Some(&format!("{}.{}", base_name, format)),
            subdirectory: None,
            wrap_key: None,
            is_grouped_array: false,
            root_element_name: &root_element_name,
            root_attributes: root_attributes.clone(),
            format,
            xml_declaration: xml_declaration.clone(),
            unique_id_elements: None,
        })
        .await;
    }

    if post_purge {
        let _ = fs::remove_file(file_path).await;
    }

    Ok(())
}
