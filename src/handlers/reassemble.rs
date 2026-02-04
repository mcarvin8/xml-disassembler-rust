//! Reassemble XML from disassembled directory.

use crate::builders::{build_xml_string, merge_xml_elements, reorder_root_keys};
use crate::parsers::parse_to_xml_object;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tokio::fs;

pub struct ReassembleXmlFileHandler;

impl ReassembleXmlFileHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn reassemble(
        &self,
        file_path: &str,
        file_extension: Option<&str>,
        post_purge: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.validate_directory(file_path).await? {
            return Ok(());
        }

        log::debug!("Parsing directory to reassemble: {}", file_path);
        let parsed_objects = self.process_files_in_directory(file_path.to_string()).await?;

        if parsed_objects.is_empty() {
            log::error!(
                "No files under {} were parsed successfully. A reassembled XML file was not created.",
                file_path
            );
            return Ok(());
        }

        let mut merged = match merge_xml_elements(&parsed_objects) {
            Some(m) => m,
            None => return Ok(()),
        };

        // Apply stored key order so reassembled XML matches original document order.
        let key_order_path = Path::new(file_path).join(".key_order.json");
        if key_order_path.exists() {
            if let Ok(bytes) = fs::read(&key_order_path).await {
                if let Ok(key_order) = serde_json::from_slice::<Vec<String>>(&bytes) {
                    if let Some(reordered) = reorder_root_keys(&merged, &key_order) {
                        merged = reordered;
                    }
                }
            }
        }

        let final_xml = build_xml_string(&merged);
        let output_path = self.get_output_path(file_path, file_extension);

        fs::write(&output_path, final_xml).await?;

        if post_purge {
            fs::remove_dir_all(file_path).await.ok();
        }

        Ok(())
    }

    fn process_files_in_directory<'a>(
        &'a self,
        dir_path: String,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<crate::types::XmlElement>, Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            let mut parsed = Vec::new();
            let mut entries = Vec::new();
            let mut read_dir = fs::read_dir(&dir_path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                entries.push(entry);
            }
            entries.sort_by(|a, b| {
                let a_base: String = a.file_name().to_str().unwrap_or("").split('.').next().unwrap_or("").to_string();
                let b_base: String = b.file_name().to_str().unwrap_or("").split('.').next().unwrap_or("").to_string();
                a_base.cmp(&b_base)
            });

            for entry in entries {
                let path = entry.path();
                let file_path = path.to_string_lossy().to_string();

                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && self.is_parsable_file(name) {
                        if let Some(parsed_obj) = parse_to_xml_object(&file_path).await {
                            parsed.push(parsed_obj);
                        }
                    }
                } else if path.is_dir() {
                    let sub_parsed = self.process_files_in_directory(file_path).await?;
                    parsed.extend(sub_parsed);
                }
            }

            Ok(parsed)
        })
    }

    fn is_parsable_file(&self, file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.ends_with(".xml")
            || lower.ends_with(".json")
            || lower.ends_with(".json5")
            || lower.ends_with(".yaml")
            || lower.ends_with(".yml")
            || lower.ends_with(".toml")
            || lower.ends_with(".ini")
    }

    async fn validate_directory(&self, path: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let meta = fs::metadata(path).await?;
        if !meta.is_dir() {
            log::error!(
                "The provided path to reassemble is not a directory: {}",
                path
            );
            return Ok(false);
        }
        Ok(true)
    }

    fn get_output_path(&self, dir_path: &str, extension: Option<&str>) -> String {
        let path = Path::new(dir_path);
        let parent = path.parent().unwrap_or(Path::new("."));
        let base_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("output");
        let ext = extension.unwrap_or("xml");
        parent.join(format!("{}.{}", base_name, ext)).to_string_lossy().to_string()
    }
}

impl Default for ReassembleXmlFileHandler {
    fn default() -> Self {
        Self::new()
    }
}
