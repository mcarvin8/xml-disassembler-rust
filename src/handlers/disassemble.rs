//! Disassemble XML file handler.

use crate::builders::build_disassembled_files_unified;
use crate::types::BuildDisassembledFilesOptions;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;
use tokio::fs;

pub struct DisassembleXmlFileHandler {
    ign: Option<ignore::gitignore::Gitignore>,
}

impl DisassembleXmlFileHandler {
    pub fn new() -> Self {
        Self { ign: None }
    }

    async fn load_ignore_rules(&mut self, ignore_path: &str) {
        let path = Path::new(ignore_path);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path).await {
                let root = path.parent().unwrap_or(Path::new("."));
                let mut builder = GitignoreBuilder::new(root);
                for line in content.lines() {
                    let _ = builder.add_line(None, line);
                }
                if let Ok(gi) = builder.build() {
                    self.ign = Some(gi);
                }
            }
        }
    }

    fn posix_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    fn is_xml_file(file_path: &str) -> bool {
        file_path.to_lowercase().ends_with(".xml")
    }

    fn is_ignored(&self, path: &str) -> bool {
        self.ign
            .as_ref()
            .map(|ign| ign.matched(path, false).is_ignore())
            .unwrap_or(false)
    }

    pub async fn disassemble(
        &mut self,
        file_path: &str,
        unique_id_elements: Option<&str>,
        strategy: Option<&str>,
        pre_purge: bool,
        post_purge: bool,
        ignore_path: &str,
        format: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let strategy = strategy.unwrap_or("unique-id");
        let strategy = if ["unique-id", "grouped-by-tag"].contains(&strategy) {
            strategy
        } else {
            log::warn!("Unsupported strategy \"{}\", defaulting to \"unique-id\".", strategy);
            "unique-id"
        };

        self.load_ignore_rules(ignore_path).await;

        let path = Path::new(file_path);
        let meta = fs::metadata(path).await?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let relative_path = path
            .strip_prefix(&cwd)
            .unwrap_or(path)
            .to_string_lossy();
        let relative_path = Self::posix_path(&relative_path);

        if meta.is_file() {
            self.handle_file(file_path, &relative_path, unique_id_elements, strategy, pre_purge, post_purge, format)
                .await?;
        } else if meta.is_dir() {
            self.handle_directory(file_path, unique_id_elements, strategy, pre_purge, post_purge, format)
                .await?;
        }

        Ok(())
    }

    async fn handle_file(
        &self,
        file_path: &str,
        relative_path: &str,
        unique_id_elements: Option<&str>,
        strategy: &str,
        pre_purge: bool,
        post_purge: bool,
        format: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resolved = Path::new(file_path).canonicalize().unwrap_or_else(|_| Path::new(file_path).to_path_buf());
        let resolved_str = resolved.to_string_lossy();

        if !Self::is_xml_file(&resolved_str) {
            log::error!("The file path provided is not an XML file: {}", resolved_str);
            return Ok(());
        }

        if self.is_ignored(relative_path) {
            log::warn!("File ignored by ignore rules: {}", resolved_str);
            return Ok(());
        }

        let dir_path = resolved.parent().unwrap_or(Path::new("."));
        self.process_file(
            dir_path.to_str().unwrap_or("."),
            strategy,
            &resolved_str,
            unique_id_elements,
            pre_purge,
            post_purge,
            format,
        )
        .await
    }

    async fn handle_directory(
        &self,
        dir_path: &str,
        unique_id_elements: Option<&str>,
        strategy: &str,
        pre_purge: bool,
        post_purge: bool,
        format: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut entries = fs::read_dir(dir_path).await?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

        while let Some(entry) = entries.next_entry().await? {
            let sub_path = entry.path();
            let sub_file_path = sub_path.to_string_lossy();
            let relative_sub = sub_path
                .strip_prefix(&cwd)
                .unwrap_or(&sub_path)
                .to_string_lossy();
            let relative_sub = Self::posix_path(&relative_sub);

            if sub_path.is_file() && Self::is_xml_file(&sub_file_path) {
                if self.is_ignored(&relative_sub) {
                    log::warn!("File ignored by ignore rules: {}", sub_file_path);
                } else {
                    self.process_file(
                        dir_path,
                        strategy,
                        &sub_file_path,
                        unique_id_elements,
                        pre_purge,
                        post_purge,
                        format,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn process_file(
        &self,
        dir_path: &str,
        strategy: &str,
        file_path: &str,
        unique_id_elements: Option<&str>,
        pre_purge: bool,
        post_purge: bool,
        format: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Parsing file to disassemble: {}", file_path);

        let file_name = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let base_name = file_name.split('.').next().unwrap_or(file_name);
        let output_path = Path::new(dir_path).join(base_name);

        if pre_purge && output_path.exists() {
            fs::remove_dir_all(&output_path).await.ok();
        }

        build_disassembled_files_unified(BuildDisassembledFilesOptions {
            file_path,
            disassembled_path: output_path.to_str().unwrap_or("."),
            base_name: file_name,
            post_purge,
            format,
            unique_id_elements,
            strategy,
        })
        .await
    }
}

impl Default for DisassembleXmlFileHandler {
    fn default() -> Self {
        Self::new()
    }
}
