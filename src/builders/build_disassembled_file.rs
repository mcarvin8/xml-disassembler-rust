//! Build a single disassembled file.

use crate::builders::build_xml_string;
use crate::parsers::parse_unique_id_element;
use crate::transformers::transform_format;
use crate::types::BuildDisassembledFileOptions;
use serde_json::{Map, Value};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn build_disassembled_file(
    options: BuildDisassembledFileOptions<'_>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let BuildDisassembledFileOptions {
        content,
        disassembled_path,
        output_file_name,
        subdirectory,
        wrap_key,
        is_grouped_array,
        root_element_name,
        root_attributes,
        xml_declaration,
        format,
        unique_id_elements,
    } = options;

    let target_directory = if let Some(subdir) = subdirectory {
        Path::new(disassembled_path).join(subdir)
    } else {
        Path::new(disassembled_path).to_path_buf()
    };

    let file_name = if let Some(name) = output_file_name {
        name.to_string()
    } else if let Some(wk) = wrap_key {
        if !is_grouped_array && content.is_object() {
            let id = parse_unique_id_element(&content, unique_id_elements);
            format!("{}.{}-meta.{}", id, wk, format)
        } else {
            "output".to_string()
        }
    } else {
        "output".to_string()
    };

    let output_path = target_directory.join(&file_name);

    fs::create_dir_all(&target_directory).await?;

    let root_attrs_obj = root_attributes.as_object().cloned().unwrap_or_default();
    let mut inner = root_attrs_obj.clone();

    if let Some(wk) = wrap_key {
        inner.insert(wk.to_string(), content.clone());
    } else if let Some(obj) = content.as_object() {
        for (k, v) in obj {
            inner.insert(k.clone(), v.clone());
        }
    }

    let mut wrapped_xml: Value = Value::Object({
        let mut m = Map::new();
        m.insert(root_element_name.to_string(), Value::Object(inner));
        m
    });

    if let Some(decl) = xml_declaration {
        if decl.is_object() {
            let mut root = Map::new();
            root.insert("?xml".to_string(), decl.clone());
            if let Some(obj) = wrapped_xml.as_object() {
                for (k, v) in obj {
                    root.insert(k.clone(), v.clone());
                }
            }
            wrapped_xml = Value::Object(root);
        }
    }

    let output_string = if let Some(s) = transform_format(format, &wrapped_xml).await {
        s
    } else {
        build_xml_string(&wrapped_xml)
    };

    let mut file = fs::File::create(&output_path).await?;
    file.write_all(output_string.as_bytes()).await?;
    log::debug!("Created disassembled file: {}", output_path.display());

    Ok(())
}
